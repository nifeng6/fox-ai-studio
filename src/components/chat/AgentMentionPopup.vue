<script setup lang="ts">
import { computed, inject, type ComputedRef } from 'vue'
import { useChatStore } from '@/stores/chat'
import type { AgentConfig } from '@/types'

defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  (e: 'select', agentId: string, agentName: string): void
  (e: 'close'): void
}>()

const chat = useChatStore()
const topicAgents = inject<ComputedRef<AgentConfig[]>>('topicAgents')

const availableAgents = computed(() => {
  const ta = topicAgents?.value || []
  const topic = chat.currentTopic
  if (!topic?.isGroupChat) return ta
  return ta.filter(
    a => topic.participantAgentIds?.includes(a.id) || a.type === 'main'
  )
})

function selectAgent(a: { id: string; name: string }) {
  emit('select', a.id, a.name)
}
</script>

<template>
  <div v-if="visible" class="mention-popup">
    <div class="mention-title">选择 Agent</div>
    <div
      v-for="a in availableAgents"
      :key="a.id"
      class="mention-item"
      @click="selectAgent(a)"
    >
      <span class="mention-avatar">{{ a.type === 'main' ? '🤖' : '🔧' }}</span>
      <span class="mention-name">{{ a.name }}</span>
      <span class="mention-type">{{ a.type === 'main' ? '主Agent' : '子Agent' }}</span>
    </div>
    <div v-if="!availableAgents.length" class="mention-empty">
      暂无可用 Agent
    </div>
  </div>
</template>

<style lang="scss" scoped>
.mention-popup {
  position: absolute;
  bottom: 100%;
  left: 0;
  margin-bottom: 8px;
  min-width: 220px;
  max-height: 240px;
  overflow: auto;
  background: var(--color-background);
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-md, 8px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
  z-index: 20;
  padding: 6px 0;
}

.mention-title {
  padding: 6px 14px;
  font-size: 11px;
  color: var(--color-text-3);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.mention-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  cursor: pointer;
  transition: background 0.1s;

  &:hover {
    background: var(--color-hover);
  }
}

.mention-avatar {
  font-size: 16px;
  width: 24px;
  text-align: center;
}

.mention-name {
  font-size: 13px;
  color: var(--color-text-1);
  flex: 1;
}

.mention-type {
  font-size: 11px;
  color: var(--color-text-3);
}

.mention-empty {
  padding: 12px 14px;
  font-size: 12px;
  color: var(--color-text-3);
  text-align: center;
}
</style>
