<script setup lang="ts">
import { computed, ref, inject, type ComputedRef } from 'vue'
import { useI18n } from 'vue-i18n'
import { storeToRefs } from 'pinia'
import { useAgentStore, type WebSearchEngine } from '@/stores/agent'
import { useChatStore } from '@/stores/chat'
import { useSettingsStore } from '@/stores/settings'
import type { Attachment, Message, AgentConfig } from '@/types'
import ModelSelector from './ModelSelector.vue'
import AgentPanel from './AgentPanel.vue'
import MessageList from './MessageList.vue'
import MessageInput from './MessageInput.vue'

const agent = useAgentStore()
const chat = useChatStore()
const settingsStore = useSettingsStore()
const { currentAgent } = storeToRefs(agent)
const topicAgents = inject<ComputedRef<AgentConfig[]>>('topicAgents')

const enabledChannels = computed(() => settingsStore.channels.filter(c => c.enabled))
const topicChannelId = computed({
  get: () => chat.currentTopic?.channelId || null,
  set: (v: string | null) => {
    if (chat.currentTopicId) chat.setTopicChannel(chat.currentTopicId, v)
  }
})

const statusLabel = computed(() => {
  const s = currentAgent.value?.status
  const map: Record<string, string> = {
    idle: '就绪',
    thinking: '思考中',
    awaiting_tool_approval: '等待确认',
    tool_running: '执行工具',
    streaming: '生成中',
    paused: '已暂停',
    error: '错误',
    completed: '完成'
  }
  return map[s || 'idle'] || s || '就绪'
})

const props = defineProps<{
  topicTitle: string
  hasTopic: boolean
  messages: Message[]
  streaming: boolean
  providerId: string
  modelId: string
  isGroupChat?: boolean
}>()

const emit = defineEmits<{
  (e: 'send', content: string, attachments: Attachment[]): void
  (e: 'stop'): void
  (e: 'edit', message: Message): void
  (e: 'regenerate', message: Message): void
  (e: 'delete', message: Message): void
  (e: 'tool-approved', toolCallId: string): void
  (e: 'tool-rejected', toolCallId: string): void
  (e: 'update:providerId', v: string): void
  (e: 'update:modelId', v: string): void
  (e: 'new-topic'): void
  (e: 'clear-context'): void
  (e: 'clear-messages'): void
  (e: 'web-search', engine: WebSearchEngine): void
  (e: 'quick-phrase', phrase: string): void
  (e: 'mention', agentId: string, agentName: string): void
}>()

const { t } = useI18n()

const activeTab = computed(() => chat.currentTopic?.activeTab || 'group')
const isGroupTab = computed(() => activeTab.value === 'group')

const agentTabs = computed(() => {
  const tabs: Array<{ id: string; label: string; type: string }> = []
  tabs.push({ id: 'group', label: '群聊', type: 'group' })
  const ta = topicAgents?.value || []
  const main = ta.find(a => a.type === 'main')
  if (main) {
    tabs.push({ id: main.id, label: main.name, type: 'main' })
    for (const s of ta.filter(a => a.type === 'sub' && a.parentId === main.id)) {
      tabs.push({ id: s.id, label: s.name, type: 'sub' })
    }
  }
  return tabs
})

const filteredMessages = computed(() => {
  if (!props.hasTopic) return []
  if (isGroupTab.value) return props.messages
  const aid = activeTab.value
  return props.messages.filter(m => {
    if (m.role === 'system') return true
    if (m.role === 'user') {
      if (!m.agentId) return true
      return m.agentId === aid
    }
    if (m.role === 'assistant') {
      return m.agentId === aid || !m.agentId
    }
    return true
  })
})

function onSwitchTab(tabId: string) {
  if (!chat.currentTopicId) return
  chat.setActiveTab(chat.currentTopicId, tabId)
  if (tabId !== 'group') {
    agent.switchAgent(tabId)
  } else {
    const ta = topicAgents?.value || []
    const main = ta.find(a => a.type === 'main')
    if (main) agent.switchAgent(main.id)
  }
}
</script>

<template>
  <div class="chat-view">
    <header class="chat-view__head">
      <div class="chat-view__head-text">
        <div class="chat-view__title-row">
          <h1 class="chat-view__title" :title="hasTopic ? topicTitle : t('chat.noTopic')">
            {{ hasTopic ? topicTitle : t('chat.noTopic') }}
          </h1>
          <span
            v-if="hasTopic && currentAgent"
            class="agent-status"
            :class="`agent-status--${currentAgent.status}`"
          >
            <span class="agent-status__dot" />
            <span class="agent-status__text">{{ statusLabel }}</span>
          </span>
        </div>
        <p v-if="!hasTopic" class="chat-view__subtitle">{{ t('chat.emptyHint') }}</p>
      </div>
      <div v-if="hasTopic" class="chat-view__head-right">
        <div class="chat-view__head-agent">
          <AgentPanel />
        </div>
        <div class="chat-view__head-actions">
          <el-tooltip :content="t('chat.clearContext')" placement="bottom">
            <button type="button" class="head-icon-btn" @click="emit('clear-context')">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <path d="M3 3l7 7" />
                <path d="M3 10l7-7" />
                <path d="M20 3l-1 2" />
                <path d="M4 20l4-4" />
                <path d="M6 6l-3 3a2 2 0 0 0 0 2.8L9.2 20a2.2 2.2 0 0 0 3.1 0L20 12" />
              </svg>
            </button>
          </el-tooltip>
          <el-tooltip :content="t('chat.clearMessages')" placement="bottom">
            <button type="button" class="head-icon-btn" @click="emit('clear-messages')">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <polyline points="3 6 5 6 21 6" />
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
              </svg>
            </button>
          </el-tooltip>
        </div>
        <div v-if="enabledChannels.length" class="chat-view__head-channel">
          <el-select
            :model-value="topicChannelId"
            placeholder="关联渠道"
            clearable
            size="small"
            class="channel-select"
            @update:model-value="topicChannelId = $event"
          >
            <template #prefix>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 11a9 9 0 0 1 9 9" /><path d="M4 4a16 16 0 0 1 16 16" /><circle cx="5" cy="19" r="1" /></svg>
            </template>
            <el-option
              v-for="ch in enabledChannels"
              :key="ch.id"
              :label="ch.name"
              :value="ch.id"
            />
          </el-select>
        </div>
        <div class="chat-view__head-model">
          <ModelSelector
            :provider-id="providerId"
            :model-id="modelId"
            @update:provider-id="emit('update:providerId', $event)"
            @update:model-id="emit('update:modelId', $event)"
          />
        </div>
      </div>
    </header>

    <!-- Agent Tabs -->
    <div v-if="hasTopic" class="chat-view__tabs">
      <button
        v-for="tab in agentTabs"
        :key="tab.id"
        type="button"
        class="chat-tab"
        :class="{
          'chat-tab--active': activeTab === tab.id,
          'chat-tab--group': tab.type === 'group',
          'chat-tab--main': tab.type === 'main',
          'chat-tab--sub': tab.type === 'sub'
        }"
        @click="onSwitchTab(tab.id)"
      >
        <svg v-if="tab.type === 'group'" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
          <circle cx="9" cy="7" r="4" />
          <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
          <path d="M16 3.13a4 4 0 0 1 0 7.75" />
        </svg>
        <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
          <circle cx="12" cy="7" r="4" />
        </svg>
        <span class="chat-tab__label">{{ tab.label }}</span>
      </button>
    </div>

    <div class="chat-view__body">
      <template v-if="hasTopic">
        <MessageList
          v-if="filteredMessages.length"
          :messages="filteredMessages"
          @edit="emit('edit', $event)"
          @regenerate="emit('regenerate', $event)"
          @delete="emit('delete', $event)"
          @tool-approved="emit('tool-approved', $event)"
          @tool-rejected="emit('tool-rejected', $event)"
        />
        <div v-else class="chat-view__welcome">
          <div class="chat-view__welcome-inner">
            <div class="chat-view__welcome-icon">
              <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
              </svg>
            </div>
            <p class="chat-view__welcome-text" v-if="isGroupTab">群聊模式：使用 @ 提及 Agent 进行协作对话</p>
            <p class="chat-view__welcome-text" v-else>与 {{ agentTabs.find(t => t.id === activeTab)?.label || 'Agent' }} 的单独对话</p>
          </div>
        </div>
        <MessageInput
          :streaming="streaming"
          :provider-id="providerId"
          :model-id="modelId"
          :is-group-chat="isGroupTab"
          @send="(c, a) => emit('send', c, a)"
          @stop="emit('stop')"
          @update:provider-id="emit('update:providerId', $event)"
          @update:model-id="emit('update:modelId', $event)"
          @new-topic="emit('new-topic')"
          @web-search="emit('web-search', $event)"
          @quick-phrase="emit('quick-phrase', $event)"
          @mention="(id, name) => emit('mention', id, name)"
        />
      </template>
      <div v-else class="chat-view__empty chat-view__empty--hero">
        <p>{{ t('chat.emptyHint') }}</p>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.chat-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background: var(--color-background);
}

.chat-view__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
  padding: 14px 20px 12px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-background-soft);
  box-shadow: var(--shadow-sm);
  flex-shrink: 0;
}

.chat-view__head-text {
  min-width: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.chat-view__title-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.agent-status {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 10px;
  border-radius: 10px;
  font-size: 11px;
  flex-shrink: 0;
  background: var(--color-background-mute);
  border: 1px solid var(--color-border);
  animation: status-fade-in 0.35s ease both;
}

@keyframes status-fade-in {
  from { opacity: 0; transform: translateY(4px); }
  to { opacity: 1; transform: translateY(0); }
}

.agent-status__dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}

.agent-status--idle .agent-status__dot {
  background: #22c55e;
  box-shadow: 0 0 4px rgba(34, 197, 94, 0.45);
  animation: gentle-pulse 2s ease-in-out infinite;
}
.agent-status--thinking .agent-status__dot,
.agent-status--streaming .agent-status__dot {
  background: #3b82f6;
  box-shadow: 0 0 4px rgba(59, 130, 246, 0.45);
  animation: pulse-dot 1s ease infinite;
}
.agent-status--awaiting_tool_approval .agent-status__dot {
  background: #eab308;
  box-shadow: 0 0 4px rgba(234, 179, 8, 0.45);
  animation: pulse-dot 1s ease infinite;
}
.agent-status--tool_running .agent-status__dot {
  background: #8b5cf6;
  box-shadow: 0 0 4px rgba(139, 92, 246, 0.45);
  animation: pulse-dot 0.8s ease infinite;
}
.agent-status--error .agent-status__dot {
  background: #ef4444;
  box-shadow: 0 0 4px rgba(239, 68, 68, 0.4);
}
.agent-status--completed .agent-status__dot {
  background: #22c55e;
  box-shadow: 0 0 4px rgba(34, 197, 94, 0.45);
}
.agent-status--paused .agent-status__dot {
  background: #f97316;
  box-shadow: 0 0 4px rgba(249, 115, 22, 0.4);
}

.agent-status__text { color: var(--color-text-2); }

@keyframes pulse-dot {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.5; transform: scale(1.3); }
}
@keyframes gentle-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}

.chat-view__title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-1);
  line-height: 1.3;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
  flex: 1;
}

.chat-view__subtitle {
  margin: 6px 0 0;
  font-size: 12px;
  color: var(--color-text-3);
  line-height: 1.4;
}

.chat-view__head-right {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.chat-view__head-agent { display: flex; align-items: center; flex-shrink: 0; }
.chat-view__head-actions { display: flex; align-items: center; gap: 4px; }
.chat-view__head-channel { display: flex; align-items: center; flex-shrink: 0; }
.channel-select {
  width: 130px;
  :deep(.el-input__wrapper) {
    border-radius: var(--fox-radius-sm);
    background: var(--color-background-mute);
    box-shadow: 0 0 0 1px var(--color-border);
    padding-left: 6px;
  }
  :deep(.el-input__inner) {
    font-size: 12px;
  }
}

.head-icon-btn {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-sm);
  background: transparent;
  color: var(--color-text-2);
  cursor: pointer;
  padding: 0;
  transition: color 0.15s, background 0.15s, border-color 0.15s, transform 0.1s;
}
.head-icon-btn svg {
  stroke: currentColor;
}
.head-icon-btn:hover {
  color: var(--color-text-1);
  background: var(--color-background-mute);
  border-color: var(--color-text-3);
}
.head-icon-btn:active {
  background: var(--color-background-mute);
  transform: scale(0.98);
}

.chat-view__head-model { flex-shrink: 0; }

/* ---- Agent Tabs ---- */
.chat-view__tabs {
  display: flex;
  gap: 2px;
  padding: 6px 16px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-background-soft);
  flex-shrink: 0;
  overflow-x: auto;
}
.chat-view__tabs::-webkit-scrollbar { height: 0; }

.chat-tab {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 5px 14px;
  border: 1px solid transparent;
  border-radius: var(--fox-radius-sm);
  background: transparent;
  color: var(--color-text-3);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s;
  flex-shrink: 0;
}
.chat-tab:hover {
  color: var(--color-text-1);
  background: var(--color-hover);
}
.chat-tab--active {
  color: var(--color-text-1) !important;
  background: var(--color-primary-mute) !important;
  border-color: var(--color-border) !important;
  box-shadow: var(--shadow-sm);
}
.chat-tab__label {
  max-width: 100px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.chat-view__body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.chat-view__welcome {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
  min-height: 0;
  padding: 0 24px;
  animation: welcome-in 0.4s ease both;
}

.chat-view__welcome-inner {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.chat-view__welcome-icon {
  width: 56px;
  height: 56px;
  border-radius: 16px;
  background: var(--color-background-soft);
  border: 1px solid var(--color-border);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-3);
}

.chat-view__welcome-text {
  margin: 0;
  text-align: center;
  color: var(--color-text-3);
  font-size: 14px;
  line-height: 1.5;
}

@keyframes welcome-in {
  from { opacity: 0; transform: translateY(8px); }
  to { opacity: 1; transform: translateY(0); }
}

.chat-view__empty {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
  min-height: 0;
  padding: 32px 24px;
  text-align: center;
  color: var(--color-text-3);
  font-size: 14px;
}

.chat-view__empty--hero {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
  min-height: 0;
  margin: 0;
  padding: 0 24px;
  border: none;
  border-radius: 0;
  background: transparent;
  box-shadow: none;
  animation: welcome-in 0.4s ease both;
}
</style>
