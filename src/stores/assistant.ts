import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { v4 as uuidv4 } from 'uuid'
import type { Assistant } from '@/types'

const PRESET_ASSISTANTS: Partial<Assistant>[] = [
  { name: '通用助手', description: '全能型 AI 助手，可以回答各类问题', systemPrompt: '你是一个智能、有帮助的 AI 助手。请用中文回答问题，保持回答准确、详细、有条理。', category: '通用', tags: ['通用', '对话'] },
  { name: '代码专家', description: '精通各种编程语言的代码助手', systemPrompt: '你是一位资深程序员和代码专家。擅长解释代码、发现 bug、提供优化建议。请用专业但易懂的方式回答问题，并提供代码示例。', category: '编程', tags: ['编程', '代码'] },
  { name: '翻译助手', description: '专业多语言翻译', systemPrompt: '你是一位专业翻译。请准确翻译用户提供的文本，保持原文的语气和风格。', category: '翻译', tags: ['翻译'] },
  { name: '写作助手', description: '帮你优化文章和文案', systemPrompt: '你是一位专业写作教练。帮助用户改善文章结构、优化措辞、纠正语法错误。', category: '写作', tags: ['写作', '文案'] },
  { name: '数据分析师', description: '分析数据并提供洞察', systemPrompt: '你是一位数据分析专家。擅长解读数据、发现趋势、提供可视化建议。', category: '分析', tags: ['数据', '分析'] },
  { name: '前端开发', description: 'React/Vue/CSS 前端开发', systemPrompt: '你是一位前端开发专家，精通 React、Vue、TypeScript、CSS。', category: '编程', tags: ['编程', '前端'] },
  { name: 'Python 导师', description: 'Python 编程教学', systemPrompt: '你是一位 Python 编程导师。用通俗易懂的方式教授 Python 知识，从基础到高级。', category: '编程', tags: ['编程', 'Python'] },
  { name: '面试教练', description: '模拟面试和职业指导', systemPrompt: '你是一位资深面试教练。帮助用户准备面试、分析常见问题、提供回答策略。', category: '职业', tags: ['职业', '面试'] },
  { name: '创意大师', description: '激发创意和头脑风暴', systemPrompt: '你是一位创意大师。帮助用户进行头脑风暴、发散思维、生成创意方案。', category: '创意', tags: ['创意', '头脑风暴'] },
  { name: '英语教师', description: '英语学习和口语练习', systemPrompt: '你是一位友好的英语教师。帮助学生练习口语、纠正语法、扩展词汇。', category: '教育', tags: ['教育', '英语'] }
]

export const useAssistantStore = defineStore('assistant', () => {
  const assistants = ref<Assistant[]>([])
  const currentAssistantId = ref<string | null>(null)
  const searchQuery = ref('')
  const selectedCategory = ref('全部')

  const categories = computed(() => {
    const cats = new Set<string>(['全部'])
    assistants.value.forEach(a => cats.add(a.category))
    return Array.from(cats)
  })

  const filteredAssistants = computed(() => {
    let result = assistants.value
    if (selectedCategory.value !== '全部') result = result.filter(a => a.category === selectedCategory.value)
    if (searchQuery.value) {
      const q = searchQuery.value.toLowerCase()
      result = result.filter(a => a.name.toLowerCase().includes(q) || a.description.toLowerCase().includes(q) || a.tags.some(t => t.toLowerCase().includes(q)))
    }
    return result
  })

  function initPresets() {
    if (assistants.value.length > 0) return
    for (const preset of PRESET_ASSISTANTS) {
      assistants.value.push({
        id: uuidv4(), name: preset.name!, description: preset.description || '',
        avatar: '', systemPrompt: preset.systemPrompt || '', providerId: '', modelId: '',
        temperature: 0.7, maxTokens: 4096, topP: 1, category: preset.category || '通用',
        tags: preset.tags || [], isPreset: true, createdAt: Date.now(), updatedAt: Date.now()
      })
    }
  }

  function createAssistant(data: Partial<Assistant>): Assistant {
    const assistant: Assistant = {
      id: uuidv4(), name: data.name || '新助手', description: data.description || '',
      avatar: data.avatar || '', systemPrompt: data.systemPrompt || '',
      providerId: data.providerId || '', modelId: data.modelId || '',
      temperature: data.temperature ?? 0.7, maxTokens: data.maxTokens ?? 4096,
      topP: data.topP ?? 1, category: data.category || '自定义',
      tags: data.tags || [], isPreset: false, createdAt: Date.now(), updatedAt: Date.now(),
      skillIds: data.skillIds || [],
      examples: data.examples || []
    }
    assistants.value.push(assistant)
    return assistant
  }

  function updateAssistant(id: string, data: Partial<Assistant>) {
    const index = assistants.value.findIndex(a => a.id === id)
    if (index !== -1) assistants.value[index] = { ...assistants.value[index], ...data, updatedAt: Date.now() }
  }

  function deleteAssistant(id: string) {
    assistants.value = assistants.value.filter(a => a.id !== id)
    if (currentAssistantId.value === id) currentAssistantId.value = null
  }

  function getAssistantById(id: string): Assistant | undefined {
    return assistants.value.find(a => a.id === id)
  }

  return {
    assistants, currentAssistantId, searchQuery, selectedCategory,
    categories, filteredAssistants, initPresets, createAssistant,
    updateAssistant, deleteAssistant, getAssistantById
  }
}, { persist: true })
