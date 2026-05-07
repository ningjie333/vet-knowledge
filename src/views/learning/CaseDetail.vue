<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import type { Case, Disease, Tag } from '../../types'

const route = useRoute()
const caseData = ref<Case | null>(null)
const diseases = ref<Disease[]>([])
const allTags = ref<Map<string, Tag>>(new Map())
const loading = ref(true)
const expandedSections = ref<Set<string>>(new Set(['chief_complaint', 'diagnosis']))

const caseId = computed(() => route.params.id as string)

interface Section {
  key: string
  label: string
  icon: string
  content: string | null
}

const sections = computed<Section[]>(() => {
  const c = caseData.value
  if (!c) return []
  return [
    { key: 'chief_complaint', label: '主诉', icon: '🗣️', content: c.chief_complaint },
    { key: 'history', label: '病史', icon: '📋', content: c.history },
    { key: 'physical_exam', label: '体格检查', icon: '🩺', content: c.physical_exam },
    { key: 'lab_results', label: '实验室检查', icon: '🧪', content: c.lab_results },
    { key: 'imaging', label: '影像学', icon: '📷', content: c.imaging },
    { key: 'diagnosis', label: '诊断', icon: '🎯', content: c.diagnosis },
    { key: 'treatment', label: '治疗', icon: '💊', content: c.treatment },
    { key: 'outcome', label: '转归', icon: '📈', content: c.outcome },
    { key: 'learning_points', label: '学习要点', icon: '💡', content: c.learning_points },
  ].filter(s => s.content)
})

function toggleSection(key: string) {
  if (expandedSections.value.has(key)) {
    expandedSections.value.delete(key)
  } else {
    expandedSections.value.add(key)
  }
}

function expandAll() {
  sections.value.forEach(s => expandedSections.value.add(s.key))
}

function collapseAll() {
  expandedSections.value.clear()
}

function difficultyLabel(d: string | null): string {
  if (!d) return ''
  return d === 'basic' ? '基础' : d === 'intermediate' ? '临床' : d === 'advanced' ? '进阶' : d
}

function difficultyClass(d: string | null): string {
  return d || 'intermediate'
}

onMounted(async () => {
  try {
    const [c, d, tags] = await Promise.all([
      invoke<Case>('get_case_by_id', { id: caseId.value }),
      invoke<Disease[]>('get_case_diseases', { caseId: caseId.value }),
      invoke<Tag[]>('get_tags'),
    ])
    caseData.value = c
    diseases.value = d
    const tagMap = new Map<string, Tag>()
    for (const tag of tags) tagMap.set(tag.id, tag)
    allTags.value = tagMap
  } catch (e) { console.error(e) }
  loading.value = false
})
</script>

<template>
  <div v-if="loading" class="loading">加载中...</div>

  <div v-else-if="!caseData" class="not-found">
    <p>未找到该病例</p>
    <router-link to="/cases" class="back-link">← 返回病例库</router-link>
  </div>

  <div v-else class="case-detail">
    <!-- 顶部信息栏 -->
    <div class="case-header">
      <router-link to="/cases" class="back-link">← 病例库</router-link>
      <h1 class="case-title">{{ caseData.title }}</h1>
      <div class="case-info-row">
        <span v-if="caseData.species" class="info-chip species">{{ caseData.species }}</span>
        <span v-if="caseData.breed" class="info-chip">{{ caseData.breed }}</span>
        <span v-if="caseData.age" class="info-chip">{{ caseData.age }}岁</span>
        <span v-if="caseData.weight" class="info-chip">{{ caseData.weight }}kg</span>
        <span class="difficulty" :class="difficultyClass(caseData.difficulty)">{{ difficultyLabel(caseData.difficulty) }}</span>
      </div>
    </div>

    <div class="detail-layout">
      <!-- 左侧主内容 -->
      <div class="main-content">
        <!-- 展开/折叠控制 -->
        <div class="section-controls">
          <button class="ctrl-btn" @click="expandAll">全部展开</button>
          <button class="ctrl-btn" @click="collapseAll">全部折叠</button>
        </div>

        <!-- 病例章节 -->
        <div class="sections">
          <div
            v-for="section in sections"
            :key="section.key"
            class="section-card"
            :class="{ expanded: expandedSections.has(section.key), diagnosis: section.key === 'diagnosis' }"
          >
            <div class="section-header" @click="toggleSection(section.key)">
              <span class="section-icon">{{ section.icon }}</span>
              <span class="section-label">{{ section.label }}</span>
              <span class="section-toggle">{{ expandedSections.has(section.key) ? '▾' : '▸' }}</span>
            </div>
            <div v-if="expandedSections.has(section.key)" class="section-body">
              <p>{{ section.content }}</p>
            </div>
          </div>
        </div>
      </div>

      <!-- 右侧面板 -->
      <aside class="side-panel">
        <!-- 动物信息 -->
        <div class="panel-card">
          <h3>动物信息</h3>
          <div class="info-list">
            <div v-if="caseData.species" class="info-item">
              <span class="info-key">物种</span>
              <span class="info-val">{{ caseData.species }}</span>
            </div>
            <div v-if="caseData.breed" class="info-item">
              <span class="info-key">品种</span>
              <span class="info-val">{{ caseData.breed }}</span>
            </div>
            <div v-if="caseData.age" class="info-item">
              <span class="info-key">年龄</span>
              <span class="info-val">{{ caseData.age }} 岁</span>
            </div>
            <div v-if="caseData.weight" class="info-item">
              <span class="info-key">体重</span>
              <span class="info-val">{{ caseData.weight }} kg</span>
            </div>
            <div v-if="caseData.difficulty" class="info-item">
              <span class="info-key">难度</span>
              <span class="difficulty" :class="difficultyClass(caseData.difficulty)">{{ difficultyLabel(caseData.difficulty) }}</span>
            </div>
          </div>
        </div>

        <!-- 关联疾病 -->
        <div v-if="diseases.length" class="panel-card">
          <h3>关联疾病</h3>
          <div class="disease-links">
            <router-link
              v-for="d in diseases"
              :key="d.id"
              :to="`/diseases/${d.id}`"
              class="disease-link"
            >
              <span class="disease-name">{{ d.name_zh }}</span>
              <span v-if="d.name_en" class="disease-en">{{ d.name_en }}</span>
            </router-link>
          </div>
        </div>

        <!-- 操作 -->
        <div class="panel-card actions">
          <router-link :to="`/cases/${caseData.id}/study`" class="action-btn study-btn">
            🧠 开始推理训练
          </router-link>
          <router-link to="/cases" class="action-btn">
            📋 返回病例列表
          </router-link>
        </div>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.loading { text-align: center; padding: 60px; color: var(--color-text-secondary); }

.not-found {
  text-align: center;
  padding: 60px;
  color: var(--color-text-secondary);
}

.not-found .back-link {
  display: inline-block;
  margin-top: 16px;
  color: var(--color-primary);
  text-decoration: none;
}

.case-header { margin-bottom: 24px; }

.back-link {
  display: inline-block;
  margin-bottom: 12px;
  color: var(--color-primary);
  text-decoration: none;
  font-size: 14px;
}

.back-link:hover { text-decoration: underline; }

.case-title { font-size: 26px; font-weight: 700; margin-bottom: 12px; }

.case-info-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  align-items: center;
}

.info-chip {
  font-size: 12px;
  padding: 3px 10px;
  border-radius: 12px;
  background: #f1f5f9;
  color: var(--color-text-secondary);
}

.info-chip.species {
  background: #eff6ff;
  color: var(--color-primary);
}

.difficulty {
  font-size: 12px;
  padding: 3px 10px;
  border-radius: 12px;
  font-weight: 500;
}

.difficulty.basic { background: #dcfce7; color: #16a34a; }
.difficulty.intermediate { background: #fef3c7; color: #d97706; }
.difficulty.advanced { background: #fee2e2; color: #dc2626; }

/* ── 布局 ── */
.detail-layout {
  display: grid;
  grid-template-columns: 1fr 300px;
  gap: 24px;
  align-items: start;
}

/* ── 章节控制 ── */
.section-controls {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.ctrl-btn {
  padding: 6px 12px;
  font-size: 12px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
  cursor: pointer;
  color: var(--color-text-secondary);
}

.ctrl-btn:hover { border-color: var(--color-primary); color: var(--color-primary); }

/* ── 章节卡片 ── */
.sections {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.section-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  overflow: hidden;
}

.section-card.diagnosis {
  border-color: var(--color-primary);
  border-width: 2px;
}

.section-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  cursor: pointer;
  user-select: none;
  transition: background 0.1s;
}

.section-header:hover { background: #f8fafc; }

.section-card.diagnosis .section-header { background: #eff6ff; }
.section-card.diagnosis .section-header:hover { background: #dbeafe; }

.section-icon { font-size: 18px; flex-shrink: 0; }

.section-label {
  font-weight: 600;
  font-size: 15px;
  flex: 1;
}

.section-card.diagnosis .section-label { color: var(--color-primary); }

.section-toggle {
  font-size: 14px;
  color: var(--color-text-secondary);
  flex-shrink: 0;
}

.section-body {
  padding: 0 16px 16px;
  line-height: 1.8;
  color: var(--color-text);
  white-space: pre-wrap;
}

.section-body p { margin: 0; }

/* ── 右侧面板 ── */
.side-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
  position: sticky;
  top: 24px;
}

.panel-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 16px;
}

.panel-card h3 {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-secondary);
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--color-border);
}

.info-list { display: flex; flex-direction: column; gap: 8px; }

.info-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
}

.info-key { color: var(--color-text-secondary); }
.info-val { font-weight: 500; }

.disease-links { display: flex; flex-direction: column; gap: 8px; }

.disease-link {
  display: flex;
  flex-direction: column;
  padding: 10px 12px;
  background: #f8fafc;
  border-radius: var(--radius);
  text-decoration: none;
  color: var(--color-text);
  border: 1px solid transparent;
  transition: all 0.15s;
}

.disease-link:hover {
  border-color: var(--color-primary);
  background: #eff6ff;
}

.disease-name { font-weight: 500; font-size: 14px; }
.disease-en { font-size: 12px; color: var(--color-text-secondary); margin-top: 2px; }

/* ── 操作按钮 ── */
.actions { display: flex; flex-direction: column; gap: 8px; }

.action-btn {
  display: block;
  text-align: center;
  padding: 10px;
  border-radius: var(--radius);
  text-decoration: none;
  font-size: 13px;
  font-weight: 500;
  border: 1px solid var(--color-border);
  color: var(--color-text);
  transition: all 0.15s;
}

.action-btn:hover { border-color: var(--color-primary); color: var(--color-primary); }

.study-btn {
  background: var(--color-primary);
  color: white;
  border-color: var(--color-primary);
}

.study-btn:hover { background: var(--color-primary-dark); color: white; }

/* ── 响应式 ── */
@media (max-width: 900px) {
  .detail-layout { grid-template-columns: 1fr; }
  .side-panel { position: static; }
}
</style>
