import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { v4 as uuidv4 } from 'uuid'
import type { MiniProgram } from '@/types'

const PRESET_PROGRAMS: Omit<MiniProgram, 'id' | 'sortOrder'>[] = [
  { name: 'ChatGPT', url: 'https://chat.openai.com', icon: '✨' },
  { name: 'Claude', url: 'https://claude.ai', icon: '🟠' },
  { name: 'Gemini', url: 'https://gemini.google.com', icon: '💎' },
  { name: 'DeepSeek', url: 'https://chat.deepseek.com', icon: '🔵' },
  { name: 'Kimi', url: 'https://kimi.moonshot.cn', icon: '🌙' },
  { name: 'Perplexity', url: 'https://www.perplexity.ai', icon: '🔮' },
  { name: 'Poe', url: 'https://poe.com', icon: '⚡' },
  { name: '文心一言', url: 'https://yiyan.baidu.com', icon: '🎯' },
  { name: '通义千问', url: 'https://tongyi.aliyun.com', icon: '🟣' },
  { name: '豆包', url: 'https://www.doubao.com', icon: '🫘' },
  { name: 'Groq', url: 'https://groq.com', icon: '🟢' },
  { name: 'Google', url: 'https://www.google.com', icon: '🔍' }
]

export const useMiniprogramStore = defineStore('miniprogram', () => {
  const programs = ref<MiniProgram[]>([])

  const sorted = computed(() => [...programs.value].sort((a, b) => a.sortOrder - b.sortOrder))

  function initPresets() {
    if (programs.value.length > 0) return
    programs.value = PRESET_PROGRAMS.map((p, i) => ({
      id: `preset-${i}-${p.name.toLowerCase().replace(/\s+/g, '-')}`,
      name: p.name, url: p.url, icon: p.icon, sortOrder: i
    }))
  }

  function addProgram(data: { name: string; url: string; icon: string }): MiniProgram {
    const maxOrder = programs.value.reduce((m, p) => Math.max(m, p.sortOrder), -1)
    const p: MiniProgram = { id: uuidv4(), name: data.name, url: data.url, icon: data.icon || '🧩', sortOrder: maxOrder + 1 }
    programs.value.push(p)
    return p
  }

  function updateProgram(id: string, data: Partial<Pick<MiniProgram, 'name' | 'url' | 'icon'>>) {
    const p = programs.value.find(x => x.id === id)
    if (p) {
      if (data.name != null) p.name = data.name
      if (data.url != null) p.url = data.url
      if (data.icon != null) p.icon = data.icon
    }
  }

  function removeProgram(id: string) {
    programs.value = programs.value.filter(p => p.id !== id)
  }

  return { programs, sorted, initPresets, addProgram, updateProgram, removeProgram }
}, { persist: true })
