<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, nextTick } from 'vue'
import type { WebSearchEngine } from '@/stores/agent'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  (e: 'select', engine: WebSearchEngine): void
  (e: 'close'): void
}>()

const engines: { id: WebSearchEngine; label: string; icon: string }[] = [
  { id: 'examcp', label: 'ExaMCP', icon: '🔍' },
  { id: 'google', label: 'Google', icon: 'G' },
  { id: 'bing', label: 'Bing', icon: '🔎' },
  { id: 'baidu', label: 'Baidu', icon: '百' }
]

const selected = ref(-1)
const listRef = ref<HTMLDivElement | null>(null)

watch(
  () => props.visible,
  v => {
    if (v) {
      selected.value = -1
    }
  }
)

function onKeydown(e: KeyboardEvent) {
  if (!props.visible) return
  if (e.key === 'Escape') {
    e.preventDefault()
    emit('close')
    return
  }
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    selected.value = selected.value < engines.length - 1 ? selected.value + 1 : 0
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    selected.value = selected.value <= 0 ? engines.length - 1 : selected.value - 1
  } else if (e.key === 'Enter') {
    e.preventDefault()
    if (selected.value >= 0) {
      emit('select', engines[selected.value].id)
      emit('close')
    }
  } else if (e.key === 'PageDown' && e.ctrlKey) {
    e.preventDefault()
    selected.value = Math.min(engines.length - 1, selected.value + 1)
  } else if (e.key === 'PageUp' && e.ctrlKey) {
    e.preventDefault()
    selected.value = Math.max(0, selected.value - 1)
  }
}

function pick(i: number) {
  selected.value = i
}

function selectEngine(i: number) {
  pick(i)
  emit('select', engines[i].id)
  emit('close')
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onUnmounted(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <div v-show="visible" class="web-search-popup" role="listbox" aria-label="网络搜索">
    <div ref="listRef" class="web-search-popup__list">
      <button
        v-for="(eng, i) in engines"
        :key="eng.id"
        type="button"
        data-engine-item
        class="web-search-popup__item"
        :class="{ 'is-active': i === selected }"
        @click="selectEngine(i)"
      >
        <span class="web-search-popup__icon" aria-hidden="true">
          <span v-if="eng.id === 'google'" class="web-search-popup__g">G</span>
          <template v-else>{{ eng.icon }}</template>
        </span>
        <span class="web-search-popup__name">{{ eng.label }}</span>
        <span class="web-search-popup__badge">免费</span>
      </button>
    </div>
    <div class="web-search-popup__footer">
      网络搜索 &nbsp;ESC关闭 &nbsp;▲▼选择 &nbsp;Ctrl+▲▼翻页 &nbsp;↵确认
    </div>
  </div>
</template>

<style lang="scss" scoped>
.web-search-popup {
  position: absolute;
  left: 0;
  bottom: 100%;
  margin-bottom: 8px;
  min-width: 220px;
  z-index: 30;
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-md);
  background: var(--modal-background, var(--color-background-soft));
  box-shadow: var(--shadow-md);
  overflow: hidden;
}

.web-search-popup__list {
  padding: 6px 0;
  max-height: 240px;
  overflow: auto;
}

.web-search-popup__item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px 12px;
  border: none;
  background: transparent;
  color: var(--color-text-1);
  font-size: 13px;
  cursor: pointer;
  text-align: left;
  transition: background 0.12s;

  &:hover,
  &.is-active {
    background: var(--color-hover);
  }
}

.web-search-popup__icon {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--fox-radius-sm);
  background: var(--color-background-mute);
  font-size: 12px;
  flex-shrink: 0;
}

.web-search-popup__g {
  font-weight: 800;
  font-size: 13px;
  color: #4285f4;
  font-family: system-ui, sans-serif;
}

.web-search-popup__name {
  flex: 1;
  min-width: 0;
}

.web-search-popup__badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: var(--fox-radius-sm);
  background: var(--color-background-mute);
  color: var(--color-text-2);
  border: 1px solid var(--color-border);
  flex-shrink: 0;
}

.web-search-popup__footer {
  padding: 6px 10px 8px;
  font-size: 10px;
  line-height: 1.4;
  color: var(--color-text-3);
  border-top: 1px solid var(--color-border);
  background: var(--color-background-mute);
  white-space: normal;
  word-break: break-all;
}
</style>
