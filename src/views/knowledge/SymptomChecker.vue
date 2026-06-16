<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const allSymptoms = ref<any[]>([])
const selectedSymptoms = ref<string[]>([])
const species = ref('犬')
const results = ref<any[]>([])
const loading = ref(false)
const searched = ref(false)
const diseaseDetails = ref<Map<string, any>>(new Map())

onMounted(async () => {
  try {
    allSymptoms.value = await invoke('get_symptoms')
  } catch { /* silently fail, allSymptoms stays [] */ }
})

function toggleSymptom(name: string) {
  const idx = selectedSymptoms.value.indexOf(name)
  if (idx >= 0) {
    selectedSymptoms.value.splice(idx, 1)
  } else {
    selectedSymptoms.value.push(name)
  }
}

async function onDiagnose() {
  if (selectedSymptoms.value.length === 0) return
  loading.value = true
  searched.value = true
  diseaseDetails.value.clear()
  try {
    results.value = await invoke('infer_diagnosis', {
      symptoms: selectedSymptoms.value,
      species: species.value,
      age: null,
      breed: null,
    })
    // 批量获取疾病详情以显示急迫程度
    const details = await Promise.all(
      results.value.map((r: any) => invoke('get_disease_by_id', { id: r.disease_id }))
    )
    details.forEach((d: any, i: number) => {
      if (d) diseaseDetails.value.set(results.value[i].disease_id, d)
    })
  } catch (e) { console.error(e) }
  loading.value = false
}

function scoreColor(score: number) {
  if (score >= 0.7) return '#16a34a'
  if (score >= 0.4) return '#d97706'
  return '#64748b'
}
</script>

<template>
  <div class="page">
    <h1 class="page-title">症状推理</h1>
    <p class="desc">选择患宠表现的症状，系统将推理可能的鉴别诊断</p>

    <div class="input-section">
      <div class="species-select">
        <label>物种：</label>
        <select v-model="species">
          <option value="犬">犬</option>
          <option value="猫">猫</option>
        </select>
      </div>

      <h3>选择症状（点击添加）</h3>
      <div class="symptom-pool">
        <button
          v-for="s in allSymptoms"
          :key="s.id"
          class="symptom-btn"
          :class="{ selected: selectedSymptoms.includes(s.name_zh) }"
          @click="toggleSymptom(s.name_zh)"
        >
          {{ s.name_zh }}
        </button>
      </div>

      <div v-if="selectedSymptoms.length" class="selected-bar">
        <span class="label">已选症状：</span>
        <span v-for="s in selectedSymptoms" :key="s" class="selected-tag">
          {{ s }}
          <button @click="toggleSymptom(s)">x</button>
        </span>
      </div>

      <button
        class="diagnose-btn"
        :disabled="selectedSymptoms.length === 0 || loading"
        @click="onDiagnose"
      >
        {{ loading ? '推理中...' : '开始推理' }}
      </button>
    </div>

    <div v-if="searched" class="results-section">
      <h2>推理结果</h2>
      <div v-if="results.length === 0" class="no-result">未找到匹配的疾病</div>
      <div v-else class="result-list">
        <router-link
          v-for="(r, i) in results"
          :key="r.disease_id"
          :to="`/diseases/${r.disease_id}`"
          class="result-card"
        >
          <div class="result-rank">{{ i + 1 }}</div>
          <div class="result-info">
            <div class="result-name">
              {{ r.disease_name }}
              <span v-if="diseaseDetails.get(r.disease_id)?.urgency_level >= 4" class="urgency-inline">⚡紧急</span>
              <span v-if="r.input_coverage < 0.5" class="partial-badge">部分匹配</span>
            </div>
            <div class="result-matched">
              匹配症状：<span v-for="m in r.matched_symptoms" :key="m" class="match-tag">{{ m }}</span>
            </div>
            <div v-if="r.missing_key_symptoms?.length" class="result-missing">
              缺失关键症状：<span v-for="m in r.missing_key_symptoms" :key="m" class="missing-tag">{{ m }}</span>
            </div>
            <div class="result-coverage">
              症状覆盖度：{{ (r.input_coverage * 100).toFixed(0) }}%
            </div>
          </div>
          <div class="result-score" :style="{ color: scoreColor(r.match_score) }">
            {{ (r.match_score * 100).toFixed(0) }}%
          </div>
        </router-link>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page-title { font-size: 24px; font-weight: 700; margin-bottom: 4px; }
.desc { color: var(--color-text-secondary); margin-bottom: 24px; }

.input-section {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 24px;
  margin-bottom: 32px;
}

.species-select { margin-bottom: 20px; }
.species-select label { font-weight: 500; margin-right: 8px; }
.species-select select { padding: 6px 12px; border: 1px solid var(--color-border); border-radius: var(--radius); }

h3 { font-size: 15px; font-weight: 600; margin-bottom: 12px; }

.symptom-pool { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 16px; }

.symptom-btn {
  padding: 6px 14px;
  border: 1px solid var(--color-border);
  border-radius: 20px;
  background: var(--color-bg);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s;
}

.symptom-btn:hover { border-color: var(--color-primary); }
.symptom-btn.selected { background: var(--color-primary); color: white; border-color: var(--color-primary); }

.selected-bar { margin-bottom: 16px; }
.selected-bar .label { font-size: 13px; color: var(--color-text-secondary); margin-right: 8px; }
.selected-tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: #eff6ff;
  color: var(--color-primary);
  padding: 4px 10px;
  border-radius: 12px;
  font-size: 13px;
  margin-right: 6px;
}
.selected-tag button { background: none; border: none; cursor: pointer; font-size: 14px; color: var(--color-primary); }

.diagnose-btn {
  padding: 10px 24px;
  background: var(--color-primary);
  color: white;
  border: none;
  border-radius: var(--radius);
  font-size: 15px;
  font-weight: 500;
  cursor: pointer;
}
.diagnose-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.results-section h2 { font-size: 18px; font-weight: 600; margin-bottom: 16px; }

.result-list { display: flex; flex-direction: column; gap: 12px; }

.result-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 16px;
  display: flex;
  align-items: center;
  gap: 16px;
  text-decoration: none;
  color: var(--color-text);
  transition: all 0.15s;
}

.result-card:hover { border-color: var(--color-primary); box-shadow: var(--shadow); }

.result-rank {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: var(--color-primary);
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 14px;
  flex-shrink: 0;
}

.result-info { flex: 1; }
.result-name { font-weight: 600; font-size: 16px; margin-bottom: 6px; }
.result-matched, .result-missing { font-size: 13px; margin-top: 4px; }

.match-tag { background: #dcfce7; color: #16a34a; padding: 1px 6px; border-radius: 4px; margin-right: 4px; font-size: 12px; }
.missing-tag { background: #fef3c7; color: #d97706; padding: 1px 6px; border-radius: 4px; margin-right: 4px; font-size: 12px; }

.result-score { font-size: 24px; font-weight: 700; flex-shrink: 0; }
.no-result { color: var(--color-text-secondary); padding: 24px; text-align: center; }

.result-card.partial-match { border-color: #f59e0b; opacity: 0.85; }
.partial-badge {
  display: inline-block;
  background: #fef3c7;
  color: #d97706;
  font-size: 11px;
  font-weight: 500;
  padding: 2px 8px;
  border-radius: 10px;
  margin-left: 8px;
  vertical-align: middle;
}
.result-coverage { font-size: 12px; color: var(--color-text-secondary); margin-top: 4px; }
.urgency-inline {
  display: inline-block;
  background: #fee2e2;
  color: #dc2626;
  font-size: 10px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 6px;
  margin-left: 6px;
  vertical-align: middle;
}
</style>
