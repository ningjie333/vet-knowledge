<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Symptom, Tag } from '../../types'

const symptoms = ref<Symptom[]>([])
const diseaseCounts = ref<Map<string, number>>(new Map())
const allTags = ref<Map<string, Tag>>(new Map())
const loading = ref(true)

const tagMap = computed(() => allTags.value)

onMounted(async () => {
  try {
    const [s, tags] = await Promise.all([
      invoke<Symptom[]>('get_symptoms'),
      invoke<Tag[]>('get_tags'),
    ])
    symptoms.value = s

    const tMap = new Map<string, Tag>()
    for (const tag of tags) {
      tMap.set(tag.id, tag)
    }
    allTags.value = tMap

    // Batch fetch disease counts for all symptoms
    const counts = await Promise.all(
      symptoms.value.map(sym =>
        invoke<{ disease_id: string }[]>('get_diseases_by_symptom', {
          symptomId: sym.id,
          species: null,
        }).then(r => r.length).catch(() => 0)
      )
    )
    const map = new Map<string, number>()
    symptoms.value.forEach((sym, i) => map.set(sym.id, counts[i]))
    diseaseCounts.value = map
  } catch (e) { console.error(e) }
  loading.value = false
})

function getTagStyle(tagId: string): string {
  const tag = tagMap.value.get(tagId)
  return tag?.color || '#999'
}

function getTagName(tagId: string): string {
  return tagMap.value.get(tagId)?.name_zh || tagId
}
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

        <!-- 标签 -->
        <div v-if="s.tags?.length" class="symptom-tags">
          <span
            v-for="tagId in s.tags"
            :key="tagId"
            class="tag-chip"
            :style="{ background: getTagStyle(tagId) }"
          >
            {{ getTagName(tagId) }}
          </span>
        </div>

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

.symptom-tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 8px; }
.tag-chip {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
  color: white;
  font-weight: 500;
}

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
