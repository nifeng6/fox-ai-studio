<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useChatStore } from '@/stores/chat'
import { useAssistantStore } from '@/stores/assistant'

const { t } = useI18n()
const router = useRouter()
const appStore = useAppStore()
const chatStore = useChatStore()
const assistantStore = useAssistantStore()

const query = ref('')
const selectedIdx = ref(0)
const results = ref<Array<{ type: string; title: string; id: string; description?: string }>>([])

watch(query, (val) => {
  selectedIdx.value = 0
  if (!val.trim()) { results.value = []; return }
  const q = val.toLowerCase()
  const items: typeof results.value = []
  for (const topic of chatStore.searchTopics(q)) {
    items.push({ type: 'topic', title: topic.title, id: topic.id, description: t('nav.chat') })
  }
  for (const a of assistantStore.filteredAssistants) {
    if (a.name.toLowerCase().includes(q) || a.description.toLowerCase().includes(q)) {
      items.push({ type: 'assistant', title: a.name, id: a.id, description: a.description })
    }
  }
  results.value = items.slice(0, 20)
})

function close() { appStore.globalSearchVisible = false }

function selectResult(item: typeof results.value[0]) {
  if (item.type === 'topic') { chatStore.selectTopic(item.id); router.push('/chat') }
  else if (item.type === 'assistant') { assistantStore.currentAssistantId = item.id; router.push('/assistants') }
  close()
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowDown') { e.preventDefault(); selectedIdx.value = Math.min(selectedIdx.value + 1, results.value.length - 1) }
  else if (e.key === 'ArrowUp') { e.preventDefault(); selectedIdx.value = Math.max(selectedIdx.value - 1, 0) }
  else if (e.key === 'Enter' && results.value.length) { selectResult(results.value[selectedIdx.value]) }
}
</script>

<template>
  <Teleport to="body">
    <div class="search-overlay" @click.self="close">
      <div class="search-dialog glass-heavy">
        <div class="search-input-wrapper">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
          </svg>
          <input v-model="query" class="search-input" :placeholder="t('common.search') + '...'" autofocus @keydown.esc="close" @keydown="onKeydown" />
          <kbd class="esc-hint">ESC</kbd>
        </div>
        <div v-if="results.length" class="search-results">
          <div v-for="(item, idx) in results" :key="item.id" class="search-result-item" :class="{ selected: idx === selectedIdx }" @click="selectResult(item)" @mouseenter="selectedIdx = idx">
            <span class="result-type">{{ item.type === 'topic' ? '💬' : '🤖' }}</span>
            <div class="result-info">
              <div class="result-title">{{ item.title }}</div>
              <div v-if="item.description" class="result-desc">{{ item.description }}</div>
            </div>
          </div>
        </div>
        <div v-else-if="query" class="search-empty">{{ t('common.noData') }}</div>
      </div>
    </div>
  </Teleport>
</template>

<style lang="scss" scoped>
.search-overlay {
  position: fixed; inset: 0; z-index: 9999;
  background: rgba(0, 0, 0, 0.4); backdrop-filter: blur(6px);
  display: flex; align-items: flex-start; justify-content: center;
  padding-top: 100px; animation: fade-scale-in 0.15s ease-out;
}
.search-dialog { width: 560px; max-height: 480px; border-radius: var(--fox-radius-lg); overflow: hidden; animation: slide-down-in 0.2s ease-out; }
.search-input-wrapper { display: flex; align-items: center; gap: 10px; padding: 14px 18px; border-bottom: 1px solid var(--color-border-soft); color: var(--color-text-3); }
.search-input { flex: 1; border: none; outline: none; font-size: 15px; background: transparent; color: var(--color-text-1); &::placeholder { color: var(--color-text-3); } }
.esc-hint { padding: 2px 6px; border-radius: 4px; border: 1px solid var(--color-border); background: var(--color-background-mute); color: var(--color-text-3); font-size: 10px; }
.search-results { max-height: 400px; overflow-y: auto; padding: 6px; }
.search-result-item { display: flex; align-items: center; gap: 10px; padding: 8px 10px; border-radius: var(--fox-radius-sm); cursor: pointer; &:hover, &.selected { background: var(--color-hover); } }
.result-type { font-size: 18px; flex-shrink: 0; }
.result-info { flex: 1; min-width: 0; }
.result-title { font-weight: 500; font-size: 13px; color: var(--color-text-1); }
.result-desc { font-size: 11px; color: var(--color-text-3); margin-top: 1px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.search-empty { padding: 40px; text-align: center; color: var(--color-text-3); font-size: 13px; }
</style>
