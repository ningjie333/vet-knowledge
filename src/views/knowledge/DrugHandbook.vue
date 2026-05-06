<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Drug, Tag } from '../../types'

const drugs = ref<Drug[]>([])
const allTags = ref<Map<string, Tag>>(new Map())
const selectedDrug = ref<Drug | null>(null)
const loading = ref(true)

const drugTags = computed(() => {
  if (!selectedDrug.value) return []
  const tags = selectedDrug.value.tags || []
  return tags.map(id => allTags.value.get(id)).filter(Boolean) as Tag[]
})

const mechanismTags = computed(() =>
  drugTags.value.filter(t => t.tag_group === 'mechanism')
)

onMounted(async () => {
  try {
    const [d, tags] = await Promise.all([
      invoke<Drug[]>('get_drugs', { drugClass: null }),
      invoke<Tag[]>('get_tags'),
    ])
    drugs.value = d
    const tagMap = new Map<string, Tag>()
    for (const tag of tags) {
      tagMap.set(tag.id, tag)
    }
    allTags.value = tagMap
  } catch (e) { console.error(e) }
  loading.value = false
})

function openDetail(drug: Drug) {
  selectedDrug.value = drug
}

function closeDetail() {
  selectedDrug.value = null
}
</script>

<template>
  <div class="page">
    <h1 class="page-title">药物手册</h1>

    <div v-if="loading" class="loading">加载中...</div>
    <p v-else-if="drugs.length === 0" class="empty">暂无药物数据</p>

    <div v-else class="drug-layout">
      <!-- 药物列表 -->
      <div class="drug-list">
        <div
          v-for="d in drugs"
          :key="d.id"
          class="drug-card"
          :class="{ active: selectedDrug?.id === d.id }"
          @click="openDetail(d)"
        >
          <div class="drug-name">{{ d.name_zh }}</div>
          <div class="drug-en">{{ d.name_en }}</div>
          <div v-if="d.drug_class" class="drug-class">{{ d.drug_class }}</div>
          <div v-if="d.tags?.length" class="drug-tags-preview">
            <span
              v-for="tagId in d.tags.slice(0, 3)"
              :key="tagId"
              class="tag-dot"
              :style="{ background: allTags.get(tagId)?.color || '#999' }"
              :title="allTags.get(tagId)?.name_zh || tagId"
            ></span>
          </div>
        </div>
      </div>

      <!-- 药物详情面板 -->
      <div v-if="selectedDrug" class="drug-detail">
        <button class="close-btn" @click="closeDetail">✕</button>
        <h2>{{ selectedDrug.name_zh }}</h2>
        <span class="detail-en">{{ selectedDrug.name_en }}</span>
        <div v-if="selectedDrug.drug_class" detail-class>{{ selectedDrug.drug_class }}</div>

        <!-- 机制标签 -->
        <div v-if="mechanismTags.length" class="tag-group">
          <span class="tag-group-label">作用机制</span>
          <span
            v-for="tag in mechanismTags"
            :key="tag.id"
            class="tag-chip"
            :style="{ background: tag.color || '#666' }"
          >
            {{ tag.name_zh }}
          </span>
        </div>

        <!-- 作用机制详情 -->
        <section v-if="selectedDrug.mechanism_of_action" class="detail-section">
          <h3>作用机制</h3>
          <p>{{ selectedDrug.mechanism_of_action }}</p>
        </section>

        <section v-if="selectedDrug.indications" class="detail-section">
          <h3>适应症</h3>
          <p>{{ selectedDrug.indications }}</p>
        </section>

        <section v-if="selectedDrug.contraindications" class="detail-section">
          <h3>禁忌症</h3>
          <p>{{ selectedDrug.contraindications }}</p>
        </section>

        <section v-if="selectedDrug.side_effects" class="detail-section">
          <h3>不良反应</h3>
          <p>{{ selectedDrug.side_effects }}</p>
        </section>

        <!-- 不良反应机制 -->
        <section v-if="selectedDrug.adverse_mechanism" class="detail-section">
          <h3>不良反应机制</h3>
          <p>{{ selectedDrug.adverse_mechanism }}</p>
        </section>

        <!-- PK/PD -->
        <section v-if="selectedDrug.pk_pd" class="detail-section">
          <h3>药代动力学/药效学</h3>
          <p>{{ selectedDrug.pk_pd }}</p>
        </section>

        <section v-if="selectedDrug.species_dosing" class="detail-section">
          <h3>物种剂量</h3>
          <pre class="dosing-text">{{ selectedDrug.species_dosing }}</pre>
        </section>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page-title { font-size: 24px; font-weight: 700; margin-bottom: 24px; }
.empty { color: var(--color-text-secondary); padding: 40px; text-align: center; }
.loading { text-align: center; padding: 40px; color: var(--color-text-secondary); }

.drug-layout { display: flex; gap: 24px; }
.drug-list {
  flex: 0 0 320px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: calc(100vh - 200px);
  overflow-y: auto;
}
.drug-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 12px 14px;
  cursor: pointer;
  transition: all 0.15s;
}
.drug-card:hover { border-color: var(--color-primary); }
.drug-card.active { border-color: var(--color-primary); background: #eff6ff; }
.drug-name { font-weight: 600; font-size: 15px; }
.drug-en { font-size: 12px; color: var(--color-text-secondary); }
.drug-class { font-size: 12px; color: var(--color-primary); margin-top: 4px; }
.drug-tags-preview { display: flex; gap: 4px; margin-top: 6px; }
.tag-dot { width: 8px; height: 8px; border-radius: 50%; display: inline-block; }

.drug-detail {
  flex: 1;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 24px;
  position: relative;
  max-height: calc(100vh - 200px);
  overflow-y: auto;
}
.close-btn {
  position: absolute;
  top: 12px;
  right: 12px;
  background: none;
  border: none;
  font-size: 18px;
  cursor: pointer;
  color: var(--color-text-secondary);
  padding: 4px 8px;
}
.close-btn:hover { color: var(--color-text); }
.drug-detail h2 { font-size: 22px; font-weight: 700; margin-bottom: 4px; }
.detail-en { font-size: 13px; color: var(--color-text-secondary); display: block; margin-bottom: 8px; }
.detail-class { font-size: 13px; color: var(--color-primary); margin-bottom: 16px; }

.tag-group {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}
.tag-group-label { font-size: 12px; font-weight: 600; color: var(--color-text-secondary); }
.tag-chip {
  font-size: 12px;
  padding: 3px 10px;
  border-radius: 12px;
  color: white;
  font-weight: 500;
}

.detail-section { margin-bottom: 20px; }
.detail-section h3 {
  font-size: 15px;
  font-weight: 600;
  color: var(--color-primary);
  margin-bottom: 8px;
}
.detail-section p { font-size: 14px; line-height: 1.7; color: var(--color-text); }
.dosing-text {
  font-size: 13px;
  line-height: 1.8;
  color: var(--color-text);
  white-space: pre-wrap;
  font-family: inherit;
}
</style>
