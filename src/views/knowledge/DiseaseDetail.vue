<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import type { Disease, Tag } from '../../types'

const route = useRoute()
const disease = ref<Disease | null>(null)
const symptoms = ref<any[]>([])
const ddx = ref<any[]>([])
const treatments = ref<any[]>([])
const entityTags = ref<Tag[]>([])
const allTags = ref<Map<string, Tag>>(new Map())
const loading = ref(true)

const emergencyInfo = computed(() => {
  const tag = entityTags.value.find(t => t.tag_group === 'emergency')
  return tag || null
})

const bodySystemTags = computed(() =>
  entityTags.value.filter(t => t.tag_group === 'body_system')
)

const mechanismTags = computed(() =>
  entityTags.value.filter(t => t.tag_group === 'mechanism')
)

const damnitTags = computed(() =>
  entityTags.value.filter(t => t.tag_group === 'damnit_v')
)

onMounted(async () => {
  const id = route.params.id as string
  try {
    const [d, s, dx, t, tags] = await Promise.all([
      invoke<Disease>('get_disease_by_id', { id }),
      invoke('get_disease_symptoms', { diseaseId: id }),
      invoke('get_disease_ddx', { diseaseId: id }),
      invoke('get_disease_treatment_map', { diseaseId: id }),
      invoke<Tag[]>('get_tags'),
    ])
    disease.value = d
    symptoms.value = s as any[]
    ddx.value = dx as any[]
    treatments.value = t as any[]

    // Build tag lookup map
    const tagMap = new Map<string, Tag>()
    for (const tag of tags) {
      tagMap.set(tag.id, tag)
    }
    allTags.value = tagMap

    // Resolve entity tags from disease.tags
    const diseaseTags = d?.tags || []
    entityTags.value = diseaseTags
      .map((tagId: string) => tagMap.get(tagId))
      .filter(Boolean) as Tag[]
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
      <span v-if="disease.name_latin" class="latin-name"><em>{{ disease.name_latin }}</em></span>

      <div class="tags">
        <span class="difficulty" :class="disease.difficulty">{{ disease.difficulty }}</span>
        <span v-for="s in (disease.species || [])" :key="s" class="species-tag">{{ s }}</span>
      </div>
    </div>

    <!-- 急诊等级徽章 -->
    <div v-if="emergencyInfo" class="emergency-badge" :class="`emergency-${emergencyInfo.emergency_level}`">
      <span class="emergency-dot"></span>
      <span class="emergency-label">{{ emergencyInfo.name_zh }}</span>
      <span class="emergency-action">{{ emergencyInfo.clinical_action }}</span>
    </div>

    <!-- 标签云 -->
    <div v-if="bodySystemTags.length" class="tag-group">
      <span class="tag-group-label">解剖系统</span>
      <span v-for="tag in bodySystemTags" :key="tag.id" class="tag-chip" :style="{ background: tag.color || '#666' }">
        {{ tag.name_zh }}
      </span>
    </div>
    <div v-if="mechanismTags.length" class="tag-group">
      <span class="tag-group-label">病理机制</span>
      <span v-for="tag in mechanismTags" :key="tag.id" class="tag-chip mechanism" :style="{ background: tag.color || '#666' }">
        {{ tag.name_zh }}
      </span>
    </div>
    <div v-if="damnitTags.length" class="tag-group">
      <span class="tag-group-label">DAMNIT-V</span>
      <span v-for="tag in damnitTags" :key="tag.id" class="tag-chip damnit" :style="{ background: tag.color || '#666' }">
        {{ tag.name_zh }}
      </span>
    </div>

    <!-- 高急迫性警告 -->
    <div v-if="disease.urgency_level && disease.urgency_level >= 4" class="urgency-alert">
      <strong>⚡ 高急迫性疾病</strong>
      <span v-if="disease.urgency_level === 5"> — 危及生命，需立即干预</span>
      <span v-else> — 需尽快处理，延误可能加重病情</span>
    </div>

    <!-- 流行病学 -->
    <section v-if="disease.epidemiology" class="section">
      <h2>流行病学</h2>
      <p>{{ disease.epidemiology }}</p>
    </section>

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

    <!-- 发病机制详情 -->
    <section v-if="disease.physiological_basis" class="section">
      <h2>发病机制</h2>
      <p>{{ disease.physiological_basis }}</p>
    </section>

    <section class="section">
      <h2>病理生理</h2>
      <p>{{ disease.pathophysiology }}</p>
    </section>

    <section class="section">
      <h2>临床症状</h2>
      <div class="symptom-list">
        <div v-for="s in symptoms" :key="s[0].id" class="symptom-item" :class="{ pathognomonic: s[3] === 1 }">
          <router-link :to="{ name: 'symptom-explorer', query: { symptomId: s[0].id } }" class="symptom-name">{{ s[0].name_zh }}</router-link>
          <span v-if="s[3] === 1" class="pathognomonic-tag">核心</span>
          <span class="freq" :class="s[1]">{{ s[1] }}</span>
          <span v-if="s[2]" class="stage">{{ s[2] }}</span>
        </div>
      </div>
    </section>

    <!-- 治疗方案 -->
    <section v-if="treatments.length" class="section">
      <h2>治疗方案</h2>
      <div class="treatment-list">
        <div v-for="t in treatments" :key="t[0].id" class="treatment-card">
          <div class="treatment-name">{{ t[0].name_zh }}</div>
          <div v-if="t[0].therapy_type" class="treatment-type">{{ t[0].therapy_type }}</div>
          <div v-if="t[1]" class="treatment-line" :class="t[1]">{{ t[1] }}线方案</div>
          <p v-if="t[0].principle" class="treatment-principle">{{ t[0].principle }}</p>
          <p v-if="t[3]" class="treatment-notes">{{ t[3] }}</p>
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
.header { margin-bottom: 24px; }
.header h1 { font-size: 28px; font-weight: 700; }
.en-name { font-size: 14px; color: var(--color-text-secondary); display: block; margin-top: 4px; }
.latin-name { font-size: 13px; color: var(--color-text-secondary); display: block; margin-top: 2px; }
.tags { display: flex; gap: 8px; margin-top: 12px; flex-wrap: wrap; }

/* 急诊等级徽章 */
.emergency-badge {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-radius: var(--radius);
  margin-bottom: 20px;
  font-size: 14px;
}
.emergency-red { background: #fef2f2; border: 1px solid #fecaca; color: #991b1b; }
.emergency-orange { background: #fff7ed; border: 1px solid #fed7aa; color: #9a3412; }
.emergency-yellow { background: #fefce8; border: 1px solid #fef08a; color: #854d0e; }
.emergency-green { background: #f0fdf4; border: 1px solid #bbf7d0; color: #166534; }
.emergency-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}
.emergency-red .emergency-dot { background: #dc2626; }
.emergency-orange .emergency-dot { background: #ea580c; }
.emergency-yellow .emergency-dot { background: #ca8a04; }
.emergency-green .emergency-dot { background: #16a34a; }
.emergency-label { font-weight: 700; }
.emergency-action { font-size: 13px; opacity: 0.85; }

/* 标签组 */
.tag-group {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
  flex-wrap: wrap;
}
.tag-group-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-secondary);
  min-width: fit-content;
}
.tag-chip {
  font-size: 12px;
  padding: 3px 10px;
  border-radius: 12px;
  color: white;
  font-weight: 500;
}

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

.symptom-name {
  color: var(--color-primary);
  font-weight: 500;
  text-decoration: none;
  cursor: pointer;
}
.symptom-name:hover {
  text-decoration: underline;
}
.pathognomonic .symptom-name {
  color: #ca8a04;
  font-weight: 600;
}

.freq { font-size: 11px; padding: 2px 6px; border-radius: 8px; }
.freq.common { background: #fee2e2; color: #dc2626; }
.freq.uncommon { background: #fef3c7; color: #d97706; }
.freq.rare { background: #e0e7ff; color: #4f46e5; }
.stage { font-size: 11px; color: var(--color-text-secondary); }

.treatment-list { display: flex; flex-direction: column; gap: 12px; }
.treatment-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 16px;
}
.treatment-name { font-weight: 600; font-size: 16px; color: var(--color-primary); }
.treatment-type { font-size: 12px; color: var(--color-text-secondary); margin-top: 4px; }
.treatment-line {
  display: inline-block;
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 8px;
  margin-top: 6px;
  font-weight: 500;
}
.treatment-line.first { background: #dcfce7; color: #16a34a; }
.treatment-line.second { background: #fef3c7; color: #d97706; }
.treatment-line.adjunctive { background: #e0e7ff; color: #4f46e5; }
.treatment-principle { font-size: 14px; color: var(--color-text); margin-top: 8px; line-height: 1.6; }
.treatment-notes { font-size: 13px; color: var(--color-text-secondary); margin-top: 6px; font-style: italic; }

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
