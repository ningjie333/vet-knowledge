<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

interface Symptom {
  id: string
  name_zh: string
  name_en: string | null
  definition: string | null
  species_notes: string | null
}

interface DiseaseResult {
  disease: {
    id: string
    name_zh: string
    name_en: string | null
    category: string | null
    species: string | null
    difficulty: string | null
    urgency_level: number | null
    overview: string | null
  }
  frequency: string
  stage: string
  is_pathognomonic: number
}

const allSymptoms = ref<Symptom[]>([])
const selectedSymptom = ref<Symptom | null>(null)
const speciesFilter = ref('')
const results = ref<DiseaseResult[]>([])
const loading = ref(false)
const searched = ref(false)
const searchQuery = ref('')
const route = useRoute()

const filteredSymptoms = computed(() => {
  if (!searchQuery.value.trim()) return allSymptoms.value
  const q = searchQuery.value.toLowerCase()
  return allSymptoms.value.filter(s =>
    s.name_zh.toLowerCase().includes(q) ||
    (s.name_en && s.name_en.toLowerCase().includes(q))
  )
})

const speciesOptions = computed(() => {
  const speciesSet = new Set<string>()
  results.value.forEach(r => {
    if (r.disease.species) {
      r.disease.species.split(',').forEach(s => speciesSet.add(s.trim()))
    }
  })
  return Array.from(speciesSet).sort()
})

onMounted(async () => {
  try {
    allSymptoms.value = await invoke<Symptom[]>('get_symptoms')

    // Auto-select from query parameter (e.g., from DiseaseDetail click)
    const preSelectedId = route.query.symptomId as string
    if (preSelectedId) {
      const found = allSymptoms.value.find(s => s.id === preSelectedId)
      if (found) {
        await selectSymptom(found)
      }
    }
  } catch (e) { console.error(e) }
})

async function selectSymptom(symptom: Symptom) {
  selectedSymptom.value = symptom
  loading.value = true
  searched.value = true
  speciesFilter.value = ''
  try {
    results.value = await invoke<DiseaseResult[]>('get_diseases_by_symptom', {
      symptomId: symptom.id,
      species: null,
    })
  } catch (e) { console.error(e) }
  loading.value = false
}

async function applySpeciesFilter() {
  if (!selectedSymptom.value) return
  loading.value = true
  try {
    results.value = await invoke<DiseaseResult[]>('get_diseases_by_symptom', {
      symptomId: selectedSymptom.value.id,
      species: speciesFilter.value || null,
    })
  } catch (e) { console.error(e) }
  loading.value = false
}

function frequencyLabel(f: string) {
  const map: Record<string, string> = { common: '常见', uncommon: '少见', rare: '罕见' }
  return map[f] || f
}

function frequencyClass(f: string) {
  return `freq-${f}`
}

function urgencyBadge(level: number | null) {
  if (!level) return null
  if (level >= 4) return { text: '紧急', cls: 'urgency-high' }
  if (level >= 3) return { text: '优先', cls: 'urgency-mid' }
  return { text: '常规', cls: 'urgency-low' }
}
</script>

<template>
  <div class="page">
    <div class="header-row">
      <div>
        <h1 class="page-title">症状检索</h1>
        <p class="desc">共 {{ allSymptoms.length }} 个症状，点击查看相关疾病</p>
      </div>
    </div>

    <!-- Search bar -->
    <div class="search-bar">
      <input
        v-model="searchQuery"
        type="text"
        class="symptom-search"
        placeholder="搜索症状..."
      />
    </div>

    <div class="layout">
      <!-- Left: Symptom grid -->
      <div class="symptom-panel">
        <div v-if="loading && !searched" class="loading-state">
          <p>加载中...</p>
        </div>
        <div v-else class="symptom-grid">
          <button
            v-for="s in filteredSymptoms"
            :key="s.id"
            class="symptom-card"
            :class="{ active: selectedSymptom?.id === s.id }"
            @click="selectSymptom(s)"
          >
            <div class="card-top">
              <span class="card-name">{{ s.name_zh }}</span>
            </div>
            <span class="card-en">{{ s.name_en }}</span>
            <p v-if="s.definition" class="card-def">{{ s.definition?.slice(0, 80) }}{{ (s.definition?.length || 0) > 80 ? '...' : '' }}</p>
          </button>
        </div>
      </div>

      <!-- Right: Disease results -->
      <div class="result-panel">
        <div v-if="!searched" class="empty-state">
          <div class="empty-icon">🔍</div>
          <p>从左侧选择一个症状查看相关疾病</p>
        </div>

        <div v-else-if="loading" class="loading-state">
          <p>查询中...</p>
        </div>

        <div v-else-if="selectedSymptom" class="results-content">
          <div class="result-header">
            <h2>
              <span class="symptom-highlight">{{ selectedSymptom.name_zh }}</span>
              相关疾病
            </h2>
            <span class="result-count">{{ results.length }} 个结果</span>
          </div>

          <!-- 症状定义展示 -->
          <div v-if="selectedSymptom.definition" class="symptom-definition">
            <h3>症状定义</h3>
            <p>{{ selectedSymptom.definition }}</p>
          </div>

          <!-- 物种特异性 -->
          <div v-if="selectedSymptom.species_notes" class="symptom-definition species-notes">
            <h3>物种特异性</h3>
            <p>{{ selectedSymptom.species_notes }}</p>
          </div>

          <div v-if="speciesOptions.length > 1" class="species-filter">
            <label>物种筛选：</label>
            <select v-model="speciesFilter" @change="applySpeciesFilter">
              <option value="">全部</option>
              <option v-for="sp in speciesOptions" :key="sp" :value="sp">{{ sp }}</option>
            </select>
          </div>

          <div v-if="results.length === 0" class="no-result">
            未找到与此症状相关的疾病
          </div>

          <div v-else class="disease-list">
            <router-link
              v-for="(r, i) in results"
              :key="r.disease.id"
              :to="`/diseases/${r.disease.id}`"
              class="disease-card"
            >
              <div class="card-left">
                <div class="card-rank">{{ i + 1 }}</div>
                <div class="card-info">
                  <div class="card-name">
                    {{ r.disease.name_zh }}
                    <span class="card-en">{{ r.disease.name_en }}</span>
                    <span v-if="r.is_pathognomonic === 1" class="patho-badge" title="该症状为此疾病的核心/特征性症状">★ 核心症状</span>
                    <span v-if="urgencyBadge(r.disease.urgency_level)" :class="['urgency-badge', urgencyBadge(r.disease.urgency_level)!.cls]">
                      {{ urgencyBadge(r.disease.urgency_level)!.text }}
                    </span>
                  </div>
                  <div class="card-meta">
                    <span :class="['freq-tag', frequencyClass(r.frequency)]">
                      {{ frequencyLabel(r.frequency) }}
                    </span>
                    <span v-if="r.stage" class="stage-tag">{{ r.stage }}</span>
                    <span v-if="r.disease.category" class="cat-tag">{{ r.disease.category }}</span>
                    <span v-if="r.disease.difficulty" :class="['diff-tag', r.disease.difficulty]">{{ r.disease.difficulty }}</span>
                  </div>
                  <div v-if="r.disease.overview" class="card-overview">
                    {{ r.disease.overview.slice(0, 120) }}{{ r.disease.overview.length > 120 ? '...' : '' }}
                  </div>
                </div>
              </div>
            </router-link>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page-title { font-size: 24px; font-weight: 700; margin-bottom: 4px; }
.desc { color: var(--color-text-secondary); margin-bottom: 8px; }
.header-row { display: flex; justify-content: space-between; align-items: flex-start; }

.search-bar { margin-bottom: 20px; }
.symptom-search {
  width: 100%;
  max-width: 400px;
  padding: 10px 16px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  font-size: 14px;
  background: var(--color-surface);
  color: var(--color-text);
}
.symptom-search:focus { outline: none; border-color: var(--color-primary); }

.layout {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 24px;
  align-items: start;
}

/* Left panel */
.symptom-panel {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 16px;
}

.symptom-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 12px;
  max-height: calc(100vh - 200px);
  overflow-y: auto;
}

.symptom-card {
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 14px;
  text-align: left;
  cursor: pointer;
  transition: all 0.15s;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.symptom-card:hover { border-color: var(--color-primary); box-shadow: var(--shadow); }
.symptom-card.active { border-color: var(--color-primary); background: #eff6ff; }

.card-top { display: flex; align-items: center; gap: 8px; }
.card-name { font-weight: 600; font-size: 15px; }
.tag-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
.card-en { font-size: 12px; color: var(--color-text-secondary); }
.card-def { font-size: 12px; color: var(--color-text-secondary); line-height: 1.4; margin-top: 4px; }

.card-tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 6px; }
.tag-chip {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 8px;
  color: white;
  font-weight: 500;
}

/* Right panel */
.result-panel {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 24px;
  min-height: 400px;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 80px 0;
  color: var(--color-text-secondary);
}
.empty-icon { font-size: 48px; margin-bottom: 16px; }

.result-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 16px;
}
.result-header h2 { font-size: 18px; font-weight: 600; }
.symptom-highlight { color: var(--color-primary); }
.result-count { font-size: 13px; color: var(--color-text-secondary); }

.symptom-definition {
  background: #f8fafc;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 16px 20px;
  margin-bottom: 16px;
}
.symptom-definition.species-notes { background: #f0fdf4; }
.symptom-definition h3 {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-secondary);
  margin-bottom: 8px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.symptom-definition p {
  font-size: 14px;
  line-height: 1.6;
  color: var(--color-text);
  margin: 0;
}

.species-filter {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
}
.species-filter label { font-size: 13px; color: var(--color-text-secondary); }
.species-filter select { padding: 4px 10px; border: 1px solid var(--color-border); border-radius: var(--radius); font-size: 13px; }

.disease-list { display: flex; flex-direction: column; gap: 10px; }

.disease-card {
  display: flex;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 14px 16px;
  text-decoration: none;
  color: var(--color-text);
  transition: all 0.15s;
}
.disease-card:hover { border-color: var(--color-primary); box-shadow: var(--shadow); }

.card-left { display: flex; gap: 12px; flex: 1; }
.card-rank {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: var(--color-primary);
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 13px;
  flex-shrink: 0;
}
.card-info { flex: 1; }
.card-name { font-weight: 600; font-size: 15px; margin-bottom: 6px; }
.card-en { font-size: 12px; color: var(--color-text-secondary); margin-left: 6px; font-weight: 400; }

.card-meta { display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 6px; }
.card-meta span { font-size: 11px; padding: 1px 8px; border-radius: 10px; }

.freq-common { background: #dcfce7; color: #16a34a; }
.freq-uncommon { background: #fef3c7; color: #d97706; }
.freq-rare { background: #f1f5f9; color: #64748b; }
.stage-tag { background: #e0e7ff; color: #4f46e5; }
.cat-tag { background: #f3e8ff; color: #9333ea; }
.diff-tag.basic { background: #dcfce7; color: #16a34a; }
.diff-tag.intermediate { background: #fef3c7; color: #d97706; }
.diff-tag.advanced { background: #fee2e2; color: #dc2626; }

.patho-badge {
  display: inline-block;
  background: #fef9c3;
  color: #a16207;
  font-size: 10px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 6px;
  margin-left: 6px;
  vertical-align: middle;
}
.urgency-badge {
  display: inline-block;
  font-size: 10px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 6px;
  margin-left: 4px;
  vertical-align: middle;
}
.urgency-high { background: #fee2e2; color: #dc2626; }
.urgency-mid { background: #fef3c7; color: #d97706; }
.urgency-low { background: #f1f5f9; color: #64748b; }

.card-overview { font-size: 13px; color: var(--color-text-secondary); line-height: 1.5; }
.no-result { text-align: center; padding: 40px; color: var(--color-text-secondary); }
.loading-state { text-align: center; padding: 40px; }
</style>