<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const drugs = ref<any[]>([])
onMounted(async () => {
  try { drugs.value = await invoke('get_drugs', { drugClass: null }) } catch (e) { console.error(e) }
})
</script>

<template>
  <div class="page">
    <h1 class="page-title">药物手册</h1>
    <p v-if="drugs.length === 0" class="empty">暂无药物数据，敬请期待</p>
    <div v-else class="drug-list">
      <div v-for="d in drugs" :key="d.id" class="drug-card">
        <div class="drug-name">{{ d.name_zh }}</div>
        <div class="drug-en">{{ d.name_en }}</div>
        <div v-if="d.drug_class" class="drug-class">{{ d.drug_class }}</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page-title { font-size: 24px; font-weight: 700; margin-bottom: 24px; }
.empty { color: var(--color-text-secondary); padding: 40px; text-align: center; }
.drug-list { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 16px; }
.drug-card { background: var(--color-surface); border: 1px solid var(--color-border); border-radius: var(--radius); padding: 16px; }
.drug-name { font-weight: 600; font-size: 16px; }
.drug-en { font-size: 12px; color: var(--color-text-secondary); }
.drug-class { font-size: 13px; color: var(--color-primary); margin-top: 8px; }
</style>
