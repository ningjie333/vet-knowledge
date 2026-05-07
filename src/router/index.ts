import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: () => import('@/views/knowledge/Home.vue')
    },
    {
      path: '/diseases',
      name: 'diseases',
      component: () => import('@/views/knowledge/DiseaseList.vue')
    },
    {
      path: '/diseases/:id',
      name: 'disease-detail',
      component: () => import('@/views/knowledge/DiseaseDetail.vue')
    },
    {
      path: '/symptoms',
      name: 'symptoms',
      component: () => import('@/views/knowledge/SymptomList.vue')
    },
    {
      path: '/symptom-explorer',
      name: 'symptom-explorer',
      component: () => import('@/views/knowledge/SymptomExplorer.vue')
    },
    {
      path: '/drugs',
      name: 'drugs',
      component: () => import('@/views/knowledge/DrugHandbook.vue')
    },
    {
      path: '/diagnose',
      name: 'diagnose',
      component: () => import('@/views/knowledge/SymptomChecker.vue')
    },
    {
      path: '/graph',
      name: 'graph',
      component: () => import('@/views/graph/KnowledgeGraph.vue')
    },
    {
      path: '/cases',
      name: 'cases',
      component: () => import('@/views/learning/CaseLibrary.vue')
    },
    {
      path: '/cases/:id',
      name: 'case-detail',
      component: () => import('@/views/learning/CaseDetail.vue')
    },
    {
      path: '/cases/:id/study',
      name: 'case-study',
      component: () => import('@/views/learning/CaseStudy.vue')
    },
    {
      path: '/flashcards',
      name: 'flashcards',
      component: () => import('@/views/learning/FlashcardStudy.vue')
    },
    {
      path: '/compare',
      name: 'compare',
      component: () => import('@/views/knowledge/DiseaseCompare.vue')
    },
    {
      path: '/game',
      name: 'game',
      component: () => import('@/views/game/GameHome.vue')
    },
  ]
})

export default router
