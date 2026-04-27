import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { v4 as uuidv4 } from 'uuid'
import type { Skill } from '@/types'

const PRESET_SEED: Array<Omit<Skill, 'id' | 'usageCount' | 'lastUsed' | 'createdAt'>> = [
  {
    name: '代码审查',
    description: '审查代码质量、漏洞与可维护性',
    trigger: '代码、review、bug、审查',
    category: 'development',
    instructions:
      '对提供的代码做结构化审查：正确性、边界情况、安全、性能、可读性；用条目列出问题与建议。',
    enabled: true
  },
  {
    name: '文档生成',
    description: '从代码或说明生成技术文档',
    trigger: '文档、说明、doc、README',
    category: 'writing',
    instructions:
      '根据上下文生成清晰的技术说明：可含概述、安装、API、示例与注意事项，使用 Markdown 风格分段。',
    enabled: true
  },
  {
    name: '翻译助手',
    description: '多语言互译与术语一致',
    trigger: '翻译、translate、英文、中文',
    category: 'language',
    instructions:
      '在需要时进行准确翻译，保留专有名词与格式；若未指定目标语言，先询问或根据上下文选择。',
    enabled: true
  },
  {
    name: '数据分析',
    description: '解读数据、指标与可视化建议',
    trigger: '数据、分析、统计、图表',
    category: 'analysis',
    instructions:
      '对数据问题给出分析框架：可能假设、需要的数据、关键指标、可视化建议与可验证结论，避免过度推断。',
    enabled: true
  }
]

function seedPresets(): Skill[] {
  const t = Date.now()
  return PRESET_SEED.map(p => ({
    ...p,
    id: uuidv4(),
    usageCount: 0,
    lastUsed: null,
    createdAt: t,
    version: '1.0.0',
  }))
}

/**
 * Serialize a skill to SKILL.md format (YAML frontmatter + Markdown body)
 */
export function skillToMarkdown(skill: Skill): string {
  const lines = [
    '---',
    `name: "${skill.name}"`,
    `description: "${skill.description}"`,
    `version: "${skill.version || '1.0.0'}"`,
    `trigger: "${skill.trigger}"`,
    `category: "${skill.category || 'general'}"`,
    `enabled: ${skill.enabled}`,
    '---',
    '',
    skill.instructions,
  ]
  return lines.join('\n')
}

/**
 * Parse a SKILL.md file back into skill properties
 */
export function parseSkillMarkdown(content: string): Partial<Skill> {
  const result: Partial<Skill> = {}
  const fmMatch = content.match(/^---\s*\n([\s\S]*?)\n---\s*\n/)
  if (fmMatch) {
    const fm = fmMatch[1]
    const getString = (key: string) => {
      const m = fm.match(new RegExp(`^${key}:\\s*"?(.*?)"?\\s*$`, 'm'))
      return m?.[1] || ''
    }
    result.name = getString('name')
    result.description = getString('description')
    result.version = getString('version') || '1.0.0'
    result.trigger = getString('trigger')
    result.category = getString('category') || 'general'
    const enabledStr = getString('enabled')
    result.enabled = enabledStr !== 'false'
    result.instructions = content.slice(fmMatch[0].length).trim()
  } else {
    result.instructions = content.trim()
  }
  return result
}

export const useSkillStore = defineStore('skill', () => {
  const skills = ref<Skill[]>([])

  function ensurePresets() {
    if (skills.value.length) return
    skills.value = seedPresets()
  }

  function createSkill(data: {
    name: string
    description: string
    trigger: string
    instructions: string
    enabled?: boolean
    category?: string
    version?: string
  }): Skill {
    const s: Skill = {
      id: uuidv4(),
      name: data.name.trim() || 'Skill',
      description: data.description,
      trigger: data.trigger,
      instructions: data.instructions,
      enabled: data.enabled !== false,
      usageCount: 0,
      lastUsed: null,
      createdAt: Date.now(),
      version: data.version || '1.0.0',
      category: data.category || 'general',
    }
    skills.value = [...skills.value, s]
    return s
  }

  function updateSkill(
    id: string,
    partial: Partial<Pick<Skill, 'name' | 'description' | 'trigger' | 'instructions' | 'enabled' | 'category' | 'version'>>
  ) {
    const s = skills.value.find(x => x.id === id)
    if (!s) return
    if (partial.name !== undefined) s.name = partial.name
    if (partial.description !== undefined) s.description = partial.description
    if (partial.trigger !== undefined) s.trigger = partial.trigger
    if (partial.instructions !== undefined) s.instructions = partial.instructions
    if (partial.enabled !== undefined) s.enabled = partial.enabled
    if (partial.category !== undefined) s.category = partial.category
    if (partial.version !== undefined) s.version = partial.version
  }

  function patchSkill(id: string, oldStr: string, newStr: string): boolean {
    const s = skills.value.find(x => x.id === id)
    if (!s) return false
    if (!s.instructions.includes(oldStr)) return false
    s.instructions = s.instructions.replace(oldStr, newStr)
    const ver = (s.version || '1.0.0').split('.')
    ver[2] = String(Number(ver[2] || 0) + 1)
    s.version = ver.join('.')
    return true
  }

  function deleteSkill(id: string) {
    skills.value = skills.value.filter(s => s.id !== id)
  }

  function toggleSkill(id: string) {
    const s = skills.value.find(x => x.id === id)
    if (s) s.enabled = !s.enabled
  }

  function incrementUsage(id: string) {
    const s = skills.value.find(x => x.id === id)
    if (s) {
      s.usageCount++
      s.lastUsed = Date.now()
    }
  }

  const enabledList = computed(() => skills.value.filter(s => s.enabled))

  function getEnabledSkills(): Skill[] {
    return skills.value.filter(s => s.enabled)
  }

  function searchSkills(query: string): Skill[] {
    const q = query.trim().toLowerCase()
    if (!q) return [...skills.value]
    return skills.value.filter(
      s =>
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q) ||
        s.trigger.toLowerCase().includes(q) ||
        s.instructions.toLowerCase().includes(q)
    )
  }

  function getByName(name: string): Skill | undefined {
    return skills.value.find(s => s.name === name)
  }

  function getCategories(): string[] {
    const cats = new Set(skills.value.map(s => s.category || 'general'))
    return [...cats].sort()
  }

  function buildSkillsIndex(): string {
    if (!skills.value.length) return ''
    const grouped: Record<string, Skill[]> = {}
    for (const s of skills.value.filter(s => s.enabled)) {
      const cat = s.category || 'general'
      if (!grouped[cat]) grouped[cat] = []
      grouped[cat].push(s)
    }
    const lines: string[] = []
    for (const [cat, catSkills] of Object.entries(grouped)) {
      lines.push(`## ${cat}`)
      for (const s of catSkills) {
        lines.push(`- **${s.name}**: ${s.description} (trigger: ${s.trigger})`)
      }
    }
    return lines.join('\n')
  }

  return {
    skills,
    enabledList,
    ensurePresets,
    createSkill,
    updateSkill,
    patchSkill,
    deleteSkill,
    toggleSkill,
    incrementUsage,
    getEnabledSkills,
    searchSkills,
    getByName,
    getCategories,
    buildSkillsIndex,
  }
}, {
  persist: {
    pick: ['skills'] as const
  }
})
