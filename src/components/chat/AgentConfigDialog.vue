<script setup lang="ts">
import { ref, watch, computed, inject, type ComputedRef } from 'vue'
import { useI18n } from 'vue-i18n'
import { v4 as uuidv4 } from 'uuid'
import { useAgentStore } from '@/stores/agent'
import { useSettingsStore } from '@/stores/settings'
import ModelSelector from './ModelSelector.vue'
import type { AgentConfig } from '@/types'

const props = defineProps<{
  modelValue: boolean
  /** Sub-agent is created under this parent; omit when editing. */
  parentIdForNew: string | null
  /** When set, dialog edits this agent. */
  editingId: string | null
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
  (e: 'saved'): void
}>()

const { t } = useI18n()
const agentStore = useAgentStore()
const settingsStore = useSettingsStore()
const topicAgents = inject<ComputedRef<AgentConfig[]>>('topicAgents')
const updateTopicAgent = inject<(agentId: string, data: Partial<AgentConfig>) => void>('updateTopicAgent')
const addAgentToTopic = inject<(agent: AgentConfig) => void>('addAgentToTopic')

const name = ref('')
const systemPrompt = ref('')
const providerId = ref('')
const modelId = ref('')
const enableVision = ref(false)
const visionProviderId = ref('')
const visionModelId = ref('')
const enableTool = ref(false)
const toolProviderId = ref('')
const toolModelId = ref('')
const maxIterations = ref(50)
const selectedTools = ref<string[]>([])
const channelId = ref<string | null>(null)

const channelOptions = computed(() =>
  settingsStore.channels.filter(c => c.enabled)
)

const isEdit = computed(() => !!props.editingId)

const title = computed(() =>
  isEdit.value ? t('agent.edit') : t('agent.createSub')
)

function resetFromEdit(agent: AgentConfig) {
  name.value = agent.name
  systemPrompt.value = agent.systemPrompt
  providerId.value = agent.languageModel.providerId
  modelId.value = agent.languageModel.modelId
  const vis = agent.visionModel
  enableVision.value = !!vis && !!(vis.providerId && vis.modelId)
  visionProviderId.value = vis?.providerId || ''
  visionModelId.value = vis?.modelId || ''
  const tm = agent.toolModel
  enableTool.value = !!tm && !!(tm.providerId && tm.modelId)
  toolProviderId.value = tm?.providerId || ''
  toolModelId.value = tm?.modelId || ''
  maxIterations.value = agent.maxIterations
  selectedTools.value = [...agent.tools]
  channelId.value = agent.channelId ?? null
}

function resetFromParent(parentId: string) {
  const ta = topicAgents?.value
  const p = ta?.find(a => a.id === parentId)
  if (!p) {
    providerId.value = ''
    modelId.value = ''
    name.value = ''
    systemPrompt.value = ''
    maxIterations.value = 50
    selectedTools.value = []
  } else {
    name.value = ''
    systemPrompt.value = ''
    providerId.value = p.languageModel.providerId
    modelId.value = p.languageModel.modelId
    enableVision.value = !!p.visionModel
    visionProviderId.value = p.visionModel?.providerId || ''
    visionModelId.value = p.visionModel?.modelId || ''
    enableTool.value = !!p.toolModel
    toolProviderId.value = p.toolModel?.providerId || ''
    toolModelId.value = p.toolModel?.modelId || ''
    maxIterations.value = 50
    selectedTools.value = [...p.tools]
    channelId.value = null
  }
}

watch(
  () => [props.modelValue, props.editingId, props.parentIdForNew] as const,
  () => {
    if (!props.modelValue) return
    if (props.editingId) {
      const a = topicAgents?.value?.find(x => x.id === props.editingId)
      if (a) resetFromEdit(a)
    } else if (props.parentIdForNew) {
      resetFromParent(props.parentIdForNew)
    }
  },
  { immediate: true }
)

const toolOptions = computed(() => agentStore.agentTools)

function onClose() {
  emit('update:modelValue', false)
}

function onSave() {
  const updateData: Partial<AgentConfig> = {
    systemPrompt: systemPrompt.value,
    languageModel: { providerId: providerId.value, modelId: modelId.value },
    visionModel: enableVision.value
      ? { providerId: visionProviderId.value, modelId: visionModelId.value }
      : null,
    toolModel: enableTool.value
      ? { providerId: toolProviderId.value, modelId: toolModelId.value }
      : null,
    maxIterations: maxIterations.value,
    tools: [...selectedTools.value],
    channelId: channelId.value,
  }

  if (props.editingId) {
    const prev = topicAgents?.value?.find(x => x.id === props.editingId)
    const fallback = prev?.name || (prev?.type === 'main' ? t('agent.main') : t('agent.sub'))
    updateData.name = name.value.trim() || fallback

    // Only update topic snapshot — NOT global agents
    if (updateTopicAgent) {
      updateTopicAgent(props.editingId, updateData)
    }
    emit('saved')
    onClose()
    return
  }

  if (!props.parentIdForNew) return
  const newAgent: AgentConfig = {
    id: uuidv4(),
    name: name.value.trim() || t('agent.sub'),
    type: 'sub',
    parentId: props.parentIdForNew,
    systemPrompt: systemPrompt.value,
    languageModel: { providerId: providerId.value, modelId: modelId.value },
    visionModel: enableVision.value
      ? { providerId: visionProviderId.value, modelId: visionModelId.value }
      : null,
    toolModel: enableTool.value
      ? { providerId: toolProviderId.value, modelId: toolModelId.value }
      : null,
    maxIterations: maxIterations.value,
    tools: [...selectedTools.value],
    channelId: channelId.value,
    delegateDepth: 1,
    status: 'idle' as const,
    createdAt: Date.now(),
    personalityId: null,
    memoryEnabled: true,
    skillIds: [],
    sessionSearchEnabled: true,
  }
  if (addAgentToTopic) addAgentToTopic(newAgent)
  agentStore.switchAgent(newAgent.id)
  emit('saved')
  onClose()
}

function toggleTool(id: string, on: boolean) {
  if (on) {
    if (!selectedTools.value.includes(id)) selectedTools.value = [...selectedTools.value, id]
  } else {
    selectedTools.value = selectedTools.value.filter(x => x !== id)
  }
}
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    :title="title"
    class="agent-config-dialog"
    width="520px"
    align-center
    :close-on-click-modal="false"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <el-form label-position="top" class="agent-config-dialog__form">
      <el-form-item :label="t('agent.name')">
        <el-input v-model="name" :placeholder="t('agent.name')" clearable />
      </el-form-item>
      <el-form-item :label="t('agent.systemPrompt')">
        <el-input
          v-model="systemPrompt"
          type="textarea"
          :rows="5"
          :placeholder="t('assistant.systemPrompt')"
        />
      </el-form-item>
      <el-form-item :label="t('agent.languageModel')">
        <ModelSelector
          :provider-id="providerId"
          :model-id="modelId"
          @update:provider-id="providerId = $event"
          @update:model-id="modelId = $event"
        />
      </el-form-item>
      <el-form-item :label="t('agent.enableVision')">
        <div class="agent-config-dialog__row">
          <el-switch v-model="enableVision" />
          <div v-show="enableVision" class="agent-config-dialog__slot">
            <ModelSelector
              :provider-id="visionProviderId"
              :model-id="visionModelId"
              @update:provider-id="visionProviderId = $event"
              @update:model-id="visionModelId = $event"
            />
          </div>
        </div>
      </el-form-item>
      <el-form-item :label="t('agent.enableTool')">
        <div class="agent-config-dialog__row">
          <el-switch v-model="enableTool" />
          <div v-show="enableTool" class="agent-config-dialog__slot">
            <ModelSelector
              :provider-id="toolProviderId"
              :model-id="toolModelId"
              @update:provider-id="toolProviderId = $event"
              @update:model-id="toolModelId = $event"
            />
          </div>
        </div>
      </el-form-item>
      <el-form-item :label="t('agent.maxIterations')">
        <div class="agent-config-dialog__iter-row">
          <el-slider v-model="maxIterations" :min="0" :max="200" :show-tooltip="true" :format-tooltip="(v: number) => v === 0 ? '无限' : String(v)" class="agent-config-dialog__iter-slider" />
          <span class="agent-config-dialog__iter-label">{{ maxIterations === 0 ? '♾️ 无限' : maxIterations }}</span>
        </div>
        <div class="agent-config-dialog__iter-hint">设为 0 表示无限迭代，AI 将持续工作直到任务完成</div>
      </el-form-item>
      <el-form-item :label="t('mcp.tools')">
        <div class="agent-config-dialog__tools">
          <el-checkbox
            v-for="tool in toolOptions"
            :key="tool.id"
            :model-value="selectedTools.includes(tool.id)"
            @update:model-value="(v: string | number | boolean) => toggleTool(tool.id, Boolean(v))"
          >
            {{ tool.name }}
          </el-checkbox>
        </div>
      </el-form-item>
      <el-form-item label="绑定频道">
        <el-select
          v-model="channelId"
          placeholder="选择频道（可选）"
          clearable
          class="agent-config-dialog__channel"
        >
          <el-option
            v-for="ch in channelOptions"
            :key="ch.id"
            :label="ch.name"
            :value="ch.id"
          />
        </el-select>
        <p class="agent-config-dialog__hint">绑定频道后，该 Agent 将通过此频道发送/接收消息</p>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onClose">{{ t('common.cancel') }}</el-button>
      <el-button type="primary" @click="onSave">{{ t('common.save') }}</el-button>
    </template>
  </el-dialog>
</template>

<style lang="scss" scoped>
.agent-config-dialog {
  --el-dialog-bg-color: var(--color-background);
  --el-border-color: var(--color-border);
  --el-text-color-primary: var(--color-text-1);
}

.agent-config-dialog__form {
  max-height: 70vh;
  overflow-y: auto;
  padding-right: 4px;
}

.agent-config-dialog__row {
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: 100%;
}

.agent-config-dialog__slot {
  width: 100%;
  min-width: 0;
}

.agent-config-dialog__slider {
  width: 100%;
  padding: 0 4px 8px;
}
.agent-config-dialog__iter-row {
  display: flex; align-items: center; gap: 12px; width: 100%;
}
.agent-config-dialog__iter-slider { flex: 1; }
.agent-config-dialog__iter-label {
  min-width: 48px; text-align: right; font-size: 13px; font-weight: 600;
  color: var(--color-text-1); white-space: nowrap;
}
.agent-config-dialog__iter-hint {
  font-size: 11px; color: var(--color-text-3); margin-top: 2px;
}

.agent-config-dialog__hint {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--color-text-3);
}

.agent-config-dialog__tools {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.agent-config-dialog__channel {
  width: 100%;
}
</style>
