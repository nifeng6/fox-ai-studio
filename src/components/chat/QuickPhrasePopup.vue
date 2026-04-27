<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { ElMessageBox } from 'element-plus'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  visible: boolean
  phrases: string[]
}>()

const emit = defineEmits<{
  (e: 'select', phrase: string): void
  (e: 'close'): void
  (e: 'add', phrase: string): void
}>()

const { t } = useI18n()
const filter = ref('')
const selected = ref(-1)
const listRef = ref<HTMLDivElement | null>(null)

const filtered = computed(() => {
  const q = filter.value.trim().toLowerCase()
  if (!q) return props.phrases
  return props.phrases.filter(p => p.toLowerCase().includes(q))
})

watch(
  () => props.visible,
  v => {
    if (v) {
      filter.value = ''
      selected.value = -1
    }
  }
)

watch(filtered, () => {
  if (selected.value >= filtered.value.length) selected.value = -1
})

function onKeydown(e: KeyboardEvent) {
  if (!props.visible) return
  if (e.key === 'Escape') {
    e.preventDefault()
    emit('close')
    return
  }
  const list = filtered.value
  if (!list.length) return
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    selected.value = (selected.value + 1) % list.length
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    selected.value = (selected.value - 1 + list.length) % list.length
  } else if (e.key === 'Enter') {
    e.preventDefault()
    const p = list[selected.value]
    if (p) {
      emit('select', p)
      emit('close')
    }
  } else if (e.key === 'PageDown' && e.ctrlKey) {
    e.preventDefault()
    selected.value = Math.min(list.length - 1, selected.value + 1)
  } else if (e.key === 'PageUp' && e.ctrlKey) {
    e.preventDefault()
    selected.value = Math.max(0, selected.value - 1)
  }
}

function selectPhrase(phrase: string) {
  emit('select', phrase)
  emit('close')
}

async function onAdd() {
  try {
    const { value } = await ElMessageBox.prompt(
      t('chat.newPhrase') as string,
      t('chat.quickPhrases') as string,
      {
        inputPlaceholder: t('chat.phrasePlaceholder') as string,
        confirmButtonText: t('common.save') as string,
        cancelButtonText: t('common.cancel') as string
      }
    )
    const p = (value || '').trim()
    if (p) {
      emit('add', p)
      nextTick(() => {
        const idx = filtered.value.indexOf(p)
        if (idx >= 0) selected.value = idx
      })
    }
  } catch {
    /* cancel */
  }
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onUnmounted(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <div v-show="visible" class="quick-phrase-popup" role="listbox" aria-label="快捷短语">
    <div class="quick-phrase-popup__head">
      <input
        v-model="filter"
        type="search"
        class="quick-phrase-popup__input"
        :placeholder="(t('common.search') as string)"
        @keydown.stop
        @click.stop
      />
      <button
        type="button"
        class="quick-phrase-popup__add"
        :title="(t('common.add') as string)"
        @click="onAdd"
      >
        +
      </button>
    </div>
    <div ref="listRef" class="quick-phrase-popup__list">
      <button
        v-for="(phrase, i) in filtered"
        :key="phrase + i"
        type="button"
        data-phrase-item
        class="quick-phrase-popup__item"
        :class="{ 'is-active': i === selected }"
        @click="selectPhrase(phrase)"
      >
        {{ phrase }}
      </button>
      <div v-if="!filtered.length" class="quick-phrase-popup__empty">
        {{ t('common.noData') }}
      </div>
    </div>
    <div class="quick-phrase-popup__footer">
      快捷短语 &nbsp;ESC关闭 &nbsp;▲▼选择 &nbsp;↵插入
    </div>
  </div>
</template>

<style lang="scss" scoped>
.quick-phrase-popup {
  position: absolute;
  left: 0;
  bottom: 100%;
  margin-bottom: 8px;
  min-width: 240px;
  max-width: min(90vw, 360px);
  z-index: 30;
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-md);
  background: var(--modal-background, var(--color-background-soft));
  box-shadow: var(--shadow-md);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  max-height: 320px;
}

.quick-phrase-popup__head {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 8px 6px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-background-mute);
}

.quick-phrase-popup__input {
  flex: 1;
  min-width: 0;
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-sm);
  background: var(--color-background);
  color: var(--color-text-1);
  font-size: 12px;
  padding: 4px 8px;
  outline: none;
  transition: border-color 0.15s;
  &:focus {
    border-color: var(--color-text-3);
  }
  &::placeholder {
    color: var(--color-text-3);
  }
}

.quick-phrase-popup__add {
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-sm);
  background: var(--color-background);
  color: var(--color-text-1);
  font-size: 16px;
  line-height: 1;
  cursor: pointer;
  transition: background 0.12s;
  &:hover {
    background: var(--color-hover);
  }
}

.quick-phrase-popup__list {
  padding: 4px 0;
  flex: 1;
  min-height: 0;
  overflow: auto;
}

.quick-phrase-popup__item {
  display: block;
  width: 100%;
  padding: 8px 12px;
  border: none;
  background: transparent;
  color: var(--color-text-1);
  font-size: 13px;
  text-align: left;
  cursor: pointer;
  transition: background 0.12s;
  word-break: break-word;
  &:hover,
  &.is-active {
    background: var(--color-hover);
  }
}

.quick-phrase-popup__empty {
  padding: 16px 12px;
  text-align: center;
  color: var(--color-text-3);
  font-size: 12px;
}

.quick-phrase-popup__footer {
  padding: 6px 10px 8px;
  font-size: 10px;
  line-height: 1.4;
  color: var(--color-text-3);
  border-top: 1px solid var(--color-border);
  background: var(--color-background-mute);
  flex-shrink: 0;
}
</style>
