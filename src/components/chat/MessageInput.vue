<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { storeToRefs } from 'pinia'
import { useSettingsStore } from '@/stores/settings'
import { useAgentStore, type WebSearchEngine, type PermissionLevel } from '@/stores/agent'
import { ElMessage } from 'element-plus'
import type { Attachment } from '@/types'
import WebSearchPopup from './WebSearchPopup.vue'
import QuickPhrasePopup from './QuickPhrasePopup.vue'
import AgentMentionPopup from './AgentMentionPopup.vue'

const props = withDefaults(
  defineProps<{
    streaming: boolean
    providerId: string
    modelId: string
    isGroupChat?: boolean
  }>(),
  { streaming: false, providerId: '', modelId: '', isGroupChat: false }
)

const emit = defineEmits<{
  (e: 'send', content: string, attachments: Attachment[]): void
  (e: 'stop'): void
  (e: 'update:providerId', v: string): void
  (e: 'update:modelId', v: string): void
  (e: 'new-topic'): void
  (e: 'web-search', engine: WebSearchEngine): void
  (e: 'quick-phrase', phrase: string): void
  (e: 'mention', agentId: string, agentName: string): void
}>()

const { t } = useI18n()
const settings = useSettingsStore()
const agent = useAgentStore()
const { webSearchEnabled, quickPhrases, permissionLevel } = storeToRefs(agent)

const text = ref('')
const files = ref<Attachment[]>([])
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const isDragOver = ref(false)
const showWebSearchPopup = ref(false)
const showQuickPhrasePopup = ref(false)
const showMentionPopup = ref(false)
const barLeftRef = ref<HTMLDivElement | null>(null)

const sendOnEnter = computed(() => settings.sendKey === 'Enter')

function autoResize() {
  nextTick(() => {
    const el = textareaRef.value
    if (!el) return
    el.style.height = 'auto'
    const h = Math.min(200, Math.max(36, el.scrollHeight))
    el.style.height = `${h}px`
  })
}

watch(text, autoResize, { immediate: true })

function doSend() {
  if (props.streaming) return
  const content = text.value.trim()
  if (!content && !files.value.length) return
  emit('send', content, [...files.value])
  text.value = ''
  files.value = []
  nextTick(autoResize)
}

function onKeydown(e: KeyboardEvent) {
  if (sendOnEnter.value) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      doSend()
    }
  } else {
    if (e.key === 'Enter' && e.shiftKey) {
      e.preventDefault()
      doSend()
    }
  }
}

async function onAttach() {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      multiple: true,
      title: t('chat.attach') as string,
    })
    if (!selected) return
    const paths = Array.isArray(selected) ? selected : [selected]
    for (const p of paths) {
      const filePath = typeof p === 'string' ? p : (p as any).path || String(p)
      const name = filePath.replace(/^.*[\\/]/, '') || filePath
      const ext = name.split('.').pop()?.toLowerCase() || ''
      const imageExts = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp']
      const isImage = imageExts.includes(ext)

      let url = filePath
      let mimeType = 'file'

      if (isImage) {
        try {
          const { invoke } = await import('@tauri-apps/api/core')
          const b64: string = await invoke('read_file_base64', { path: filePath })
          const mime = ext === 'jpg' ? 'image/jpeg' : `image/${ext}`
          mimeType = mime
          url = `data:${mime};base64,${b64}`
        } catch {
          // If read fails, keep original path
        }
      }

      files.value.push({
        id: crypto.randomUUID?.() || `${Date.now()}-${name}`,
        name,
        type: mimeType,
        size: 0,
        url,
      })
    }
  } catch (e: any) {
    if (e?.toString?.()?.includes?.('cancelled')) return
    ElMessage.error(t('common.error'))
  }
}

function removeFile(id: string) {
  files.value = files.value.filter(f => f.id !== id)
}

function onDragOver(e: DragEvent) {
  e.preventDefault()
  isDragOver.value = true
}
function onDragLeave() {
  isDragOver.value = false
}
function onDrop(e: DragEvent) {
  e.preventDefault()
  isDragOver.value = false
  const items = e.dataTransfer?.files
  if (!items?.length) return
  for (const f of Array.from(items)) {
    const ext = f.name.split('.').pop()?.toLowerCase() || ''
    const imageExts = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp']
    const isImage = imageExts.includes(ext) || f.type.startsWith('image/')
    if (isImage) {
      const reader = new FileReader()
      const name = f.name
      const size = f.size
      reader.onload = () => {
        files.value.push({
          id: crypto.randomUUID?.() || `${Date.now()}-${name}`,
          name,
          type: f.type || `image/${ext}`,
          size,
          url: reader.result as string,
        })
      }
      reader.readAsDataURL(f)
    } else {
      files.value.push({
        id: crypto.randomUUID?.() || `${Date.now()}-${f.name}`,
        name: f.name,
        type: 'file',
        size: f.size,
        url: (f as { path?: string }).path || f.name,
      })
    }
  }
}

function toggleWebSearchPopup() {
  showQuickPhrasePopup.value = false
  if (webSearchEnabled.value) {
    agent.setWebSearchEnabled(false)
    showWebSearchPopup.value = false
  } else {
    showWebSearchPopup.value = !showWebSearchPopup.value
  }
}

function toggleQuickPhrasePopup() {
  showWebSearchPopup.value = false
  showQuickPhrasePopup.value = !showQuickPhrasePopup.value
}

function onWebSearchSelect(engine: WebSearchEngine) {
  agent.setSearchEngine(engine)
  agent.setWebSearchEnabled(true)
  emit('web-search', engine)
  showWebSearchPopup.value = false
}

function insertQuickPhrase(phrase: string) {
  text.value = phrase
  void nextTick(() => {
    textareaRef.value?.focus()
    autoResize()
  })
  emit('quick-phrase', phrase)
  showQuickPhrasePopup.value = false
}

function onAddQuickPhrase(phrase: string) {
  agent.addQuickPhrase(phrase)
}

function onDocumentClick(e: MouseEvent) {
  const root = barLeftRef.value
  if (!root || (!showWebSearchPopup.value && !showQuickPhrasePopup.value && !showMentionPopup.value)) return
  if (e.target instanceof Node && !root.contains(e.target)) {
    showWebSearchPopup.value = false
    showQuickPhrasePopup.value = false
    showMentionPopup.value = false
  }
}

onMounted(() => {
  document.addEventListener('click', onDocumentClick, true)
})
onUnmounted(() => {
  document.removeEventListener('click', onDocumentClick, true)
})

function closePopups() {
  showWebSearchPopup.value = false
  showQuickPhrasePopup.value = false
  showMentionPopup.value = false
}

function toggleMentionPopup() {
  showWebSearchPopup.value = false
  showQuickPhrasePopup.value = false
  showMentionPopup.value = !showMentionPopup.value
}

function onMentionSelect(agentId: string, agentName: string) {
  text.value += `@${agentName} `
  showMentionPopup.value = false
  emit('mention', agentId, agentName)
  nextTick(() => textareaRef.value?.focus())
}

function onPermissionChange(v: PermissionLevel) {
  closePopups()
  agent.setPermission(v)
}
</script>

<template>
  <div class="inputbar-outer">
    <div
      class="inputbar-container"
      :class="{ 'file-dragging': isDragOver }"
      @dragover="onDragOver"
      @dragleave="onDragLeave"
      @drop="onDrop"
    >
      <div v-if="files.length" class="file-chips">
        <div v-for="f in files" :key="f.id" class="chip">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
            <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
            <polyline points="14 2 14 8 20 8" />
          </svg>
          <span class="chip-name">{{ f.name }}</span>
          <button type="button" class="chip-rm" @click="removeFile(f.id)">&times;</button>
        </div>
      </div>

      <textarea
        id="message-input-bar"
        ref="textareaRef"
        v-model="text"
        class="textarea"
        :placeholder="streaming ? t('chat.streaming') : t('chat.placeholder')"
        :disabled="streaming"
        rows="1"
        @keydown="onKeydown"
        @input="autoResize"
      />

      <div class="bottom-bar">
        <div ref="barLeftRef" class="bar-left">
          <WebSearchPopup
            :visible="showWebSearchPopup"
            @select="onWebSearchSelect"
            @close="showWebSearchPopup = false"
          />
          <QuickPhrasePopup
            :visible="showQuickPhrasePopup"
            :phrases="quickPhrases"
            @select="insertQuickPhrase"
            @add="onAddQuickPhrase"
            @close="showQuickPhrasePopup = false"
          />
          <AgentMentionPopup
            :visible="showMentionPopup"
            @select="onMentionSelect"
            @close="showMentionPopup = false"
          />

          <el-tooltip :content="t('chat.newTopic')" placement="top" :show-after="400">
            <button type="button" class="bar-btn" @click="closePopups(); emit('new-topic')">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
              </svg>
            </button>
          </el-tooltip>

          <el-tooltip v-if="!streaming" :content="t('chat.attach')" placement="top" :show-after="400">
            <button type="button" class="bar-btn" @click="closePopups(); onAttach()">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <path d="M21.44 11.05l-9.19 9.19a6 6 0 01-8.49-8.49l9.19-9.19a4 4 0 015.66 5.66l-9.2 9.19a2 2 0 01-2.83-2.83l8.49-8.48" />
              </svg>
            </button>
          </el-tooltip>

          <el-tooltip :content="t('chat.webSearch')" placement="top" :show-after="400">
            <button type="button" class="bar-btn" :class="{ 'bar-btn--active': webSearchEnabled }" @click="toggleWebSearchPopup">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <circle cx="12" cy="12" r="10" /><line x1="2" y1="12" x2="22" y2="12" />
                <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
              </svg>
              <span v-if="webSearchEnabled" class="bar-btn__dot" />
            </button>
          </el-tooltip>

          <el-tooltip v-if="props.isGroupChat" :content="t('chat.mentionAgent')" placement="top" :show-after="400">
            <button type="button" class="bar-btn" :class="{ 'bar-btn--active': showMentionPopup }" @click="toggleMentionPopup">
              <span class="bar-btn__at">@</span>
            </button>
          </el-tooltip>

          <el-tooltip :content="t('chat.quickPhrases')" placement="top" :show-after="400">
            <button type="button" class="bar-btn" @click="toggleQuickPhrasePopup">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8z" />
              </svg>
            </button>
          </el-tooltip>

          <el-select
            :model-value="permissionLevel"
            class="perm-select"
            size="small"
            @change="onPermissionChange"
          >
            <template #prefix>
              <svg
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                aria-hidden="true"
              >
                <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
                <path d="M7 11V7a5 5 0 0 1 10 0v4" />
              </svg>
            </template>
            <el-option value="default" label="默认权限" />
            <el-option value="full" label="完全权限" />
          </el-select>

        </div>

        <div class="bar-right">
          <span class="hint">
            {{ sendOnEnter ? 'Enter' : 'Shift+Enter' }}
          </span>
          <button
            v-if="streaming"
            type="button"
            class="stop-btn"
            :title="t('chat.stop')"
            @click="emit('stop')"
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
              <rect x="6" y="6" width="12" height="12" rx="2" />
            </svg>
          </button>
          <button
            v-else
            type="button"
            class="send-btn"
            :class="{ dim: !text.trim() && !files.length }"
            :disabled="!text.trim() && !files.length"
            :title="t('chat.send')"
            @click="doSend"
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"
            >
              <line x1="22" y1="2" x2="11" y2="13" />
              <polygon points="22 2 15 22 11 13 2 9 22 2" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.inputbar-outer {
  flex-shrink: 0;
  padding: 0 18px 18px 18px;
  position: relative;
  z-index: 2;
}

.inputbar-container {
  position: relative;
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-md);
  background-color: var(--color-background-opacity);
  backdrop-filter: blur(var(--glass-blur-light));
  -webkit-backdrop-filter: blur(var(--glass-blur-light));
  padding-top: 8px;
  transition: border-color 0.1s var(--fox-ease), box-shadow 0.1s var(--fox-ease);

  &:focus-within {
    border-color: var(--color-text-3);
    box-shadow: var(--shadow-md);
  }

  &.file-dragging {
    border: 1px dashed var(--color-text-3);
    background: var(--color-primary-mute);
  }
}

.file-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 0 15px 6px;
}

.chip {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  background: var(--color-background-mute);
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-sm);
  font-size: 11px;
  color: var(--color-text-2);
  transition: background 0.15s, border-color 0.15s;

  &:hover {
    background: var(--color-hover);
  }
}

.chip-name {
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chip-rm {
  background: none;
  border: none;
  color: var(--color-text-3);
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
  padding: 0 1px;
  transition: color 0.15s;
  border-radius: var(--fox-radius-sm);

  &:hover {
    color: var(--color-error);
  }
  &:active {
    background: var(--color-active);
  }
}

.textarea {
  display: block;
  width: 100%;
  border: none;
  background: transparent;
  color: var(--color-text-1);
  font-size: 14px;
  font-family: inherit;
  line-height: 1.5;
  resize: none;
  outline: none;
  padding: 6px 15px 0;
  min-height: 30px;
  max-height: 200px;

  &::-webkit-scrollbar {
    width: 3px;
  }
  &::placeholder {
    color: var(--color-text-3);
  }
  &:disabled {
    opacity: 0.5;
  }
}

.bottom-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 40px;
  padding: 0 8px;
  gap: 6px;
}

.bar-left,
.bar-right {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  flex-shrink: 0;
}

.bar-left {
  position: relative;
  flex: 1;
}

.bar-btn {
  position: relative;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--color-border);
  background: var(--color-background-mute);
  color: var(--color-icon);
  cursor: pointer;
  border-radius: var(--fox-radius-sm);
  flex-shrink: 0;
  transition: all 0.15s;

  &:hover {
    color: var(--color-icon-white);
    background: var(--color-hover);
    border-color: var(--color-text-3);
  }
  &:active {
    background: var(--color-active);
  }

  &--active {
    border-color: #22c55e;
    color: #22c55e;
  }

  &--dim {
    opacity: 0.55;
    cursor: default;
    pointer-events: none;
  }
}

.bar-btn__dot {
  position: absolute;
  top: 2px;
  right: 2px;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #22c55e;
  border: 1px solid var(--color-background);
}

.bar-btn__at {
  font-size: 13px;
  font-weight: 700;
  line-height: 1;
  color: var(--color-text-2);
}

.perm-select {
  max-width: 120px;
  width: 120px;
  flex-shrink: 0;
  --el-border-radius-base: var(--fox-radius-sm);

  :deep(.el-select__wrapper) {
    min-height: 28px !important;
    height: 28px;
    font-size: 12px;
    background: var(--color-background-mute) !important;
    border: 1px solid var(--color-border) !important;
    box-shadow: none !important;
  }
  :deep(.el-select__selected-item) {
    color: var(--color-text-1);
  }
  :deep(.el-select__prefix) {
    display: flex;
    align-items: center;
    color: var(--color-icon);
    margin-right: 4px;
  }
  :deep(.el-select__caret) {
    color: var(--color-text-3);
  }
}

.hint {
  font-size: 11px;
  color: var(--color-text-3);
  user-select: none;
}

.send-btn,
.stop-btn {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid transparent;
  border-radius: 50%;
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.1s var(--fox-ease);
}

.send-btn {
  background: var(--color-text-1);
  color: var(--color-background);
  box-shadow: var(--shadow-sm);

  &:hover:not(:disabled) {
    transform: scale(1.04);
    box-shadow: var(--shadow-md);
  }

  &:active:not(:disabled) {
    transform: scale(0.98);
  }

  &.dim,
  &:disabled {
    opacity: 0.2;
    cursor: not-allowed;
    transform: none;
  }
}

.stop-btn {
  background: var(--color-error);
  color: #fff;
  box-shadow: var(--shadow-sm);
  border-color: rgba(0, 0, 0, 0.05);

  &:hover {
    transform: scale(1.04);
    filter: brightness(1.05);
  }
  &:active {
    transform: scale(0.98);
  }
}
</style>
