<script setup lang="ts">
import { useRoute } from 'vue-router'
const route = useRoute()
const caseId = route.params.id
</script>

<template>
  <div class="page">
    <div class="page-header">
      <h1 class="page-title">🧠 推理训练</h1>
      <p class="page-desc">病例 #{{ caseId }} — 逐步推理学习模式</p>
    </div>
    <div class="coming-soon">
      <p>🚧 推理训练模式开发中</p>
      <router-link :to="`/cases/${caseId}`" class="back-link">← 返回病例详情</router-link>
    </div>
  </div>
</template>

<style scoped>
.page-header { margin-bottom: 20px; }
.page-title { font-size: 24px; font-weight: 700; margin-bottom: 4px; }
.page-desc { color: var(--color-text-secondary); font-size: 14px; }
.coming-soon {
  background: var(--color-surface);
  border: 1px dashed var(--color-border);
  border-radius: var(--radius);
  padding: 60px;
  text-align: center;
  color: var(--color-text-secondary);
  font-size: 16px;
}
.back-link {
  display: inline-block;
  margin-top: 16px;
  color: var(--color-primary);
  text-decoration: none;
  font-size: 14px;
}
.back-link:hover { text-decoration: underline; }
</style>
