<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type {
  GameCaseSummary,
  GameSnapshot,
  NewSessionResponse,
  DiagnosisCandidate,
  Disease,
  DrugInfo,
} from '@/types'

// ── 状态 ──
const cases = ref<GameCaseSummary[]>([])
const sessions = reactive(new Map<string, SessionTab>())
const activeSessionId = ref<string | null>(null)
const loadingCases = ref(false)

// 诊断提示状态（按 session 隔离）
interface HintState {
  candidates: DiagnosisCandidate[]
  diseaseDetails: Map<string, Disease>
  loading: boolean
  searched: boolean
}
const hintStates = reactive(new Map<string, HintState>())

// 给药弹窗状态
const drugInput = ref({ drugName: '', doseMgKg: 0, visible: false })
const drugList = ref<DrugInfo[]>([])

interface SessionTab {
  sessionId: string
  caseId: string
  caseTitle: string
  snapshot: GameSnapshot
  loading: boolean
}

// ── 计算属性 ──
const activeTab = computed((): SessionTab | null => {
  if (!activeSessionId.value) return null
  return sessions.get(activeSessionId.value) ?? null
})

const activeHint = computed((): HintState | null => {
  if (!activeSessionId.value) return null
  return hintStates.get(activeSessionId.value) ?? null
})

// ── 生命周期 ──
onMounted(async () => {
  await loadCases()
  await loadDrugs()
})

onUnmounted(() => {
  // 退出时关闭所有 session（best effort）
  sessions.forEach((tab) => {
    invoke('game_end_session', { sessionId: tab.sessionId }).catch(() => {})
  })
})

// ── 病历列表 ──
async function loadCases() {
  loadingCases.value = true
  try {
    cases.value = await invoke('game_list_cases') as GameCaseSummary[]
  } catch (e) {
    console.error('loadCases failed:', e)
  } finally {
    loadingCases.value = false
  }
}

// ── 药物列表（给药弹窗下拉选项）──
async function loadDrugs() {
  try {
    drugList.value = await invoke('game_list_drugs') as DrugInfo[]
  } catch (e) {
    console.error('loadDrugs failed:', e)
  }
}

// ── 会话管理 ──
async function openCase(caseSummary: GameCaseSummary) {
  try {
    const resp = await invoke('game_new_session', { caseId: caseSummary.id }) as NewSessionResponse
    const tab: SessionTab = {
      sessionId: resp.session_id,
      caseId: caseSummary.id,
      caseTitle: caseSummary.title,
      snapshot: resp.initial_snapshot,
      loading: false,
    }
    sessions.set(resp.session_id, tab)
    activeSessionId.value = resp.session_id
    // 初始化诊断提示状态
    hintStates.set(resp.session_id, {
      candidates: [],
      diseaseDetails: new Map(),
      loading: false,
      searched: false,
    })
  } catch (e) {
    console.error('openCase failed:', e)
    alert(`开病历失败: ${e}`)
  }
}

function switchTab(sessionId: string) {
  activeSessionId.value = sessionId
}

async function closeTab(sessionId: string) {
  try {
    await invoke('game_end_session', { sessionId })
  } catch {
    // 忽略关闭错误
  }
  sessions.delete(sessionId)
  hintStates.delete(sessionId)
  if (activeSessionId.value === sessionId) {
    const remaining = Array.from(sessions.keys())
    activeSessionId.value = remaining[0] ?? null
  }
}

// ── 游戏动作 ──
async function doAdvance() {
  if (!activeTab.value) return
  activeTab.value.loading = true
  try {
    const snap = await invoke('game_advance', { sessionId: activeTab.value.sessionId }) as GameSnapshot
    activeTab.value.snapshot = snap
  } catch (e) {
    console.error('advance failed:', e)
  } finally {
    activeTab.value.loading = false
  }
}

async function doExamine(testType: string) {
  if (!activeTab.value) return
  activeTab.value.loading = true
  try {
    const snap = await invoke('game_examine', {
      sessionId: activeTab.value.sessionId,
      testType,
    }) as GameSnapshot
    activeTab.value.snapshot = snap
  } catch (e) {
    console.error('examine failed:', e)
  } finally {
    activeTab.value.loading = false
  }
}

async function doAdministerDrug() {
  if (!activeTab.value) return
  if (!drugInput.value.drugName.trim()) return
  activeTab.value.loading = true
  drugInput.value.visible = false
  try {
    const snap = await invoke('game_administer_drug', {
      sessionId: activeTab.value.sessionId,
      drugName: drugInput.value.drugName,
      doseMgKg: drugInput.value.doseMgKg > 0 ? drugInput.value.doseMgKg : null,
      volumeMl: null,
    }) as GameSnapshot
    activeTab.value.snapshot = snap
    drugInput.value.drugName = ''
    drugInput.value.doseMgKg = 0
  } catch (e) {
    console.error('administer failed:', e)
  } finally {
    activeTab.value.loading = false
  }
}

async function doDiagnose(diseaseId: string) {
  if (!activeTab.value) return
  if (!diseaseId.trim()) return
  activeTab.value.loading = true
  try {
    const snap = await invoke('game_diagnose', {
      sessionId: activeTab.value.sessionId,
      diagnosis: diseaseId,
    }) as GameSnapshot
    activeTab.value.snapshot = snap
    if (snap.phase === 'won') {
      alert('🎉 诊断正确！')
    } else if (snap.phase === 'lost') {
      alert('❌ 诊断错误，游戏结束')
    }
  } catch (e) {
    console.error('diagnose failed:', e)
  } finally {
    activeTab.value.loading = false
  }
}

// ── 诊断提示（本地 infer_diagnosis）──
async function fetchDiagnosisHint() {
  if (!activeTab.value || !activeHint.value) return
  const signs = activeTab.value.snapshot.active_signs.map((s) => s.display_name)
  if (signs.length === 0) {
    alert('当前无活跃症状，无法推理')
    return
  }
  const species = activeTab.value.snapshot.case?.animal.species ?? '犬'

  activeHint.value.loading = true
  activeHint.value.searched = true
  activeHint.value.diseaseDetails.clear()
  try {
    activeHint.value.candidates = await invoke('game_diagnosis_hint', {
      symptoms: signs,
      species,
    }) as DiagnosisCandidate[]
    // 批量获取疾病详情（仅 Top 3，不暴露推理链）
    const top3 = activeHint.value.candidates.slice(0, 3)
    const details = await Promise.all(
      top3.map((c) => invoke('get_disease_by_id', { id: c.disease_id })),
    )
    details.forEach((d: any, i: number) => {
      if (d) {
        activeHint.value!.diseaseDetails.set(top3[i].disease_id, d)
      }
    })
  } catch (e) {
    console.error('hint failed:', e)
  } finally {
    activeHint.value.loading = false
  }
}

// ── 辅助函数 ──
function scoreColor(score: number): string {
  if (score >= 0.7) return '#16a34a'
  if (score >= 0.4) return '#d97706'
  return '#64748b'
}

function phaseColor(phase: string): string {
  switch (phase) {
    case 'stable': return '#16a34a'
    case 'worsening': return '#d97706'
    case 'critical': return '#dc2626'
    case 'moribund': return '#7f1d1d'
    default: return '#64748b'
  }
}
</script>

<template>
  <div class="game-page">
    <h1 class="page-title">诊断游戏</h1>

    <!-- 病历库（未开任何会话时显示） -->
    <div v-if="sessions.size === 0" class="case-library">
      <p class="desc">选择一个病历开始诊断游戏。每个病历来自 virtual-vet 仿真引擎。</p>
      <div v-if="loadingCases" class="loading">加载病历中...</div>
      <div v-else class="case-grid">
        <div
          v-for="c in cases"
          :key="c.id"
          class="case-card"
          @click="openCase(c)"
        >
          <div class="case-card-header">
            <span class="case-title">{{ c.title }}</span>
            <span class="case-difficulty">{{ c.difficulty_label }}</span>
          </div>
          <div class="case-card-body">
            <span>{{ c.species }} · {{ c.breed }}</span>
            <span>{{ c.age }} · {{ c.weight_kg }}kg</span>
          </div>
          <div class="case-card-complaint">{{ c.chief_complaint }}</div>
        </div>
      </div>
    </div>

    <!-- 选项卡栏 + 游戏面板 -->
    <div v-else class="game-session">
      <!-- 选项卡栏 -->
      <div class="tab-bar">
        <div
          v-for="tab in Array.from(sessions.values())"
          :key="tab.sessionId"
          class="tab"
          :class="{ active: tab.sessionId === activeSessionId }"
          @click="switchTab(tab.sessionId)"
        >
          <span class="tab-title">{{ tab.caseTitle }}</span>
          <span
            class="tab-close"
            @click.stop="closeTab(tab.sessionId)"
          >×</span>
        </div>
        <div class="tab tab-new" @click="sessions.clear(); activeSessionId = null">
          + 病历库
        </div>
      </div>

      <!-- 当前会话面板 -->
      <div v-if="activeTab" class="session-panel">
        <div class="session-layout">
          <!-- 左侧：体征面板 -->
          <div class="vitals-panel">
            <div class="panel-header">
              <h2>体征监控</h2>
              <span
                class="phase-badge"
                :style="{ background: phaseColor(activeTab.snapshot.medical_phase) }"
              >
                {{ activeTab.snapshot.medical_phase }}
              </span>
            </div>

            <!-- 游戏元信息 -->
            <div class="meta-info">
              <span>⏱ {{ activeTab.snapshot.time_elapsed_min.toFixed(0) }} / {{ activeTab.snapshot.time_budget_min.toFixed(0) }} min</span>
              <span v-if="activeTab.snapshot.death_timer != null && activeTab.snapshot.death_timer > 0" class="death-timer">
                ⚠ 死亡倒计时: {{ (activeTab.snapshot.death_timer ?? 0).toFixed(0) }} min
              </span>
            </div>

            <!-- 生命体征 -->
            <div class="vitals-grid">
              <div class="vital-item">
                <span class="vital-label">心率</span>
                <span class="vital-value">{{ activeTab.snapshot.vitals.hr_bpm.toFixed(0) }}</span>
                <span class="vital-unit">bpm</span>
              </div>
              <div class="vital-item">
                <span class="vital-label">MAP</span>
                <span class="vital-value">{{ activeTab.snapshot.vitals.map_mmhg.toFixed(0) }}</span>
                <span class="vital-unit">mmHg</span>
              </div>
              <div class="vital-item">
                <span class="vital-label">呼吸</span>
                <span class="vital-value">{{ activeTab.snapshot.vitals.rr_bpm.toFixed(0) }}</span>
                <span class="vital-unit">/min</span>
              </div>
              <div class="vital-item">
                <span class="vital-label">SpO2</span>
                <span class="vital-value">{{ activeTab.snapshot.vitals.spo2_pct.toFixed(0) }}</span>
                <span class="vital-unit">%</span>
              </div>
              <div class="vital-item">
                <span class="vital-label">体温</span>
                <span class="vital-value">{{ activeTab.snapshot.vitals.temp_c.toFixed(1) }}</span>
                <span class="vital-unit">°C</span>
              </div>
              <div class="vital-item">
                <span class="vital-label">pH</span>
                <span class="vital-value">{{ activeTab.snapshot.vitals.ph.toFixed(2) }}</span>
                <span class="vital-unit"></span>
              </div>
            </div>

            <!-- 活跃症状（virtual-vet display_name 原样展示） -->
            <div class="signs-section">
              <h3>活跃症状</h3>
              <div v-if="activeTab.snapshot.active_signs.length === 0" class="empty-hint">
                暂无活跃症状
              </div>
              <div v-else class="signs-list">
                <div
                  v-for="sign in activeTab.snapshot.active_signs"
                  :key="sign.sign_id"
                  class="sign-item"
                >
                  <span class="sign-name">{{ sign.display_name }}</span>
                  <span class="sign-severity">{{ sign.severity }}</span>
                </div>
              </div>
            </div>

            <!-- 报告区 -->
            <div v-if="activeTab.snapshot.new_reports.length > 0" class="reports-section">
              <h3>新报告 ({{ activeTab.snapshot.new_reports.length }})</h3>
              <div
                v-for="(r, i) in activeTab.snapshot.new_reports"
                :key="i"
                class="report-item"
              >
                <div class="report-header">
                  <span class="report-type">{{ r.test_type || '未知检查' }}</span>
                  <span v-if="r.summary" class="report-summary">{{ r.summary }}</span>
                </div>
                <div v-if="r.results && r.results.length > 0" class="report-results">
                  <div v-for="(item, j) in r.results" :key="j" class="report-result-row">
                    <span class="result-param">{{ item.param }}</span>
                    <span class="result-value">{{ item.value }} {{ item.unit }}</span>
                    <span class="result-range">正常: {{ item.normal_range }}</span>
                    <span class="result-flag" :class="`flag-${item.flag}`">{{ item.flag }}</span>
                  </div>
                </div>
                <pre v-else class="report-raw">{{ JSON.stringify(r, null, 2) }}</pre>
              </div>
            </div>

            <!-- 操作按钮 -->
            <div class="actions">
              <button @click="doAdvance" :disabled="activeTab.loading || activeTab.snapshot.phase !== 'playing'">
                ⏩ +10min
              </button>
              <button @click="doExamine('physical')" :disabled="activeTab.loading || activeTab.snapshot.phase !== 'playing'">
                🔍 体格检查
              </button>
              <button @click="doExamine('cbc')" :disabled="activeTab.loading || activeTab.snapshot.phase !== 'playing'">
                🧪 血常规
              </button>
              <button @click="doExamine('blood_chem')" :disabled="activeTab.loading || activeTab.snapshot.phase !== 'playing'">
                🧪 生化
              </button>
              <button @click="drugInput.visible = true" :disabled="activeTab.loading || activeTab.snapshot.phase !== 'playing'">
                💉 给药
              </button>
            </div>

            <!-- 游戏结束提示 -->
            <div v-if="activeTab.snapshot.phase !== 'playing'" class="game-end">
              <span v-if="activeTab.snapshot.phase === 'won'">🎉 诊断正确！</span>
              <span v-else-if="activeTab.snapshot.phase === 'lost'">❌ 游戏结束</span>
            </div>
          </div>

          <!-- 右侧：诊断提示面板 -->
          <div class="hint-panel">
            <div class="panel-header">
              <h2>📖 诊断提示</h2>
            </div>

            <div class="hint-content">
              <button @click="fetchDiagnosisHint" :disabled="!activeHint || activeHint.loading">
                {{ activeHint?.loading ? '推理中...' : '基于当前症状推理' }}
              </button>

              <div v-if="activeHint?.searched && activeHint.candidates.length === 0" class="empty-hint">
                无匹配候选疾病
              </div>

              <div v-else class="candidates-list">
                <div
                  v-for="(c, i) in (activeHint?.candidates.slice(0, 3) ?? [])"
                  :key="c.disease_id"
                  class="candidate-item clickable"
                  @click="doDiagnose(c.disease_id)"
                  :title="`点击提交诊断: ${c.disease_name}`"
                >
                  <div class="candidate-header">
                    <span class="candidate-rank">{{ i + 1 }}</span>
                    <span class="candidate-name">{{ c.disease_name }}</span>
                    <span
                      class="candidate-score"
                      :style="{ color: scoreColor(c.match_score) }"
                    >
                      {{ c.match_score.toFixed(2) }}
                    </span>
                  </div>
                  <!-- 不显示 matched_symptoms / missing_key_symptoms / suggested_tests.purpose -->
                  <div
                    v-if="hintStates.get(activeSessionId!)?.diseaseDetails.get(c.disease_id)?.urgency_level"
                    class="candidate-urgency"
                  >
                    急迫度: {{ hintStates.get(activeSessionId!)!.diseaseDetails.get(c.disease_id)!.urgency_level }}
                  </div>
                </div>
              </div>

              <div class="hint-note">
                <p>💡 提示: 仅展示 Top 3 候选。诊断考验依靠你自己判断——</p>
                <p>· 不显示已匹配/缺失症状</p>
                <p>· 不显示推荐检查目的</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 给药弹窗 -->
    <div v-if="drugInput.visible" class="modal-overlay" @click.self="drugInput.visible = false">
      <div class="modal">
        <h3>给药</h3>
        <div class="form-row">
          <label>药物名称:</label>
          <select v-model="drugInput.drugName">
            <option value="" disabled>选择药物</option>
            <option v-for="d in drugList" :key="d.drug_name" :value="d.drug_name">
              {{ d.name }} ({{ d.drug_name }}) — {{ d.description }}
            </option>
          </select>
        </div>
        <div class="form-row">
          <label>剂量 (mg/kg):</label>
          <input v-model.number="drugInput.doseMgKg" type="number" min="0" step="0.1" />
        </div>
        <div class="modal-actions">
          <button @click="drugInput.visible = false">取消</button>
          <button @click="doAdministerDrug">给药</button>
        </div>
      </div>
    </div>

  </div>
</template>

<style scoped>
.game-page { padding: 20px; }
.page-title { font-size: 24px; font-weight: 700; margin-bottom: 20px; }
.desc { color: var(--color-text-secondary); margin-bottom: 16px; }

/* 病历库 */
.case-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 16px; }
.case-card { background: var(--color-surface); border: 1px solid var(--color-border); border-radius: var(--radius); padding: 16px; cursor: pointer; transition: all 0.2s; }
.case-card:hover { border-color: var(--color-primary); box-shadow: 0 2px 8px rgba(0,0,0,0.08); }
.case-card-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
.case-title { font-weight: 600; }
.case-difficulty { color: var(--color-text-secondary); font-size: 14px; }
.case-card-body { display: flex; flex-direction: column; gap: 4px; color: var(--color-text-secondary); font-size: 14px; margin-bottom: 8px; }
.case-card-complaint { font-size: 13px; color: var(--color-text-secondary); border-top: 1px solid var(--color-border); padding-top: 8px; }

/* 选项卡栏 */
.tab-bar { display: flex; gap: 4px; border-bottom: 1px solid var(--color-border); margin-bottom: 16px; flex-wrap: wrap; }
.tab { padding: 8px 16px; cursor: pointer; border-radius: var(--radius) var(--radius) 0 0; background: var(--color-surface); border: 1px solid var(--color-border); border-bottom: none; display: flex; align-items: center; gap: 8px; }
.tab.active { background: var(--color-primary); color: white; }
.tab-new { color: var(--color-text-secondary); }
.tab-close { font-size: 18px; line-height: 1; opacity: 0.6; }
.tab-close:hover { opacity: 1; }

/* 会话面板 */
.session-layout { display: grid; grid-template-columns: 1fr 320px; gap: 16px; }
.vitals-panel, .hint-panel { background: var(--color-surface); border: 1px solid var(--color-border); border-radius: var(--radius); padding: 16px; }
.panel-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; cursor: pointer; }
.panel-header h2 { font-size: 18px; margin: 0; }
.phase-badge { color: white; padding: 2px 8px; border-radius: 4px; font-size: 12px; }

.meta-info { display: flex; gap: 16px; margin-bottom: 12px; font-size: 14px; color: var(--color-text-secondary); }
.death-timer { color: #dc2626; font-weight: 600; }

.vitals-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; margin-bottom: 16px; }
.vital-item { display: flex; flex-direction: column; align-items: center; padding: 8px; background: var(--color-bg); border-radius: var(--radius); }
.vital-label { font-size: 12px; color: var(--color-text-secondary); }
.vital-value { font-size: 24px; font-weight: 700; }
.vital-unit { font-size: 12px; color: var(--color-text-secondary); }

.signs-section, .reports-section { margin-bottom: 16px; }
.signs-section h3, .reports-section h3 { font-size: 14px; margin-bottom: 8px; }
.signs-list { display: flex; flex-direction: column; gap: 4px; }
.sign-item { display: flex; justify-content: space-between; padding: 6px 8px; background: var(--color-bg); border-radius: 4px; font-size: 14px; }
.sign-severity { color: var(--color-text-secondary); }
.report-item { padding: 10px; background: var(--color-bg); border-radius: var(--radius); margin-bottom: 8px; border-left: 3px solid var(--color-primary); }
.report-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
.report-type { font-weight: 600; color: var(--color-text, #1f2937); text-transform: uppercase; font-size: 13px; }
.report-summary { font-size: 12px; color: var(--color-text-secondary); text-align: right; flex: 1; margin-left: 12px; }
.report-results { display: flex; flex-direction: column; gap: 4px; }
.report-result-row { display: grid; grid-template-columns: 100px 100px 1fr 60px; gap: 8px; padding: 4px 6px; font-size: 13px; align-items: center; }
.report-result-row:nth-child(odd) { background: rgba(0,0,0,0.02); border-radius: 3px; }
.result-param { font-weight: 500; color: var(--color-text, #1f2937); }
.result-value { font-weight: 700; color: var(--color-text, #1f2937); }
.result-range { font-size: 11px; color: var(--color-text-secondary); }
.result-flag { font-size: 11px; padding: 1px 6px; border-radius: 3px; text-align: center; font-weight: 600; }
.flag-normal { background: #dcfce7; color: #16a34a; }
.flag-high, .flag-critical { background: #fee2e2; color: #dc2626; }
.flag-low { background: #fef3c7; color: #d97706; }
.report-raw { font-size: 11px; color: var(--color-text-secondary); white-space: pre-wrap; word-break: break-all; margin: 0; padding: 8px; background: rgba(0,0,0,0.03); border-radius: 4px; max-height: 200px; overflow-y: auto; }
.empty-hint { color: var(--color-text-secondary); font-size: 14px; padding: 8px; }

.actions { display: flex; flex-wrap: wrap; gap: 8px; padding-top: 12px; border-top: 1px solid var(--color-border); }
.actions button { padding: 6px 12px; border: 1px solid var(--color-border); background: var(--color-surface); color: var(--color-text, #1f2937); border-radius: var(--radius); cursor: pointer; }
.actions button:hover:not(:disabled) { background: var(--color-primary); color: white; }
.actions button:disabled { opacity: 0.5; cursor: not-allowed; }

.game-end { margin-top: 16px; padding: 12px; text-align: center; font-size: 18px; font-weight: 700; }

/* 诊断提示面板 */
.hint-content { display: flex; flex-direction: column; gap: 12px; }
.hint-content button { padding: 8px; background: var(--color-primary); color: white; border: none; border-radius: var(--radius); cursor: pointer; }
.hint-content button:disabled { opacity: 0.5; }
.candidates-list { display: flex; flex-direction: column; gap: 8px; }
.candidate-item { padding: 8px; background: var(--color-bg); border-radius: var(--radius); }
.candidate-item.clickable { cursor: pointer; transition: background 0.15s, color 0.15s; }
.candidate-item.clickable:hover { background: var(--color-primary); color: white; }
.candidate-item.clickable:hover .candidate-name { color: white; }
.candidate-header { display: flex; align-items: center; gap: 8px; }
.candidate-rank { background: var(--color-primary); color: white; width: 20px; height: 20px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 12px; }
.candidate-name { flex: 1; font-weight: 600; color: var(--color-text, #1f2937); }
.candidate-score { font-weight: 700; }
.candidate-urgency { font-size: 12px; color: var(--color-text-secondary); margin-top: 4px; }
.hint-note { font-size: 12px; color: var(--color-text-secondary); padding: 8px; background: var(--color-bg); border-radius: var(--radius); }
.hint-note p { margin: 2px 0; }

/* 弹窗 */
.modal-overlay { position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000; }
.modal { background: var(--color-surface); border-radius: var(--radius); padding: 24px; min-width: 320px; }
.modal h3 { margin: 0 0 16px; }
.form-row { display: flex; flex-direction: column; gap: 4px; margin-bottom: 12px; }
.form-row label { font-size: 14px; }
.form-row input { padding: 8px; border: 1px solid var(--color-border); border-radius: var(--radius); }
.modal-actions { display: flex; justify-content: flex-end; gap: 8px; }
.modal-actions button { padding: 6px 16px; border: 1px solid var(--color-border); background: var(--color-surface); border-radius: var(--radius); cursor: pointer; }
.modal-actions button:last-child { background: var(--color-primary); color: white; }
</style>
