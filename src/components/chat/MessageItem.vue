<script setup lang="ts">
import { ref, computed, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import type { Message } from '@/types'
import MarkdownRenderer from '@/components/markdown/MarkdownRenderer.vue'
import ToolCallCard from './ToolCallCard.vue'

const thinkingOpen = reactive<Record<string, boolean>>({})

const props = defineProps<{
  message: Message
}>()

const emit = defineEmits<{
  (e: 'edit', message: Message): void
  (e: 'regenerate', message: Message): void
  (e: 'delete', message: Message): void
  (e: 'tool-approved', toolCallId: string): void
  (e: 'tool-rejected', toolCallId: string): void
}>()

const { t } = useI18n()
const isHover = ref(false)

const isUser = computed(() => props.message.role === 'user')
const isAssistant = computed(() => props.message.role === 'assistant')
const isTool = computed(() => props.message.role === 'tool')
const isError = computed(() => props.message.status === 'error')
const isStreaming = computed(() => props.message.status === 'streaming')
const hasParts = computed(() => props.message.parts && props.message.parts.length > 0)
const partsHaveText = computed(() => props.message.parts?.some(p => p.type === 'text' && p.content) ?? false)

const sortedParts = computed(() => {
  if (!props.message.parts?.length) return []
  const order: Record<string, number> = {
    reasoning: 0,
    step: 1,
    tool_call: 2,
    tool_result: 3,
    text: 4,
    error: 5,
  }
  return [...props.message.parts].sort((a, b) => (order[a.type] ?? 9) - (order[b.type] ?? 9))
})

const copyableText = computed(() => {
  const parts = props.message.parts
  if (props.message.role === 'assistant' && parts?.length) {
    return parts
      .map(p => {
        if (p.type === 'text' && p.content) return p.content
        if (p.type === 'tool_result' && p.content) return p.content
        if (p.type === 'reasoning' && p.content) return p.content
        if (p.type === 'error' && p.content) return p.content
        if (p.type === 'step') return p.stepTitle || p.content || ''
        if (p.type === 'tool_call' && p.toolResult) return p.toolResult
        return ''
      })
      .filter(Boolean)
      .join('\n\n')
  }
  return props.message.content || ''
})

function onToolApproved(id: string) {
  emit('tool-approved', id)
}

function onToolRejected(id: string) {
  emit('tool-rejected', id)
}

async function onCopy() {
  try {
    await navigator.clipboard.writeText(copyableText.value)
    ElMessage.success(t('common.copied'))
  } catch {
    ElMessage.error(t('common.error'))
  }
}
</script>

<template>
  <div
    v-if="!isTool"
    class="message-item"
    :class="{
      'message-item--user': isUser,
      'message-item--assistant': isAssistant,
      'message-item--error': isError
    }"
    @mouseenter="isHover = true"
    @mouseleave="isHover = false"
  >
    <div class="message-item__row">
      <div class="message-item__bubble" :class="{ 'message-item__bubble--user': isUser, 'message-item__bubble--sys': !isUser && !isAssistant }">
        <div v-if="isAssistant && message.agentName" class="message-item__agent">{{ message.agentName }}</div>

        <!-- User message -->
        <div v-if="isUser" class="message-item__text">
          {{ message.content }}
          <div v-if="message.attachments?.length" class="message-item__attachments">
            <template v-for="att in message.attachments" :key="att.id">
              <img v-if="att.type?.startsWith('image/')" :src="att.url" :alt="att.name" class="message-item__attachment-img" />
              <div v-else class="message-item__attachment-file">{{ att.name }}</div>
            </template>
          </div>
        </div>

        <!-- Assistant with parts -->
        <template v-else-if="isAssistant && hasParts">
          <!-- Only show message.content if parts don't already contain text -->
          <div v-if="message.content && !partsHaveText" class="message-item__md">
            <MarkdownRenderer :content="message.content" />
          </div>
          <template v-for="(part, pi) in sortedParts" :key="part.id || pi">
            <!-- Text part -->
            <div v-if="part.type === 'text' && part.content" class="message-item__md">
              <MarkdownRenderer :content="part.content" />
            </div>
            <!-- Tool call part -->
            <div v-else-if="part.type === 'tool_call'" class="message-item__tool">
              <ToolCallCard
                :id="part.toolCallId || part.id"
                :tool-name="part.toolName || 'unknown'"
                :args="part.toolArgs || {}"
                :status="(part.toolStatus as any) || 'pending'"
                :result="part.toolResult"
                @approved="onToolApproved"
                @rejected="onToolRejected"
              />
            </div>
            <!-- Tool result part -->
            <div v-else-if="part.type === 'tool_result'" class="message-item__tool-result">
              <div class="tool-result-header">
                <span class="tool-result-icon">✓</span>
                <span class="tool-result-name">{{ part.toolName || '工具结果' }}</span>
              </div>
              <pre v-if="part.content" class="tool-result-body">{{ part.content.length > 500 ? part.content.slice(0, 500) + '...' : part.content }}</pre>
            </div>
            <!-- Reasoning/thinking part -->
            <div v-else-if="part.type === 'reasoning'" class="message-item__thinking">
              <div class="thinking-summary" @click="thinkingOpen[part.id || pi] = !thinkingOpen[part.id || pi]">
                <svg class="thinking-chevron" :class="{ 'thinking-chevron--open': thinkingOpen[part.id || pi] }" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 18 15 12 9 6" /></svg>
                思考过程
              </div>
              <Transition name="slide-fade">
                <div v-show="thinkingOpen[part.id || pi]" class="thinking-content">
                  <MarkdownRenderer :content="part.content || ''" />
                </div>
              </Transition>
            </div>
            <!-- Step indicator -->
            <div v-else-if="part.type === 'step'" class="message-item__step">
              <span class="step-icon" :class="{ 'step-icon--running': part.stepStatus === 'running', 'step-icon--done': part.stepStatus === 'completed', 'step-icon--err': part.stepStatus === 'error' }">
                <span v-if="part.stepStatus === 'running'" class="step-spinner" />
                <span v-else-if="part.stepStatus === 'completed'">✓</span>
                <span v-else>✗</span>
              </span>
              <span class="step-title">{{ part.stepTitle || part.content || '步骤' }}</span>
            </div>
            <!-- Error part -->
            <div v-else-if="part.type === 'error'" class="message-item__part-error">
              {{ part.content }}
            </div>
          </template>
        </template>

        <!-- Assistant without parts (backward compatible) -->
        <div v-else-if="isAssistant" class="message-item__md">
          <MarkdownRenderer :content="message.content" />
        </div>

        <!-- System message -->
        <div v-else class="message-item__text message-item__text--system">{{ message.content }}</div>

        <div v-if="isStreaming && isAssistant" class="message-item__stream">
          <span class="message-item__dot" />
          <span class="message-item__dot" />
          <span class="message-item__dot" />
        </div>

        <p v-if="isError && message.error" class="message-item__err">{{ message.error }}</p>
      </div>
    </div>

    <div
      class="message-item__actions"
      :class="{ 'message-item__actions--user': isUser, 'message-item__actions--visible': isHover }"
    >
      <button type="button" class="msg-btn" :title="t('chat.copy')" @click="onCopy">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
          <rect x="9" y="9" width="13" height="13" rx="2" />
          <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" />
        </svg>
      </button>
      <button v-if="isUser" type="button" class="msg-btn" :title="t('chat.edit')" @click="emit('edit', message)">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
          <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7" />
          <path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z" />
        </svg>
      </button>
      <button v-if="isAssistant" type="button" class="msg-btn" :title="t('chat.regenerate')" @click="emit('regenerate', message)">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
          <path d="M23 4v6h-6" />
          <path d="M20.49 15a9 9 0 11-2.12-5.86L23 10" />
        </svg>
      </button>
      <button type="button" class="msg-btn msg-btn--danger" :title="t('chat.delete')" @click="emit('delete', message)">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
          <polyline points="3 6 5 6 21 6" />
          <path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2" />
        </svg>
      </button>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.message-item {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.message-item--user .message-item__row {
  align-self: flex-end;
  max-width: min(100%, 720px);
}

.message-item--assistant .message-item__row,
.message-item:not(.message-item--user) .message-item__row {
  align-self: flex-start;
  max-width: min(100%, 720px);
}

.message-item__agent {
  font-size: 11px;
  font-weight: 600;
  color: var(--color-text-3);
  margin-bottom: 6px;
  letter-spacing: 0.02em;
}

.message-item__row {
  display: flex;
  width: 100%;
  justify-content: flex-start;
}

.message-item--user .message-item__row {
  justify-content: flex-end;
}

.message-item__bubble {
  border-radius: var(--fox-radius-md);
  border: 1px solid var(--color-border);
  background: var(--color-background-mute);
  color: var(--color-text-1);
  box-shadow: var(--shadow-sm);
  padding: 10px 14px;
  font-size: var(--fox-fs, 14px);
  line-height: 1.55;
  transition: border-color 0.2s var(--fox-ease), box-shadow 0.2s var(--fox-ease);
}

.message-item__bubble--user {
  background: var(--color-background-soft);
  border-color: var(--color-border);
}

.message-item__bubble--user:hover,
.message-item__bubble--sys:hover,
.message-item__md:hover {
  border-color: var(--color-text-3);
}

.message-item__text {
  white-space: pre-wrap;
  word-break: break-word;
}

.message-item__text--system {
  color: var(--color-text-2);
  font-size: 12px;
}

.message-item__attachments {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 8px;
}

.message-item__attachment-img {
  max-width: 300px;
  max-height: 200px;
  border-radius: 8px;
  object-fit: contain;
  cursor: pointer;
}

.message-item__attachment-file {
  padding: 4px 10px;
  background: var(--color-fill-3);
  border-radius: 6px;
  font-size: 12px;
  color: var(--color-text-2);
}

.message-item--error .message-item__bubble {
  border-color: rgba(239, 68, 68, 0.4);
}

.message-item__err {
  margin: 8px 0 0;
  color: var(--color-error);
  font-size: 12px;
}

.message-item__stream {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: 8px;
  padding: 0 2px;
}

.message-item__dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--color-text-3);
  animation: msg-bounce 1.1s ease-in-out infinite;
}

.message-item__dot:nth-child(2) { animation-delay: 0.12s; }
.message-item__dot:nth-child(3) { animation-delay: 0.24s; }

@keyframes msg-bounce {
  0%, 100% { transform: translateY(0); opacity: 0.4; }
  50% { transform: translateY(-3px); opacity: 1; }
}

.message-item__actions {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 2px 0;
  align-self: flex-start;
  opacity: 0;
  transition: opacity 0.2s ease;
  height: 30px;
}

.message-item__actions--visible {
  opacity: 1;
}

.message-item__actions--user {
  align-self: flex-end;
}

.msg-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-sm);
  background: var(--color-background-mute);
  color: var(--color-text-2);
  cursor: pointer;
  transition: color 0.15s, background 0.15s, border-color 0.15s, transform 0.1s;
}

.msg-btn:hover {
  color: var(--color-text-1);
  background: var(--color-hover);
  border-color: var(--color-text-3);
}

.msg-btn:active {
  background: var(--color-active);
  transform: scale(0.97);
}

.msg-btn--danger:hover {
  color: var(--color-error);
  border-color: rgba(239, 68, 68, 0.45);
  background: rgba(239, 68, 68, 0.08);
}

:deep(.message-item__md) {
  min-width: 0;
}

.message-item__tool {
  margin: 6px 0;
  animation: partEnter 0.3s cubic-bezier(.4,0,.2,1) both;
}

.message-item__tool-result {
  margin: 6px 0;
  padding: 8px 10px;
  border-radius: var(--fox-radius-sm);
  border: 1px solid rgba(34, 197, 94, 0.25);
  background: rgba(34, 197, 94, 0.04);
  font-size: 12px;
  animation: partEnter 0.3s cubic-bezier(.4,0,.2,1) both;
}
.tool-result-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
  font-weight: 500;
  color: #22c55e;
}
.tool-result-icon { font-size: 14px; }
.tool-result-name { font-size: 12px; }
.tool-result-body {
  margin: 0;
  padding: 6px 8px;
  background: var(--color-background);
  border-radius: var(--fox-radius-sm);
  font-size: 11px;
  line-height: 1.4;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 160px;
  overflow: auto;
  color: var(--color-text-2);
}

.message-item__thinking {
  margin: 6px 0;
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-sm);
  overflow: hidden;
}
.thinking-summary {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  cursor: pointer;
  font-size: 12px;
  color: var(--color-text-3);
  background: var(--color-background-mute);
  user-select: none;
  transition: color 0.15s ease;
  &:hover { color: var(--color-text-2); }
}
.thinking-chevron {
  transition: transform 0.25s cubic-bezier(.4,0,.2,1);
  flex-shrink: 0;
}
.thinking-chevron--open {
  transform: rotate(90deg);
}
.thinking-content {
  padding: 8px 12px;
  font-size: 13px;
  border-top: 1px solid var(--color-border);
}

/* Expand/collapse animation */
.slide-fade-enter-active {
  transition: all 0.25s cubic-bezier(.4,0,.2,1);
}
.slide-fade-leave-active {
  transition: all 0.2s cubic-bezier(.4,0,.2,1);
}
.slide-fade-enter-from {
  max-height: 0;
  opacity: 0;
  padding-top: 0;
  padding-bottom: 0;
  overflow: hidden;
}
.slide-fade-enter-to {
  max-height: 600px;
  opacity: 1;
}
.slide-fade-leave-from {
  max-height: 600px;
  opacity: 1;
}
.slide-fade-leave-to {
  max-height: 0;
  opacity: 0;
  padding-top: 0;
  padding-bottom: 0;
  overflow: hidden;
}

.message-item__step {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  margin: 4px 0;
  border-radius: var(--fox-radius-sm);
  background: var(--color-background-mute);
  font-size: 12px;
  color: var(--color-text-2);
  animation: partEnter 0.3s cubic-bezier(.4,0,.2,1) both;
}

@keyframes partEnter {
  from {
    opacity: 0;
    transform: translateY(6px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
.step-icon {
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  font-size: 10px;
  flex-shrink: 0;
  &--running { color: var(--color-text-3); }
  &--done { color: #22c55e; background: rgba(34, 197, 94, 0.1); }
  &--err { color: #ef4444; background: rgba(239, 68, 68, 0.1); }
}
.step-spinner {
  width: 12px;
  height: 12px;
  border: 2px solid var(--color-border);
  border-top-color: var(--color-text-2);
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
.step-title { flex: 1; }

.message-item__part-error {
  margin: 4px 0;
  padding: 6px 10px;
  border-radius: var(--fox-radius-sm);
  background: rgba(239, 68, 68, 0.06);
  border: 1px solid rgba(239, 68, 68, 0.2);
  color: var(--color-error);
  font-size: 12px;
}
</style>
