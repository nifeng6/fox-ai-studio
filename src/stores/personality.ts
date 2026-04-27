import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { v4 as uuidv4 } from 'uuid'
import type { Personality } from '@/types'

const PRESET_SEED: Array<Omit<Personality, 'id' | 'createdAt' | 'isDefault'>> = [
  {
    name: 'Default',
    avatar: '🤖',
    description: '通用助手，友好且清晰。',
    systemPrompt: '你是一个有帮助的AI助手。请用清晰、准确的中文回答。'
  },
  {
    name: 'Coder',
    avatar: '💻',
    description: '擅长编程、架构与代码审查。',
    systemPrompt:
      '你是一位资深软件工程师，精通多种编程语言、系统设计与最佳实践。请给出可执行、可维护的解决方案，必要时用代码说明。'
  },
  {
    name: 'Writer',
    avatar: '✍️',
    description: '创意写作、润色与叙事。',
    systemPrompt:
      '你是一位专业作家，擅长叙述、润色与多文体创作。请注重结构、节奏与读者体验，在需要时保持简洁。'
  },
  {
    name: 'Analyst',
    avatar: '📊',
    description: '数据解读、指标与业务分析。',
    systemPrompt:
      '你是一位数据分析师，擅长从数据与信息中提取洞察，说明假设、局限与可验证结论，避免无根据的推断。'
  }
]

function seedPresets(): Personality[] {
  const t = Date.now()
  return PRESET_SEED.map((p, i) => ({
    ...p,
    id: uuidv4(),
    isDefault: i === 0,
    createdAt: t
  }))
}

export const usePersonalityStore = defineStore('personality', () => {
  const personalities = ref<Personality[]>([])

  const defaultPersonality = computed(() => personalities.value.find(p => p.isDefault) || null)

  function ensurePresets() {
    if (personalities.value.length) return
    personalities.value = seedPresets()
  }

  function getById(id: string): Personality | null {
    return personalities.value.find(p => p.id === id) || null
  }

  function getDefault(): Personality | null {
    return defaultPersonality.value
  }

  function createPersonality(data: {
    name: string
    avatar: string
    description: string
    systemPrompt: string
    isDefault?: boolean
  }): Personality {
    const p: Personality = {
      id: uuidv4(),
      name: data.name.trim() || 'Unnamed',
      avatar: data.avatar,
      description: data.description,
      systemPrompt: data.systemPrompt,
      isDefault: !!data.isDefault,
      createdAt: Date.now()
    }
    if (p.isDefault) {
      for (const x of personalities.value) x.isDefault = false
    }
    personalities.value = [...personalities.value, p]
    return p
  }

  function updatePersonality(id: string, partial: Partial<Omit<Personality, 'id' | 'createdAt'>>) {
    const p = personalities.value.find(x => x.id === id)
    if (!p) return
    if (partial.name !== undefined) p.name = partial.name
    if (partial.avatar !== undefined) p.avatar = partial.avatar
    if (partial.description !== undefined) p.description = partial.description
    if (partial.systemPrompt !== undefined) p.systemPrompt = partial.systemPrompt
    if (partial.isDefault === true) {
      for (const x of personalities.value) x.isDefault = x.id === id
    }
  }

  function deletePersonality(id: string) {
    const was = personalities.value.find(p => p.id === id)
    personalities.value = personalities.value.filter(p => p.id !== id)
    if (was?.isDefault && personalities.value[0]) {
      setDefault(personalities.value[0].id)
    }
  }

  function setDefault(id: string) {
    for (const p of personalities.value) p.isDefault = p.id === id
  }

  return {
    personalities,
    defaultPersonality,
    ensurePresets,
    getById,
    getDefault,
    createPersonality,
    updatePersonality,
    deletePersonality,
    setDefault
  }
}, {
  persist: {
    pick: ['personalities'] as const
  }
})
