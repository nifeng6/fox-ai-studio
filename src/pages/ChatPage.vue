<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, provide } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useChatStore } from '@/stores/chat'
import { useProviderStore } from '@/stores/provider'
import { useSettingsStore } from '@/stores/settings'
import { useAgentStore } from '@/stores/agent'
import { usePersonalityStore } from '@/stores/personality'
import { useMemoryStore } from '@/stores/memory'
import { useSkillStore } from '@/stores/skill'
import { useCacheStore } from '@/stores/cache'
import {
  sendChatMessage,
  executeChatTool,
  abortChat,
  onStreamChunk,
  onStreamThinking,
  onStreamToolCall,
  onStreamEnd,
  onStreamError,
  onToolResult,
} from '@/utils/tauri-api'
import type { Attachment, Message, Topic } from '@/types'
import type { WebSearchEngine, AgentConfig } from '@/stores/agent'
import TopicList from '@/components/chat/TopicList.vue'
import ChatView from '@/components/chat/ChatView.vue'
import {
  applyHarnessConstraints,
  budgetContext,
  shouldUsePlanningGate,
  buildPlanningPrompt,
  DEFAULT_HARNESS_CONFIG
} from '@/utils/harness'
import {
  buildAgentSystemPrompt,
  detectDangerousCommand,
  isParallelizableTool,
} from '@/utils/tool-executor'
import { compressContextWithLLM } from '@/utils/auxiliary-router'

const { t } = useI18n()
const chat = useChatStore()
const provider = useProviderStore()
const settings = useSettingsStore()
const agent = useAgentStore()
const personalityStore = usePersonalityStore()
const memoryStore = useMemoryStore()
const skillStore = useSkillStore()
const cacheStore = useCacheStore()

const sidebarW = ref(280)
const isDrag = ref(false)
let startX = 0
let startW = 0

const hasTopic = computed(() => !!chat.currentTopicId)
const topicTitle = computed(() => chat.currentTopic?.title || '')

/** Resolve the effective agent for the current (or given) topic, using topic snapshot if available */
function effectiveAgent(topicId?: string | null, agentId?: string): AgentConfig | null {
  const tid = topicId ?? chat.currentTopicId
  if (tid) {
    const topic = chat.topics.find(t => t.id === tid)
    if (topic) {
      if (!topic.agentSnapshots) {
        ensureTopicSnapshots(tid)
      }
      return agent.getEffectiveAgent(topic.agentSnapshots, agentId)
    }
  }
  return agent.getEffectiveAgent(undefined, agentId)
}

const currentProviderId = computed(
  () => effectiveAgent()?.languageModel?.providerId || chat.currentTopic?.providerId || ''
)
const currentModelId = computed(
  () => effectiveAgent()?.languageModel?.modelId || chat.currentTopic?.modelId || ''
)

const streamTopicId = ref<string | null>(null)
const streamAssistantId = ref<string | null>(null)
const pendingToolCalls = ref<Array<{ toolCallId: string; name: string; arguments: string }>>([])
const aborted = ref(false)
const streamRetryCount = ref(0)
const MAX_STREAM_RETRIES = 5
const contextCompressCache = ref<Record<string, { summary: string; upToIndex: number }>>({})
const COMPRESS_THRESHOLD = 40

const screenshotScaleFactors = ref<{ scaleX: number; scaleY: number; origW: number; origH: number }>({
  scaleX: 1, scaleY: 1, origW: 0, origH: 0,
})
// regionCoordinateMap removed — no longer using grid regions

function isMultimodalModel(modelId: string): boolean {
  const id = modelId.toLowerCase()
  const multimodalPatterns = [
    'gpt-4o', 'gpt-4-turbo', 'gpt-4-vision', 'chatgpt-4o',
    'claude-3', 'claude-sonnet', 'claude-opus', 'claude-haiku',
    'gemini-pro-vision', 'gemini-1.5', 'gemini-2',
    'qwen-vl', 'qwen2-vl', 'qwen2.5-vl',
    'glm-4v', 'cogvlm',
    'llava', 'internvl', 'yi-vision', 'yi-vl',
    'pixtral', 'llama-3.2-vision', 'llama-3.2-11b', 'llama-3.2-90b',
    'grok-2-vision', 'grok-vision',
    'deepseek-vl',
    'abab6.5',
    'step-1v', 'step-2v',
    'moonshot-v1-vision',
  ]
  return multimodalPatterns.some(p => id.includes(p))
}

// Cache the latest screenshot base64 for vision_analyze("screenshot") references
let lastScreenshotBase64 = ''
let lastScreenshotDims = { w: 0, h: 0 }

// parseGridMapToCoords removed — no longer using grid regions
const systemPromptCache = ref<Record<string, { key: string; messages: { role: 'system'; content: string }[] }>>({})

// ── Topic-level agent isolation (provide to child components) ──

function ensureTopicSnapshots(topicId: string) {
  const topic = chat.topics.find(t => t.id === topicId)
  if (topic && !topic.agentSnapshots) {
    const mainAgent = agent.getMainAgent()
    if (mainAgent) {
      chat.setTopicAgentSnapshots(topicId, [mainAgent])
    }
  }
}

const topicAgents = computed<AgentConfig[]>(() => {
  const tid = chat.currentTopicId
  if (!tid) return []
  const topic = chat.topics.find(t => t.id === tid)
  if (!topic?.agentSnapshots) return []
  return Object.values(topic.agentSnapshots)
})

// Ensure snapshots exist when topic changes
watch(() => chat.currentTopicId, (tid) => {
  if (tid) ensureTopicSnapshots(tid)
}, { immediate: true })

function updateTopicAgentConfig(agentId: string, data: Partial<AgentConfig>) {
  const tid = chat.currentTopicId
  if (!tid) return
  ensureTopicSnapshots(tid)
  chat.updateTopicAgent(tid, agentId, data)
  systemPromptCache.value[tid] = undefined as any
}

function addAgentToTopic(newAgent: AgentConfig) {
  const tid = chat.currentTopicId
  if (!tid) return
  const topic = chat.topics.find(t => t.id === tid)
  if (!topic) return
  if (!topic.agentSnapshots) topic.agentSnapshots = {}
  topic.agentSnapshots[newAgent.id] = JSON.parse(JSON.stringify(newAgent))
  topic.updatedAt = Date.now()
}

function removeAgentFromTopic(agentId: string) {
  const tid = chat.currentTopicId
  if (!tid) return
  const topic = chat.topics.find(t => t.id === tid)
  if (!topic?.agentSnapshots) return
  delete topic.agentSnapshots[agentId]
  topic.updatedAt = Date.now()
}

provide('topicAgents', topicAgents)
provide('updateTopicAgent', updateTopicAgentConfig)
provide('addAgentToTopic', addAgentToTopic)
provide('removeAgentFromTopic', removeAgentFromTopic)
provide('currentTopicId', computed(() => chat.currentTopicId))

function getToolOptions(forAgent?: any): { toolsEnabled: boolean; enabledToolIds: string[] } {
  const a = forAgent || effectiveAgent()
  if (!a?.tools?.length) return { toolsEnabled: false, enabledToolIds: [] }
  return { toolsEnabled: true, enabledToolIds: a.tools as string[] }
}
/** In group + @ flow, remaining agents to reply after the current stream finishes. */
const pendingGroupReplies = ref<AgentConfig[]>([])
/** Order of @ picks from the input bar; merged with @Name tokens in the message. */
const mentionOrderFromInput = ref<string[]>([])
let unlistenFns: (() => void)[] = []

const streaming = computed(() => {
  if (!chat.isStreaming) return false
  // Only show streaming state if the current topic is the one being streamed
  if (streamTopicId.value && streamTopicId.value !== chat.currentTopicId) return false
  return true
})

function onResizeStart(e: MouseEvent) {
  e.preventDefault()
  isDrag.value = true
  startX = e.clientX
  startW = sidebarW.value
  window.addEventListener('mousemove', onResizeMove)
  window.addEventListener('mouseup', onResizeEnd)
  document.body.style.userSelect = 'none'
  document.body.style.cursor = 'col-resize'
}

function onResizeMove(e: MouseEvent) {
  if (!isDrag.value) return
  const w = startW + (e.clientX - startX)
  sidebarW.value = Math.min(480, Math.max(200, w))
}

function onResizeEnd() {
  isDrag.value = false
  window.removeEventListener('mousemove', onResizeMove)
  window.removeEventListener('mouseup', onResizeEnd)
  document.body.style.userSelect = ''
  document.body.style.cursor = ''
}

const compressingTopics = new Set<string>()

async function maybeCompressContext(topicId: string) {
  if (compressingTopics.has(topicId)) return
  const allMsgs = chat.getTopicMessages(topicId)
    .filter(m => m.role === 'user' || m.role === 'assistant' || m.role === 'tool')
  if (allMsgs.length < COMPRESS_THRESHOLD) return
  const cache = contextCompressCache.value[topicId]
  if (cache && allMsgs.length - cache.upToIndex < 20) return

  compressingTopics.add(topicId)
  try {
    const compressEnd = Math.max(0, allMsgs.length - 15)
    const toCompress = allMsgs.slice(0, compressEnd).map(m => ({
      role: m.role,
      content: typeof m.content === 'string' ? m.content : JSON.stringify(m.content),
    }))
    const summary = await compressContextWithLLM(toCompress, effectiveAgent(topicId))
    if (summary && summary.length > 20) {
      contextCompressCache.value[topicId] = { summary, upToIndex: compressEnd }
      memoryStore.addSessionSummary(topicId, summary)
    }
  } catch (e: any) {
    console.warn('[context-compress] failed:', e?.message || e)
  } finally {
    compressingTopics.delete(topicId)
  }
}

function mapMessageToApi(m: any): Record<string, any> {
  const obj: Record<string, any> = { role: m.role, content: m.content || '' }
  if (m.role === 'tool' && m.toolCallId) {
    obj.toolCallId = m.toolCallId
  }
  if (m.role === 'assistant' && m.toolCallsRaw?.length) {
    obj.toolCalls = m.toolCallsRaw
  }
  if (m.role === 'user' && m.attachments?.length) {
    const parts: any[] = []
    if (m.content) {
      parts.push({ type: 'text', text: m.content })
    }
    for (const att of m.attachments) {
      if (att.type?.startsWith('image/') && att.url) {
        parts.push({
          type: 'image_url',
          image_url: { url: att.url, detail: 'auto' }
        })
      }
    }
    if (parts.length > 1 || (parts.length === 1 && parts[0].type === 'image_url')) {
      obj.content = parts
    }
  }
  return obj
}

const MAX_SCREENSHOT_IMAGES_IN_HISTORY = 3

function buildApiMessages(topicId: string, excludeAssistantId: string) {
  const allMsgs = chat
    .getTopicMessages(topicId)
    .filter(m => m.id !== excludeAssistantId)
    .filter(m => m.role === 'user' || m.role === 'assistant' || m.role === 'system' || m.role === 'tool')

  const cache = contextCompressCache.value[topicId]
  let mapped: any[]
  if (cache && allMsgs.length > COMPRESS_THRESHOLD) {
    const keepRecent = 15
    const recentStart = Math.max(cache.upToIndex, allMsgs.length - keepRecent)
    mapped = [
      { role: 'system', content: `[Context Summary of earlier conversation]:\n${cache.summary}` },
      ...allMsgs.slice(recentStart).map(mapMessageToApi),
    ]
  } else {
    mapped = allMsgs.map(mapMessageToApi)
  }

  let imgCount = 0
  for (let i = mapped.length - 1; i >= 0; i--) {
    const m = mapped[i]
    if (!Array.isArray(m.content)) continue
    const hasImage = m.content.some((p: any) => p.type === 'image_url')
    if (!hasImage) continue
    imgCount++
    if (imgCount > MAX_SCREENSHOT_IMAGES_IN_HISTORY) {
      const textParts = m.content.filter((p: any) => p.type === 'text')
      mapped[i] = { ...m, content: textParts.length ? textParts.map((p: any) => p.text).join('\n') : '[screenshot - image omitted from history]' }
    }
  }

  return mapped
}

function getLastUserText(topicId: string, excludeAssistantId: string) {
  const msgs = chat.getTopicMessages(topicId).filter(m => m.id !== excludeAssistantId)
  for (let i = msgs.length - 1; i >= 0; i--) {
    if (msgs[i].role === 'user') return msgs[i].content || ''
  }
  return ''
}

function canMentionAgentInGroupTopic(agentId: string, topic: Topic): boolean {
  const c = topic.agentSnapshots?.[agentId]
  if (!c) return false
  if (c.type === 'main') return true
  return topic.participantAgentIds?.includes(agentId) ?? false
}

function uniqueMentionOrder(ids: string[]): string[] {
  const s = new Set<string>()
  const out: string[] = []
  for (const x of ids) {
    if (s.has(x)) continue
    s.add(x)
    out.push(x)
  }
  return out
}

function resolveGroupMentionIds(
  userText: string,
  fromUi: string[],
  topic: Topic
): string[] {
  const re = /@([^\s@]+)/g
  const fromText: string[] = []
  let m: RegExpExecArray | null
  while ((m = re.exec(userText)) !== null) {
    const name = m[1]
    const ta = topicAgents.value
    const ag = ta.find(a => a.name === name)
    if (ag) fromText.push(ag.id)
  }
  const base = fromText.length ? fromText : fromUi
  return uniqueMentionOrder(base).filter(id => canMentionAgentInGroupTopic(id, topic))
}

/**
 * Assembles Hermes-style system context: personality, sub-agent prompt, memory, session search, skills.
 * Pass `forAgent` to build context for a specific agent (e.g. group @ replies).
 */
function computeAgentCacheKey(a: AgentConfig): string {
  const parts = [
    a.id, a.personalityId || '', a.systemPrompt || '',
    a.languageModel?.modelId || '', a.visionModel?.modelId || '',
    JSON.stringify(a.tools?.map(t => t.id + (t.enabled ? '1' : '0')) || []),
    JSON.stringify(a.skillIds || []),
  ]
  return parts.join('|')
}

function buildHermesSystemMessages(
  topicId: string,
  userContextText: string,
  forAgent: AgentConfig | null | undefined = undefined
) {
  const a = forAgent !== undefined && forAgent !== null ? forAgent : effectiveAgent(topicId)
  if (!a) return [] as { role: 'system'; content: string }[]

  const cacheKey = computeAgentCacheKey(a)
  const cached = systemPromptCache.value[topicId]
  if (cached && cached.key === cacheKey) {
    return cached.messages
  }

  const out: { role: 'system'; content: string }[] = []
  if (a.personalityId) {
    const p = personalityStore.getById(a.personalityId)
    if (p?.systemPrompt?.trim()) {
      out.push({ role: 'system', content: p.systemPrompt.trim() })
    }
  }
  if (a.type === 'sub' && a.systemPrompt?.trim()) {
    out.push({ role: 'system', content: a.systemPrompt.trim() })
  }

  const contextParts: string[] = []

  if (a.memoryEnabled) {
    const q = userContextText.trim()
    let mems = q ? memoryStore.searchMemories(q) : []
    if (!mems.length) {
      mems = memoryStore.memories.slice(0, 20)
    } else {
      mems = mems.slice(0, 20)
    }
    const up = memoryStore.userProfile
    const memLines: string[] = []
    if (up.name?.trim() || up.notes?.trim() || Object.keys(up.preferences).length) {
      if (up.name?.trim()) memLines.push(`User name: ${up.name.trim()}`)
      for (const [k, v] of Object.entries(up.preferences)) {
        if (k && v) memLines.push(`Preference · ${k}: ${v}`)
      }
      if (up.notes?.trim()) memLines.push(`Notes: ${up.notes.trim()}`)
    }
    for (const e of mems) {
      memLines.push(`[${e.category}] ${e.content}`)
    }
    if (memLines.length) {
      contextParts.push('## Persistent memory & profile\n' + memLines.join('\n'))
    }
  }

  if (a.sessionSearchEnabled) {
    const q = userContextText.trim()
    if (q) {
      const hits = chat.searchMessages(q)
      const lines: string[] = []
      const seen = new Set<string>()
      for (const m of hits) {
        if (m.topicId === topicId) continue
        if (m.role === 'system') continue
        if (seen.has(m.id)) continue
        seen.add(m.id)
        const top = chat.topics.find(x => x.id === m.topicId)
        const title = top?.title || '…'
        const snip = (m.content || '').replace(/\s+/g, ' ').slice(0, 500)
        lines.push(`· [${title}] (${m.role}) ${snip}`)
        if (lines.length >= 5) break
      }
      if (lines.length) {
        contextParts.push('## Relevant past conversations (search)\n' + lines.join('\n'))
      }
    }
  }

  if (a.skillIds?.length) {
    const pick = skillStore.getEnabledSkills().filter(s => a.skillIds.includes(s.id))
    if (pick.length) {
      const block =
        '## Active skills (match triggers when applicable)\n' +
        pick
          .map(
            s =>
              `- **${s.name}** (trigger: ${s.trigger})\n  Instructions: ${s.instructions}\n  About: ${s.description}`
          )
          .join('\n')
      contextParts.push(block)
    }
  }

  if (a.tools?.length) {
    const skillsIndex = skillStore.buildSkillsIndex()
    out.unshift({ role: 'system', content: buildAgentSystemPrompt({
      skillsIndex: skillsIndex || undefined,
      dateTime: new Date().toLocaleString(),
      environmentHints: `OS: ${navigator.platform}`,
    }) })
  }

  // Tool definitions are sent via the API `tools` parameter, not in the system prompt.
  // Only add text tool hints when no API tools are configured (fallback for providers that don't support function calling).
  if (!a.tools?.length) {
    const toolSchemas = agent.getToolSchemas()
    if (toolSchemas.length) {
      const toolBlock =
        '## Available tools\n' + toolSchemas.map(t => `- **${t.name}**: ${t.description}`).join('\n')
      contextParts.push(toolBlock)
    }
  }

  const sessionSum = memoryStore.getSessionSummary(topicId)
  if (sessionSum) {
    contextParts.push('## Session summary (prior context)\n' + sessionSum)
  }

  const cacheStats = cacheStore.getStats()
  contextParts.push(
    `## Response cache (hint)\nEntries: ${cacheStats.totalEntries}, hits: ${cacheStats.totalHits}, max: ${cacheStats.maxSize} (tool results may be cached for repeat prompts).`
  )

  if (contextParts.length) {
    out.push({ role: 'system', content: contextParts.join('\n\n') })
  }

  systemPromptCache.value[topicId] = { key: cacheKey, messages: out }
  return out
}

function buildApiMessagesForSend(
  topicId: string,
  excludeAssistantId: string,
  userContextText: string,
  forAgent: AgentConfig | null | undefined = undefined
) {
  const hermes = buildHermesSystemMessages(topicId, userContextText, forAgent)
  const history = buildApiMessages(topicId, excludeAssistantId)

  if (!settings.harnessEnabled) {
    if (hermes.length) return [...hermes, ...history]
    return history
  }

  const harnessConfig = {
    ...DEFAULT_HARNESS_CONFIG,
    enablePlanningGate: settings.harnessPlanningGate,
    enableOutputConstraints: settings.harnessOutputConstraints,
    enableFeedbackLoop: settings.harnessFeedbackLoop,
    maxRetries: settings.harnessMaxRetries
  }

  const harnessedSystem = applyHarnessConstraints(hermes, userContextText, harnessConfig)

  if (harnessConfig.enablePlanningGate && shouldUsePlanningGate(userContextText) && history.length <= 2) {
    const lastUserIdx = history.findLastIndex(m => m.role === 'user')
    if (lastUserIdx >= 0) {
      history[lastUserIdx] = {
        ...history[lastUserIdx],
        content: buildPlanningPrompt(history[lastUserIdx].content)
      }
    }
  }

  const { system, history: budgetedHistory } = budgetContext(harnessedSystem, history)

  return [...system, ...budgetedHistory]
}

async function onSend(text: string, _attachments: Attachment[]) {
  aborted.value = false
  toolLoopDepth.value = 0
  taskCompleteRound.value = 0
  pendingToolCalls.value = []
  recentToolSignatures.value = []
  streamRetryCount.value = 0
  const tid = chat.currentTopicId
  if (!tid) {
    ElMessage.info(t('chat.emptyHint') as string)
    return
  }
  if (!text.trim() && !_attachments.length) return
  const topic = chat.currentTopic
  const rawText = text
  const activeTab = topic?.activeTab || 'group'
  const isGroupTab = activeTab === 'group'

  let userContent = text
  if (agent.webSearchEnabled && userContent.trim()) {
    userContent =
      (t('chat.webSearchUserPrefix', { engine: agent.webSearchEngine }) as string) +
      '\n\n' +
      userContent
  }

  if (isGroupTab) {
    const mentionIdsFromTopic =
      topic?.isGroupChat && topic
        ? resolveGroupMentionIds(rawText, mentionOrderFromInput.value, topic)
        : []
    mentionOrderFromInput.value = []

    if (mentionIdsFromTopic.length) {
      for (const id of mentionIdsFromTopic) {
        const a = effectiveAgent(tid, id)
        if (!a?.languageModel?.providerId || !a.languageModel.modelId) {
          ElMessage.warning(t('pageUi.noModel') as string)
          return
        }
      }
    }

    chat.addMessage(tid, {
      role: 'user',
      content: userContent,
      status: 'complete',
      attachments: _attachments,
      mentions: mentionIdsFromTopic.length ? mentionIdsFromTopic : undefined
    })

    if (mentionIdsFromTopic.length) {
      const agentsInOrder: AgentConfig[] = []
      for (const id of mentionIdsFromTopic) {
        const a = effectiveAgent(tid, id)
        if (a) agentsInOrder.push(a)
      }
      if (!agentsInOrder.length) return

      const [first, ...rest] = agentsInOrder
      pendingGroupReplies.value = rest

      const assistant = chat.addMessage(tid, {
        role: 'assistant',
        content: '',
        status: 'streaming',
        agentId: first.id,
        agentName: first.name
      })
      streamTopicId.value = tid
      streamAssistantId.value = assistant.id
      chat.setStreaming(true)

      const messages = buildApiMessagesForSend(tid, assistant.id, rawText, first)
      const lang0 = first.languageModel
      try {
        await sendChatMessage({
          providerId: lang0.providerId,
          modelId: lang0.modelId,
          messageId: assistant.id,
          messages,
          options: { ...getToolOptions(first) }
        })
      } catch (e) {
        console.error(e)
        chat.updateMessage(tid, assistant.id, { status: 'error', error: t('common.error') as string })
        chat.setStreaming(false)
        streamTopicId.value = null
        streamAssistantId.value = null
        pendingGroupReplies.value = []
      }
      return
    }

    const ea = effectiveAgent(tid)
    const lang = ea?.languageModel
    if (!lang?.providerId || !lang?.modelId) {
      ElMessage.warning(t('pageUi.noModel') as string)
      return
    }
    const assistant = chat.addMessage(tid, {
      role: 'assistant',
      content: '',
      status: 'streaming'
    })
    streamTopicId.value = tid
    streamAssistantId.value = assistant.id
    chat.setStreaming(true)
    const messages = buildApiMessagesForSend(tid, assistant.id, text)
    try {
      await sendChatMessage({
        providerId: lang.providerId,
        modelId: lang.modelId,
        messageId: assistant.id,
        messages,
        options: { ...getToolOptions() }
      })
    } catch (e) {
      console.error(e)
      chat.updateMessage(tid, assistant.id, { status: 'error', error: t('common.error') as string })
      chat.setStreaming(false)
      streamTopicId.value = null
      streamAssistantId.value = null
    }
  } else {
    const targetAgent = effectiveAgent(tid, activeTab)
    if (!targetAgent) {
      ElMessage.warning('Agent not found')
      return
    }
    const lang = targetAgent.languageModel
    if (!lang?.providerId || !lang?.modelId) {
      ElMessage.warning(t('pageUi.noModel') as string)
      return
    }

    chat.addMessage(tid, {
      role: 'user',
      content: userContent,
      status: 'complete',
      attachments: _attachments,
      agentId: targetAgent.id
    })

    const assistant = chat.addMessage(tid, {
      role: 'assistant',
      content: '',
      status: 'streaming',
      agentId: targetAgent.id,
      agentName: targetAgent.name
    })

    streamTopicId.value = tid
    streamAssistantId.value = assistant.id
    chat.setStreaming(true)

    const messages = buildApiMessagesForSend(tid, assistant.id, text, targetAgent)
    try {
      await sendChatMessage({
        providerId: lang.providerId,
        modelId: lang.modelId,
        messageId: assistant.id,
        messages,
        options: { ...getToolOptions(targetAgent) }
      })
    } catch (e) {
      console.error(e)
      chat.updateMessage(tid, assistant.id, { status: 'error', error: t('common.error') as string })
      chat.setStreaming(false)
      streamTopicId.value = null
      streamAssistantId.value = null
    }
  }
}

function onStop() {
  aborted.value = true
  const tid = streamTopicId.value
  const mid = streamAssistantId.value
  if (mid) {
    void abortChat(mid)
  }
  if (tid && mid) {
    chat.updateMessage(tid, mid, { status: 'complete' })
    // Finalize any running step spinners
    const msg = chat.getTopicMessages(tid).find(m => m.id === mid)
    if (msg?.parts) {
      for (const p of msg.parts) {
        if (p.type === 'step' && p.stepStatus === 'running') {
          chat.updateMessagePart(tid, mid, p.id!, { stepStatus: 'error' })
        }
        if (p.type === 'tool_call' && (p.toolStatus === 'pending' || p.toolStatus === 'running')) {
          chat.updateMessagePart(tid, mid, p.id!, { toolStatus: 'error' })
        }
      }
    }
  }
  pendingToolCalls.value = []
  pendingGroupReplies.value = []
  toolLoopDepth.value = 0
  taskCompleteRound.value = 0
  chat.setStreaming(false)
  streamTopicId.value = null
  streamAssistantId.value = null
}

function onUpdProvider(v: string) {
  const tid = chat.currentTopicId
  const ea = effectiveAgent(tid)
  if (!ea) return
  const currentLM = ea.languageModel
  if (tid) {
    chat.updateTopic(tid, { providerId: v })
    chat.updateTopicAgent(tid, ea.id, { languageModel: { ...currentLM, providerId: v } })
    systemPromptCache.value[tid] = undefined as any
  }
}

function onUpdModel(v: string) {
  const tid = chat.currentTopicId
  const ea = effectiveAgent(tid)
  if (!ea) return
  const currentLM = ea.languageModel
  if (tid) {
    chat.updateTopic(tid, { modelId: v })
    chat.updateTopicAgent(tid, ea.id, { languageModel: { ...currentLM, modelId: v } })
    systemPromptCache.value[tid] = undefined as any
  }
}

async function onEditMessage(msg: Message) {
  if (msg.role !== 'user' || !chat.currentTopicId) return
  const { value } = await ElMessageBox.prompt(
    t('chat.edit') as string,
    t('chat.edit') as string,
    { inputValue: msg.content, confirmButtonText: t('common.save') as string, cancelButtonText: t('common.cancel') as string }
  )
  if (value != null) {
    chat.updateMessage(chat.currentTopicId, msg.id, { content: value })
  }
}

function onRegenerateMessage(msg: Message) {
  if (msg.role !== 'assistant' || !chat.currentTopicId) return
  const tid = chat.currentTopicId
  const ea = effectiveAgent(tid)
  const lang = ea?.languageModel
  if (!lang?.providerId || !lang?.modelId) {
    ElMessage.warning(t('pageUi.noModel') as string)
    return
  }
  const msgs = chat.getTopicMessages(tid)
  const idx = msgs.findIndex(m => m.id === msg.id)
  if (idx < 0) return

  toolLoopDepth.value = 0
  aborted.value = false
  pendingToolCalls.value = []
  streamRetryCount.value = 0

  chat.deleteMessage(tid, msg.id)
  const asst = chat.addMessage(tid, { role: 'assistant', content: '', status: 'streaming' })
  streamTopicId.value = tid
  streamAssistantId.value = asst.id
  chat.setStreaming(true)
  const history = buildApiMessagesForSend(tid, asst.id, getLastUserText(tid, asst.id), ea)
  void sendChatMessage({
    providerId: lang.providerId,
    modelId: lang.modelId,
    messageId: asst.id,
    messages: history,
    options: { ...getToolOptions(ea) },
  }).catch(e => {
    console.error(e)
    chat.updateMessage(tid, asst.id, { status: 'error', error: t('common.error') as string })
    chat.setStreaming(false)
    streamTopicId.value = null
    streamAssistantId.value = null
  })
}

async function onDeleteMessage(msg: Message) {
  if (!chat.currentTopicId) return
  try {
    await ElMessageBox.confirm(
      t('chat.deleteConfirm') as string,
      t('chat.delete') as string,
      { type: 'warning', confirmButtonText: t('common.delete') as string, cancelButtonText: t('common.cancel') as string }
    )
    chat.deleteMessage(chat.currentTopicId, msg.id)
  } catch {
    /* skip */
  }
}

function onToolApproved(id: string) {
  void agent.executeToolCall(id)
}

function onToolRejected(_id: string) {
  /* Rejection is applied in ToolCallCard via agent.rejectToolCall */
}

function onNewTopic() {
  const fp = settings.defaultProviderId
  const fm = settings.defaultModelId
  const p = provider.getProviderById(fp)
  const hasModel = p?.models?.some(m => m === fm) ?? false

  const mainAgent = agent.getMainAgent() || agent.createMainAgent()
  const freshMain: AgentConfig = JSON.parse(JSON.stringify(mainAgent))

  const topic = chat.createTopic({
    title: t('chat.newTitle') as string,
    providerId: fp && p ? fp : (provider.getEnabledProviders()[0]?.id || ''),
    modelId: hasModel ? fm : (provider.getEnabledProviders()[0]?.models?.[0] || ''),
    participantAgentIds: [freshMain.id],
    activeTab: freshMain.id
  })
  chat.setTopicAgentSnapshots(topic.id, [freshMain])
  agent.switchAgent(freshMain.id)
}

function onClearContext() {
  const tid = chat.currentTopicId
  if (!tid) return
  chat.addMessage(tid, {
    role: 'system',
    content: t('chat.contextClearedSystem') as string,
    status: 'complete'
  })
  ElMessage.success(t('chat.contextCleared') as string)
}

async function onClearMessages() {
  if (!chat.currentTopicId) return
  try {
    await ElMessageBox.confirm(
      t('chat.clearCurrentMessages') as string,
      t('chat.clearMessages') as string,
      { type: 'warning', confirmButtonText: t('common.delete') as string, cancelButtonText: t('common.cancel') as string }
    )
    chat.clearTopicMessages(chat.currentTopicId)
  } catch {
    /* skip */
  }
}

function onWebSearch(engine: WebSearchEngine) {
  agent.setSearchEngine(engine)
}

function onQuickPhrase(_phrase: string) {
  /* Filled in MessageInput; engine/history are handled in onSend and agent store. */
}

function onMentionFromInput(agentId: string) {
  mentionOrderFromInput.value.push(agentId)
}

function onChunk(payload: { messageId: string; chunk: string }) {
  if (!streamTopicId.value || !streamAssistantId.value) return
  if (payload.messageId !== streamAssistantId.value) return
  chat.appendToMessage(streamTopicId.value, streamAssistantId.value, payload.chunk)
}

function onThinking(payload: { messageId: string; chunk: string }) {
  if (!streamTopicId.value || !streamAssistantId.value) return
  if (payload.messageId !== streamAssistantId.value) return
  chat.appendToThinking(streamTopicId.value, streamAssistantId.value, payload.chunk)
}

function onToolCallEvent(payload: { messageId: string; toolCallId: string; name: string; arguments: string }) {
  if (!streamTopicId.value || !streamAssistantId.value) return
  if (payload.messageId !== streamAssistantId.value) return
  pendingToolCalls.value.push({
    toolCallId: payload.toolCallId,
    name: payload.name,
    arguments: payload.arguments,
  })
  chat.addMessagePart(streamTopicId.value, streamAssistantId.value, {
    id: payload.toolCallId,
    type: 'tool_call',
    toolCallId: payload.toolCallId,
    toolName: payload.name,
    toolArgs: payload.arguments,
    toolStatus: 'pending',
  })
}

const toolLoopDepth = ref(0)
const maxToolLoop = computed(() => {
  const configured = effectiveAgent()?.maxIterations
  if (configured === 0 || configured === -1) return Infinity
  return (!configured || configured < 0) ? 50 : configured
})
const todoList = ref<{ text: string; done: boolean }[]>([])
const recentToolSignatures = ref<string[]>([])
const taskCompleteRound = ref(0)
let agentSafetyTimer: ReturnType<typeof setTimeout> | null = null

async function executeToolCallsAndContinue(tid: string, mid: string) {
  try {
  if (aborted.value) return
  const calls = [...pendingToolCalls.value]
  pendingToolCalls.value = []
  if (!calls.length || toolLoopDepth.value >= maxToolLoop.value) {
    if (toolLoopDepth.value >= maxToolLoop.value) {
      chat.addMessagePart(tid, mid, {
        id: `max_iterations_${Date.now()}`, type: 'error',
        content: `已达最大迭代次数 (${maxToolLoop.value})，agent 循环已停止。请重新描述需求或调整任务。`,
      })
    }
    toolLoopDepth.value = 0
    recentToolSignatures.value = []
    chat.setStreaming(false)
    streamTopicId.value = null
    streamAssistantId.value = null
    return
  }
  toolLoopDepth.value++

  const noRepeatCheckTools = new Set(['screenshot', 'wait', 'task_complete'])
  const significantCalls = calls.filter(c => !noRepeatCheckTools.has(c.name))
  const currentSigs = significantCalls.length > 0
    ? significantCalls.map(c => `${c.name}:${JSON.stringify(c.arguments)}`).sort().join('|')
    : ''
  if (currentSigs) {
    recentToolSignatures.value.push(currentSigs)
    if (recentToolSignatures.value.length > 8) recentToolSignatures.value.shift()
  }
  const repeatCount = currentSigs ? recentToolSignatures.value.filter(s => s === currentSigs).length : 0
  if (repeatCount >= 4) {
    console.warn('[agent-loop] Detected repeated tool call pattern, breaking loop')
    chat.addMessagePart(tid, mid, {
      id: `loop_break_${Date.now()}`, type: 'error',
      content: '检测到重复的工具调用模式，agent 循环已自动停止。请尝试换一种方式描述需求。',
    })
    toolLoopDepth.value = 0
    recentToolSignatures.value = []
    chat.updateMessage(tid, mid, { status: 'complete' })
    chat.setStreaming(false)
    streamTopicId.value = null
    streamAssistantId.value = null
    return
  }

  chat.addMessagePart(tid, mid, {
    id: `step_${toolLoopDepth.value}`,
    type: 'step',
    stepTitle: `Agent loop iteration ${toolLoopDepth.value}`,
    stepStatus: 'running',
  })

  const toolCallsRaw: any[] = []
  const toolResultMessages: any[] = []
  let taskCompletedInBatch = false

  // ── Parallel fast-path for independent Rust tools ──
  const FRONTEND_TOOL_SET = new Set([
    'skill_manage', 'skills_list', 'skill_view', 'todo', 'memory', 'memory_save',
    'session_search', 'vision_analyze', 'web_search', 'task_complete',
  ])
  const MOUSE_COORD_TOOLS = new Set(['mouse_click', 'mouse_double_click', 'mouse_move', 'mouse_scroll', 'mouse_drag'])
  const allParallelizable = calls.length > 1 && calls.every(
    c => isParallelizableTool(c.name) && !FRONTEND_TOOL_SET.has(c.name)
  )

  if (allParallelizable && !aborted.value) {
    for (const c of calls) {
      chat.updateMessagePart(tid, mid, c.toolCallId, { toolStatus: 'running' })
      toolCallsRaw.push({ id: c.toolCallId, type: 'function', function: { name: c.name, arguments: c.arguments } })
    }
    const results = await Promise.all(calls.map(async (c) => {
      const a = (() => { try { return JSON.parse(c.arguments || '{}') } catch { return {} } })()

      if (MOUSE_COORD_TOOLS.has(c.name)) {
        console.log(`[coord] ${c.name}: AI gave (${a.x}, ${a.y}) → physical pixels. Screenshot: ${lastScreenshotDims.w}x${lastScreenshotDims.h}`)
      }
      try {
        return { call: c, result: await executeChatTool({ name: c.name, arguments: a, toolCallId: c.toolCallId, messageId: mid }), error: null }
      } catch (e: any) {
        return { call: c, result: null, error: e?.message || String(e) }
      }
    }))
    for (const { call: c, result, error } of results) {
      if (error) {
        chat.updateMessagePart(tid, mid, c.toolCallId, { toolStatus: 'error' })
        chat.addMessagePart(tid, mid, { id: c.toolCallId + '_result', type: 'tool_result', toolCallId: c.toolCallId, content: `Error: ${error}` })
        toolResultMessages.push({ role: 'tool', content: `Error: ${error}`, tool_call_id: c.toolCallId })
      } else if (result) {
        chat.updateMessagePart(tid, mid, c.toolCallId, { toolStatus: result.success ? 'done' : 'error' })
        chat.addMessagePart(tid, mid, { id: c.toolCallId + '_result', type: 'tool_result', toolCallId: c.toolCallId, content: result.result })
        toolResultMessages.push({ role: 'tool', content: result.result, tool_call_id: c.toolCallId })
      }
    }
  } else {

  for (const call of calls) {
    if (aborted.value) return
    let args: Record<string, any> = {}
    try { args = JSON.parse(call.arguments || '{}') } catch { /* ignore */ }

    chat.updateMessagePart(tid, mid, call.toolCallId, { toolStatus: 'running' })

    toolCallsRaw.push({
      id: call.toolCallId,
      type: 'function',
      function: { name: call.name, arguments: call.arguments },
    })

    // ── task_complete: summarize and prompt self-evolution ──
    if (call.name === 'task_complete') {
      taskCompletedInBatch = true
      const iterations = toolLoopDepth.value
      let feedback = `Task completed: ${args.summary || 'Done'}.`
      if (iterations >= 3) {
        feedback += ` This task used ${iterations} iterations. If this workflow could be reused, save it as a skill with skill_manage(action='create'). Also save any new knowledge with memory(action='add').`
      }
      chat.updateMessagePart(tid, mid, call.toolCallId, { toolStatus: 'done' })
      chat.addMessagePart(tid, mid, {
        id: call.toolCallId + '_result',
        type: 'tool_result',
        toolCallId: call.toolCallId,
        content: feedback,
      })
      toolResultMessages.push({ role: 'tool', content: feedback, tool_call_id: call.toolCallId })
      continue
    }

    // ── Frontend-intercepted tools (Hermes agent-loop pattern) ──

    if (call.name === 'skill_manage') {
      const action = args.action || 'create'
      let resultText = ''
      try {
        if (action === 'create') {
          const newSkill = skillStore.createSkill({
            name: args.name || 'Skill',
            description: args.description || '',
            trigger: args.trigger || '',
            instructions: args.content || args.instructions || '',
            enabled: true,
          })
          const agentObj = effectiveAgent(tid)
          if (agentObj && !agentObj.skillIds.includes(newSkill.id)) {
            chat.updateTopicAgent(tid, agentObj.id, { skillIds: [...agentObj.skillIds, newSkill.id] })
          }
          resultText = `Skill "${newSkill.name}" created and saved.`
        } else if (action === 'edit') {
          const existing = skillStore.skills.find(s => s.name === args.name)
          if (existing) {
            skillStore.updateSkill(existing.id, {
              ...(args.description ? { description: args.description } : {}),
              ...(args.trigger ? { trigger: args.trigger } : {}),
              ...(args.content ? { instructions: args.content } : {}),
            })
            resultText = `Skill "${args.name}" updated.`
          } else {
            resultText = `Skill "${args.name}" not found. Use action="create" to create it.`
          }
        } else if (action === 'patch') {
          const existing = skillStore.skills.find(s => s.name === args.name)
          if (existing) {
            const oldStr = args.old_string as string || ''
            const newStr = args.new_string as string || ''
            if (!oldStr) {
              resultText = `Patch requires "old_string" parameter. Use action="edit" for full overwrite.`
            } else {
              const ok = skillStore.patchSkill(existing.id, oldStr, newStr)
              resultText = ok
                ? `Skill "${args.name}" patched: replaced "${oldStr.slice(0, 50)}" → "${newStr.slice(0, 50)}"`
                : `Patch failed: old_string not found in skill "${args.name}" instructions.`
            }
          } else {
            resultText = `Skill "${args.name}" not found.`
          }
        } else if (action === 'delete') {
          const existing = skillStore.skills.find(s => s.name === args.name)
          if (existing) {
            skillStore.deleteSkill(existing.id)
            resultText = `Skill "${args.name}" deleted.`
          } else {
            resultText = `Skill "${args.name}" not found.`
          }
        }
      } catch (e: any) {
        resultText = `skill_manage error: ${e?.message || e}`
      }
      chat.updateMessagePart(tid, mid, call.toolCallId, { toolStatus: 'done' })
      chat.addMessagePart(tid, mid, {
        id: call.toolCallId + '_result', type: 'tool_result',
        toolCallId: call.toolCallId, content: resultText,
      })
      toolResultMessages.push({ role: 'tool', content: resultText, tool_call_id: call.toolCallId })
      continue
    }

    if (call.name === 'skills_list') {
      const allSkills = skillStore.getEnabledSkills()
      const listing = allSkills.length
        ? allSkills.map((s, i) => `${i + 1}. **${s.name}** — ${s.description} (trigger: ${s.trigger})`).join('\n')
        : 'No skills available.'
      chat.updateMessagePart(tid, mid, call.toolCallId, { toolStatus: 'done' })
      chat.addMessagePart(tid, mid, {
        id: call.toolCallId + '_result', type: 'tool_result',
        toolCallId: call.toolCallId, content: listing,
      })
      toolResultMessages.push({ role: 'tool', content: listing, tool_call_id: call.toolCallId })
      continue
    }

    if (call.name === 'skill_view') {
      const skill = skillStore.skills.find(s => s.name === args.name)
      const content = skill
        ? `# ${skill.name}\n${skill.description}\n\nTrigger: ${skill.trigger}\n\n## Instructions\n${skill.instructions}`
        : `Skill "${args.name}" not found. Use skills_list to see available skills.`
      chat.updateMessagePart(tid, mid, call.toolCallId, { toolStatus: 'done' })
      chat.addMessagePart(tid, mid, {
        id: call.toolCallId + '_result', type: 'tool_result',
        toolCallId: call.toolCallId, content,
      })
      toolResultMessages.push({ role: 'tool', content, tool_call_id: call.toolCallId })
      continue
    }

    if (call.name === 'todo') {
      const action = args.action || 'list'
      let content = ''
      if (!todoList.value) todoList.value = []
      if (action === 'add' && args.text) {
        todoList.value.push({ text: args.text as string, done: false })
        content = `Added: "${args.text}" (${todoList.value.length} items total)`
      } else if (action === 'complete' && args.index !== undefined) {
        const idx = args.index as number
        if (idx >= 0 && idx < todoList.value.length) {
          todoList.value[idx].done = true
          content = `Completed: "${todoList.value[idx].text}"`
        } else {
          content = `Invalid index: ${idx}`
        }
      } else if (action === 'clear') {
        todoList.value = []
        content = 'Todo list cleared.'
      } else {
        content = todoList.value.length
          ? todoList.value.map((t, i) => `${i}. [${t.done ? 'x' : ' '}] ${t.text}`).join('\n')
          : 'Todo list is empty.'
      }
      chat.updateMessagePart(tid, mid, call.toolCallId, { toolStatus: 'done' })
      chat.addMessagePart(tid, mid, {
        id: call.toolCallId + '_result', type: 'tool_result',
        toolCallId: call.toolCallId, content,
      })
      toolResultMessages.push({ role: 'tool', content, tool_call_id: call.toolCallId })
      continue
    }

    if (call.name === 'memory' || call.name === 'memory_save') {
      const action = (args.action as string) || 'add'
      const target = (args.target as string) || 'memory'
      const content = (args.content as string) || ''
      const category = (args.category as string) || 'insight'
      const categoryMap: Record<string, string> = {
        preference: 'preference', fact: 'fact', workflow: 'instruction',
        correction: 'instruction', insight: 'context',
      }
      try {
        let resultText = ''

        if (target === 'user') {
          const up = args.update_profile as Record<string, any> | undefined
          if (up && typeof up === 'object') {
            memoryStore.updateUserProfile({
              ...(up.name ? { name: up.name as string } : {}),
              ...(up.preferences ? { preferences: up.preferences as Record<string, string> } : {}),
              ...(up.notes ? { notes: up.notes as string } : {}),
            })
          }
          if (content) {
            memoryStore.updateUserProfile({ notes: content })
          }
          resultText = `User profile updated.`
        } else if (action === 'add') {
          memoryStore.addMemory({
            content,
            category: (categoryMap[category] || 'knowledge') as any,
            source: 'agent',
          })
          resultText = `Memory saved: [${category}] ${content.slice(0, 100)}${content.length > 100 ? '...' : ''}`
        } else if (action === 'replace') {
          const oldContent = (args.old_content as string) || ''
          const found = memoryStore.memories.find(
            m => m.content.includes(oldContent) || m.content === oldContent
          )
          if (found) {
            memoryStore.updateMemory(found.id, {
              content,
              ...(category ? { category: (categoryMap[category] || found.category) as any } : {}),
            })
            resultText = `Memory replaced: "${oldContent.slice(0, 50)}" → "${content.slice(0, 50)}"`
          } else {
            memoryStore.addMemory({ content, category: (categoryMap[category] || 'knowledge') as any, source: 'agent' })
            resultText = `No matching memory found for replace. Added as new: [${category}] ${content.slice(0, 80)}`
          }
        } else if (action === 'remove') {
          const found = memoryStore.memories.find(
            m => m.content.includes(content) || m.content === content
          )
          if (found) {
            memoryStore.deleteMemory(found.id)
            resultText = `Memory removed: "${content.slice(0, 100)}"`
          } else {
            resultText = `No matching memory found to remove for: "${content.slice(0, 100)}"`
          }
        } else {
          resultText = `Unknown memory action: ${action}`
        }

        if (args.update_profile && target !== 'user' && typeof args.update_profile === 'object') {
          const up = args.update_profile as Record<string, any>
          memoryStore.updateUserProfile({
            ...(up.name ? { name: up.name as string } : {}),
            ...(up.preferences ? { preferences: up.preferences as Record<string, string> } : {}),
            ...(up.notes ? { notes: up.notes as string } : {}),
          })
          resultText += ' User profile also updated.'
        }

        chat.updateMessagePart(tid, mid, call.toolCallId, { toolStatus: 'done' })
        chat.addMessagePart(tid, mid, {
          id: call.toolCallId + '_result', type: 'tool_result',
          toolCallId: call.toolCallId, content: resultText,
        })
        toolResultMessages.push({ role: 'tool', content: resultText, tool_call_id: call.toolCallId })
      } catch (e: any) {
        const errText = `memory error: ${e?.message || e}`
        chat.updateMessagePart(tid, mid, call.toolCallId, { toolStatus: 'error' })
        chat.addMessagePart(tid, mid, {
          id: call.toolCallId + '_result', type: 'tool_result',
          toolCallId: call.toolCallId, content: errText,
        })
        toolResultMessages.push({ role: 'tool', content: errText, tool_call_id: call.toolCallId })
      }
      continue
    }

    if (call.name === 'vision_analyze') {
      let imageUrl = args.image_url as string || ''
      const question = args.question as string || 'Describe what you see in this image in detail.'

      // Smart handling: if AI passes "screenshot" or similar, use cached screenshot
      const isScreenshotRef = !imageUrl || imageUrl === 'screenshot' || imageUrl === 'last_screenshot' || imageUrl === 'current_screen'
      if (isScreenshotRef && lastScreenshotBase64) {
        imageUrl = lastScreenshotBase64
      }

      // Show progress in UI
      chat.addMessagePart(tid, mid, {
        id: call.toolCallId + '_progress', type: 'step',
        stepTitle: '正在使用视觉模型分析图片...',
        stepStatus: 'running',
      })

      let visionResult = ''
      try {
        const { analyzeImageWithVision } = await import('@/utils/auxiliary-router')
        visionResult = await analyzeImageWithVision(imageUrl, question, effectiveAgent(tid))
      } catch (e: any) {
        visionResult = `Vision analysis failed: ${e?.message || e}`
      }

      chat.updateMessagePart(tid, mid, call.toolCallId + '_progress', { stepStatus: 'completed', stepTitle: '视觉分析完成' })
      chat.updateMessagePart(tid, mid, call.toolCallId, { toolStatus: 'done' })
      chat.addMessagePart(tid, mid, {
        id: call.toolCallId + '_result', type: 'tool_result',
        toolCallId: call.toolCallId, content: visionResult,
      })
      toolResultMessages.push({ role: 'tool', content: visionResult, tool_call_id: call.toolCallId })
      continue
    }

    if (call.name === 'web_search') {
      // Use configured search engine or fallback to fetch
      const query = args.query as string || ''
      let searchResult = ''
      try {
        const result = await executeChatTool({
          name: 'fetch_url',
          arguments: { url: `https://www.bing.com/search?q=${encodeURIComponent(query)}&count=${args.num_results || 5}` },
          toolCallId: call.toolCallId,
          messageId: mid,
        })
        searchResult = result.result
      } catch (e: any) {
        searchResult = `Search failed: ${e?.message || e}`
      }
      chat.updateMessagePart(tid, mid, call.toolCallId, { toolStatus: 'done' })
      chat.addMessagePart(tid, mid, {
        id: call.toolCallId + '_result', type: 'tool_result',
        toolCallId: call.toolCallId, content: searchResult,
      })
      toolResultMessages.push({ role: 'tool', content: searchResult, tool_call_id: call.toolCallId })
      continue
    }

    if (call.name === 'session_search') {
      const query = (args.query as string) || ''
      const maxResults = (args.max_results as number) || 5
      const topicMessages = chat.getTopicMessages(tid)
      const queryLower = query.toLowerCase()
      const matches: string[] = []
      for (const m of topicMessages) {
        if (matches.length >= maxResults) break
        const text = typeof m.content === 'string' ? m.content : JSON.stringify(m.content)
        if (text.toLowerCase().includes(queryLower)) {
          const snippet = text.length > 300 ? text.slice(0, 300) + '...' : text
          matches.push(`[${m.role}] ${snippet}`)
        }
      }
      const resultContent = matches.length
        ? `Found ${matches.length} matching message(s):\n\n${matches.join('\n\n---\n\n')}`
        : `No messages found matching "${query}" in current conversation.`
      chat.updateMessagePart(tid, mid, call.toolCallId, { toolStatus: 'done' })
      chat.addMessagePart(tid, mid, {
        id: call.toolCallId + '_result', type: 'tool_result',
        toolCallId: call.toolCallId, content: resultContent,
      })
      toolResultMessages.push({ role: 'tool', content: resultContent, tool_call_id: call.toolCallId })
      continue
    }

    // ── Dangerous command approval ──
    if (call.name === 'terminal' || call.name === 'execute_code') {
      const cmd = (args.command as string) || (args.code as string) || ''
      const danger = detectDangerousCommand(cmd)
      if (danger) {
        try {
          await ElMessageBox.confirm(
            `检测到潜在危险命令:\n\n${cmd.slice(0, 200)}\n\n${danger}\n\n是否允许执行？`,
            '⚠️ 危险命令警告',
            { confirmButtonText: '允许执行', cancelButtonText: '拒绝', type: 'warning' }
          )
        } catch {
          const rejectText = `Command rejected by user: ${danger}`
          chat.updateMessagePart(tid, mid, call.toolCallId, { toolStatus: 'error' })
          chat.addMessagePart(tid, mid, {
            id: call.toolCallId + '_result', type: 'tool_result',
            toolCallId: call.toolCallId, content: rejectText,
          })
          toolResultMessages.push({ role: 'tool', content: rejectText, tool_call_id: call.toolCallId })
          continue
        }
      }
    }

    // Coordinates from AI are already in physical pixel space — pass directly to backend
    if (MOUSE_COORD_TOOLS.has(call.name)) {
      console.log(`[coord] ${call.name}: AI gave (${args.x}, ${args.y}) → physical pixels. Screenshot: ${lastScreenshotDims.w}x${lastScreenshotDims.h}`)
    } else if (call.name === 'action_sequence') {
      console.log(`[coord] action_sequence: steps=${JSON.stringify(args.steps || args)}`)
    }

    // ── Rust-executed tools ──
    try {
      const result = await executeChatTool({
        name: call.name,
        arguments: args,
        toolCallId: call.toolCallId,
        messageId: mid,
      })
      chat.updateMessagePart(tid, mid, call.toolCallId, {
        toolStatus: result.success ? 'done' : 'error',
      })

      const isScreenshot = result.result?.startsWith('__SCREENSHOT__')
      const isLegacyImage = !isScreenshot && result.result?.startsWith('__IMAGE_BASE64__')
      const isImageResult = isScreenshot || isLegacyImage

      let origWidth = 0, origHeight = 0
      let resizedImageUrl = '', fullImageUrl = ''
      let resizedWidth = 0, resizedHeight = 0
      let windowsInfo = ''
      let cursorX = 0, cursorY = 0

      if (isScreenshot) {
        const payload = result.result.substring('__SCREENSHOT__:'.length)
        // Format: physWxH:displayWxH:CURSOR:cx,cy:metadata:data:image/jpeg;base64,...
        const colonPos1 = payload.indexOf(':')
        const colonPos2 = payload.indexOf(':', colonPos1 + 1)
        const origRes = payload.substring(0, colonPos1)
        const resizedRes = payload.substring(colonPos1 + 1, colonPos2)
        let rest = payload.substring(colonPos2 + 1)

        const [ow, oh] = origRes.split('x').map(Number)
        const [rw, rh] = resizedRes.split('x').map(Number)
        origWidth = ow; origHeight = oh
        resizedWidth = rw; resizedHeight = rh

        // Parse CURSOR:x,y: prefix
        if (rest.startsWith('CURSOR:')) {
          rest = rest.substring('CURSOR:'.length)
          const cursorEnd = rest.indexOf(':')
          if (cursorEnd > 0) {
            const cursorParts = rest.substring(0, cursorEnd).split(',')
            cursorX = parseInt(cursorParts[0]) || 0
            cursorY = parseInt(cursorParts[1]) || 0
            rest = rest.substring(cursorEnd + 1)
          }
        }

        const firstDataIdx = rest.indexOf('data:')
        if (firstDataIdx > 0) {
          let rawMeta = rest.substring(0, firstDataIdx - 1).replace(/\\n/g, '\n')
          const winIdx = rawMeta.indexOf('WINDOWS:')
          if (winIdx >= 0) {
            windowsInfo = rawMeta.substring(winIdx + 'WINDOWS:'.length).replace(/\|/g, '\n')
          }
          resizedImageUrl = rest.substring(firstDataIdx)
          fullImageUrl = resizedImageUrl
        } else {
          resizedImageUrl = rest
          fullImageUrl = rest
        }
      } else if (isLegacyImage) {
        const raw = result.result.replace('__IMAGE_BASE64__:', '')
        resizedImageUrl = raw
        fullImageUrl = raw
      }

      if (isImageResult && resizedWidth > 0 && resizedHeight > 0) {
        screenshotScaleFactors.value = {
          scaleX: origWidth / resizedWidth,
          scaleY: origHeight / resizedHeight,
          origW: origWidth,
          origH: origHeight,
        }
        console.log(`[screenshot] physical=${origWidth}x${origHeight}, display=${resizedWidth}x${resizedHeight}, scaleX=${(origWidth / resizedWidth).toFixed(3)}, scaleY=${(origHeight / resizedHeight).toFixed(3)}`)
      }

      const displayResult = isImageResult
        ? `[screenshot captured, ${origWidth}x${origHeight}px]`
        : result.result

      chat.addMessagePart(tid, mid, {
        id: call.toolCallId + '_result', type: 'tool_result',
        toolCallId: call.toolCallId, content: displayResult,
      })

      if (isImageResult) {
        const imageUrlForModel = resizedImageUrl || fullImageUrl
        const imgW = resizedWidth || origWidth
        const imgH = resizedHeight || origHeight

        const eaForScreenshot = effectiveAgent(tid)
        const hasVisionModel = !!(eaForScreenshot?.visionModel?.providerId && eaForScreenshot?.visionModel?.modelId)

        // Cache this screenshot for vision_analyze("screenshot") references
        lastScreenshotBase64 = imageUrlForModel
        lastScreenshotDims = { w: imgW, h: imgH }

        // Detect if main model can accept images directly:
        // Priority: 1. model-level setting  2. provider-level setting  3. name pattern
        const mainProviderId = eaForScreenshot?.languageModel?.providerId
        const mainProv = mainProviderId ? provider.getProviderById(mainProviderId) : undefined
        const mainModelId = (eaForScreenshot?.languageModel?.modelId || '')
        const modelLevelVision = mainProv?.modelSettings?.[mainModelId]?.supportsVision
        const providerLevelVision = mainProv?.supportsVision === true

        let mainModelIsMultimodal = false
        if (modelLevelVision !== undefined) {
          mainModelIsMultimodal = modelLevelVision
        } else if (providerLevelVision) {
          mainModelIsMultimodal = true
        } else {
          mainModelIsMultimodal = isMultimodalModel(mainModelId.toLowerCase())
        }

        // If we have a separate vision model configured, always use it for analysis
        // (the user explicitly set it up for this purpose)
        const useVisionAnalysis = hasVisionModel

        // Save debug log for every screenshot interaction
        const debugScreenshotLog = async (logType: string, prompt: string, response?: string) => {
          try {
            const { invoke } = await import('@tauri-apps/api/core')
            const ts = Date.now()
            const logContent = JSON.stringify({
              timestamp: new Date().toISOString(),
              type: logType,
              imageSize: `${imgW}x${imgH}`,
              physicalSize: `${origWidth}x${origHeight}`,
              scaleFactors: screenshotScaleFactors.value,
              useVisionAnalysis,
              mainModelIsMultimodal,
              visionModel: hasVisionModel ? `${eaForScreenshot?.visionModel?.providerId}/${eaForScreenshot?.visionModel?.modelId}` : 'none',
              mainModel: `${eaForScreenshot?.languageModel?.providerId}/${eaForScreenshot?.languageModel?.modelId}`,
              prompt,
              response: response || '(pending)',
            }, null, 2)
            await invoke('write_debug_log', { filename: `screenshot_${ts}.json`, content: logContent })
          } catch (e) { console.warn('[debug-log] save failed:', e) }
        }

        const winSection = windowsInfo ? `\nVISIBLE WINDOWS:\n${windowsInfo}\n` : ''

        if (useVisionAnalysis) {
          // === Strategy: Use dedicated vision model to analyze, then give text result to main model ===
          chat.addMessagePart(tid, mid, {
            id: call.toolCallId + '_vision_step', type: 'step',
            stepTitle: '正在用视觉模型分析截图...',
            stepStatus: 'running',
          })
          try {
            const { analyzeImageWithVision } = await import('@/utils/auxiliary-router')
            const visionPrompt = `你是精确坐标分析专家。分析这张 ${imgW}x${imgH} 像素的电脑截图。
坐标原点(0,0)在截图左上角，x向右增大，y向下增大。

## 你的唯一任务
找到截图中每个可见元素，返回它的【可点击区域的正中心】坐标。
代理会在你返回的(x,y)精确点击，偏差超过10像素就会点错！

## 输出格式（严格遵守，每个元素一行）
📌 [名称] | 中心: (x, y) | 区域: WxH

## ⚠️ 如何正确计算中心坐标

### 桌面图标
Windows桌面图标的可点击区域包含【图标图片】+【下方文字标签】。
在 ${imgW}x${imgH} 分辨率下，典型参数：
- 可点击区域宽度约 75 像素，高度约 85-95 像素
- 桌面左侧留白约 10 像素
- 第一列图标的中心 x 坐标 ≈ 10 + 75/2 = 47（约 40-55 之间）
- 列间距约 80-100 像素（第二列中心 x ≈ 125-145）
- 行间距约 85-100 像素
- 第一行图标中心 y 坐标 ≈ 留白 + 高度/2 ≈ 45-55
- 图标图片在上方，文字在下方，中心y应该在图标图片中间偏下的位置

⚠️ 常见错误：返回了图标图片的左上角像素坐标，而不是整个可点击区域的中心。
正确做法：找到图标图片的边界矩形，向下扩展包含文字，取整个区域的中心。

示例：假设你看到第一列第五个图标（间距90px），上方留白10px：
- 这个图标的可点击区域约从 y=10+4*90=370 到 y=370+90=460
- 中心 y = (370+460)/2 = 415
- 中心 x ≈ 47
→ 应返回：📌 图标名 | 中心: (47, 415) | 区域: 75x90

### 窗口和弹窗
- 按钮：找到按钮的矩形边界，取正中心
- 输入框：取输入框矩形的正中心
- 标题栏关闭按钮(X)：通常在窗口右上角

### 任务栏
- 通常在屏幕底部，高约 40-50 像素
- 中心 y ≈ 屏幕高度 - 25

## 自检（返回前必须确认）
1. 桌面图标的 x 坐标是否 ≥ 35？如果 < 35，说明你给的是图标左边缘而非中心，请修正
2. 同一列的图标 x 坐标是否一致（差异 < 5px）？
3. 相邻行图标间距是否在 80-100 之间？如果不一致，重新计算
4. 每个坐标是否在 [0, ${imgW - 1}] x [0, ${imgH - 1}] 范围内？`

            await debugScreenshotLog('vision_request', visionPrompt)
            const analysis = await analyzeImageWithVision(imageUrlForModel, visionPrompt, eaForScreenshot)
            await debugScreenshotLog('vision_response', visionPrompt, analysis)
            chat.updateMessagePart(tid, mid, call.toolCallId + '_vision_step', { stepStatus: 'completed', stepTitle: '截图分析完成' })

            const toolContent = `✅ 截图完成，屏幕内容已分析。

📏 分辨率：${imgW}x${imgH}  🖱️ 鼠标：(${cursorX}, ${cursorY})  📐 坐标范围：[0,${imgW - 1}]x[0,${imgH - 1}]

${analysis}
${winSection}

🎯 操作指南：
• mouse_double_click(x, y) — 双击打开桌面图标
• mouse_click(x, y) — 单击按钮或选中
• action_sequence(steps=[...]) — 拖拽等连续操作
• keyboard_type(text="...") — 输入文字
• keyboard_hotkey(keys=[...]) — 快捷键

⚠️ 重要规则：
1. 上面的坐标是元素可点击区域的中心点，直接传给 mouse_click/mouse_double_click
2. 用户说"双击/打开桌面上的XX" → 必须用鼠标操作，禁止用 open_application 或 terminal
3. 每次操作后 screenshot() 验证
4. 不要调 vision_analyze
5. 找不到目标才考虑键盘替代
6. 如果点击后没有反应，坐标可能有 ±15 像素误差，尝试微调坐标重试`

            await debugScreenshotLog('tool_result_to_main_model', toolContent)
            toolResultMessages.push({
              role: 'tool',
              content: toolContent,
              tool_call_id: call.toolCallId,
            })
          } catch (vErr: any) {
            chat.updateMessagePart(tid, mid, call.toolCallId + '_vision_step', { stepStatus: 'error', stepTitle: '视觉分析失败' })
            const fallbackContent = `截图已完成（${imgW}x${imgH}像素），但视觉分析暂时失败：${vErr?.message || vErr}
${winSection}
⚠️ 请再次调用 screenshot() 重试。如果连续失败3次以上，可以考虑：
- 用户说了桌面操作 → 继续重试截图，不要用 terminal 替代
- 非桌面操作 → 可以使用 keyboard_hotkey 等键盘方式`
            toolResultMessages.push({
              role: 'tool',
              content: fallbackContent,
              tool_call_id: call.toolCallId,
            })
          }
        } else if (mainModelIsMultimodal) {
          // === Strategy: Main model can see images — send tool result as text + inject image via user msg ===
          const toolText = `✅ 截图已完成，图片在下条消息中。

📏 屏幕分辨率：${imgW} x ${imgH} 像素
🖱️ 用户鼠标位置：(${cursorX}, ${cursorY})
📐 坐标范围：x ∈ [0, ${imgW - 1}], y ∈ [0, ${imgH - 1}]，原点(0,0)在左上角
${winSection}
## 坐标获取方法
仔细观察截图，找到目标元素的【可点击区域的正中心】坐标。
重要：桌面图标的可点击区域包括图标图片+下方文字标签，中心点在整个区域的正中间。
在 ${imgW}x${imgH} 分辨率下，桌面图标中心 x 通常在 40-55 左右（不是20-30）。

## 操作方法
- 双击打开桌面图标：mouse_double_click(x=中心X, y=中心Y)
- 单击按钮/链接：mouse_click(x=中心X, y=中心Y)
- 拖拽：action_sequence(steps=[...])
- 用户要求桌面操作 → 必须用鼠标，禁止用 open_application 或 terminal 替代
- 每次操作后 screenshot() 验证
- 禁止猜测坐标！`

          await debugScreenshotLog('multimodal_to_main_model', toolText)
          toolResultMessages.push({
            role: 'tool',
            content: toolText,
            tool_call_id: call.toolCallId,
          })
          // Inject screenshot as a separate user message so the model can actually see it
          toolResultMessages.push({
            role: 'user',
            content: [
              { type: 'text', text: `[截图已附上。请仔细分析后操作。注意：桌面图标的可点击区域包含图标图片+下方文字标签，中心坐标应在整个区域的正中间，而不是图标图片的左上角。在 ${imgW}x${imgH} 分辨率下，第一列桌面图标的中心 x 通常约 40-55，第二列约 120-140。]` },
              { type: 'image_url', image_url: { url: imageUrlForModel, detail: 'high' } },
            ],
          })
        } else {
          // === Strategy: No vision at all — text-only fallback ===
          const toolText = `Screenshot captured (${imgW}x${imgH}px) but NO vision model is configured. Cannot see the screen content.
${winSection}
SUGGESTION: Use keyboard shortcuts to accomplish the task:
- Win key → type app name → Enter (to open apps)
- Alt+Tab (switch windows), Alt+F4 (close), Ctrl+C/V (copy/paste)
- Win+D (show desktop)
Consider configuring a vision model in Settings for visual desktop control.`
          toolResultMessages.push({
            role: 'tool',
            content: toolText,
            tool_call_id: call.toolCallId,
          })
        }
      } else {
        toolResultMessages.push({
          role: 'tool', content: result.result, tool_call_id: call.toolCallId,
        })
      }
    } catch (e: any) {
      chat.updateMessagePart(tid, mid, call.toolCallId, { toolStatus: 'error' })
      chat.addMessagePart(tid, mid, {
        id: call.toolCallId + '_result', type: 'tool_result',
        toolCallId: call.toolCallId, content: `Error: ${e?.message || e}`,
      })
      toolResultMessages.push({
        role: 'tool', content: `Error: ${e?.message || e}`, tool_call_id: call.toolCallId,
      })
    }
  }
  } // end else (sequential tool execution)

  chat.updateMessagePart(tid, mid, `step_${toolLoopDepth.value}`, { stepStatus: 'completed' })

  // Persist toolCallsRaw on the assistant message so future rounds can see them
  chat.updateMessage(tid, mid, { toolCallsRaw: toolCallsRaw })

  for (const trm of toolResultMessages) {
    chat.addMessage(tid, {
      role: 'tool',
      content: trm.content,
      status: 'complete',
      toolCallId: trm.tool_call_id,
    })
  }

  // If task_complete was called and this is NOT the self-evolution round,
  // allow one more round to save skills/memory. If it IS the self-evolution
  // round (model didn't call any new tools needing execution), stop.
  if (taskCompletedInBatch) {
    if (taskCompleteRound.value >= 1) {
      toolLoopDepth.value = 0
      recentToolSignatures.value = []
      taskCompleteRound.value = 0
      chat.updateMessage(tid, mid, { status: 'complete' })
      chat.setStreaming(false)
      streamTopicId.value = null
      streamAssistantId.value = null
      return
    }
    taskCompleteRound.value++
  }

  // Trigger context compression in background if needed
  void maybeCompressContext(tid)

  // Auto-continue: send next round with tool results
  if (aborted.value) return
  const topicAgent = effectiveAgent(tid)
  const lang = topicAgent?.languageModel
  if (!lang?.providerId || !lang?.modelId) {
    chat.setStreaming(false)
    streamTopicId.value = null
    streamAssistantId.value = null
    return
  }

  const asst = chat.addMessage(tid, {
    role: 'assistant',
    content: '',
    status: 'streaming',
    parts: [],
  })
  streamAssistantId.value = asst.id

  const messages = buildApiMessagesForSend(tid, asst.id, getLastUserText(tid, asst.id))

  if (taskCompletedInBatch && taskCompleteRound.value === 1) {
    messages.push({
      role: 'user',
      content: `[SYSTEM: Background Review] The task is complete. Reflect on this session:
1. Were there any reusable workflows? If yes, save as a skill via skill_manage(action='create').
2. Did you learn anything new (user preferences, API quirks, system config)? Save with memory(action='add').
3. Did you make mistakes that should be avoided next time? Save corrections with memory(action='add', category='correction').
Only save genuinely useful knowledge. If nothing is worth saving, simply respond with a brief summary.`,
    } as any)
  }

  // Safety timeout: if the stream doesn't complete within 120s, force cleanup
  if (agentSafetyTimer) clearTimeout(agentSafetyTimer)
  agentSafetyTimer = setTimeout(() => {
    if (streamAssistantId.value === asst.id && chat.getTopicMessages(tid).find(m => m.id === asst.id)?.status === 'streaming') {
      console.warn('[agent-loop] Safety timeout: forcing stream cleanup for', asst.id)
      chat.updateMessage(tid, asst.id, { status: 'error', error: '请求超时，请重试' })
      toolLoopDepth.value = 0
      chat.setStreaming(false)
      streamTopicId.value = null
      streamAssistantId.value = null
    }
  }, 120000)

  try {
    await sendChatMessage({
      providerId: lang.providerId,
      modelId: lang.modelId,
      messageId: asst.id,
      messages,
      options: { ...getToolOptions() },
    })
  } catch (e) {
    if (agentSafetyTimer) { clearTimeout(agentSafetyTimer); agentSafetyTimer = null }
    console.error(e)
    chat.updateMessage(tid, asst.id, { status: 'error', error: String(e) })
    toolLoopDepth.value = 0
    chat.setStreaming(false)
    streamTopicId.value = null
    streamAssistantId.value = null
  }
  } catch (outerErr: any) {
    console.error('[agent-loop] Unhandled error in executeToolCallsAndContinue:', outerErr)
    chat.updateMessage(tid, mid, { status: 'error', error: `Agent 循环异常: ${outerErr?.message || outerErr}` })
    toolLoopDepth.value = 0
    pendingToolCalls.value = []
    chat.setStreaming(false)
    streamTopicId.value = null
    streamAssistantId.value = null
  }
}

async function notifyChannels(topicId: string, messageId: string) {
  try {
    const settingsStore = useSettingsStore()
    const channels = settingsStore.channels.filter(c => c.enabled && c.notifyOnReply && c.webhookUrl)
    if (!channels.length) return

    const msg = chat.getTopicMessages(topicId).find(m => m.id === messageId)
    if (!msg) return

    const content = msg.content || msg.parts?.filter((p: any) => p.type === 'text').map((p: any) => p.content).join('\n') || ''
    if (!content.trim()) return

    const topic = chat.topics.find(t => t.id === topicId)
    const topicTitle = topic?.title || '未命名话题'
    const modelName = msg.agentName || '未知模型'
    const maxLen = 500
    const truncated = content.length > maxLen ? content.slice(0, maxLen) + '...' : content

    const { invoke } = await import('@tauri-apps/api/core')
    for (const ch of channels) {
      const tpl = ch.messageTemplate?.trim()
      const text = tpl
        ? tpl.replace(/\{content\}/g, truncated).replace(/\{model\}/g, modelName).replace(/\{topic\}/g, topicTitle)
        : `🦊 Fox AI - ${topicTitle}\n模型: ${modelName}\n\n${truncated}`
      invoke('send_channel_notification', {
        notification: { platformId: ch.platformId, webhookUrl: ch.webhookUrl, secret: ch.secret || null, content: text }
      }).catch(e => console.warn(`Channel notify failed (${ch.name}):`, e))
    }
  } catch (e) {
    console.warn('notifyChannels error:', e)
  }
}

async function onEnd(_payload: { messageId: string; hasToolCalls: boolean }) {
  if (agentSafetyTimer) { clearTimeout(agentSafetyTimer); agentSafetyTimer = null }
  if (aborted.value) {
    streamRetryCount.value = 0
    chat.setStreaming(false)
    streamTopicId.value = null
    streamAssistantId.value = null
    return
  }
  if (!streamTopicId.value || !streamAssistantId.value) {
    chat.setStreaming(false)
    return
  }
  if (_payload.messageId !== streamAssistantId.value) {
    return
  }
  const tid = streamTopicId.value
  const mid = streamAssistantId.value

  const msg = chat.getTopicMessages(tid).find(m => m.id === mid)
  const hasContent = msg?.content?.trim()
    || msg?.parts?.some((p: any) => (p.type === 'text' || p.type === 'reasoning') && p.content?.trim())
  if (!hasContent && !_payload.hasToolCalls && pendingToolCalls.value.length === 0) {
    if (toolLoopDepth.value > 0) {
      if (streamRetryCount.value < MAX_STREAM_RETRIES && !aborted.value) {
        console.warn(`[ChatPage] Empty response in agent loop, retrying (${streamRetryCount.value + 1}/${MAX_STREAM_RETRIES})...`)
        const ok = await retryCurrentStream(tid, mid)
        if (ok) return
      }
      console.warn('[ChatPage] Empty response in agent loop, ending loop gracefully')
      toolLoopDepth.value = 0
      streamRetryCount.value = 0
      chat.updateMessage(tid, mid, { status: 'complete' })
    } else {
      if (streamRetryCount.value < MAX_STREAM_RETRIES && !aborted.value) {
        console.warn(`[ChatPage] Empty response, auto-retrying (${streamRetryCount.value + 1}/${MAX_STREAM_RETRIES})...`)
        const ok = await retryCurrentStream(tid, mid)
        if (ok) return
      }
      streamRetryCount.value = 0
      chat.updateMessage(tid, mid, { status: 'error', error: 'AI 返回为空，请检查模型配置或重试' })
    }
    chat.setStreaming(false)
    streamTopicId.value = null
    streamAssistantId.value = null
    return
  }

  streamRetryCount.value = 0
  chat.updateMessage(tid, mid, { status: 'complete' })

  if (_payload.hasToolCalls && pendingToolCalls.value.length > 0) {
    void executeToolCallsAndContinue(tid, mid)
    return
  }

  toolLoopDepth.value = 0

  const next = pendingGroupReplies.value.shift()
  if (next) {
    const asst = chat.addMessage(tid, {
      role: 'assistant',
      content: '',
      status: 'streaming',
      agentId: next.id,
      agentName: next.name,
      parts: [],
    })
    streamAssistantId.value = asst.id
    chat.setStreaming(true)
    const userCtx = getLastUserText(tid, asst.id)
    const history = buildApiMessagesForSend(tid, asst.id, userCtx, next)
    const lm = next.languageModel
    void sendChatMessage({
      providerId: lm.providerId,
      modelId: lm.modelId,
      messageId: asst.id,
      messages: history,
      options: { ...getToolOptions(next) },
    }).catch(e => {
      console.error(e)
      chat.updateMessage(tid, asst.id, { status: 'error', error: t('common.error') as string })
      chat.setStreaming(false)
      streamTopicId.value = null
      streamAssistantId.value = null
      pendingGroupReplies.value = []
    })
    return
  }

  chat.setStreaming(false)
  notifyChannels(tid, mid)
  streamTopicId.value = null
  streamAssistantId.value = null
  pendingGroupReplies.value = []
}

async function retryCurrentStream(tid: string, mid: string): Promise<boolean> {
  if (aborted.value || streamRetryCount.value >= MAX_STREAM_RETRIES) return false
  streamRetryCount.value++
  console.warn(`[agent-loop] Retrying stream (attempt ${streamRetryCount.value}/${MAX_STREAM_RETRIES})...`)

  const ea = effectiveAgent(tid)
  const lang = ea?.languageModel
  if (!lang?.providerId || !lang?.modelId) return false

  chat.updateMessage(tid, mid, { status: 'streaming', content: '', error: undefined })

  await new Promise(r => setTimeout(r, 2000 + streamRetryCount.value * 1000))
  if (aborted.value) return false

  let history = buildApiMessagesForSend(tid, mid, getLastUserText(tid, mid), ea)

  // On each retry, aggressively trim history to reduce context size
  // This handles context_length_exceeded and similar issues
  if (streamRetryCount.value >= 2 && history.length > 4) {
    const systemMsgs = history.filter(m => m.role === 'system')
    const nonSystem = history.filter(m => m.role !== 'system')
    // Keep last N messages, reducing N with each retry
    const keepCount = Math.max(4, nonSystem.length - streamRetryCount.value * 3)
    const trimmed = nonSystem.slice(-keepCount)
    // Also strip any remaining image content from older messages to save tokens
    for (let i = 0; i < trimmed.length - 2; i++) {
      const m = trimmed[i]
      if (Array.isArray(m.content)) {
        trimmed[i] = { ...m, content: m.content.filter((p: any) => p.type !== 'image_url').map((p: any) => p.text || p.content || '').join('\n') || '[image omitted]' }
      }
    }
    history = [...systemMsgs, ...trimmed]
    console.warn(`[agent-loop] Trimmed history from ${nonSystem.length} to ${trimmed.length} messages for retry ${streamRetryCount.value}`)
  }

  try {
    await sendChatMessage({
      providerId: lang.providerId,
      modelId: lang.modelId,
      messageId: mid,
      messages: history,
      options: { ...getToolOptions(ea) },
    })
    return true
  } catch (e) {
    console.error('[agent-loop] Retry failed:', e)
    return false
  }
}

async function onErr(payload: { messageId: string; error: string }) {
  if (streamAssistantId.value && payload.messageId !== streamAssistantId.value) {
    return
  }
  const tid = streamTopicId.value
  const mid = streamAssistantId.value
  if (tid && mid && streamRetryCount.value < MAX_STREAM_RETRIES && !aborted.value) {
    console.warn(`[agent-loop] Stream error, will retry: ${payload.error}`)
    const ok = await retryCurrentStream(tid, mid)
    if (ok) return
  }

  if (tid) {
    chat.updateMessage(tid, payload.messageId, {
      status: 'error',
      error: payload.error || (t('common.error') as string),
    })
  }
  streamRetryCount.value = 0
  pendingGroupReplies.value = []
  pendingToolCalls.value = []
  toolLoopDepth.value = 0
  recentToolSignatures.value = []
  chat.setStreaming(false)
  streamTopicId.value = null
  streamAssistantId.value = null
}

watch(
  () => topicAgents.value.map(a => a.id),
  (ids) => {
    const topic = chat.currentTopic
    if (!topic || !topic.isGroupChat) return
    const current = new Set(topic.participantAgentIds || [])
    const hasNew = ids.some(id => !current.has(id))
    if (hasNew) {
      chat.updateTopic(topic.id, { participantAgentIds: [...ids] })
    }
  },
  { deep: true }
)

onMounted(async () => {
  await provider.loadProviders()
  agent.initDefaultAgent()
  unlistenFns = [
    await onStreamChunk(onChunk),
    await onStreamThinking(onThinking),
    await onStreamToolCall(onToolCallEvent),
    await onStreamEnd(onEnd),
    await onStreamError(onErr),
  ]
})

onUnmounted(() => {
  for (const u of unlistenFns) u()
  unlistenFns = []
  window.removeEventListener('mousemove', onResizeMove)
  window.removeEventListener('mouseup', onResizeEnd)
})
</script>

<template>
  <div class="chat-page" :class="{ 'chat-page--resizing': isDrag }">
    <aside
      class="chat-page__side"
      :style="{ width: `${sidebarW}px` }"
    >
      <TopicList />
    </aside>
    <div
      class="chat-page__gutter"
      title="Drag to resize"
      @mousedown="onResizeStart"
    />
    <section class="chat-page__main">
      <ChatView
        :topic-title="topicTitle"
        :has-topic="hasTopic"
        :messages="chat.currentMessages"
        :streaming="streaming"
        :provider-id="currentProviderId"
        :model-id="currentModelId"
        :is-group-chat="!!chat.currentTopic?.isGroupChat"
        @send="onSend"
        @stop="onStop"
        @update:provider-id="onUpdProvider"
        @update:model-id="onUpdModel"
        @edit="onEditMessage"
        @regenerate="onRegenerateMessage"
        @delete="onDeleteMessage"
        @new-topic="onNewTopic"
        @clear-context="onClearContext"
        @clear-messages="onClearMessages"
        @web-search="onWebSearch"
        @quick-phrase="onQuickPhrase"
        @mention="onMentionFromInput"
        @tool-approved="onToolApproved"
        @tool-rejected="onToolRejected"
      />
    </section>
  </div>
</template>

<style lang="scss" scoped>
.chat-page {
  display: flex;
  height: 100%;
  min-height: 0;
  min-width: 0;
  background: var(--color-background);
}

.chat-page--resizing * {
  cursor: col-resize !important;
}

.chat-page__side {
  flex-shrink: 0;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.chat-page__gutter {
  width: 5px;
  flex-shrink: 0;
  cursor: col-resize;
  background: var(--color-border);
  position: relative;
  transition: background 0.15s;
  z-index: 2;
}

.chat-page__gutter:hover,
.chat-page__gutter-active {
  background: var(--color-text-3);
}

.chat-page__gutter::after {
  content: '';
  position: absolute;
  inset: 0;
  top: 40%;
  bottom: 40%;
  left: 50%;
  width: 1px;
  margin-left: -0.5px;
  background: var(--color-text-3);
  border-radius: 1px;
  opacity: 0.6;
}

.chat-page__main {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>
