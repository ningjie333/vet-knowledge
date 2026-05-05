<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

const route = useRoute()
const disease = ref<any>(null)
const symptoms = ref<any[]>([])
const ddx = ref<any[]>([])
const loading = ref(true)

onMounted(async () => {
  const id = route.params.id as string
  try {
    const [d, s, dx] = await Promise.all([
      invoke('get_disease_by_id', { id }),
      invoke('get_disease_symptoms', { diseaseId: id }),
      invoke('get_disease_ddx', { diseaseId: id }),
    ])
    disease.value = d
    symptoms.value = s as any[]
    ddx.value = dx as any[]
  } catch (e) { console.error(e) }
  loading.value = false
})
</script>

<template>
  <div v-if="loading" class="loading">加载中...</div>
  <div v-else-if="disease" class="detail-page">
    <div class="header">
      <h1>{{ disease.name_zh }}</h1>
      <span class="en-name">{{ disease.name_en }}</span>
      <div class="tags">
        <span class="difficulty" :class="disease.difficulty">{{ disease.difficulty }}</span>
        <span v-for="s in (JSON.parse(disease.species || '[]'))" :key="s" class="species-tag">{{ s }}</span>
        <span v-if="disease.urgency_level >= 4" class="urgency-badge">
          ⚠️ 紧急程度 {{ disease.urgency_level }}/5
        </span>
      </div>
    </div>

    <div v-if="disease.urgency_level >= 4" class="urgency-alert">
      <strong>⚡ 高急迫性疾病</strong>
      <span v-if="disease.urgency_level === 5"> — 危及生命，需立即干预</span>
      <span v-else> — 需尽快处理，延误可能加重病情</span>
    </div>

    <section class="section">
      <h2>概述</h2>
      <p>{{ disease.overview }}</p>
    </section>

    <section class="section">
      <h2>病因</h2>
      <ul>
        <li v-for="e in (JSON.parse(disease.etiology || '[]'))" :key="e">{{ e }}</li>
      </ul>
    </section>

    <section class="section">
      <h2>病理生理</h2>
      <p>{{ disease.pathophysiology }}</p>
    </section>

    <section class="section">
      <h2>临床症状</h2>
      <div class="symptom-list">
        <div v-for="s in symptoms" :key="s[0].id" class="symptom-item" :class="{ pathognomonic: s[3] === 1 }">
          <span class="symptom-name">{{ s[0].name_zh }}</span>
          <span v-if="s[3] === 1" class="pathognomonic-tag">核心</span>
          <span class="freq" :class="s[1]">{{ s[1] }}</span>
          <span v-if="s[2]" class="stage">{{ s[2] }}</span>
        </div>
      </div>
    </section>

    <section v-if="ddx.length" class="section">
      <h2>鉴别诊断</h2>
      <div class="ddx-list">
        <router-link
          v-for="d in ddx"
          :key="d[0].id"
          :to="`/diseases/${d[0].id}`"
          class="ddx-card"
        >
          <div class="ddx-name">{{ d[0].name_zh }}</div>
          <div class="ddx-feature">{{ d[1] }}</div>
        </router-link>
      </div>
    </section>

    <section class="section">
      <h2>预后</h2>
      <p>{{ disease.prognosis }}</p>
    </section>
  </div>
  <div v-else class="not-found">未找到该疾病</div>
</template>

<style scoped>
.header { margin-bottom: 32px; }
.header h1 { font-size: 28px; font-weight: 700; }
.en-name { font-size: 14px; color: var(--color-text-secondary); }
.tags { display: flex; gap: 8px; margin-top: 12px; }

.section { margin-bottom: 28px; }
.section h2 { font-size: 18px; font-weight: 600; margin-bottom: 12px; color: var(--color-primary); }
.section p { line-height: 1.7; color: var(--color-text); }
.section ul { padding-left: 20px; }
.section li { margin-bottom: 6px; line-height: 1.6; }

.symptom-list { display: flex; flex-wrap: wrap; gap: 10px; }
.symptom-item {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 8px 14px;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
}

.freq { font-size: 11px; padding: 2px 6px; border-radius: 8px; }
.freq.common { background: #fee2e2; color: #dc2626; }
.freq.uncommon { background: #fef3c7; color: #d97706; }
.freq.rare { background: #e0e7ff; color: #4f46e5; }
.stage { font-size: 11px; color: var(--color-text-secondary); }

.ddx-list { display: flex; flex-direction: column; gap: 10px; }
.ddx-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 14px;
  text-decoration: none;
  color: var(--color-text);
}
.ddx-card:hover { border-color: var(--color-primary); }
.ddx-name { font-weight: 600; font-size: 15px; }
.ddx-feature { font-size: 13px; color: var(--color-text-secondary); margin-top: 4px; }

.difficulty { font-size: 11px; padding: 2px 8px; border-radius: 10px; }
.difficulty.basic { background: #dcfce7; color: #16a34a; }
.difficulty.intermediate { background: #fef3c7; color: #d97706; }
.difficulty.advanced { background: #fee2e2; color: #dc2626; }
.species-tag { font-size: 11px; background: #eff6ff; color: var(--color-primary); padding: 2px 8px; border-radius: 10px; }

.urgency-badge { font-size: 11px; background: #fee2e2; color: #dc2626; padding: 2px 8px; border-radius: 10px; font-weight: 600; }
.urgency-alert {
  background: linear-gradient(135deg, #fef2f2 0%, #fff7ed 100%);
  border: 1px solid #fecaca;
  border-left: 4px solid #dc2626;
  border-radius: var(--radius);
  padding: 12px 16px;
  margin-bottom: 24px;
  font-size: 14px;
  color: #991b1b;
}
.symptom-item.pathognomonic { border-color: #dc2626; background: #fef2f2; }
.pathognomonic-tag {
  font-size: 10px;
  background: #dc2626;
  color: white;
  padding: 1px 5px;
  border-radius: 6px;
  font-weight: 600;
}
</style>
