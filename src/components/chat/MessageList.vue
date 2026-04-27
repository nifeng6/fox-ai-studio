<script setup lang="ts">
import { ref, watch, nextTick, onMounted, onActivated } from 'vue'
import type { Message } from '@/types'
import MessageItem from './MessageItem.vue'

const props = defineProps<{
  messages: Message[]
}>()

const emit = defineEmits<{
  (e: 'edit', message: Message): void
  (e: 'regenerate', message: Message): void
  (e: 'delete', message: Message): void
  (e: 'tool-approved', toolCallId: string): void
  (e: 'tool-rejected', toolCallId: string): void
}>()

const rootRef = ref<HTMLElement | null>(null)

function isNearBottom(): boolean {
  const el = rootRef.value
  if (!el) return true
  return el.scrollHeight - el.scrollTop - el.clientHeight < 120
}

function scrollToBottom(force = false) {
  nextTick(() => {
    const el = rootRef.value
    if (!el) return
    if (force || isNearBottom()) {
      el.scrollTop = el.scrollHeight
    }
  })
}

watch(
  () => props.messages.length,
  () => scrollToBottom(true)
)

watch(
  () => props.messages,
  () => scrollToBottom(),
  { deep: true }
)

onMounted(() => {
  nextTick(() => nextTick(() => scrollToBottom(true)))
})

onActivated(() => {
  nextTick(() => scrollToBottom(true))
})
</script>

<template>
  <div ref="rootRef" class="message-list" role="log" aria-relevant="additions text">
    <div class="message-list__inner">
      <MessageItem
        v-for="m in messages"
        :key="m.id"
        :message="m"
        @edit="emit('edit', $event)"
        @regenerate="emit('regenerate', $event)"
        @delete="emit('delete', $event)"
        @tool-approved="emit('tool-approved', $event)"
        @tool-rejected="emit('tool-rejected', $event)"
      />
    </div>
  </div>
</template>

<style lang="scss" scoped>
.message-list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 16px 20px 8px;
  scroll-behavior: smooth;
}

.message-list__inner {
  max-width: 900px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 14px;
}
</style>
