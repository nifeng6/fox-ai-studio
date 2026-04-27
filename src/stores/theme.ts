import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import type { ThemeMode } from '@/types'

export const useThemeStore = defineStore('theme', () => {
  const mode = ref<ThemeMode>('light')
  const accentColor = ref('#1a1a1a')
  const fontSize = ref(14)
  const fontFamily = ref('')

  function setMode(newMode: ThemeMode) {
    mode.value = newMode
    applyTheme()
  }

  function setAccentColor(color: string) {
    accentColor.value = color
    applyAccent(color)
  }

  function applyAccent(color: string) {
    const root = document.documentElement
    root.style.setProperty('--fox-accent', color)
    root.style.setProperty('--color-primary', color)
    root.style.setProperty('--color-primary-soft', `${color}80`)
    root.style.setProperty('--color-primary-mute', `${color}20`)
    root.style.setProperty('--el-color-primary', color)
  }

  function applyTheme() {
    let effective = mode.value
    if (effective === 'system') {
      effective = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
    }
    document.body.setAttribute('theme-mode', effective)
    document.documentElement.setAttribute('theme-mode', effective)
  }

  function applyFontSize() {
    const root = document.documentElement
    root.style.setProperty('--fox-fs', `${fontSize.value}px`)
    root.style.fontSize = `${fontSize.value}px`
  }

  function applyFontFamily() {
    if (fontFamily.value) {
      document.documentElement.style.setProperty('--user-font-family', `'${fontFamily.value}'`)
    }
  }

  function init() {
    applyTheme()
    applyFontSize()
    applyFontFamily()
    applyAccent(accentColor.value)
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
      if (mode.value === 'system') applyTheme()
    })
  }

  watch(mode, applyTheme)
  watch(fontSize, applyFontSize)
  watch(fontFamily, applyFontFamily)

  return {
    mode, accentColor, fontSize, fontFamily,
    setMode, setAccentColor, applyTheme, applyFontSize, applyFontFamily, init
  }
}, {
  persist: true
})
