<template>
  <div class="translate-page">
    <h1 class="title">{{ t('translate.title') }}</h1>
    <div class="split">
      <section class="pane source">
        <div class="pane-head no-wrap">
          <el-select v-model="sourceLang" class="lang-select" :aria-label="t('translate.sourceLang')">
            <el-option
              v-for="opt in langOptions"
              :key="'s-' + opt.value"
              :value="opt.value"
              :label="t(opt.labelKey)"
            />
          </el-select>
        </div>
        <el-input
          v-model="sourceText"
          type="textarea"
          :rows="16"
          :placeholder="t('translate.inputPlaceholder')"
          class="text-area"
        />
      </section>

      <div class="center-actions">
        <el-button class="swap-btn" circle :aria-label="t('translate.swap')" @click="swapLangs">
          <el-icon><Sort /></el-icon>
        </el-button>
        <el-button
          type="primary"
          class="run-btn"
          :loading="loading"
          :disabled="!sourceText.trim() || !canTranslate"
          @click="runTranslate"
        >
          {{ t('translate.translate') }}
        </el-button>
        <p v-if="!canTranslate" class="warn">{{ t('pageUi.noProvider') }}</p>
      </div>

      <section class="pane target">
        <div class="pane-head no-wrap">
          <el-select v-model="targetLang" class="lang-select" :aria-label="t('translate.targetLang')">
            <el-option
              v-for="opt in targetOptions"
              :key="'t-' + opt.value"
              :value="opt.value"
              :label="t(opt.labelKey)"
            />
          </el-select>
        </div>
        <div class="out-wrap">
          <div v-if="loading" class="loading-line">{{ t('pageUi.translating') }}</div>
          <el-input
            v-model="resultText"
            type="textarea"
            :rows="16"
            readonly
            :placeholder="t('pageUi.translateResult')"
            class="text-area"
          />
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { Sort } from '@element-plus/icons-vue'
import { v4 as uuidv4 } from 'uuid'
import { sendChatMessage, onStreamChunk, onStreamEnd, onStreamError, abortChat } from '@/utils/tauri-api'
import { useProviderStore } from '@/stores/provider'
import { useSettingsStore } from '@/stores/settings'
import type { Provider } from '@/types'

const { t } = useI18n()
const provider = useProviderStore()
const settings = useSettingsStore()

const langOptions = [
  { value: 'auto', labelKey: 'translate.auto' as const },
  { value: 'zh', labelKey: 'translate.langZh' as const },
  { value: 'en', labelKey: 'translate.langEn' as const },
  { value: 'ja', labelKey: 'translate.langJa' as const },
  { value: 'ko', labelKey: 'translate.langKo' as const },
  { value: 'fr', labelKey: 'translate.langFr' as const },
  { value: 'de', labelKey: 'translate.langDe' as const },
  { value: 'es', labelKey: 'translate.langEs' as const }
] as const

const targetOptions = langOptions.filter(o => o.value !== 'auto')

const sourceLang = ref<string>('auto')
const targetLang = ref<string>('en')
const sourceText = ref('')
const resultText = ref('')
const loading = ref(false)
const activeMsgId = ref<string | null>(null)
let unSubChunk: (() => void) | null = null
let unSubEnd: (() => void) | null = null
let unSubErr: (() => void) | null = null

const canTranslate = computed(() => {
  const p = resolveProvider()
  if (!p) return false
  const mid = resolveModelId(p)
  return Boolean(mid)
})

function labelForCode(code: string) {
  const o = langOptions.find(x => x.value === code)
  return o ? t(o.labelKey) : code
}

function resolveProvider() {
  if (settings.defaultProviderId) {
    const p = provider.getProviderById(settings.defaultProviderId)
    if (p?.enabled) return p
  }
  return provider.getEnabledProviders()[0]
}

function resolveModelId(p: Provider) {
  if (settings.defaultModelId && p.models?.some(m => m === settings.defaultModelId)) {
    return settings.defaultModelId
  }
  return p.models?.[0] ?? null
}

function swapLangs() {
  if (sourceLang.value === 'auto') {
    const t0 = targetLang.value
    targetLang.value = 'zh'
    sourceLang.value = t0
  } else {
    const s = sourceLang.value
    sourceLang.value = targetLang.value
    targetLang.value = s
  }
}

async function runTranslate() {
  const p = resolveProvider()
  if (!p) {
    ElMessage.error(t('pageUi.noProvider'))
    return
  }
  const mid = resolveModelId(p)
  if (!mid) {
    ElMessage.error(t('pageUi.noModel'))
    return
  }

  const fromL = sourceLang.value === 'auto' ? t('translate.auto') : labelForCode(sourceLang.value)
  const toL = labelForCode(targetLang.value)
  const userPrompt = t('pageUi.translatePrompt', { from: fromL, to: toL, text: sourceText.value })

  if (activeMsgId.value) {
    try { await abortChat(activeMsgId.value) } catch { /* ignore */ }
  }
  const messageId = uuidv4()
  activeMsgId.value = messageId
  resultText.value = ''
  loading.value = true

  try {
    await sendChatMessage({
      providerId: p.id,
      modelId: mid,
      messageId,
      messages: [{ role: 'user', content: userPrompt }]
    })
  } catch {
    loading.value = false
    activeMsgId.value = null
    ElMessage.error(t('pageUi.translateError'))
  }
}

function cleanupStreamListeners() {
  unSubChunk?.()
  unSubEnd?.()
  unSubErr?.()
  unSubChunk = unSubEnd = unSubErr = null
}

onMounted(async () => {
  await provider.loadProviders()
  unSubChunk = await onStreamChunk(payload => {
    if (activeMsgId.value && payload.messageId === activeMsgId.value) {
      resultText.value += payload.chunk
    }
  })
  unSubEnd = await onStreamEnd(payload => {
    if (activeMsgId.value && payload.messageId === activeMsgId.value) {
      loading.value = false
      activeMsgId.value = null
    }
  })
  unSubErr = await onStreamError(payload => {
    if (activeMsgId.value && payload.messageId === activeMsgId.value) {
      loading.value = false
      activeMsgId.value = null
      ElMessage.error(payload.error || t('pageUi.translateError'))
    }
  })
})

onUnmounted(async () => {
  if (activeMsgId.value) {
    try { await abortChat(activeMsgId.value) } catch { /* ignore */ }
  }
  cleanupStreamListeners()
})
</script>

<style lang="scss" scoped>
.translate-page {
  height: 100%;
  min-height: 0;
  overflow: hidden;
  padding: 20px 24px 24px;
  background: var(--color-background);
  color: var(--color-text-1);
  display: flex;
  flex-direction: column;
}

.title {
  margin: 0 0 20px;
  font-size: 1.5rem;
  font-weight: 600;
  white-space: nowrap;
}

.split {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  gap: 16px;
  align-items: stretch;
  min-height: 0;
  flex: 1;
  & > * {
    min-width: 0;
    overflow: auto;
  }
  & > .center-actions {
    min-width: 100px;
    flex-shrink: 0;
  }
}

.pane {
  display: flex;
  flex-direction: column;
  border-radius: var(--fox-radius-lg);
  border: 1px solid var(--color-border);
  background: var(--color-background-soft);
  padding: 14px;
  min-height: 0;
}

.pane-head {
  margin-bottom: 10px;
  &.no-wrap {
    white-space: nowrap;
  }
}

.lang-select {
  width: 100%;
  min-width: 0;
  :deep(.el-select__wrapper) {
    border-radius: var(--fox-radius-sm);
  }
}

.text-area {
  flex: 1;
  :deep(.el-textarea__inner) {
    border-radius: var(--fox-radius-md);
    background: var(--color-background-mute);
    color: var(--color-text-1);
    min-height: 100%;
    box-shadow: 0 0 0 1px var(--color-border);
  }
}

.out-wrap {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  position: relative;
}

.loading-line {
  position: absolute;
  top: 4px;
  right: 8px;
  z-index: 1;
  font-size: 0.8rem;
  color: var(--color-text-3);
  pointer-events: none;
}

.center-actions {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 8px 0;
  white-space: nowrap;
}

.swap-btn {
  border-color: var(--color-border);
  background: var(--color-background-mute);
  color: var(--color-text-1);
}

.run-btn {
  background: var(--fox-accent-fg) !important;
  border-color: var(--fox-accent-border) !important;
  color: var(--fox-accent-on) !important;
  border-radius: var(--fox-radius-sm);
  padding-left: 20px;
  padding-right: 20px;
}

.warn {
  margin: 0;
  max-width: 200px;
  text-align: center;
  font-size: 0.75rem;
  color: var(--color-warning);
  line-height: 1.3;
}
</style>
