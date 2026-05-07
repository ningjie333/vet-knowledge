<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Case } from '../../types'

const cases = ref<Case[]>([])
const loading = ref(true)
const filterSpecies = ref('')
const filterDifficulty = ref('')
const searchQuery = ref('')

async function load() {
  loading.value = true
  try {
    cases.value = await invoke<Case[]>('get_cases', {
      species: filterSpecies.value || null,
      difficulty: filterDifficulty.value || null,
    })
  } catch (e) { console.error(e) }
  loading.value = false
}

async function doSearch() {
  if (!searchQuery.value.trim()) { load(); return }
  loading.value = true
  try {
    const results = await invoke<any[]>('full_text_search', {
      query: searchQuery.value,
      limit: 20,
    })
    const caseIds = results
      .filter(r => r.entity_type === 'case')
      .map(r => r.entity_id)
    if (caseIds.length === 0) {
      cases.value = []
    } else {
      const all = await invoke<Case[]>('get_cases', { species: null, difficulty: null })
      const idSet = new Set(caseIds)
      cases.value = all.filter(c => idSet.has(c.id))
    }
  } catch (e) { console.error(e) }
  loading.value = false
}

function clearFilters() {
  filterSpecies.value = ''
  filterDifficulty.value = ''
  searchQuery.value = ''
  load()
}

function difficultyLabel(d: string | null): string {
  if (!d) return ''
  return d === 'basic' ? '基础' : d === 'intermediate' ? '临床' : '进阶'
}

function difficultyClass(d: string | null): string {
  return d || 'intermediate'
}

onMounted(load)
watch([filterSpecies, filterDifficulty], load)
</script>

<template>
  <div class="page">
    <div class="page-header">
      <div>
        <h1 class="page-title">病例库</h1>
        <p class="page-desc">共 {{ cases.length }} 个病例，覆盖常见临床场景</p>
      </div>
    </div>

    <!-- 筛选栏 -->
    <div class="filters">
      <div class="filter-group">
        <select v-model="filterSpecies">
          <option value="">全部物种</option>
          <option value="犬">犬</option>
          <option value="猫">猫</option>
        </select>
        <select v-model="filterDifficulty">
          <option value="">全部难度</option>
          <option value="basic">基础</option>
          <option value="intermediate">临床</option>
          <option value="advanced">进阶</option>
        </select>
      </div>
      <div class="search-inline">
        <input
          v-model="searchQuery"
          type="text"
          placeholder="搜索病例..."
          @keyup.enter="doSearch"
        />
        <button class="btn-search" @click="doSearch">搜索</button>
      </div>
      <button v-if="filterSpecies || filterDifficulty || searchQuery" class="btn-clear" @click="clearFilters">
        清除筛选
      </button>
    </div>

    <div v-if="loading" class="loading">加载中...</div>

    <div v-else-if="!cases.length" class="empty">
      <p>📋 暂无匹配病例</p>
    </div>

    <div v-else class="case-grid">
      <router-link
        v-for="c in cases"
        :key="c.id"
        :to="`/cases/${c.id}`"
        class="case-card"
      >
        <div class="card-top">
          <div class="card-title">{{ c.title }}</div>
          <span class="difficulty" :class="difficultyClass(c.difficulty)">{{ difficultyLabel(c.difficulty) }}</span>
        </div>
        <div class="card-meta">
          <span v-if="c.species" class="meta-tag species">{{ c.species }}</span>
          <span v-if="c.breed" class="meta-tag">{{ c.breed }}</span>
          <span v-if="c.age" class="meta-tag">{{ c.age }}岁</span>
          <span v-if="c.weight" class="meta-tag">{{ c.weight }}kg</span>
        </div>
        <div class="card-complaint">{{ c.chief_complaint }}</div>
      </router-link>
    </div>
  </div>
</template>

<style scoped>
.page-header { margin-bottom: 20px; }
.page-title { font-size: 24px; font-weight: 700; margin-bottom: 4px; }
.page-desc { color: var(--color-text-secondary); font-size: 14px; }

.filters {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 24px;
  flex-wrap: wrap;
}

.filter-group { display: flex; gap: 10px; }

.filters select {
  padding: 8px 12px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  font-size: 14px;
  background: var(--color-surface);
}

.search-inline {
  display: flex;
  flex: 1;
  max-width: 360px;
}

.search-inline input {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid var(--color-border);
  border-right: none;
  border-radius: var(--radius) 0 0 var(--radius);
  font-size: 14px;
  outline: none;
}

.search-inline input:focus { border-color: var(--color-primary); }

.btn-search {
  padding: 8px 14px;
  background: var(--color-primary);
  color: white;
  border: none;
  border-radius: 0 var(--radius) var(--radius) 0;
  cursor: pointer;
  font-size: 14px;
  white-space: nowrap;
}

.btn-search:hover { background: var(--color-primary-dark); }

.btn-clear {
  padding: 8px 12px;
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  font-size: 13px;
  color: var(--color-text-secondary);
  cursor: pointer;
}

.btn-clear:hover { border-color: var(--color-danger); color: var(--color-danger); }

.loading { text-align: center; padding: 60px; color: var(--color-text-secondary); }

.empty {
  background: var(--color-surface);
  border: 1px dashed var(--color-border);
  border-radius: var(--radius);
  padding: 60px;
  text-align: center;
  color: var(--color-text-secondary);
  font-size: 16px;
}

.case-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 16px;
}

.case-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 16px;
  text-decoration: none;
  color: var(--color-text);
  transition: all 0.15s;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.case-card:hover {
  border-color: var(--color-primary);
  box-shadow: var(--shadow);
  transform: translateY(-1px);
}

.card-top {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 8px;
}

.card-title {
  font-weight: 600;
  font-size: 15px;
  line-height: 1.4;
  flex: 1;
}

.difficulty {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
  white-space: nowrap;
  flex-shrink: 0;
}

.difficulty.basic { background: #dcfce7; color: #16a34a; }
.difficulty.intermediate { background: #fef3c7; color: #d97706; }
.difficulty.advanced { background: #fee2e2; color: #dc2626; }

.card-meta {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.meta-tag {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
  background: #f1f5f9;
  color: var(--color-text-secondary);
}

.meta-tag.species {
  background: #eff6ff;
  color: var(--color-primary);
}

.card-complaint {
  font-size: 13px;
  color: var(--color-text-secondary);
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
