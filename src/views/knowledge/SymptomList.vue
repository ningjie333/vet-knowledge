<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface Symptom {
  id: string
  name_zh: string
  name_en: string | null
  definition: string | null
}

const symptoms = ref<Symptom[]>([])
const diseaseCounts = ref<Map<string, number>>(new Map())
const loading = ref(true)

onMounted(async () => {
  try {
    symptoms.value = await invoke<Symptom[]>('get_symptoms')
    // Batch fetch disease counts for all symptoms
    const counts = await Promise.all(
      symptoms.value.map(s =>
        invoke<{ disease_id: string }[]>('get_diseases_by_symptom', {
          symptomId: s.id,
          species: null,
        }).then(r => r.length).catch(() => 0)
      )
    )
    const map = new Map<string, number>()
    symptoms.value.forEach((s, i) => map.set(s.id, counts[i]))
    diseaseCounts.value = map
  } catch (e) { console.error(e) }
  loading.value = false
})
</script>

<template>
  <div class="page">
    <div class="header-row">
      <div>
        <h1 class="page-title">症状检索</h1>
        <p class="desc">共 {{ symptoms.length }} 个症状，点击可查看相关疾病</p>
      </div>
      <router-link to="/symptom-explorer" class="explorer-link">
        🔄 症状→疾病反向查找
      </router-link>
    </div>
    <div v-if="loading" class="loading">加载中...</div>
    <div v-else class="symptom-grid">
      <router-link
        v-for="s in symptoms"
        :key="s.id"
        :to="`/symptom-explorer?symptom=${s.id}`"
        class="symptom-card"
      >
        <div class="symptom-name">{{ s.name_zh }}</div>
        <div class="symptom-en">{{ s.name_en }}</div>
        <div class="symptom-def">{{ s.definition?.slice(0, 100) }}</div>
        <div class="disease-count">
          <span class="count-badge">{{ diseaseCounts.get(s.id) || 0 }} 个相关疾病</span>
        </div>
      </router-link>
    </div>
  </div>
</template>

<style scoped>
.page-title { font-size: 24px; font-weight: 700; margin-bottom: 4px; }
.desc { color: var(--color-text-secondary); margin-bottom: 24px; }
.header-row { display: flex; justify-content: space-between; align-items: flex-start; }
.explorer-link {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  background: var(--color-primary);
  color: white;
  border-radius: var(--radius);
  text-decoration: none;
  font-size: 13px;
  font-weight: 500;
  transition: opacity 0.15s;
}
.explorer-link:hover { opacity: 0.9; }
.loading { text-align: center; padding: 40px; color: var(--color-text-secondary); }

.symptom-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 16px; }
.symptom-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 16px;
  text-decoration: none;
  color: var(--color-text);
  transition: all 0.15s;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.symptom-card:hover { border-color: var(--color-primary); box-shadow: var(--shadow); }
.symptom-name { font-weight: 600; font-size: 16px; }
.symptom-en { font-size: 12px; color: var(--color-text-secondary); }
.symptom-def { font-size: 13px; color: var(--color-text-secondary); line-height: 1.5; margin-top: 4px; }
.disease-count { margin-top: 8px; }
.count-badge {
  display: inline-block;
  font-size: 11px;
  padding: 2px 8px;
  background: #eff6ff;
  color: var(--color-primary);
  border-radius: 10px;
  font-weight: 500;
}
</style>
