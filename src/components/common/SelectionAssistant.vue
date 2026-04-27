<script setup lang="ts">
import { onMounted, onUnmounted, watch, computed } from 'vue'
import { useSettingsStore } from '@/stores/settings'
import { tauriInvoke } from '@/utils/tauri-api'

const settings = useSettingsStore()
const isEnabled = computed(() => settings.selectionAssistantEnabled)

let unlistenSelection: (() => void) | null = null
let popupLabel: string | null = null
let popupCreating = false

let _WebviewWindow: any = null
let _PhysicalPosition: any = null
let _PhysicalSize: any = null
let _listen: any = null

async function preloadModules() {
  try {
    const [wv, dpi, ev] = await Promise.all([
      import('@tauri-apps/api/webviewWindow'),
      import('@tauri-apps/api/dpi'),
      import('@tauri-apps/api/event'),
    ])
    _WebviewWindow = wv.WebviewWindow
    _PhysicalPosition = dpi.PhysicalPosition
    _PhysicalSize = dpi.PhysicalSize
    _listen = ev.listen
  } catch {}
}

async function startWatcher() {
  try {
    await tauriInvoke('start_selection_watcher')
  } catch (e) {
    console.warn('[SelectionAssistant] Failed to start watcher:', e)
  }

  try {
    const listen = _listen || (await import('@tauri-apps/api/event')).listen
    const unlisten = await listen(
      'selection:detected',
      (event: any) => {
        if (!isEnabled.value) return
        openPopup(event.payload.text, event.payload.mouseX, event.payload.mouseY)
      }
    )
    unlistenSelection = unlisten
  } catch (e) {
    console.warn('[SelectionAssistant] Failed to listen:', e)
  }
}

async function stopWatcher() {
  try { await tauriInvoke('stop_selection_watcher') } catch {}
  unlistenSelection?.()
  unlistenSelection = null
}

async function openPopup(text: string, physX: number, physY: number) {
  if (popupCreating) return
  popupCreating = true

  await closePopup()

  try {
    const WW = _WebviewWindow || (await import('@tauri-apps/api/webviewWindow')).WebviewWindow
    const PP = _PhysicalPosition || (await import('@tauri-apps/api/dpi')).PhysicalPosition
    const PS = _PhysicalSize || (await import('@tauri-apps/api/dpi')).PhysicalSize

    const label = `sel-popup-${Date.now()}`
    popupLabel = label

    const encodedText = encodeURIComponent(text)
    const url = `index.html#/selection-popup?text=${encodedText}`

    const px = Math.round(physX) + 12
    const py = Math.round(physY) + 16

    const popup = new WW(label, {
      url,
      title: '',
      width: 340,
      height: 90,
      x: px,
      y: py,
      resizable: false,
      decorations: false,
      transparent: false,
      alwaysOnTop: true,
      skipTaskbar: true,
      focus: false,
      visible: false,
      shadow: true,
    })

    popup.once('tauri://created', async () => {
      try {
        await popup.setPosition(new PP(px, py))
        await popup.setSize(new PS(340, 90))
        await popup.show()
        await popup.setFocus()
      } catch {}
    })

    popup.once('tauri://error', (e: any) => {
      console.warn('[SelectionPopup] Error:', e)
      popupLabel = null
    })
  } catch (e) {
    console.warn('[SelectionAssistant] Popup failed:', e)
    popupLabel = null
  } finally {
    popupCreating = false
  }
}

async function closePopup() {
  if (!popupLabel) return
  const label = popupLabel
  popupLabel = null
  try {
    const WW = _WebviewWindow || (await import('@tauri-apps/api/webviewWindow')).WebviewWindow
    const existing = await WW.getByLabel(label)
    if (existing) await existing.close()
  } catch {}
}

let unlistenClosePopup: (() => void) | null = null

async function setupCloseListener() {
  try {
    const listen = _listen || (await import('@tauri-apps/api/event')).listen
    unlistenClosePopup = await listen('selection-popup:request-close', () => {
      closePopup()
    })
  } catch {}
}

watch(isEnabled, (on) => {
  if (on) startWatcher()
  else { stopWatcher(); closePopup() }
})

let unlistenTray: (() => void) | null = null

onMounted(async () => {
  await preloadModules()
  if (isEnabled.value) await startWatcher()
  await setupCloseListener()
  try {
    const listen = _listen || (await import('@tauri-apps/api/event')).listen
    unlistenTray = await listen('tray:toggle-selection', () => {
      settings.selectionAssistantEnabled = !settings.selectionAssistantEnabled
    })
  } catch {}
})

onUnmounted(() => {
  stopWatcher()
  closePopup()
  unlistenTray?.()
  unlistenClosePopup?.()
})
</script>

<template><!-- Controller only --></template>
