<template>
  <div class="sp-root" @mouseleave="startAutoClose" @mouseenter="cancelAutoClose">
    <div class="sp-toolbar" v-if="!showResult">
      <div class="sp-text-preview" :title="selectedText">
        {{ selectedText.length > 80 ? selectedText.slice(0, 80) + '...' : selectedText }}
      </div>
      <div class="sp-actions">
        <button v-for="act in actions" :key="act.id" class="sp-btn" @click="doAction(act)">
          {{ act.label }}
        </button>
        <button class="sp-btn" @click="doCopy">复制</button>
        <span class="sp-divider" />
        <button class="sp-close" @click="closeWindow">✕</button>
      </div>
    </div>

    <div class="sp-result" v-if="showResult">
      <div class="sp-result-header">
        <span class="sp-result-label">{{ currentActionLabel }}</span>
        <button class="sp-btn sp-btn--sm" @click="goBack">← 返回</button>
        <button class="sp-close" @click="closeWindow">✕</button>
      </div>
      <div class="sp-result-body" ref="resultRef">
        <div v-if="loading && !resultText" class="sp-loading">
          <span class="sp-dot" /><span class="sp-dot" /><span class="sp-dot" />
        </div>
        <div v-if="resultText" class="sp-result-text">{{ resultText }}</div>
      </div>
      <div class="sp-result-footer" v-if="resultText && !loading">
        <button class="sp-btn" @click="copyResult">复制结果</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useSettingsStore } from '@/stores/settings'
import { useAgentStore } from '@/stores/agent'
import {
  sendChatMessage,
  onStreamChunk,
  onStreamEnd,
  onStreamError,
} from '@/utils/tauri-api'

const settings = useSettingsStore()
const agent = useAgentStore()

const selectedText = ref('')
const resultText = ref('')
const loading = ref(false)
const showResult = ref(false)
const currentActionLabel = ref('')
const resultRef = ref<HTMLElement>()
let autoCloseTimer: ReturnType<typeof setTimeout> | null = null

const actions = [
  { id: 'translate', label: '翻译', prompt: '请将以下文本翻译成中文（如果是中文则翻译成英文），仅输出译文：\n\n' },
  { id: 'explain', label: '解释', prompt: '请简要解释以下内容（50字以内）：\n\n' },
  { id: 'summarize', label: '总结', prompt: '请用3个要点总结以下内容：\n\n' },
  { id: 'rewrite', label: '改写', prompt: '请改写以下文本使其更流畅，仅输出改写结果：\n\n' },
]

onMounted(async () => {
  const params = new URLSearchParams(window.location.hash.split('?')[1] || '')
  selectedText.value = decodeURIComponent(params.get('text') || '')

  if (!selectedText.value) {
    try {
      const { listen } = await import('@tauri-apps/api/event')
      const unlisten = await listen<{ text: string }>('selection-popup:set-text', (e) => {
        selectedText.value = e.payload.text
      })
      cleanups.push(unlisten)
    } catch {}
  }

  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    const win = getCurrentWindow()
    await win.setFocus()

    const unlisten = await win.onFocusChanged(({ payload: focused }) => {
      if (!focused && !loading.value) {
        requestClose()
      }
    })
    cleanups.push(unlisten)
  } catch {}
})

const cleanups: Array<() => void> = []
onBeforeUnmount(() => { cleanups.forEach(fn => fn()) })

async function requestClose() {
  try {
    const { emit } = await import('@tauri-apps/api/event')
    await emit('selection-popup:request-close')
  } catch {}
  setTimeout(() => closeWindow(), 50)
}

function startAutoClose() {
  cancelAutoClose()
  if (!showResult.value) autoCloseTimer = setTimeout(() => closeWindow(), 4000)
}
function cancelAutoClose() {
  if (autoCloseTimer) { clearTimeout(autoCloseTimer); autoCloseTimer = null }
}

async function closeWindow() {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    await getCurrentWindow().close()
  } catch { window.close() }
}

async function doCopy() {
  try {
    const clip = await import('@tauri-apps/plugin-clipboard-manager')
    await clip.writeText(selectedText.value)
  } catch {
    try { await navigator.clipboard.writeText(selectedText.value) } catch {}
  }
  closeWindow()
}

async function copyResult() {
  if (!resultText.value) return
  try {
    const clip = await import('@tauri-apps/plugin-clipboard-manager')
    await clip.writeText(resultText.value)
  } catch {
    try { await navigator.clipboard.writeText(resultText.value) } catch {}
  }
}

function goBack() {
  showResult.value = false
  resultText.value = ''
  loading.value = false
  resizeWindow(340, 90)
}

async function resizeWindow(w: number, h: number) {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    const { LogicalSize } = await import('@tauri-apps/api/dpi')
    await getCurrentWindow().setSize(new LogicalSize(w, h))
  } catch {}
}

async function doAction(action: typeof actions[0]) {
  if (!selectedText.value.trim()) return
  cancelAutoClose()
  showResult.value = true
  currentActionLabel.value = action.label
  loading.value = true
  resultText.value = ''
  await resizeWindow(400, 280)

  try {
    const lang = agent.currentAgent?.languageModel
    if (!lang?.providerId || !lang?.modelId) {
      resultText.value = '请先在设置中配置模型'
      loading.value = false
      return
    }
    const messageId = `sel-${Date.now()}`
    const unChunk = await onStreamChunk((p) => {
      if (p.messageId === messageId) {
        resultText.value += p.chunk
        nextTick(() => { if (resultRef.value) resultRef.value.scrollTop = resultRef.value.scrollHeight })
      }
    })
    let unEnd: (() => void) | undefined
    let unErr: (() => void) | undefined
    const stop = () => { unChunk(); unEnd?.(); unErr?.() }
    unEnd = await onStreamEnd((p) => { if (p.messageId === messageId) { loading.value = false; stop() } })
    unErr = await onStreamError((p) => {
      if (p.messageId === messageId) { resultText.value = p.error || '处理失败'; loading.value = false; stop() }
    })
    await sendChatMessage({
      providerId: lang.providerId, modelId: lang.modelId, messageId,
      messages: [{ role: 'user', content: action.prompt + selectedText.value }]
    })
  } catch (err) {
    resultText.value = '处理失败: ' + String(err)
    loading.value = false
  }
}
</script>

<style>
html, body, #app, .popup-shell {
  background: #fff !important;
  margin: 0 !important;
  padding: 0 !important;
  overflow: hidden !important;
}
</style>

<style scoped>
* { margin: 0; padding: 0; box-sizing: border-box; }

.sp-root {
  width: 100%;
  height: 100%;
  background: #ffffff;
  color: #1d1d1f;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  font-size: 13px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.sp-toolbar {
  display: flex;
  flex-direction: column;
  padding: 10px 12px 8px;
  gap: 6px;
}

.sp-text-preview {
  font-size: 11px;
  color: rgba(0, 0, 0, 0.38);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  line-height: 1.3;
  letter-spacing: 0.01em;
}

.sp-actions {
  display: flex;
  gap: 5px;
  align-items: center;
}

.sp-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 4px 12px;
  border: none;
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.05);
  color: #1d1d1f;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.1s;
  white-space: nowrap;
  letter-spacing: 0.02em;
}
.sp-btn:hover {
  background: rgba(0, 0, 0, 0.10);
}
.sp-btn:active { transform: scale(0.96); }
.sp-btn--sm { padding: 2px 8px; font-size: 11px; }

.sp-divider {
  width: 1px;
  height: 14px;
  background: rgba(0, 0, 0, 0.08);
  flex-shrink: 0;
}

.sp-close {
  margin-left: auto;
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: rgba(0, 0, 0, 0.30);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.1s;
  flex-shrink: 0;
}
.sp-close:hover { background: rgba(0, 0, 0, 0.06); color: #e53e3e; }

.sp-result {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.sp-result-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
}

.sp-result-label {
  font-size: 13px;
  font-weight: 600;
  color: #1d1d1f;
  flex: 1;
}

.sp-result-body {
  flex: 1;
  overflow-y: auto;
  padding: 10px 12px;
  line-height: 1.65;
}
.sp-result-body::-webkit-scrollbar { width: 3px; }
.sp-result-body::-webkit-scrollbar-thumb { background: rgba(0, 0, 0, 0.1); border-radius: 2px; }

.sp-result-text {
  white-space: pre-wrap;
  word-break: break-word;
  font-size: 13px;
  color: #333;
}

.sp-loading {
  display: flex;
  gap: 4px;
  padding: 8px 0;
}
.sp-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.25);
  animation: dot-bounce 1.2s infinite ease-in-out;
}
.sp-dot:nth-child(2) { animation-delay: 0.15s; }
.sp-dot:nth-child(3) { animation-delay: 0.3s; }

@keyframes dot-bounce {
  0%, 80%, 100% { opacity: 0.3; transform: scale(0.8); }
  40% { opacity: 1; transform: scale(1.1); }
}

.sp-result-footer {
  padding: 6px 12px;
  border-top: 1px solid rgba(0, 0, 0, 0.05);
  display: flex;
  justify-content: flex-end;
}
</style>
