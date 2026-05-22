<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useRouter } from 'vue-router'

const router = useRouter()

function goCategory(cat: string) {
  router.push({ path: '/diseases', query: { category: cat } })
}

const stats = ref({ diseases: 0, symptoms: 0, drugs: 0, cases: 0 })
const recentDiseases = ref<any[]>([])

onMounted(async () => {
  try {
    const d = await invoke<any[]>('get_diseases', { species: null, category: null })
    const s = await invoke<any[]>('get_symptoms')
    const dr = await invoke<any[]>('get_drugs', { drugClass: null })
    const c = await invoke<any[]>('get_cases', { species: null, difficulty: null })
    stats.value = { diseases: d.length, symptoms: s.length, drugs: dr.length, cases: c.length }
    recentDiseases.value = d.slice(0, 6)
  } catch (e) { console.error(e) }
})
</script>

<template>
  <div class="home">
    <h1 class="page-title">兽医知识库</h1>
    <p class="page-subtitle">结构化兽医知识学习与诊断推理平台</p>

    <div class="stats-grid">
      <div class="stat-card">
        <div class="stat-number">{{ stats.diseases }}</div>
        <div class="stat-label">疾病条目</div>
      </div>
      <div class="stat-card">
        <div class="stat-number">{{ stats.symptoms }}</div>
        <div class="stat-label">症状体征</div>
      </div>
      <div class="stat-card">
        <div class="stat-number">{{ stats.drugs }}</div>
        <div class="stat-label">药物条目</div>
      </div>
      <div class="stat-card">
        <div class="stat-number">{{ stats.cases }}</div>
        <div class="stat-label">病例数据</div>
      </div>
    </div>

    <section class="section">
      <h2>疾病分类</h2>
      <div class="category-grid">
        <div class="category-card" @click="goCategory('呼吸系统')">
          <span class="cat-icon">🫁</span>
          <span class="cat-name">呼吸系统</span>
        </div>
        <div class="category-card" @click="goCategory('泌尿系统')">
          <span class="cat-icon">🫘</span>
          <span class="cat-name">泌尿系统</span>
        </div>
        <div class="category-card" @click="goCategory('消化系统')">
          <span class="cat-icon">🍽️</span>
          <span class="cat-name">消化系统</span>
        </div>
        <div class="category-card" @click="goCategory('心血管系统')">
          <span class="cat-icon">❤️</span>
          <span class="cat-name">心血管系统</span>
        </div>
        <div class="category-card" @click="goCategory('内分泌系统')">
          <span class="cat-icon">⚗️</span>
          <span class="cat-name">内分泌系统</span>
        </div>
        <div class="category-card" @click="goCategory('中毒与急救')">
          <span class="cat-icon">☠️</span>
          <span class="cat-name">中毒与急救</span>
        </div>
        <div class="category-card" @click="goCategory('传染病')">
          <span class="cat-icon">🦠</span>
          <span class="cat-name">传染病</span>
        </div>
        <div class="category-card" @click="goCategory('外科')">
          <span class="cat-icon">🔪</span>
          <span class="cat-name">外科</span>
        </div>
      </div>
    </section>

    <section class="section">
      <h2>最近更新</h2>
      <div class="disease-list">
        <router-link
          v-for="d in recentDiseases"
          :key="d.id"
          :to="`/diseases/${d.id}`"
          class="disease-card"
        >
          <div class="disease-name">{{ d.name_zh }}</div>
          <div class="disease-en">{{ d.name_en }}</div>
          <span class="difficulty" :class="d.difficulty">{{ d.difficulty }}</span>
        </router-link>
      </div>
    </section>
  </div>
</template>

<style scoped>
.page-title { font-size: 28px; font-weight: 700; margin-bottom: 4px; }
.page-subtitle { color: var(--color-text-secondary); margin-bottom: 32px; }

.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin-bottom: 40px;
}

.stat-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 20px;
  text-align: center;
}

.stat-number { font-size: 32px; font-weight: 700; color: var(--color-primary); }
.stat-label { font-size: 14px; color: var(--color-text-secondary); margin-top: 4px; }

.section { margin-bottom: 40px; }
.section h2 { font-size: 18px; font-weight: 600; margin-bottom: 16px; }

.category-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
}

.category-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 16px;
  text-align: center;
  text-decoration: none;
  color: var(--color-text);
  transition: all 0.15s;
}

.category-card:hover { border-color: var(--color-primary); box-shadow: var(--shadow); }
.cat-icon { font-size: 28px; display: block; margin-bottom: 8px; }
.cat-name { font-size: 14px; font-weight: 500; }

.disease-list {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
}

.disease-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 16px;
  text-decoration: none;
  color: var(--color-text);
  position: relative;
}

.disease-card:hover { border-color: var(--color-primary); }
.disease-name { font-weight: 600; font-size: 15px; }
.disease-en { font-size: 12px; color: var(--color-text-secondary); margin-top: 2px; }

.difficulty {
  position: absolute;
  top: 12px;
  right: 12px;
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
}

.difficulty.basic { background: #dcfce7; color: #16a34a; }
.difficulty.intermediate { background: #fef3c7; color: #d97706; }
.difficulty.advanced { background: #fee2e2; color: #dc2626; }
</style>
