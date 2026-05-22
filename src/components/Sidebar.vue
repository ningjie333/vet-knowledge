<script setup lang="ts">
import { useRoute } from 'vue-router'
const route = useRoute()

const menuItems = [
  { path: '/', label: '首页', icon: '🏠' },
  { path: '/diseases', label: '疾病百科', icon: '📚' },
  { path: '/symptom-explorer', label: '症状→疾病', icon: '🔄' },
  { path: '/drugs', label: '药物手册', icon: '💊' },
  { path: '/diagnose', label: '症状推理', icon: '🧠' },
  { path: '/graph', label: '知识图谱', icon: '🕸️' },
  { path: '/cases', label: '病例库', icon: '📋' },
  { path: '/flashcards', label: '闪卡复习', icon: '🃏' },
  { path: '/compare', label: '疾病对比', icon: '⚖️' },
  { path: '/game', label: '诊断游戏', icon: '🎮' },
]
</script>

<template>
  <aside class="sidebar">
    <div class="logo">
      <span class="logo-icon">🐾</span>
      <span class="logo-text">兽医知识库</span>
    </div>
    <nav class="menu">
      <router-link
        v-for="item in menuItems"
        :key="item.path"
        :to="item.path"
        class="menu-item"
        :class="{ active: route.path === item.path }"
      >
        <span class="menu-icon">{{ item.icon }}</span>
        <span class="menu-label">{{ item.label }}</span>
      </router-link>
    </nav>
  </aside>
</template>

<style scoped>
.sidebar {
  width: var(--sidebar-width);
  height: 100vh;
  background: var(--color-surface);
  border-right: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  overflow: hidden;
}

.logo {
  padding: 20px 16px;
  display: flex;
  align-items: center;
  gap: 8px;
  border-bottom: 1px solid var(--color-border);
}

.logo-icon { font-size: 24px; }
.logo-text { font-size: 16px; font-weight: 700; color: var(--color-primary); }

.menu {
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-y: auto;
  flex: 1;
  min-height: 0;
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: var(--radius);
  text-decoration: none;
  color: var(--color-text-secondary);
  font-size: 13px;
  transition: all 0.15s;
  flex-shrink: 0;
}

.menu-item:hover { background: var(--color-bg); color: var(--color-text); }
.menu-item.active { background: #eff6ff; color: var(--color-primary); font-weight: 500; }
.menu-icon { font-size: 16px; }
</style>
