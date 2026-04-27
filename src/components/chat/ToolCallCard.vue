<script setup lang="ts">
import { computed, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAgentStore, type ToolCallStatus } from '@/stores/agent'
import { storeToRefs } from 'pinia'

const { t } = useI18n()

const props = defineProps<{
  id: string
  toolName: string
  args: Record<string, unknown> | string
  status: ToolCallStatus | 'done'
  result?: string
}>()

const emit = defineEmits<{
  (e: 'approved', id: string): void
  (e: 'rejected', id: string): void
}>()

const agent = useAgentStore()
const { permissionLevel, pendingToolCalls } = storeToRefs(agent)

const storeEntry = computed(() => pendingToolCalls.value.find(p => p.id === props.id))

const display = computed(() => {
  const s = storeEntry.value
  if (s) {
    return {
      toolName: s.toolName,
      args: s.args,
      status: s.status,
      result: s.result
    }
  }
  return {
    toolName: props.toolName,
    args: props.args,
    status: props.status,
    result: props.result
  }
})

const argsText = computed(() => {
  const a = display.value.args
  if (typeof a === 'string') {
    try { return JSON.stringify(JSON.parse(a), null, 2) } catch { return a }
  }
  try {
    return JSON.stringify(a, null, 2)
  } catch {
    return String(a)
  }
})

const canInteract = computed(
  () => permissionLevel.value === 'default' && display.value.status === 'pending'
)

function onApprove() {
  if (storeEntry.value) agent.approveToolCall(props.id)
  emit('approved', props.id)
}

function onReject() {
  if (storeEntry.value) agent.rejectToolCall(props.id)
  emit('rejected', props.id)
}

function autoIfFull() {
  if (permissionLevel.value !== 'full' || display.value.status !== 'pending') return
  if (storeEntry.value) agent.approveToolCall(props.id)
  emit('approved', props.id)
}

onMounted(() => {
  autoIfFull()
})

watch(permissionLevel, () => {
  autoIfFull()
})
</script>

<template>
  <div
    class="tool-call-card"
    :class="{
      'tool-call-card--pending': display.status === 'pending',
      'tool-call-card--approved': display.status === 'approved',
      'tool-call-card--rejected': display.status === 'rejected',
      'tool-call-card--running': display.status === 'running',
      'tool-call-card--completed': display.status === 'completed' || display.status === 'done',
      'tool-call-card--error': display.status === 'error'
    }"
  >
    <div class="tool-call-card__head">
      <span class="tool-call-card__name">{{ display.toolName }}</span>
      <span class="tool-call-card__status" :data-s="display.status">
        <span v-if="display.status === 'pending'" class="tool-call-card__dot tool-call-card__dot--y" />
        <span v-else-if="display.status === 'approved'" class="tool-call-card__dot tool-call-card__dot--g tool-call-card__flash" />
        <span v-else-if="display.status === 'rejected'" class="tool-call-card__dot tool-call-card__dot--r" />
        <span v-else-if="display.status === 'running'" class="tool-call-card__spinner" />
        <span v-else class="tool-call-card__check">✓</span>
        {{ display.status }}
      </span>
    </div>
    <details class="tool-call-card__args-wrap" open>
      <summary class="tool-call-card__args-label">{{ t('chat.toolArgs') }}</summary>
      <pre class="tool-call-card__args">{{ argsText }}</pre>
    </details>
    <div v-if="canInteract" class="tool-call-card__actions">
      <button type="button" class="tool-call-card__btn tool-call-card__btn--ok" @click="onApprove">
        {{ t('chat.approve') }}
      </button>
      <button type="button" class="tool-call-card__btn tool-call-card__btn--no" @click="onReject">
        {{ t('chat.reject') }}
      </button>
    </div>
    <div v-if="display.status === 'completed' && display.result" class="tool-call-card__result">
      <div class="tool-call-card__result-label">{{ t('chat.toolResult') }}</div>
      <pre class="tool-call-card__result-body">{{ display.result }}</pre>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.tool-call-card {
  margin: 8px 0;
  padding: 10px 12px;
  border-radius: var(--fox-radius-md);
  border: 1px solid var(--color-border);
  background: var(--color-background-mute);
  font-size: 12px;
  color: var(--color-text-1);
  max-width: 100%;
}

.tool-call-card__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 8px;
}

.tool-call-card__name {
  font-weight: 600;
  color: var(--color-text-1);
}

.tool-call-card__status {
  display: flex;
  align-items: center;
  gap: 4px;
  text-transform: capitalize;
  font-size: 11px;
  color: var(--color-text-2);
}

.tool-call-card__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  &--y {
    background: #eab308;
  }
  &--g {
    background: #22c55e;
  }
  &--r {
    background: #ef4444;
  }
}

.tool-call-card__flash {
  animation: flash-ok 0.6s ease;
}

@keyframes flash-ok {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.35;
    box-shadow: 0 0 0 3px rgba(34, 197, 94, 0.4);
  }
}

.tool-call-card__spinner {
  width: 12px;
  height: 12px;
  border: 2px solid var(--color-border);
  border-top-color: var(--fox-accent-border);
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.tool-call-card__check {
  color: #22c55e;
  font-weight: 700;
}

.tool-call-card__args-wrap {
  margin: 0;
}

.tool-call-card__args-label {
  list-style: none;
  font-size: 10px;
  color: var(--color-text-3);
  margin-bottom: 4px;
  cursor: pointer;
  user-select: none;
  &::-webkit-details-marker {
    display: none;
  }
}

.tool-call-card__args {
  margin: 0;
  padding: 8px;
  background: var(--color-background);
  border-radius: var(--fox-radius-sm);
  border: 1px solid var(--color-border);
  font-size: 11px;
  line-height: 1.4;
  color: var(--color-text-2);
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 160px;
  overflow: auto;
}

.tool-call-card__actions {
  display: flex;
  gap: 8px;
  margin-top: 10px;
}

.tool-call-card__btn {
  padding: 4px 12px;
  font-size: 12px;
  border-radius: var(--fox-radius-sm);
  cursor: pointer;
  border: 1px solid var(--color-border);
  transition: background 0.15s;
  &--ok {
    background: var(--color-background);
    color: var(--color-text-1);
    &:hover {
      background: var(--color-hover);
    }
  }
  &--no {
    background: var(--color-background);
    color: var(--color-error, #c00);
    &:hover {
      background: var(--color-hover);
    }
  }
}

.tool-call-card__result {
  margin-top: 10px;
  padding-top: 8px;
  border-top: 1px solid var(--color-border);
}

.tool-call-card__result-label {
  font-size: 10px;
  color: var(--color-text-3);
  margin-bottom: 4px;
}

.tool-call-card__result-body {
  margin: 0;
  padding: 8px;
  background: var(--color-background);
  border-radius: var(--fox-radius-sm);
  font-size: 11px;
  line-height: 1.4;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 200px;
  overflow: auto;
}
</style>
