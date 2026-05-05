<script setup lang="ts">
import Sidebar from '@/components/Sidebar.vue'
import TopBar from '@/components/TopBar.vue'
</script>

<template>
  <div class="app-layout">
    <Sidebar />
    <div class="main-content">
      <TopBar />
      <main class="page-container">
        <router-view />
      </main>
    </div>
  </div>
</template>

<style>
* { margin: 0; padding: 0; box-sizing: border-box; }

:root {
  --sidebar-width: 220px;
  --topbar-height: 48px;
  --color-primary: #2563eb;
  --color-primary-dark: #1d4ed8;
  --color-bg: #f8fafc;
  --color-surface: #ffffff;
  --color-border: #e2e8f0;
  --color-text: #1e293b;
  --color-text-secondary: #64748b;
  --color-success: #16a34a;
  --color-warning: #d97706;
  --color-danger: #dc2626;
  --radius: 8px;
  --shadow: 0 1px 3px rgba(0,0,0,0.1);
}

body {
  font-family: 'Noto Sans SC', -apple-system, sans-serif;
  background: var(--color-bg);
  color: var(--color-text);
}

.app-layout {
  display: flex;
  height: 100vh;
  overflow: hidden;
}

.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.page-container {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}
</style>
