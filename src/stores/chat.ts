import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { v4 as uuidv4 } from 'uuid'
import type { Topic, Message, AgentConfig } from '@/types'

export const useChatStore = defineStore('chat', () => {
  const topics = ref<Topic[]>([])
  const messages = ref<Map<string, Message[]>>(new Map())
  const currentTopicId = ref<string | null>(null)
  const isStreaming = ref(false)

  const currentTopic = computed(() => topics.value.find(t => t.id === currentTopicId.value) || null)
  const currentMessages = computed(() => currentTopicId.value ? (messages.value.get(currentTopicId.value) || []) : [])

  function createTopic(data: Partial<Topic> = {}): Topic {
    const topic: Topic = {
      id: uuidv4(), title: data.title || '新对话', assistantId: data.assistantId,
      providerId: data.providerId || '', modelId: data.modelId || '',
      messageCount: 0, pinned: false, createdAt: Date.now(), updatedAt: Date.now(),
      isGroupChat: true,
      participantAgentIds: data.participantAgentIds || [],
      activeTab: data.activeTab || 'group'
    }
    topics.value.unshift(topic)
    messages.value.set(topic.id, [])
    currentTopicId.value = topic.id
    return topic
  }

  function setActiveTab(topicId: string, tab: string) {
    const topic = topics.value.find(t => t.id === topicId)
    if (topic) { topic.activeTab = tab; topic.updatedAt = Date.now() }
  }

  function setTopicChannel(topicId: string, channelId: string | null) {
    const topic = topics.value.find(t => t.id === topicId)
    if (topic) { topic.channelId = channelId; topic.updatedAt = Date.now() }
  }

  function selectTopic(id: string) { currentTopicId.value = id }

  function deleteTopic(id: string) {
    topics.value = topics.value.filter(t => t.id !== id)
    messages.value.delete(id)
    if (currentTopicId.value === id) currentTopicId.value = topics.value[0]?.id || null
  }

  function updateTopicTitle(id: string, title: string) {
    const topic = topics.value.find(t => t.id === id)
    if (topic) { topic.title = title; topic.updatedAt = Date.now() }
  }

  function togglePinTopic(id: string) {
    const topic = topics.value.find(t => t.id === id)
    if (topic) topic.pinned = !topic.pinned
  }

  function addMessage(topicId: string, msg: Partial<Message>): Message {
    const message: Message = {
      id: msg.id || uuidv4(), topicId, role: msg.role || 'user', content: msg.content || '',
      model: msg.model, provider: msg.provider, status: msg.status || 'complete',
      attachments: msg.attachments || [],
      parts: msg.parts || [],
      createdAt: Date.now(),
      agentId: msg.agentId,
      agentName: msg.agentName,
      mentions: msg.mentions,
      toolCallId: msg.toolCallId,
      toolCallsRaw: msg.toolCallsRaw,
    }
    if (!messages.value.has(topicId)) messages.value.set(topicId, [])
    messages.value.get(topicId)!.push(message)
    const topic = topics.value.find(t => t.id === topicId)
    if (topic) { topic.messageCount++; topic.updatedAt = Date.now() }
    return message
  }

  function updateMessage(topicId: string, messageId: string, data: Partial<Message>) {
    const msgs = messages.value.get(topicId)
    if (!msgs) return
    const msg = msgs.find(m => m.id === messageId)
    if (msg) Object.assign(msg, data)
  }

  function appendToMessage(topicId: string, messageId: string, content: string) {
    const msgs = messages.value.get(topicId)
    if (!msgs) return
    const msg = msgs.find(m => m.id === messageId)
    if (msg) msg.content += content
  }

  function addMessagePart(topicId: string, messageId: string, part: any) {
    const msgs = messages.value.get(topicId)
    if (!msgs) return
    const msg = msgs.find(m => m.id === messageId)
    if (!msg) return
    if (!msg.parts) msg.parts = []
    msg.parts.push(part)
  }

  function updateMessagePart(topicId: string, messageId: string, partId: string, data: Record<string, any>) {
    const msgs = messages.value.get(topicId)
    if (!msgs) return
    const msg = msgs.find(m => m.id === messageId)
    if (!msg?.parts) return
    const part = msg.parts.find((p: any) => p.id === partId || p.toolCallId === partId)
    if (part) Object.assign(part, data)
  }

  function appendToThinking(topicId: string, messageId: string, chunk: string) {
    const msgs = messages.value.get(topicId)
    if (!msgs) return
    const msg = msgs.find(m => m.id === messageId)
    if (!msg) return
    if (!msg.parts) msg.parts = []
    const existing = msg.parts.find((p: any) => p.type === 'reasoning')
    if (existing) {
      existing.content = (existing.content || '') + chunk
    } else {
      msg.parts.push({ id: uuidv4(), type: 'reasoning', content: chunk })
    }
  }

  function deleteMessage(topicId: string, messageId: string) {
    const msgs = messages.value.get(topicId)
    if (!msgs) return
    const index = msgs.findIndex(m => m.id === messageId)
    if (index !== -1) {
      msgs.splice(index, 1)
      const topic = topics.value.find(t => t.id === topicId)
      if (topic) topic.messageCount--
    }
  }

  function clearTopicMessages(topicId: string) {
    messages.value.set(topicId, [])
    const topic = topics.value.find(t => t.id === topicId)
    if (topic) topic.messageCount = 0
  }

  function clearAllTopics() {
    topics.value = []
    messages.value.clear()
    currentTopicId.value = null
  }

  function searchTopics(query: string): Topic[] {
    if (!query) return topics.value
    const q = query.toLowerCase()
    return topics.value.filter(t => t.title.toLowerCase().includes(q))
  }

  function updateTopic(id: string, data: Partial<Topic>) {
    const topic = topics.value.find(t => t.id === id)
    if (topic) Object.assign(topic, { ...data, updatedAt: Date.now() })
  }

  function setStreaming(loading: boolean) { isStreaming.value = loading }

  function searchMessages(query: string): Message[] {
    const q = query.toLowerCase()
    const results: Message[] = []
    for (const [, msgs] of messages.value) {
      for (const msg of msgs) {
        if (typeof msg.content === 'string' && msg.content.toLowerCase().includes(q)) results.push(msg)
      }
    }
    return results
  }

  function getTopicMessages(topicId: string): Message[] {
    return messages.value.get(topicId) || []
  }

  function exportTopic(topicId: string): string {
    const topic = topics.value.find(t => t.id === topicId)
    if (!topic) return ''
    const msgs = messages.value.get(topicId) || []
    const data = {
      topic: {
        id: topic.id,
        title: topic.title,
        createdAt: topic.createdAt,
        updatedAt: topic.updatedAt,
      },
      messages: msgs.map(m => ({
        id: m.id,
        role: m.role,
        content: m.content,
        status: m.status,
        error: m.error,
        agentName: m.agentName,
        toolCallId: m.toolCallId,
        toolCallsRaw: m.toolCallsRaw,
        parts: m.parts,
        createdAt: m.createdAt,
      })),
      exportedAt: new Date().toISOString(),
    }
    return JSON.stringify(data, null, 2)
  }

  function exportTopicAsText(topicId: string): string {
    const topic = topics.value.find(t => t.id === topicId)
    if (!topic) return ''
    const msgs = messages.value.get(topicId) || []
    const lines: string[] = [`# ${topic.title}`, `导出时间: ${new Date().toISOString()}`, '']
    for (const m of msgs) {
      const label = m.role === 'user' ? '👤 用户' : m.role === 'assistant' ? `🤖 AI${m.agentName ? ` (${m.agentName})` : ''}` : m.role === 'tool' ? '🔧 工具' : '📋 系统'
      lines.push(`### ${label}`)
      if (m.parts?.length) {
        for (const p of m.parts) {
          if (p.type === 'reasoning' && p.content) lines.push(`> 💭 思考: ${p.content}`)
          else if (p.type === 'tool_call') lines.push(`> 🛠️ 调用 ${p.toolName}(${typeof p.toolArgs === 'string' ? p.toolArgs : JSON.stringify(p.toolArgs)})`)
          else if (p.type === 'tool_result') lines.push(`> ✅ 结果: ${p.content}`)
          else if (p.type === 'text' && p.content) lines.push(p.content)
          else if (p.type === 'error' && p.content) lines.push(`> ❌ 错误: ${p.content}`)
          else if (p.type === 'step') lines.push(`> 📌 ${p.stepTitle || '步骤'}`)
        }
      }
      if (m.content) lines.push(m.content)
      if (m.error) lines.push(`> ❌ 错误: ${m.error}`)
      lines.push('')
    }
    return lines.join('\n')
  }

  function setTopicAgentSnapshots(topicId: string, agents: AgentConfig[]) {
    const topic = topics.value.find(t => t.id === topicId)
    if (!topic) return
    const snaps: Record<string, AgentConfig> = {}
    for (const a of agents) snaps[a.id] = JSON.parse(JSON.stringify(a))
    topic.agentSnapshots = snaps
    topic.updatedAt = Date.now()
  }

  function getTopicAgent(topicId: string, agentId: string): AgentConfig | null {
    const topic = topics.value.find(t => t.id === topicId)
    return topic?.agentSnapshots?.[agentId] ?? null
  }

  function getTopicAgents(topicId: string): AgentConfig[] {
    const topic = topics.value.find(t => t.id === topicId)
    if (!topic?.agentSnapshots) return []
    return Object.values(topic.agentSnapshots)
  }

  function updateTopicAgent(topicId: string, agentId: string, data: Partial<AgentConfig>) {
    const topic = topics.value.find(t => t.id === topicId)
    if (!topic?.agentSnapshots?.[agentId]) return
    Object.assign(topic.agentSnapshots[agentId], data)
    topic.updatedAt = Date.now()
  }

  return {
    topics, messages, currentTopicId, currentTopic, currentMessages, isStreaming,
    createTopic, selectTopic, deleteTopic, updateTopicTitle, togglePinTopic,
    addMessage, updateMessage, appendToMessage, addMessagePart, updateMessagePart, appendToThinking,
    deleteMessage, clearTopicMessages, clearAllTopics,
    searchTopics, searchMessages, getTopicMessages, updateTopic, setStreaming, setActiveTab, setTopicChannel,
    exportTopic, exportTopicAsText,
    setTopicAgentSnapshots, getTopicAgent, getTopicAgents, updateTopicAgent,
  }
}, {
  persist: {
    serializer: {
      serialize: (state: any) => {
        try {
          const s = { ...state }
          if (state.messages instanceof Map) {
            s.messages = Object.fromEntries(state.messages)
          }
          return JSON.stringify(s)
        } catch {
          return '{}'
        }
      },
      deserialize: (str: string) => {
        try {
          const s = JSON.parse(str)
          if (s.messages && !(s.messages instanceof Map)) {
            s.messages = new Map(Object.entries(s.messages))
          } else {
            s.messages = new Map()
          }
          return s
        } catch {
          return { topics: [], messages: new Map(), currentTopicId: null, isStreaming: false }
        }
      }
    }
  }
})
