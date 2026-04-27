<template>
  <div class="computer-use">
    <div class="computer-use__header">
      <h2 class="computer-use__title">
        <svg class="icon" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="2" y="3" width="20" height="14" rx="2" />
          <path d="M8 21h8M12 17v4" />
        </svg>
        {{ t('computerUse.title') }}
      </h2>
      <div class="computer-use__controls">
        <el-select v-model="permissionMode" size="small" style="width: 130px">
          <el-option value="supervised" :label="t('computerUse.supervised')" />
          <el-option value="semi-auto" :label="t('computerUse.semiAuto')" />
          <el-option value="full-auto" :label="t('computerUse.fullAuto')" />
        </el-select>
        <el-input-number v-model="maxSteps" :min="0" :max="500" size="small" style="width: 100px" />
        <span class="computer-use__steps-label">{{ maxSteps === 0 ? '♾️ 无限步' : t('computerUse.maxSteps') }}</span>
      </div>
    </div>

    <div class="computer-use__body">
      <div class="computer-use__left">
        <div class="computer-use__goal-area">
          <el-input
            v-model="goal"
            type="textarea"
            :rows="3"
            :placeholder="t('computerUse.goalPlaceholder')"
            :disabled="isRunning"
            resize="none"
          />
          <div class="computer-use__action-bar">
            <el-select v-model="selectedProviderId" size="small" :placeholder="t('computerUse.selectProvider')" style="flex:1">
              <el-option v-for="p in providers" :key="p.id" :value="p.id" :label="p.name" />
            </el-select>
            <el-select v-model="selectedModelId" size="small" :placeholder="t('computerUse.selectModel')" style="flex:1">
              <el-option v-for="m in availableModels" :key="m" :value="m" :label="m" />
            </el-select>
            <el-button
              v-if="!isRunning"
              type="primary"
              size="small"
              :disabled="!canStart"
              @click="startSession"
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor"><polygon points="5,3 19,12 5,21" /></svg>
              {{ t('computerUse.start') }}
            </el-button>
            <el-button
              v-else
              type="danger"
              size="small"
              @click="stopSession"
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor"><rect x="6" y="6" width="12" height="12" /></svg>
              {{ t('computerUse.stop') }}
            </el-button>
          </div>
        </div>

        <div class="computer-use__log">
          <div class="computer-use__log-header">
            <span>{{ t('computerUse.actionLog') }}</span>
            <el-tag v-if="isRunning" type="success" size="small" effect="dark">
              {{ t('computerUse.running') }} — {{ t('computerUse.step') }} {{ currentStep }}{{ maxSteps === 0 ? '' : '/' + maxSteps }}
            </el-tag>
            <el-tag v-else-if="completeSummary" type="info" size="small">{{ t('computerUse.completed') }}</el-tag>
          </div>
          <div class="computer-use__log-list" ref="logListRef">
            <div
              v-for="step in steps"
              :key="step.step"
              class="computer-use__log-item"
              :class="{ active: step.step === selectedStep }"
              @click="selectedStep = step.step"
            >
              <div class="log-item__header">
                <span class="log-item__step">{{ t('computerUse.step') }} {{ step.step }}</span>
                <el-tag :type="step.status === 'executing' ? 'warning' : step.status === 'completed' ? 'success' : 'info'" size="small">
                  {{ step.status }}
                </el-tag>
              </div>
              <div class="log-item__desc">{{ step.actionDescription || '...' }}</div>
              <div v-if="step.toolCalls.length" class="log-item__tools">
                <span v-for="tc in step.toolCalls" :key="tc.name" class="log-item__tool-tag">
                  {{ tc.name }}
                </span>
              </div>
            </div>
            <div v-if="!steps.length" class="computer-use__log-empty">
              {{ t('computerUse.noSteps') }}
            </div>
          </div>
        </div>
      </div>

      <div class="computer-use__right">
        <div class="computer-use__preview">
          <div v-if="currentScreenshot" class="computer-use__screenshot-wrap">
            <img
              :src="'data:image/jpeg;base64,' + currentScreenshot"
              class="computer-use__screenshot"
              alt="Screen"
            />
            <div v-if="lastAction" class="computer-use__action-overlay">
              <div
                v-if="lastAction.name.startsWith('mouse_') && lastAction.arguments.x != null"
                class="computer-use__cursor-marker"
                :style="cursorMarkerStyle"
              >
                <svg viewBox="0 0 24 24" width="20" height="20" fill="rgba(255,50,50,0.9)" stroke="white" stroke-width="1">
                  <circle cx="12" cy="12" r="6" />
                </svg>
              </div>
            </div>
          </div>
          <div v-else class="computer-use__preview-empty">
            <svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="var(--text-muted)" stroke-width="1.5">
              <rect x="2" y="3" width="20" height="14" rx="2" />
              <path d="M8 21h8M12 17v4" />
            </svg>
            <p>{{ t('computerUse.previewHint') }}</p>
          </div>
        </div>

        <div v-if="completeSummary" class="computer-use__summary">
          <h4>{{ t('computerUse.result') }}</h4>
          <p>{{ completeSummary }}</p>
        </div>

        <div v-if="errorMsg" class="computer-use__error">
          <h4>{{ t('computerUse.error') }}</h4>
          <p>{{ errorMsg }}</p>
        </div>
      </div>
    </div>

    <el-dialog
      v-model="showApproval"
      :title="t('computerUse.approvalTitle')"
      width="480px"
      :close-on-click-modal="false"
      :close-on-press-escape="false"
      align-center
    >
      <div class="computer-use__approval-body" v-if="pendingAction">
        <p class="approval__desc">{{ t('computerUse.approvalDesc') }}</p>
        <div class="approval__action">
          <el-tag type="warning" size="large">{{ pendingAction.action.name }}</el-tag>
          <pre class="approval__args">{{ JSON.stringify(pendingAction.action.arguments, null, 2) }}</pre>
        </div>
      </div>
      <template #footer>
        <el-button @click="respondApproval(false)">{{ t('computerUse.reject') }}</el-button>
        <el-button type="primary" @click="respondApproval(true)">{{ t('computerUse.approve') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useProviderStore } from '@/stores/provider'
import { useSettingsStore } from '@/stores/settings'
import {
  computerUseApi,
  onComputerUseStep,
  onComputerUseAction,
  onComputerUseComplete,
  onComputerUseError
} from '@/utils/tauri-api'
import type { ComputerUseStep, ComputerUseToolCall, ComputerUseActionRequest } from '@/types'

const { t } = useI18n()
const providerStore = useProviderStore()
const settings = useSettingsStore()

const goal = ref('')
const permissionMode = ref<string>('supervised')
const maxSteps = ref(50)
const selectedProviderId = ref('')
const selectedModelId = ref('')
const sessionId = ref('')
const isRunning = ref(false)
const currentStep = ref(0)
const steps = ref<ComputerUseStep[]>([])
const selectedStep = ref(0)
const currentScreenshot = ref('')
const completeSummary = ref('')
const errorMsg = ref('')
const showApproval = ref(false)
const pendingAction = ref<ComputerUseActionRequest | null>(null)
const logListRef = ref<HTMLElement>()
const lastAction = ref<ComputerUseToolCall | null>(null)

const providers = computed(() => providerStore.getEnabledProviders())
const availableModels = computed(() => {
  const p = providers.value.find(p => p.id === selectedProviderId.value)
  return p?.models || []
})
const canStart = computed(() =>
  goal.value.trim() && selectedProviderId.value && selectedModelId.value
)

const screenDimensions = ref({ w: 1920, h: 1080 })

const cursorMarkerStyle = computed(() => {
  if (!lastAction.value) return {}
  const args = lastAction.value.arguments
  const x = (args.x as number) ?? 0
  const y = (args.y as number) ?? 0
  const sw = screenDimensions.value.w || 1920
  const sh = screenDimensions.value.h || 1080
  return {
    left: `${(x / sw) * 100}%`,
    top: `${(y / sh) * 100}%`,
    position: 'absolute' as const,
    transform: 'translate(-50%, -50%)',
    pointerEvents: 'none' as const,
    zIndex: 10
  }
})

const unlisteners: Array<() => void> = []

onMounted(async () => {
  if (settings.defaultProviderId) selectedProviderId.value = settings.defaultProviderId
  if (settings.defaultModelId) selectedModelId.value = settings.defaultModelId

  try {
    const { getActualScreenSize } = await import('@/utils/harness')
    const s = await getActualScreenSize()
    if (s.width > 0) screenDimensions.value = { w: s.width, h: s.height }
  } catch { /* use defaults */ }

  const u1 = await onComputerUseStep((payload) => {
    const step: ComputerUseStep = {
      step: payload.step,
      screenshotBase64: payload.screenshotBase64,
      actionDescription: payload.actionDescription,
      toolCalls: payload.toolCalls || [],
      status: payload.status || 'executing',
      timestamp: Date.now()
    }
    const idx = steps.value.findIndex(s => s.step === step.step)
    if (idx >= 0) {
      steps.value[idx] = step
    } else {
      steps.value.push(step)
    }
    currentStep.value = step.step
    currentScreenshot.value = step.screenshotBase64
    selectedStep.value = step.step
    if (step.toolCalls.length) {
      lastAction.value = step.toolCalls[step.toolCalls.length - 1]
    }
    nextTick(() => {
      if (logListRef.value) {
        logListRef.value.scrollTop = logListRef.value.scrollHeight
      }
    })
  })
  unlisteners.push(u1)

  const u2 = await onComputerUseAction((payload: ComputerUseActionRequest) => {
    if (payload.needsApproval) {
      pendingAction.value = payload
      showApproval.value = true
    }
  })
  unlisteners.push(u2)

  const u3 = await onComputerUseComplete((payload) => {
    isRunning.value = false
    completeSummary.value = payload.summary || `Completed in ${payload.totalSteps} steps`
  })
  unlisteners.push(u3)

  const u4 = await onComputerUseError((payload) => {
    isRunning.value = false
    errorMsg.value = payload.error || 'Unknown error'
  })
  unlisteners.push(u4)
})

onBeforeUnmount(() => {
  unlisteners.forEach(fn => fn())
})

watch(selectedStep, (step) => {
  const s = steps.value.find(s => s.step === step)
  if (s?.screenshotBase64) {
    currentScreenshot.value = s.screenshotBase64
    if (s.toolCalls.length) {
      lastAction.value = s.toolCalls[s.toolCalls.length - 1]
    }
  }
})

async function startSession() {
  if (!canStart.value) return
  steps.value = []
  currentStep.value = 0
  currentScreenshot.value = ''
  completeSummary.value = ''
  errorMsg.value = ''
  lastAction.value = null

  try {
    const sid = await computerUseApi.start(
      goal.value,
      selectedProviderId.value,
      selectedModelId.value,
      maxSteps.value,
      permissionMode.value
    )
    sessionId.value = sid
    isRunning.value = true
  } catch (e: any) {
    errorMsg.value = e?.message || String(e)
  }
}

async function stopSession() {
  if (!sessionId.value) return
  try {
    await computerUseApi.stop(sessionId.value)
  } catch (_) {}
  isRunning.value = false
}

async function respondApproval(approved: boolean) {
  if (!sessionId.value) return
  showApproval.value = false
  try {
    await computerUseApi.approveAction(sessionId.value, approved)
  } catch (_) {}
  pendingAction.value = null
}
</script>

<style scoped lang="scss">
.computer-use {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  background: var(--bg-page, #0d0d0d);
  color: var(--text-primary, #e0e0e0);
}

.computer-use__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 20px;
  border-bottom: 1px solid var(--border-color, rgba(255,255,255,0.08));
  flex-shrink: 0;
}

.computer-use__title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 16px;
  font-weight: 600;
  margin: 0;
  .icon { opacity: 0.7; }
}

.computer-use__controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.computer-use__steps-label {
  font-size: 12px;
  opacity: 0.6;
}

.computer-use__body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.computer-use__left {
  width: 360px;
  min-width: 280px;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--border-color, rgba(255,255,255,0.08));
}

.computer-use__goal-area {
  padding: 12px;
  border-bottom: 1px solid var(--border-color, rgba(255,255,255,0.06));
  :deep(.el-textarea__inner) {
    background: var(--bg-input, rgba(255,255,255,0.04));
    border-color: var(--border-color, rgba(255,255,255,0.1));
    color: var(--text-primary);
    border-radius: 8px;
    &::placeholder { color: var(--text-muted, rgba(255,255,255,0.35)); }
  }
}

.computer-use__action-bar {
  display: flex;
  gap: 6px;
  margin-top: 8px;
  :deep(.el-select) {
    .el-input__wrapper {
      background: var(--bg-input, rgba(255,255,255,0.04));
      border-color: var(--border-color, rgba(255,255,255,0.1));
      box-shadow: none;
    }
  }
}

.computer-use__log {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.computer-use__log-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  font-size: 13px;
  font-weight: 500;
  border-bottom: 1px solid var(--border-color, rgba(255,255,255,0.06));
}

.computer-use__log-list {
  flex: 1;
  overflow-y: auto;
  padding: 6px;
}

.computer-use__log-item {
  padding: 8px 10px;
  border-radius: 8px;
  margin-bottom: 4px;
  cursor: pointer;
  transition: background 0.15s;
  &:hover { background: rgba(255,255,255,0.04); }
  &.active { background: rgba(var(--accent-rgb, 99,102,241), 0.12); }
}

.log-item__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.log-item__step {
  font-size: 12px;
  font-weight: 600;
  opacity: 0.7;
}

.log-item__desc {
  font-size: 12px;
  line-height: 1.5;
  opacity: 0.85;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.log-item__tools {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 4px;
}

.log-item__tool-tag {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 4px;
  background: rgba(var(--accent-rgb, 99,102,241), 0.15);
  color: var(--accent-color, #6366f1);
}

.computer-use__log-empty {
  padding: 40px 20px;
  text-align: center;
  font-size: 13px;
  opacity: 0.4;
}

.computer-use__right {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.computer-use__preview {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  padding: 12px;
  background: rgba(0,0,0,0.3);
}

.computer-use__screenshot-wrap {
  position: relative;
  max-width: 100%;
  max-height: 100%;
  border-radius: 8px;
  overflow: hidden;
  box-shadow: 0 4px 24px rgba(0,0,0,0.4);
}

.computer-use__screenshot {
  display: block;
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  border-radius: 8px;
}

.computer-use__action-overlay {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.computer-use__cursor-marker {
  animation: pulse-marker 1s ease-in-out infinite;
}

@keyframes pulse-marker {
  0%, 100% { opacity: 1; transform: translate(-50%, -50%) scale(1); }
  50% { opacity: 0.7; transform: translate(-50%, -50%) scale(1.5); }
}

.computer-use__preview-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  opacity: 0.4;
  p { font-size: 13px; margin: 0; }
}

.computer-use__summary,
.computer-use__error {
  padding: 12px 16px;
  margin: 0 12px 12px;
  border-radius: 8px;
  font-size: 13px;
  h4 { margin: 0 0 6px; font-size: 13px; }
  p { margin: 0; line-height: 1.5; }
}

.computer-use__summary {
  background: rgba(34,197,94,0.1);
  border: 1px solid rgba(34,197,94,0.2);
}

.computer-use__error {
  background: rgba(239,68,68,0.1);
  border: 1px solid rgba(239,68,68,0.2);
  color: #f87171;
}

.computer-use__approval-body {
  .approval__desc {
    margin: 0 0 12px;
    font-size: 14px;
    opacity: 0.85;
  }
  .approval__action {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .approval__args {
    background: rgba(0,0,0,0.3);
    border-radius: 6px;
    padding: 10px;
    font-size: 12px;
    max-height: 200px;
    overflow-y: auto;
    margin: 0;
    color: var(--text-primary);
  }
}
</style>
