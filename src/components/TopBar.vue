<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { SearchResult } from '@/types'

const searchQuery = ref('')
const searchResults = ref<SearchResult[]>([])
const showResults = ref(false)

async function onSearch() {
  if (!searchQuery.value.trim()) return
  try {
    searchResults.value = await invoke<SearchResult[]>('full_text_search', {
      query: searchQuery.value,
      limit: 10,
    })
    showResults.value = true
  } catch (e) {
    console.error('Search failed:', e)
  }
}

function onClickOutside() {
  showResults.value = false
}
</script>

<template>
  <header class="topbar">
    <div class="search-box" @click.stop>
      <input
        v-model="searchQuery"
        type="text"
        placeholder="搜索疾病、症状、药物..."
        @keyup.enter="onSearch"
        @focus="showResults = searchResults.length > 0"
      />
      <button class="search-btn" @click="onSearch">🔍</button>
      <div v-if="showResults && searchResults.length" class="search-dropdown" @click="onClickOutside">
        <div
          v-for="r in searchResults"
          :key="r.entity_id"
          class="search-result-item"
        >
          <span class="result-type">{{ r.entity_type }}</span>
          <span class="result-title">{{ r.title }}</span>
          <span class="result-snippet">{{ r.snippet?.slice(0, 60) }}...</span>
        </div>
      </div>
    </div>
    <div class="topbar-actions">
      <span class="version">v0.1.0</span>
    </div>
  </header>
</template>

<style scoped>
.topbar {
  height: var(--topbar-height);
  background: var(--color-surface);
  border-bottom: 1px solid var(--color-border);
  display: flex;
  align-items: center;
  padding: 0 24px;
  gap: 16px;
  flex-shrink: 0;
}

.search-box {
  flex: 1;
  max-width: 480px;
  position: relative;
  display: flex;
}

.search-box input {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius) 0 0 var(--radius);
  font-size: 14px;
  outline: none;
}

.search-box input:focus { border-color: var(--color-primary); }

.search-btn {
  padding: 8px 12px;
  background: var(--color-primary);
  color: white;
  border: none;
  border-radius: 0 var(--radius) var(--radius) 0;
  cursor: pointer;
}

.search-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  z-index: 100;
  max-height: 400px;
  overflow-y: auto;
}

.search-result-item {
  padding: 10px 12px;
  border-bottom: 1px solid var(--color-border);
  display: flex;
  gap: 8px;
  align-items: center;
  font-size: 13px;
}

.result-type {
  background: #eff6ff;
  color: var(--color-primary);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 11px;
  white-space: nowrap;
}

.result-title { font-weight: 500; flex: 1; }
.result-snippet { color: var(--color-text-secondary); font-size: 12px; }

.version { font-size: 12px; color: var(--color-text-secondary); }
</style>
