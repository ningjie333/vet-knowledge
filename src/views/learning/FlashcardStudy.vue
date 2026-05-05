<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface Flashcard {
  id: string
  front: string
  back: string
  card_type: string
  difficulty: number
  review_count: number
  ease_factor: number
  next_review: string | null
}

interface ReviewStats {
  total_cards: number
  due_today: number
  reviewed_today: number
  mastered: number
}

const stats = ref<ReviewStats | null>(null)
const dueCards = ref<Flashcard[]>([])
const currentIndex = ref(0)
const showBack = ref(false)
const studied = ref(false)
const finished = ref(false)

const currentCard = computed(() => dueCards.value[currentIndex.value] || null)
const progress = computed(() => {
  if (dueCards.value.length === 0) return 0
  return Math.round(((currentIndex.value + (studied.value ? 1 : 0)) / dueCards.value.length) * 100)
})

onMounted(async () => {
  await loadStats()
  await loadDueCards()
})

async function loadStats() {
  try {
    stats.value = await invoke<ReviewStats>('get_review_stats')
  } catch (e) { console.error(e) }
}

async function loadDueCards() {
  try {
    dueCards.value = await invoke<Flashcard[]>('get_due_flashcards', { limit: 20 })
    currentIndex.value = 0
    showBack.value = false
    finished.value = false
  } catch (e) { console.error(e) }
}

function flip() {
  showBack.value = true
  studied.value = true
}

async function review(quality: number) {
  if (!currentCard.value) return
  try {
    await invoke('review_flashcard', { cardId: currentCard.value.id, quality })
  } catch (e) { console.error(e) }

  showBack.value = false
  studied.value = false

  if (currentIndex.value + 1 >= dueCards.value.length) {
    finished.value = true
    await loadStats()
  } else {
    currentIndex.value++
  }
}

async function generateCards(cardType: string) {
  try {
    const count = await invoke<number>('generate_flashcards_from_knowledge', { cardType })
    alert(`成功生成 ${count} 张闪卡`)
    await loadStats()
    await loadDueCards()
  } catch (e) { console.error(e) }
}

function typeLabel(type: string) {
  const map: Record<string, string> = {
    disease: '疾病',
    symptom: '症状',
    drug: '药物',
    custom: '自定义',
  }
  return map[type] || type
}

function typeIcon(type: string) {
  const map: Record<string, string> = {
    disease: '🏥',
    symptom: '🔍',
    drug: '💊',
    custom: '✏️',
  }
  return map[type] || '📝'
}
</script>

<template>
  <div class="page">
    <h1 class="page-title">闪卡复习</h1>
    <p class="desc">基于 SM-2 间隔重复算法，高效记忆兽医知识</p>

    <!-- Stats bar -->
    <div v-if="stats" class="stats-bar">
      <div class="stat-item">
        <span class="stat-num">{{ stats.total_cards }}</span>
        <span class="stat-label">总闪卡</span>
      </div>
      <div class="stat-item highlight">
        <span class="stat-num">{{ stats.due_today }}</span>
        <span class="stat-label">今日到期</span>
      </div>
      <div class="stat-item">
        <span class="stat-num">{{ stats.reviewed_today }}</span>
        <span class="stat-label">今日已复习</span>
      </div>
      <div class="stat-item">
        <span class="stat-num">{{ stats.mastered }}</span>
        <span class="stat-label">已掌握</span>
      </div>
    </div>

    <!-- Generate cards -->
    <div v-if="stats && stats.total_cards === 0" class="generate-section">
      <p>还没有闪卡，从知识库自动生成：</p>
      <div class="generate-buttons">
        <button @click="generateCards('disease')" class="gen-btn">
          🏥 生成疾病闪卡
        </button>
        <button @click="generateCards('symptom')" class="gen-btn">
          🔍 生成症状闪卡
        </button>
        <button @click="generateCards('drug')" class="gen-btn">
          💊 生成药物闪卡
        </button>
      </div>
    </div>

    <!-- Study area -->
    <div v-else-if="!finished && dueCards.length > 0" class="study-area">
      <div class="progress-bar">
        <div class="progress-fill" :style="{ width: progress + '%' }"></div>
        <span class="progress-text">{{ currentIndex + 1 }} / {{ dueCards.length }}</span>
      </div>

      <div
        class="flashcard"
        :class="{ flipped: showBack }"
        @click="!showBack && flip()"
      >
        <div class="card-inner">
          <div class="card-front">
            <div class="card-type">
              <span>{{ typeIcon(currentCard?.card_type || '') }}</span>
              {{ typeLabel(currentCard?.card_type || '') }}
            </div>
            <div class="card-content">{{ currentCard?.front }}</div>
            <div class="flip-hint">点击翻转</div>
          </div>
          <div class="card-back">
            <div class="card-type">
              <span>{{ typeIcon(currentCard?.card_type || '') }}</span>
              {{ typeLabel(currentCard?.card_type || '') }}
            </div>
            <div class="card-content">{{ currentCard?.back }}</div>
          </div>
        </div>
      </div>

      <div v-if="showBack" class="review-buttons">
        <p class="review-prompt">记得吗？</p>
        <div class="btn-row">
          <button class="review-btn again" @click="review(0)">
            <span class="btn-label">完全忘记</span>
            <span class="btn-hint">1天内复习</span>
          </button>
          <button class="review-btn hard" @click="review(2)">
            <span class="btn-label">勉强想起</span>
            <span class="btn-hint">1天内复习</span>
          </button>
          <button class="review-btn good" @click="review(3)">
            <span class="btn-label">想起来了</span>
            <span class="btn-hint">正常间隔</span>
          </button>
          <button class="review-btn easy" @click="review(5)">
            <span class="btn-label">非常轻松</span>
            <span class="btn-hint">长间隔</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Finished -->
    <div v-else-if="finished" class="finished-state">
      <div class="finished-icon">🎉</div>
      <h2>今日复习完成！</h2>
      <p>继续保持，明天再来</p>
      <button @click="loadDueCards" class="reload-btn">重新加载</button>
    </div>

    <!-- No cards due -->
    <div v-else class="empty-state">
      <div class="empty-icon">✅</div>
      <p>当前没有到期的闪卡</p>
      <button @click="loadDueCards" class="reload-btn">刷新</button>
    </div>
  </div>
</template>

<style scoped>
.page-title { font-size: 24px; font-weight: 700; margin-bottom: 4px; }
.desc { color: var(--color-text-secondary); margin-bottom: 24px; }

.stats-bar {
  display: flex;
  gap: 16px;
  margin-bottom: 32px;
}

.stat-item {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 12px 20px;
  text-align: center;
  flex: 1;
}

.stat-item.highlight {
  border-color: var(--color-primary);
  background: #eff6ff;
}

.stat-num { font-size: 24px; font-weight: 700; color: var(--color-primary); display: block; }
.stat-label { font-size: 12px; color: var(--color-text-secondary); }

.generate-section {
  text-align: center;
  padding: 40px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
}

.generate-buttons { display: flex; gap: 12px; justify-content: center; margin-top: 16px; }

.gen-btn {
  padding: 12px 20px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-bg);
  font-size: 14px;
  cursor: pointer;
  transition: all 0.15s;
}

.gen-btn:hover { border-color: var(--color-primary); background: #eff6ff; }

/* Study area */
.study-area { max-width: 600px; margin: 0 auto; }

.progress-bar {
  position: relative;
  height: 24px;
  background: var(--color-border);
  border-radius: 12px;
  overflow: hidden;
  margin-bottom: 24px;
}

.progress-fill {
  height: 100%;
  background: var(--color-primary);
  border-radius: 12px;
  transition: width 0.3s;
}

.progress-text {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text);
}

/* Flashcard */
.flashcard {
  perspective: 1000px;
  cursor: pointer;
  margin-bottom: 24px;
}

.card-inner {
  position: relative;
  min-height: 280px;
  transition: transform 0.5s;
  transform-style: preserve-3d;
}

.flashcard.flipped .card-inner {
  transform: rotateY(180deg);
}

.card-front, .card-back {
  position: absolute;
  inset: 0;
  backface-visibility: hidden;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 32px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}

.card-back {
  transform: rotateY(180deg);
  background: #f8fafc;
}

.card-type {
  position: absolute;
  top: 12px;
  left: 16px;
  font-size: 12px;
  color: var(--color-text-secondary);
  display: flex;
  align-items: center;
  gap: 4px;
}

.card-content {
  font-size: 18px;
  line-height: 1.6;
  text-align: center;
  white-space: pre-wrap;
  max-width: 100%;
}

.flip-hint {
  position: absolute;
  bottom: 16px;
  font-size: 12px;
  color: var(--color-text-secondary);
}

/* Review buttons */
.review-buttons {
  text-align: center;
}

.review-prompt { font-size: 14px; color: var(--color-text-secondary); margin-bottom: 12px; }

.btn-row { display: flex; gap: 8px; justify-content: center; }

.review-btn {
  padding: 12px 16px;
  border: none;
  border-radius: var(--radius);
  cursor: pointer;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  transition: opacity 0.15s;
  min-width: 100px;
}

.review-btn:hover { opacity: 0.85; }
.review-btn .btn-label { font-size: 14px; font-weight: 600; }
.review-btn .btn-hint { font-size: 11px; opacity: 0.7; }

.review-btn.again { background: #fee2e2; color: #dc2626; }
.review-btn.hard { background: #fef3c7; color: #d97706; }
.review-btn.good { background: #dcfce7; color: #16a34a; }
.review-btn.easy { background: #dbeafe; color: #2563eb; }

/* States */
.finished-state, .empty-state {
  text-align: center;
  padding: 60px 0;
}

.finished-icon, .empty-icon { font-size: 48px; margin-bottom: 16px; }

.finished-state h2 { font-size: 20px; font-weight: 600; margin-bottom: 8px; }
.finished-state p { color: var(--color-text-secondary); margin-bottom: 20px; }

.reload-btn {
  padding: 8px 20px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
  cursor: pointer;
  font-size: 14px;
}

.reload-btn:hover { border-color: var(--color-primary); }
</style>
