<script setup lang="ts">
import { ref, computed, inject, type Ref, type ComputedRef } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessageBox } from 'element-plus'
import { Plus, Edit, Delete } from '@element-plus/icons-vue'
import { useAgentStore } from '@/stores/agent'
import { useProviderStore } from '@/stores/provider'
import { useSettingsStore } from '@/stores/settings'
import type { AgentConfig } from '@/types'
import AgentConfigDialog from './AgentConfigDialog.vue'

const { t } = useI18n()
const agentStore = useAgentStore()
const providerStore = useProviderStore()
const settingsStore = useSettingsStore()

const topicAgents = inject<ComputedRef<AgentConfig[]>>('topicAgents')
const removeAgentFromTopic = inject<(id: string) => void>('removeAgentFromTopic')

const popoverVisible = ref(false)
const dialogVisible = ref(false)
const dialogParentId = ref<string | null>(null)
const dialogEditingId = ref<string | null>(null)

const current = computed(() => {
  const id = agentStore.currentAgentId
  const ta = topicAgents?.value
  if (ta?.length) {
    return ta.find(a => a.id === id) || ta[0] || null
  }
  return null
})

const allAgents = computed(() => {
  const list = [...(topicAgents?.value?.length ? topicAgents.value : [])]
  const main = list.find(a => a.type === 'main')
  if (!main) return list
  const rest = list.filter(a => a.id !== main.id)
  const ordered: AgentConfig[] = [main]
  function walk(parentId: string) {
    const children = rest.filter(r => r.parentId === parentId)
    for (const c of children) {
      ordered.push(c)
      walk(c.id)
    }
  }
  walk(main.id)
  for (const a of rest) {
    if (!ordered.includes(a)) ordered.push(a)
  }
  return ordered
})

function modelLabel(ag: AgentConfig) {
  const { providerId, modelId } = ag.languageModel
  if (!providerId || !modelId) return t('pageUi.noModel') as string
  const p = providerStore.getProviderById(providerId)
  const m = p?.models?.find(x => x === modelId)
  if (p && m) return `${p.name} / ${m}`
  return `${providerId} / ${modelId}`
}

function statusClass(status: AgentConfig['status']) {
  return `agent-panel__status--${status}`
}

function channelLabel(ag: AgentConfig): string | null {
  if (!ag.channelId) return null
  const ch = settingsStore.channels.find(c => c.id === ag.channelId)
  return ch?.name || null
}

function onPickAgent(id: string) {
  agentStore.switchAgent(id)
  popoverVisible.value = false
}

function openCreateSub() {
  const parent = current.value
  if (!parent) return
  dialogParentId.value = parent.id
  dialogEditingId.value = null
  dialogVisible.value = true
  popoverVisible.value = false
}

function openEdit(ag: AgentConfig, e: Event) {
  e.stopPropagation()
  dialogEditingId.value = ag.id
  dialogParentId.value = null
  dialogVisible.value = true
  popoverVisible.value = false
}

async function onDelete(ag: AgentConfig, e: Event) {
  e.stopPropagation()
  if (ag.type === 'main') return
  try {
    await ElMessageBox.confirm(
      t('agent.deleteConfirm') as string,
      t('agent.delete') as string,
      { type: 'warning', confirmButtonText: t('common.delete') as string, cancelButtonText: t('common.cancel') as string }
    )
    if (removeAgentFromTopic) {
      removeAgentFromTopic(ag.id)
    }
  } catch {
    /* dismiss */
  }
}
</script>

<template>
  <div v-if="current" class="agent-panel">
    <el-popover
      v-model:visible="popoverVisible"
      placement="bottom-start"
      :width="360"
      trigger="click"
      :show-arrow="true"
      popper-class="agent-panel__popper"
    >
      <template #reference>
        <button type="button" class="agent-panel__trigger" :title="t('agent.currentAgent')">
          <span class="agent-panel__trigger-name">{{ current.name }}</span>
          <span
            class="agent-panel__badge"
            :class="current.type === 'main' ? 'agent-panel__badge--main' : 'agent-panel__badge--sub'"
          >
            {{ current.type === 'main' ? t('agent.main') : t('agent.sub') }}
          </span>
          <span class="agent-panel__chev" aria-hidden="true" />
        </button>
      </template>

      <div class="agent-panel__head">
        <span class="agent-panel__head-title">{{ t('agent.currentAgent') }}</span>
        <el-button
          :icon="Plus"
          type="primary"
          link
          size="small"
          :title="t('agent.createSub')"
          @click="openCreateSub"
        />
      </div>
      <ul class="agent-panel__list" role="list">
        <li
          v-for="ag in allAgents"
          :key="ag.id"
          class="agent-panel__card"
          :class="{ 'agent-panel__card--active': ag.id === current.id }"
          role="listitem"
          @click="onPickAgent(ag.id)"
        >
          <div class="agent-panel__card-main">
            <span class="agent-panel__status-dot" :class="statusClass(ag.status)" :title="t(`agent.${ag.status}`)" />
            <div class="agent-panel__card-text">
              <div class="agent-panel__card-title">
                <span class="agent-panel__card-name">{{ ag.name }}</span>
                <span
                  class="agent-panel__badge"
                  :class="ag.type === 'main' ? 'agent-panel__badge--main' : 'agent-panel__badge--sub'"
                >
                  {{ ag.type === 'main' ? t('agent.main') : t('agent.sub') }}
                </span>
              </div>
              <div class="agent-panel__card-model" :title="modelLabel(ag)">
                {{ modelLabel(ag) }}
              </div>
              <div v-if="channelLabel(ag)" class="agent-panel__card-channel">
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 11a9 9 0 0 1 9 9" /><path d="M4 4a16 16 0 0 1 16 16" /><circle cx="5" cy="19" r="1" /></svg>
                {{ channelLabel(ag) }}
              </div>
            </div>
          </div>
          <div class="agent-panel__card-actions" @click.stop>
            <el-button
              :icon="Edit"
              link
              size="small"
              :title="t('agent.edit')"
              @click="openEdit(ag, $event)"
            />
            <el-button
              v-if="ag.type === 'sub'"
              :icon="Delete"
              link
              type="danger"
              size="small"
              :title="t('agent.delete')"
              @click="onDelete(ag, $event)"
            />
          </div>
        </li>
      </ul>
      <p v-if="!allAgents.length" class="agent-panel__empty">{{ t('agent.noAgents') }}</p>
    </el-popover>

    <AgentConfigDialog
      v-model="dialogVisible"
      :parent-id-for-new="dialogParentId"
      :editing-id="dialogEditingId"
    />
  </div>
</template>

<style lang="scss" scoped>
.agent-panel {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.agent-panel__trigger {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  max-width: 220px;
  padding: 6px 10px;
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-md);
  background: var(--color-background-mute);
  color: var(--color-text-1);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition:
    background 0.15s,
    border-color 0.15s;
}
.agent-panel__trigger:hover {
  background: var(--color-hover);
  border-color: var(--color-text-3);
}
.agent-panel__trigger-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}
.agent-panel__chev {
  width: 0;
  height: 0;
  border-left: 4px solid transparent;
  border-right: 4px solid transparent;
  border-top: 5px solid var(--color-text-3);
  flex-shrink: 0;
}
.agent-panel__badge {
  font-size: 10px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: var(--fox-radius-sm);
  flex-shrink: 0;
  line-height: 1.4;
}
.agent-panel__badge--main {
  background: var(--color-primary-mute);
  color: var(--fox-accent-fg);
  border: 1px solid var(--color-border);
}
.agent-panel__badge--sub {
  background: var(--color-background-mute);
  color: var(--color-text-2);
  border: 1px solid var(--color-border);
}
.agent-panel__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--color-border);
}
.agent-panel__head-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-2);
  text-transform: uppercase;
  letter-spacing: 0.02em;
}
.agent-panel__list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 320px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.agent-panel__card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 10px;
  border-radius: var(--fox-radius-sm);
  border: 1px solid var(--color-border);
  background: var(--color-background);
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}
.agent-panel__card:hover {
  background: var(--color-hover);
}
.agent-panel__card--active {
  border-color: var(--fox-accent-border);
  box-shadow: 0 0 0 1px var(--color-primary-mute);
  background: var(--color-background-soft);
}
.agent-panel__card-main {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  min-width: 0;
  flex: 1;
}
.agent-panel__status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-top: 4px;
  flex-shrink: 0;
  background: var(--color-text-3);
}
.agent-panel__status--idle {
  background: var(--color-text-3);
}
.agent-panel__status--running {
  background: var(--fox-accent-fg);
  box-shadow: 0 0 0 2px var(--color-primary-mute);
}
.agent-panel__status--thinking,
.agent-panel__status--streaming,
.agent-panel__status--tool_running {
  background: var(--fox-accent-fg);
  box-shadow: 0 0 0 2px var(--color-primary-mute);
}
.agent-panel__status--awaiting_tool_approval {
  background: #e6a23c;
}
.agent-panel__status--paused {
  background: #e6a23c;
}
.agent-panel__status--completed {
  background: #67c23a;
}
.agent-panel__status--error {
  background: #f56c6c;
}
.agent-panel__card-text {
  min-width: 0;
  flex: 1;
}
.agent-panel__card-title {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  margin-bottom: 2px;
}
.agent-panel__card-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.agent-panel__card-model {
  font-size: 11px;
  color: var(--color-text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.agent-panel__card-channel {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 10px;
  color: var(--fox-accent-fg);
  margin-top: 2px;
}
.agent-panel__card-actions {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}
.agent-panel__empty {
  font-size: 12px;
  color: var(--color-text-3);
  text-align: center;
  margin: 12px 0 4px;
}
</style>

<style lang="scss">
.agent-panel__popper.el-popper {
  --el-bg-color-overlay: var(--color-background);
  --el-box-shadow: var(--shadow-md);
  border: 1px solid var(--color-border) !important;
  border-radius: var(--fox-radius-md) !important;
  background: var(--color-background) !important;
  padding: 12px;
}
</style>
