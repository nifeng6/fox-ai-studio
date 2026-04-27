<script setup lang="ts">
import { onMounted, onUnmounted, watch, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import AppLayout from './components/common/AppLayout.vue'
import SelectionAssistant from '@/components/common/SelectionAssistant.vue'
import { useThemeStore } from './stores/theme'
import { useAssistantStore } from './stores/assistant'
import { useMiniprogramStore } from './stores/miniprogram'
import { useSettingsStore } from './stores/settings'
import { useScheduleStore } from './stores/schedule'
import { useChatStore } from './stores/chat'

const route = useRoute()
const router = useRouter()
const themeStore = useThemeStore()
const assistantStore = useAssistantStore()
const miniprogramStore = useMiniprogramStore()
const settings = useSettingsStore()
const scheduleStore = useScheduleStore()
const chatStore = useChatStore()

const isPopup = computed(() => !!route.meta?.isPopup)

function applyLayoutSettings() {
  const z = settings.zoom
  if (z >= 50 && z <= 200) {
    document.documentElement.style.zoom = String(z / 100)
  } else {
    document.documentElement.style.removeProperty('zoom')
  }
  if (settings.globalFont?.trim()) {
    document.body.style.setProperty('font-family', settings.globalFont, 'important')
  } else {
    document.body.style.removeProperty('font-family')
  }
  document.documentElement.style.setProperty(
    '--user-code-font',
    settings.codeFont?.trim() || 'inherit'
  )
  const styleEl = document.getElementById('user-custom-css') as HTMLStyleElement | null
  const css = settings.customCss?.trim() || ''
  if (css) {
    const el = styleEl || document.createElement('style')
    el.id = 'user-custom-css'
    el.textContent = css
    if (!styleEl) document.head.appendChild(el)
  } else if (styleEl) {
    styleEl.textContent = ''
  }
}

function findShortcut(id: string) {
  return settings.shortcuts.find(s => s.id === id)
}

function matchesShortcut(e: KeyboardEvent, keys: string): boolean {
  const parts = keys.split('+')
  const needCtrl = parts.includes('CommandOrControl')
  const needShift = parts.includes('Shift')
  const needAlt = parts.includes('Alt')
  const mainKey = parts.filter(p => !['CommandOrControl', 'Shift', 'Alt'].includes(p))[0] || ''

  if (needCtrl && !(e.ctrlKey || e.metaKey)) return false
  if (!needCtrl && (e.ctrlKey || e.metaKey)) return false
  if (needShift !== e.shiftKey) return false
  if (needAlt !== e.altKey) return false

  const pressed = e.key.length === 1 ? e.key.toUpperCase() : e.key
  return pressed === mainKey || e.key === mainKey
}

function handleAppShortcut(e: KeyboardEvent) {
  for (const sc of settings.shortcuts) {
    if (!sc.enabled || sc.isGlobal) continue
    if (!matchesShortcut(e, sc.keys)) continue
    e.preventDefault()
    e.stopPropagation()
    executeAction(sc.id)
    return
  }
}

function executeAction(id: string) {
  switch (id) {
    case 'zoomIn':
      settings.zoom = Math.min(200, settings.zoom + 10)
      break
    case 'zoomOut':
      settings.zoom = Math.max(50, settings.zoom - 10)
      break
    case 'zoomReset':
      settings.zoom = 100
      break
    case 'openSettings':
      router.push('/settings')
      break
    case 'newChat':
      chatStore.createTopic()
      break
  }
}

async function registerGlobalShortcuts() {
  try {
    const { register, unregisterAll } = await import('@tauri-apps/plugin-global-shortcut')
    await unregisterAll()
    for (const sc of settings.shortcuts) {
      if (!sc.enabled || !sc.isGlobal) continue
      try {
        await register(sc.keys, () => {
          if (sc.id === 'toggleApp') {
            import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
              const win = getCurrentWindow()
              win.isVisible().then(visible => {
                if (visible) { win.hide() } else { win.show(); win.setFocus() }
              })
            })
          }
        })
      } catch (err) {
        console.warn(`Failed to register global shortcut ${sc.keys}:`, err)
      }
    }
  } catch {}
}

onMounted(() => {
  themeStore.init()
  assistantStore.initPresets()
  miniprogramStore.initPresets()
  applyLayoutSettings()
  document.addEventListener('keydown', handleAppShortcut)
  registerGlobalShortcuts()

  if (isPopup.value) {
    document.documentElement.style.background = 'transparent'
    document.body.style.background = 'transparent'
    const app = document.getElementById('app')
    if (app) app.style.background = 'transparent'
  } else {
    scheduleStore.initTimers()
  }
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleAppShortcut)
})

watch(
  () => [settings.zoom, settings.globalFont, settings.codeFont, settings.customCss] as const,
  () => applyLayoutSettings(),
  { deep: true }
)

watch(
  () => JSON.stringify(settings.shortcuts),
  () => registerGlobalShortcuts()
)
</script>

<template>
  <div v-if="isPopup" class="popup-shell">
    <router-view />
  </div>
  <div v-else class="app-root">
    <AppLayout />
    <SelectionAssistant />
  </div>
</template>

<style scoped>
.popup-shell {
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  background: transparent;
}
.app-root {
  width: 100%;
  height: 100%;
}
</style>
