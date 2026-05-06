<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface Disease {
  id: string
  name_zh: string
  name_en: string | null
  category: string | null
  species: string | null
  difficulty: string | null
  urgency_level: number | null
  overview: string | null
}

interface DiseaseCompareView {
  disease: Disease
  symptoms: Array<{ name_zh: string; frequency: string; stage: string; is_pathognomonic: number }>
  treatments: Array<{ name_zh: string; line: string; drug_class: string | null }>
  diagnostics: Array<{ name_zh: string; purpose: string; evidence_level: string }>
  ddx: Array<{ name_zh: string; distinguishing_feature: string }>
}

const allDiseases = ref<Disease[]>([])
const selectedIds = ref<string[]>([])
const compareData = ref<DiseaseCompareView[]>([])
const loading = ref(false)
const searched = ref(false)
const searchQuery = ref('')

const filteredDiseases = computed(() => {
  if (!searchQuery.value.trim()) return allDiseases.value
  const q = searchQuery.value.toLowerCase()
  return allDiseases.value.filter(d =>
    d.name_zh.toLowerCase().includes(q) ||
    (d.name_en && d.name_en.toLowerCase().includes(q))
  )
})

const columns = computed(() => compareData.value.length)
const allSymptomNames = computed(() => {
  const names = new Set<string>()
  compareData.value.forEach(d => d.symptoms.forEach(s => names.add(s.name_zh)))
  return Array.from(names).sort()
})

onMounted(async () => {
  try {
    allDiseases.value = await invoke<Disease[]>('get_diseases', { species: null, category: null })
  } catch (e) { console.error(e) }
})

function toggleDisease(id: string) {
  const idx = selectedIds.value.indexOf(id)
  if (idx >= 0) {
    selectedIds.value.splice(idx, 1)
  } else if (selectedIds.value.length < 4) {
    selectedIds.value.push(id)
  }
}

async function doCompare() {
  if (selectedIds.value.length < 2) return
  loading.value = true
  searched.value = true
  try {
    compareData.value = await invoke<DiseaseCompareView[]>('get_disease_compare', {
      diseaseIds: selectedIds.value,
    })
  } catch (e) { console.error(e) }
  loading.value = false
}

function hasSymptom(data: DiseaseCompareView, name: string) {
  return data.symptoms.find(s => s.name_zh === name)
}

function urgencyBadge(level: number | null) {
  if (!level) return null
  if (level >= 4) return { text: '紧急', cls: 'urgency-high' }
  if (level >= 3) return { text: '优先', cls: 'urgency-mid' }
  return { text: '常规', cls: 'urgency-low' }
}

function lineLabel(l: string) {
  const map: Record<string, string> = { first: '一线', second: '二线', adjunctive: '辅助' }
  return map[l] || l
}

function evidenceLabel(e: string) {
  const map: Record<string, string> = { gold_standard: '金标准', supportive: '支持性', exclusionary: '排除性' }
  return map[e] || e
}
</script>

<template>
  <div class="page">
    <h1 class="page-title">疾病对比</h1>
    <p class="desc">选择 2-4 个疾病进行并排对比，快速区分相似疾病</p>

    <!-- Disease selector -->
    <div class="selector-section">
      <div class="selector-header">
        <span>选择疾病（已选 {{ selectedIds.length }}/4）</span>
        <input v-model="searchQuery" type="text" class="selector-search" placeholder="搜索疾病..." />
      </div>
      <div class="disease-pool">
        <button
          v-for="d in filteredDiseases"
          :key="d.id"
          class="disease-chip"
          :class="{ selected: selectedIds.includes(d.id) }"
          :disabled="!selectedIds.includes(d.id) && selectedIds.length >= 4"
          @click="toggleDisease(d.id)"
        >
          {{ d.name_zh }}
        </button>
      </div>
      <button
        class="compare-btn"
        :disabled="selectedIds.length < 2 || loading"
        @click="doCompare"
      >
        {{ loading ? '加载中...' : `对比 ${selectedIds.length} 个疾病` }}
      </button>
    </div>

    <!-- Comparison table -->
    <div v-if="searched && !loading && compareData.length >= 2" class="compare-table-wrapper">
      <table class="compare-table">
        <thead>
          <tr>
            <th class="row-header">项目</th>
            <th v-for="d in compareData" :key="d.disease.id" class="disease-header">
              <router-link :to="`/diseases/${d.disease.id}`" class="header-link">
                {{ d.disease.name_zh }}
              </router-link>
              <div class="header-en">{{ d.disease.name_en }}</div>
            </th>
          </tr>
        </thead>
        <tbody>
          <!-- Basic info -->
          <tr class="section-row"><td :colspan="columns + 1">基本信息</td></tr>
          <tr>
            <td class="row-header">分类</td>
            <td v-for="d in compareData" :key="d.disease.id">
              {{ d.disease.category || '—' }}
            </td>
          </tr>
          <tr>
            <td class="row-header">物种</td>
            <td v-for="d in compareData" :key="d.disease.id">
              {{ d.disease.species || '—' }}
            </td>
          </tr>
          <tr>
            <td class="row-header">紧急程度</td>
            <td v-for="d in compareData" :key="d.disease.id">
              <span v-if="urgencyBadge(d.disease.urgency_level)" :class="['urgency-badge', urgencyBadge(d.disease.urgency_level)!.cls]">
                {{ urgencyBadge(d.disease.urgency_level)!.text }} ({{ d.disease.urgency_level }}/5)
              </span>
              <span v-else>—</span>
            </td>
          </tr>
          <tr>
            <td class="row-header">难度</td>
            <td v-for="d in compareData" :key="d.disease.id">
              <span v-if="d.disease.difficulty" :class="['diff-badge', d.disease.difficulty]">{{ d.disease.difficulty }}</span>
              <span v-else>—</span>
            </td>
          </tr>

          <!-- Symptoms comparison matrix -->
          <tr class="section-row"><td :colspan="columns + 1">症状对比</td></tr>
          <tr v-for="symName in allSymptomNames" :key="symName">
            <td class="row-header">{{ symName }}</td>
            <td v-for="d in compareData" :key="d.disease.id">
              <template v-if="hasSymptom(d, symName)">
                <span :class="['freq-dot', `freq-${hasSymptom(d, symName)!.frequency}`]">
                  {{ hasSymptom(d, symName)!.is_pathognomonic ? '★' : '●' }}
                </span>
                <span class="freq-text">
                  {{ hasSymptom(d, symName)!.frequency === 'common' ? '常见' : hasSymptom(d, symName)!.frequency === 'uncommon' ? '少见' : '罕见' }}
                </span>
                <span v-if="hasSymptom(d, symName)!.stage" class="stage-text">{{ hasSymptom(d, symName)!.stage }}</span>
              </template>
              <span v-else class="no-data">—</span>
            </td>
          </tr>

          <!-- Treatments -->
          <tr class="section-row"><td :colspan="columns + 1">治疗方案</td></tr>
          <tr>
            <td class="row-header">一线用药</td>
            <td v-for="d in compareData" :key="d.disease.id">
              <div v-if="d.treatments.filter(t => t.line === 'first').length">
                <span v-for="t in d.treatments.filter(t => t.line === 'first')" :key="t.name_zh" class="drug-tag first-line">{{ t.name_zh }}</span>
              </div>
              <span v-else class="no-data">—</span>
            </td>
          </tr>
          <tr>
            <td class="row-header">二线/辅助</td>
            <td v-for="d in compareData" :key="d.disease.id">
              <div v-if="d.treatments.filter(t => t.line !== 'first').length">
                <span v-for="t in d.treatments.filter(t => t.line !== 'first')" :key="t.name_zh" class="drug-tag other-line">{{ t.name_zh }} ({{ lineLabel(t.line) }})</span>
              </div>
              <span v-else class="no-data">—</span>
            </td>
          </tr>

          <!-- Diagnostics -->
          <tr class="section-row"><td :colspan="columns + 1">诊断检查</td></tr>
          <tr v-for="(_, idx) in Math.max(...compareData.map(d => d.diagnostics.length) || [0])" :key="idx">
            <td class="row-header">检查 {{ idx + 1 }}</td>
            <td v-for="d in compareData" :key="d.disease.id">
              <template v-if="d.diagnostics[idx]">
                <span class="test-name">{{ d.diagnostics[idx].name_zh }}</span>
                <span :class="['evidence-badge', `ev-${d.diagnostics[idx].evidence_level}`]">{{ evidenceLabel(d.diagnostics[idx].evidence_level) }}</span>
              </template>
              <span v-else class="no-data">—</span>
            </td>
          </tr>

          <!-- DDX -->
          <tr class="section-row"><td :colspan="columns + 1">鉴别诊断</td></tr>
          <tr>
            <td class="row-header">需鉴别疾病</td>
            <td v-for="d in compareData" :key="d.disease.id">
              <div v-if="d.ddx.length">
                <div v-for="x in d.ddx" :key="x.name_zh" class="ddx-item">
                  <router-link :to="`/diseases/${x.name_zh}`" class="ddx-link">{{ x.name_zh }}</router-link>
                  <span class="ddx-feature">{{ x.distinguishing_feature }}</span>
                </div>
              </div>
              <span v-else class="no-data">—</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div v-else-if="searched && !loading" class="no-result">
      请选择至少 2 个疾病进行对比
    </div>
  </div>
</template>

<style scoped>
.page-title { font-size: 24px; font-weight: 700; margin-bottom: 4px; }
.desc { color: var(--color-text-secondary); margin-bottom: 24px; }

.selector-section {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 20px;
  margin-bottom: 32px;
}

.selector-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.selector-header span { font-size: 14px; font-weight: 500; }

.selector-search {
  padding: 6px 12px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  font-size: 13px;
  width: 200px;
  background: var(--color-bg);
  color: var(--color-text);
}

.disease-pool {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 16px;
  max-height: 120px;
  overflow-y: auto;
}

.disease-chip {
  padding: 4px 12px;
  border: 1px solid var(--color-border);
  border-radius: 16px;
  background: var(--color-bg);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.1s;
}

.disease-chip:hover { border-color: var(--color-primary); }
.disease-chip.selected { background: var(--color-primary); color: white; border-color: var(--color-primary); }
.disease-chip:disabled { opacity: 0.4; cursor: not-allowed; }

.compare-btn {
  padding: 8px 24px;
  background: var(--color-primary);
  color: white;
  border: none;
  border-radius: var(--radius);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
}

.compare-btn:disabled { opacity: 0.5; cursor: not-allowed; }

/* Comparison table */
.compare-table-wrapper {
  overflow-x: auto;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
}

.compare-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.compare-table th,
.compare-table td {
  padding: 10px 14px;
  border: 1px solid var(--color-border);
  text-align: left;
  vertical-align: top;
}

.row-header {
  background: var(--color-bg);
  font-weight: 600;
  white-space: nowrap;
  min-width: 100px;
}

.disease-header {
  min-width: 180px;
  background: #f8fafc;
}

.header-link {
  color: var(--color-primary);
  font-weight: 600;
  text-decoration: none;
  font-size: 14px;
}

.header-link:hover { text-decoration: underline; }
.header-en { font-size: 11px; color: var(--color-text-secondary); margin-top: 2px; }

.section-row td {
  background: var(--color-bg);
  font-weight: 700;
  font-size: 14px;
  padding: 8px 14px;
}

/* Badges */
.urgency-badge {
  display: inline-block;
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 8px;
}
.urgency-high { background: #fee2e2; color: #dc2626; }
.urgency-mid { background: #fef3c7; color: #d97706; }
.urgency-low { background: #f1f5f9; color: #64748b; }

.diff-badge {
  display: inline-block;
  font-size: 11px;
  padding: 1px 8px;
  border-radius: 8px;
}
.diff-badge.basic { background: #dcfce7; color: #16a34a; }
.diff-badge.intermediate { background: #fef3c7; color: #d97706; }
.diff-badge.advanced { background: #fee2e2; color: #dc2626; }

/* Symptoms */
.freq-dot { font-size: 10px; margin-right: 4px; }
.freq-common { color: #16a34a; }
.freq-uncommon { color: #d97706; }
.freq-rare { color: #64748b; }
.freq-text { font-size: 12px; }
.stage-text { font-size: 10px; color: var(--color-text-secondary); margin-left: 4px; }

/* Drugs */
.drug-tag {
  display: inline-block;
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 8px;
  margin: 1px 2px;
}
.drug-tag.first-line { background: #dcfce7; color: #16a34a; }
.drug-tag.other-line { background: #e0e7ff; color: #4f46e5; }

/* Tests */
.test-name { font-weight: 500; display: block; }
.evidence-badge {
  display: inline-block;
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 6px;
  margin-top: 2px;
}
.ev-gold_standard { background: #fef9c3; color: #a16207; }
.ev-supportive { background: #e0e7ff; color: #4f46e5; }
.ev-exclusionary { background: #fee2e2; color: #dc2626; }

/* DDX */
.ddx-item { margin-bottom: 4px; }
.ddx-link { color: var(--color-primary); text-decoration: none; font-size: 12px; }
.ddx-link:hover { text-decoration: underline; }
.ddx-feature { display: block; font-size: 11px; color: var(--color-text-secondary); }

.no-data { color: var(--color-text-secondary); font-style: italic; }
.no-result { text-align: center; padding: 40px; color: var(--color-text-secondary); }
</style>
