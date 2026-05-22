<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const symptoms = ref<any[]>([])
const loading = ref(true)
const searchQuery = ref('')

async function load() {
  loading.value = true
  try {
    const all = await invoke('get_symptoms')
    if (searchQuery.value) {
      const q = searchQuery.value.toLowerCase()
      symptoms.value = (all as any[]).filter(s =>
        s.name_zh.toLowerCase().includes(q) ||
        (s.name_en && s.name_en.toLowerCase().includes(q))
      )
    } else {
      symptoms.value = all as any[]
    }
  } catch (e) { console.error(e) }
  loading.value = false
}

onMounted(load)
</script>

<template>
  <div class="page">
    <h1 class="page-title">症状检索</h1>

    <div class="filters">
      <input
        v-model="searchQuery"
        type="text"
        placeholder="搜索症状..."
        class="search-input"
        @input="load"
      />
    </div>

    <div v-if="loading" class="loading">加载中...</div>

    <div v-else class="symptom-grid">
      <div
        v-for="s in symptoms"
        :key="s.id"
        class="symptom-card"
      >
        <div class="symptom-name">{{ s.name_zh }}</div>
        <div class="symptom-en">{{ s.name_en }}</div>
        <div v-if="s.definition" class="symptom-def">{{ s.definition }}</div>
      </div>
    </div>

    <div v-if="!loading && symptoms.length === 0" class="empty">
      没有找到匹配的症状
    </div>
  </div>
</template>

<style scoped>
.page-title { font-size: 24px; font-weight: 700; margin-bottom: 20px; }

.filters { margin-bottom: 24px; }

.search-input {
  padding: 8px 12px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  font-size: 14px;
  width: 320px;
  background: var(--color-surface);
}

.symptom-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 12px;
}

.symptom-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 16px;
  transition: all 0.15s;
}

.symptom-card:hover { border-color: var(--color-primary); }

.symptom-name { font-weight: 600; font-size: 15px; }
.symptom-en { font-size: 12px; color: var(--color-text-secondary); margin-top: 2px; }
.symptom-def { font-size: 13px; color: var(--color-text-secondary); margin-top: 8px; line-height: 1.5; }

.loading, .empty { color: var(--color-text-secondary); text-align: center; padding: 40px; }
</style>
