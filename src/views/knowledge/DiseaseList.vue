<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
const diseases = ref<any[]>([])
const loading = ref(true)
const filterSpecies = ref('')
const filterCategory = ref('')
const filterDifficulty = ref('')

const systems = ['呼吸系统', '泌尿系统', '消化系统', '心血管系统', '内分泌系统', '中毒与急救', '传染病', '外科']

async function load() {
  loading.value = true
  try {
    diseases.value = await invoke('get_diseases', {
      species: filterSpecies.value || null,
      category: filterCategory.value || null,
    })
    if (filterDifficulty.value) {
      diseases.value = diseases.value.filter(d => d.difficulty === filterDifficulty.value)
    }
  } catch (e) { console.error(e) }
  loading.value = false
}

onMounted(load)
watch([filterSpecies, filterCategory, filterDifficulty], load)
</script>

<template>
  <div class="page">
    <h1 class="page-title">疾病百科</h1>

    <div class="filters">
      <select v-model="filterSpecies">
        <option value="">全部物种</option>
        <option value="犬">犬</option>
        <option value="猫">猫</option>
      </select>
      <select v-model="filterCategory">
        <option value="">全部系统</option>
        <option v-for="s in systems" :key="s" :value="s">{{ s }}</option>
      </select>
      <select v-model="filterDifficulty">
        <option value="">全部难度</option>
        <option value="basic">基础</option>
        <option value="intermediate">临床</option>
        <option value="advanced">进阶</option>
      </select>
    </div>

    <div v-if="loading" class="loading">加载中...</div>

    <div v-else class="disease-grid">
      <router-link
        v-for="d in diseases"
        :key="d.id"
        :to="`/diseases/${d.id}`"
        class="disease-card"
      >
        <div class="card-header">
          <span class="disease-name">{{ d.name_zh }}</span>
          <span class="difficulty" :class="d.difficulty">{{ d.difficulty }}</span>
        </div>
        <div class="disease-en">{{ d.name_en }}</div>
        <div class="disease-species">
          <span v-for="s in (JSON.parse(d.species || '[]'))" :key="s" class="species-tag">{{ s }}</span>
        </div>
        <div class="disease-overview">{{ (d.overview || '').slice(0, 80) }}...</div>
      </router-link>
    </div>
  </div>
</template>

<style scoped>
.page-title { font-size: 24px; font-weight: 700; margin-bottom: 20px; }

.filters {
  display: flex;
  gap: 12px;
  margin-bottom: 24px;
}

.filters select {
  padding: 8px 12px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  font-size: 14px;
  background: var(--color-surface);
}

.disease-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
}

.disease-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 16px;
  text-decoration: none;
  color: var(--color-text);
  transition: all 0.15s;
}

.disease-card:hover { border-color: var(--color-primary); box-shadow: var(--shadow); }

.card-header { display: flex; justify-content: space-between; align-items: center; }
.disease-name { font-weight: 600; font-size: 16px; }
.disease-en { font-size: 12px; color: var(--color-text-secondary); margin-top: 2px; }

.disease-species { display: flex; gap: 6px; margin: 8px 0; }
.species-tag { font-size: 11px; background: #eff6ff; color: var(--color-primary); padding: 2px 8px; border-radius: 10px; }

.disease-overview { font-size: 13px; color: var(--color-text-secondary); line-height: 1.5; }

.difficulty { font-size: 11px; padding: 2px 8px; border-radius: 10px; }
.difficulty.basic { background: #dcfce7; color: #16a34a; }
.difficulty.intermediate { background: #fef3c7; color: #d97706; }
.difficulty.advanced { background: #fee2e2; color: #dc2626; }
</style>
