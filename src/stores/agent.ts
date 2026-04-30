import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { v4 as uuidv4 } from 'uuid'
import { useProviderStore } from '@/stores/provider'
import { useSettingsStore } from '@/stores/settings'
import { usePersonalityStore } from '@/stores/personality'
import { useSkillStore } from '@/stores/skill'
import type { AgentConfig, AgentState, ModelSlot } from '@/types'

export type { AgentConfig, AgentState, ModelSlot } from '@/types'
export type PermissionLevel = 'default' | 'full'
export type WebSearchEngine =
  | 'zhipu'
  | 'tavily'
  | 'searxng'
  | 'exa'
  | 'examcp'
  | 'bocha'
  | 'querit'
  | 'google'
  | 'bing'
  | 'baidu'
export type ToolCallStatus = 'pending' | 'approved' | 'rejected' | 'running' | 'completed' | 'error'

export type PermissionAction = 'ask' | 'allow' | 'deny'

export interface PermissionConfig {
  shell: PermissionAction
  file_read: PermissionAction
  file_write: PermissionAction
  web_browse: PermissionAction
  code_execute: PermissionAction
  screen_capture: PermissionAction
  mouse_control: PermissionAction
  keyboard_input: PermissionAction
  app_launch: PermissionAction
}

const DEFAULT_PERMISSION_CONFIG: PermissionConfig = {
  shell: 'ask',
  file_read: 'ask',
  file_write: 'ask',
  web_browse: 'allow',
  code_execute: 'ask',
  screen_capture: 'ask',
  mouse_control: 'ask',
  keyboard_input: 'ask',
  app_launch: 'ask'
}

export interface AgentTool {
  id: string
  name: string
  description: string
  enabled: boolean
  requiresPermission: boolean
}

export interface PendingToolCall {
  id: string
  toolName: string
  args: Record<string, unknown>
  status: ToolCallStatus
  result?: string
}

export interface ToolSchema {
  name: string
  description: string
  parameters: Record<
    string,
    {
      type: string
      description: string
      required?: boolean
    }
  >
}

export interface ToolRegistry {
  [toolId: string]: {
    schema: ToolSchema
    enabled: boolean
    requiresPermission: boolean
    backend: 'local' | 'docker' | 'remote'
  }
}

const VALID_TRANSITIONS: Record<AgentState, AgentState[]> = {
  idle: ['thinking'],
  thinking: ['streaming', 'awaiting_tool_approval', 'error', 'completed'],
  awaiting_tool_approval: ['tool_running', 'idle', 'error'],
  tool_running: ['thinking', 'error', 'completed'],
  streaming: ['thinking', 'completed', 'error', 'paused'],
  paused: ['thinking', 'idle'],
  error: ['idle', 'thinking'],
  completed: ['idle']
}

const DEFAULT_QUICK_PHRASES = [
  '翻译成英文',
  '解释这段代码',
  '总结以上内容',
  '继续',
  '优化这段代码',
  '写单元测试'
]

const DEFAULT_AGENT_TOOLS: AgentTool[] = [
  // computer_use toolset
  { id: 'screenshot', name: '截屏', description: '截取当前屏幕截图', enabled: true, requiresPermission: false },
  { id: 'mouse_click', name: '鼠标点击', description: '在指定坐标精确点击', enabled: true, requiresPermission: true },
  { id: 'mouse_double_click', name: '鼠标双击', description: '双击', enabled: true, requiresPermission: true },
  { id: 'mouse_move', name: '鼠标移动', description: '移动鼠标', enabled: true, requiresPermission: true },
  { id: 'mouse_drag', name: '鼠标拖拽', description: '拖拽操作', enabled: true, requiresPermission: true },
  { id: 'action_sequence', name: '连续操作', description: '执行多步连续鼠标/键盘操作（拖放、棋子移动等）', enabled: true, requiresPermission: true },
  { id: 'mouse_scroll', name: '鼠标滚轮', description: '滚动页面', enabled: true, requiresPermission: true },
  { id: 'open_application', name: '打开应用', description: '通过名称打开应用程序', enabled: true, requiresPermission: false },
  { id: 'open_url', name: '打开网页', description: '在浏览器中打开指定URL', enabled: true, requiresPermission: false },
  { id: 'rapid_click', name: '快速连点', description: '在同一位置快速连续点击（游戏操作）', enabled: true, requiresPermission: true },
  { id: 'keyboard_type', name: '键盘输入', description: '模拟键盘输入', enabled: true, requiresPermission: true },
  { id: 'keyboard_key', name: '按键', description: '按下单个键', enabled: true, requiresPermission: true },
  { id: 'keyboard_hotkey', name: '快捷键', description: '组合快捷键', enabled: true, requiresPermission: true },
  { id: 'wait', name: '等待', description: '等待指定毫秒', enabled: true, requiresPermission: false },
  // terminal toolset
  { id: 'terminal', name: '终端', description: '在终端中运行命令（支持后台/超时/工作目录）', enabled: true, requiresPermission: true },
  { id: 'process', name: '进程管理', description: '列出/终止进程', enabled: true, requiresPermission: true },
  // file toolset
  { id: 'read_file', name: '读取文件', description: '读取文件内容', enabled: true, requiresPermission: true },
  { id: 'write_file', name: '写入文件', description: '创建或覆盖文件', enabled: true, requiresPermission: true },
  { id: 'patch', name: '文件补丁', description: '搜索替换修补文件', enabled: true, requiresPermission: true },
  { id: 'search_files', name: '搜索文件', description: '在目录中搜索文本', enabled: true, requiresPermission: false },
  // web toolset
  { id: 'web_search', name: '网络搜索', description: '搜索引擎查询', enabled: true, requiresPermission: false },
  { id: 'web_extract', name: '提取网页', description: '抓取URL并提取内容', enabled: true, requiresPermission: false },
  { id: 'fetch_url', name: '获取网页', description: 'HTTP GET 获取原始内容', enabled: true, requiresPermission: false },
  // vision toolset
  { id: 'vision_analyze', name: '视觉分析', description: '用视觉模型分析图片', enabled: true, requiresPermission: false },
  // code_execution toolset
  { id: 'execute_code', name: '执行代码', description: '在隔离环境中运行代码', enabled: true, requiresPermission: true },
  // skills toolset
  { id: 'skills_list', name: '技能列表', description: '列出所有可用技能', enabled: true, requiresPermission: false },
  { id: 'skill_view', name: '查看技能', description: '查看技能详细内容', enabled: true, requiresPermission: false },
  { id: 'skill_manage', name: '管理技能', description: '创建/编辑/删除技能', enabled: true, requiresPermission: false },
  // planning toolset
  { id: 'task_complete', name: '任务完成', description: '声明任务已完成', enabled: true, requiresPermission: false },
  { id: 'todo', name: '待办事项', description: '管理任务列表', enabled: true, requiresPermission: false },
  { id: 'memory', name: '记忆管理', description: '管理持久记忆（新增/替换/删除）', enabled: true, requiresPermission: false },
  { id: 'session_search', name: '对话搜索', description: '搜索当前对话历史', enabled: true, requiresPermission: false },
]

const DEFAULT_ENABLED_TOOL_IDS = DEFAULT_AGENT_TOOLS.map(t => t.id)

function resolveDefaultLanguageModel(): ModelSlot {
  const settings = useSettingsStore()
  const provider = useProviderStore()
  const fp = settings.defaultProviderId
  const fm = settings.defaultModelId
  const p = provider.getProviderById(fp)
  if (p?.models?.some(m => m === fm) && fp && fm) {
    return { providerId: fp, modelId: fm }
  }
  const en = provider.getEnabledProviders()
  const first = en[0]
  if (first?.models?.[0]) {
    return { providerId: first.id, modelId: first.models[0] }
  }
  return { providerId: '', modelId: '' }
}

function hermesAgentDefaults(
  options: { personalityId: string | null; inherit?: AgentConfig } = { personalityId: null }
) {
  const { personalityId, inherit } = options
  return {
    personalityId:
      personalityId !== undefined && personalityId !== null
        ? personalityId
        : (inherit?.personalityId ?? null),
    memoryEnabled: inherit?.memoryEnabled ?? true,
    skillIds: inherit ? [...(inherit.skillIds || [])] : [],
    sessionSearchEnabled: inherit?.sessionSearchEnabled ?? true
  }
}

function normalizeAgentConfig(a: AgentConfig) {
  if (a.personalityId === undefined) a.personalityId = null
  if (a.memoryEnabled === undefined) a.memoryEnabled = true
  if (a.skillIds === undefined) a.skillIds = []
  if (a.sessionSearchEnabled === undefined) a.sessionSearchEnabled = true
  if (!a.tools?.length) a.tools = [...DEFAULT_ENABLED_TOOL_IDS]
}

export const useAgentStore = defineStore('agent', () => {
  const permissionLevel = ref<PermissionLevel>('default')
  const permissionConfig = ref<PermissionConfig>({ ...DEFAULT_PERMISSION_CONFIG })
  const permissionRules = ref<Map<string, 'once' | 'always' | 'reject'>>(new Map())
  const webSearchEnabled = ref(false)
  const webSearchEngine = ref<WebSearchEngine>('bing')
  const quickPhrases = ref<string[]>([...DEFAULT_QUICK_PHRASES])
  const agentTools = ref<AgentTool[]>([...DEFAULT_AGENT_TOOLS])
  const pendingToolCalls = ref<PendingToolCall[]>([])
  const toolRegistry = ref<ToolRegistry>({})

  const agents = ref<AgentConfig[]>([])
  const currentAgentId = ref<string>('')

  const currentAgent = computed(() => {
    const id = currentAgentId.value
    if (!id) return null
    return agents.value.find(a => a.id === id) || null
  })

  function getMainAgent(): AgentConfig | null {
    return agents.value.find(a => a.type === 'main') || null
  }

  function getSubAgents(parentId: string): AgentConfig[] {
    return agents.value.filter(a => a.parentId === parentId)
  }

  function createMainAgent(): AgentConfig {
    const existing = getMainAgent()
    if (existing) return existing

    const pers = usePersonalityStore()
    const skills = useSkillStore()
    pers.ensurePresets()
    skills.ensurePresets()
    const defPersona = pers.getDefault()
    const lm = resolveDefaultLanguageModel()
    const hd = hermesAgentDefaults({ personalityId: defPersona?.id ?? null })
    const agent: AgentConfig = {
      id: uuidv4(),
      name: 'Main',
      type: 'main',
      parentId: null,
      systemPrompt: '',
      languageModel: { ...lm },
      visionModel: null,
      toolModel: null,
      tools: [...DEFAULT_ENABLED_TOOL_IDS],
      maxIterations: 0,
      delegateDepth: 0,
      status: 'idle',
      createdAt: Date.now(),
      personalityId: hd.personalityId,
      memoryEnabled: hd.memoryEnabled,
      skillIds: skills.getEnabledSkills().map(s => s.id),
      sessionSearchEnabled: hd.sessionSearchEnabled
    }
    agents.value = [...agents.value, agent]
    if (!currentAgentId.value) currentAgentId.value = agent.id
    return agent
  }

  function createSubAgent(
    parentId: string,
    config: Partial<Omit<AgentConfig, 'id' | 'type' | 'parentId' | 'delegateDepth' | 'createdAt'>> = {}
  ): AgentConfig | null {
    const parent = agents.value.find(a => a.id === parentId)
    if (!parent) return null
    normalizeAgentConfig(parent)

    const lm = config.languageModel
      ? { ...config.languageModel }
      : { ...parent.languageModel }

    const vis =
      config.visionModel !== undefined
        ? config.visionModel
        : parent.visionModel
          ? { ...parent.visionModel }
          : null

    const tmod =
      config.toolModel !== undefined
        ? config.toolModel
        : parent.toolModel
          ? { ...parent.toolModel }
          : null

    const hd = hermesAgentDefaults({ inherit: parent })
    const agent: AgentConfig = {
      id: uuidv4(),
      name: config.name?.trim() || 'Sub',
      type: 'sub',
      parentId,
      systemPrompt: config.systemPrompt ?? '',
      languageModel: lm,
      visionModel: vis,
      toolModel: tmod,
      tools: config.tools?.length
        ? [...config.tools]
        : [...(parent.tools.length ? parent.tools : DEFAULT_ENABLED_TOOL_IDS)],
      maxIterations: config.maxIterations ?? 0,
      delegateDepth: parent.delegateDepth + 1,
      status: config.status ?? 'idle',
      createdAt: Date.now(),
      personalityId:
        config.personalityId !== undefined ? config.personalityId : hd.personalityId,
      memoryEnabled: config.memoryEnabled ?? parent.memoryEnabled,
      skillIds: config.skillIds !== undefined ? [...config.skillIds] : [...(parent.skillIds || [])],
      sessionSearchEnabled: config.sessionSearchEnabled ?? parent.sessionSearchEnabled
    }
    agents.value = [...agents.value, agent]
    return agent
  }

  function switchAgent(id: string) {
    if (agents.value.some(a => a.id === id)) {
      currentAgentId.value = id
    }
  }

  function deleteAgent(id: string) {
    const a = agents.value.find(x => x.id === id)
    if (!a || a.type === 'main') return

    const toRemove = new Set<string>([id])
    function collectSubs(pid: string) {
      for (const c of agents.value) {
        if (c.parentId === pid) {
          toRemove.add(c.id)
          collectSubs(c.id)
        }
      }
    }
    collectSubs(id)

    agents.value = agents.value.filter(x => !toRemove.has(x.id))
    if (toRemove.has(currentAgentId.value)) {
      const main = getMainAgent()
      currentAgentId.value = main?.id || agents.value[0]?.id || ''
    }
  }

  function updateAgent(id: string, partial: Partial<AgentConfig>) {
    const a = agents.value.find(x => x.id === id)
    if (!a) return
    if (partial.name !== undefined) a.name = partial.name
    if (partial.systemPrompt !== undefined) a.systemPrompt = partial.systemPrompt
    if (partial.languageModel !== undefined) a.languageModel = { ...partial.languageModel }
    if (partial.visionModel !== undefined) {
      a.visionModel = partial.visionModel ? { ...partial.visionModel } : null
    }
    if (partial.toolModel !== undefined) {
      a.toolModel = partial.toolModel ? { ...partial.toolModel } : null
    }
    if (partial.tools !== undefined) a.tools = [...partial.tools]
    if (partial.maxIterations !== undefined) a.maxIterations = partial.maxIterations
    if (partial.status !== undefined) a.status = partial.status
    if (partial.personalityId !== undefined) a.personalityId = partial.personalityId
    if (partial.memoryEnabled !== undefined) a.memoryEnabled = partial.memoryEnabled
    if (partial.skillIds !== undefined) a.skillIds = [...partial.skillIds]
    if (partial.sessionSearchEnabled !== undefined) a.sessionSearchEnabled = partial.sessionSearchEnabled
    if (partial.channelId !== undefined) a.channelId = partial.channelId
  }

  function transitionAgentState(agentId: string, newState: AgentState): boolean {
    const a = agents.value.find(x => x.id === agentId)
    if (!a) return false
    const current = a.status
    const allowed = VALID_TRANSITIONS[current]
    if (!allowed?.includes(newState)) {
      console.warn(`[Agent] Invalid state transition: ${current} -> ${newState}`)
      return false
    }
    a.status = newState
    return true
  }

  function registerTool(
    id: string,
    schema: ToolSchema,
    opts?: { requiresPermission?: boolean; backend?: 'local' | 'docker' | 'remote' }
  ) {
    toolRegistry.value[id] = {
      schema,
      enabled: true,
      requiresPermission: opts?.requiresPermission ?? true,
      backend: opts?.backend ?? 'local'
    }
  }

  function unregisterTool(id: string) {
    delete toolRegistry.value[id]
  }

  function getToolSchemas(): ToolSchema[] {
    return Object.values(toolRegistry.value)
      .filter(t => t.enabled)
      .map(t => t.schema)
  }

  async function executeToolCall(callId: string): Promise<string> {
    const tc = pendingToolCalls.value.find(p => p.id === callId)
    if (!tc) return 'Tool call not found'

    const agentId = currentAgentId.value
    if (agentId) transitionAgentState(agentId, 'tool_running')

    tc.status = 'running'

    try {
      const { executeTool } = await import('@/utils/tool-executor')
      const result = await executeTool(tc.toolName, tc.args)

      tc.status = 'completed'
      tc.result = result.result

      if (agentId) transitionAgentState(agentId, result.success ? 'thinking' : 'error')
      return tc.result
    } catch (err) {
      tc.status = 'completed'
      tc.result = `Error: ${String(err)}`
      if (agentId) transitionAgentState(agentId, 'error')
      return tc.result
    }
  }

  function initDefaultAgent() {
    const pers = usePersonalityStore()
    const sk = useSkillStore()
    const providerStore = useProviderStore()
    pers.ensurePresets()
    sk.ensurePresets()
    for (const a of agents.value) normalizeAgentConfig(a)
    for (const a of agents.value) {
      if ((a.status as string) === 'running') {
        a.status = 'idle'
      }
    }
    if (!agents.value.length) {
      const m = createMainAgent()
      currentAgentId.value = m.id
    } else {
      const main = getMainAgent()
      if (main) {
        if (main.personalityId == null && pers.getDefault()) {
          main.personalityId = pers.getDefault()!.id
        }
        if (!agents.value.some(a => a.id === currentAgentId.value)) {
          currentAgentId.value = main.id
        }
      } else {
        const m = createMainAgent()
        currentAgentId.value = m.id
      }
    }

    // Validate persisted languageModel.providerId — if stale, re-resolve
    for (const a of agents.value) {
      const pid = a.languageModel?.providerId
      if (pid) {
        const found = providerStore.getProviderById(pid)
        if (!found) {
          console.warn(`[agent] stale providerId=${pid}, re-resolving`)
          const fresh = resolveDefaultLanguageModel()
          a.languageModel = { ...fresh }
        }
      } else {
        const fresh = resolveDefaultLanguageModel()
        a.languageModel = { ...fresh }
      }
    }

    if (!Object.keys(toolRegistry.value).length) {
      for (const t of DEFAULT_AGENT_TOOLS) {
        registerTool(
          t.id,
          {
            name: t.name,
            description: t.description,
            parameters: {}
          },
          { requiresPermission: t.requiresPermission }
        )
      }
    }
  }

  function setPermission(level: PermissionLevel) {
    permissionLevel.value = level
  }

  function setPermissionCategory(category: keyof PermissionConfig, action: PermissionAction) {
    permissionConfig.value[category] = action
  }

  function checkPermission(
    category: keyof PermissionConfig,
    toolName: string
  ): 'ask' | 'allow' | 'deny' {
    const ruleKey = `${category}:${toolName}`
    const remembered = permissionRules.value.get(ruleKey)
    if (remembered === 'always') return 'allow'
    if (remembered === 'reject') return 'deny'

    if (permissionLevel.value === 'full') return 'allow'

    return permissionConfig.value[category]
  }

  function rememberPermission(category: string, toolName: string, action: 'once' | 'always' | 'reject') {
    if (action === 'always' || action === 'reject') {
      permissionRules.value.set(`${category}:${toolName}`, action)
    }
  }

  function clearPermissionRules() {
    permissionRules.value.clear()
  }

  function togglePermission() {
    permissionLevel.value = permissionLevel.value === 'default' ? 'full' : 'default'
  }

  function toggleWebSearch() {
    webSearchEnabled.value = !webSearchEnabled.value
  }

  function setWebSearchEnabled(on: boolean) {
    webSearchEnabled.value = on
  }

  function setSearchEngine(engine: WebSearchEngine) {
    webSearchEngine.value = engine
  }

  function addQuickPhrase(phrase: string) {
    const p = phrase.trim()
    if (!p || quickPhrases.value.includes(p)) return
    quickPhrases.value = [...quickPhrases.value, p]
  }

  function removeQuickPhrase(phrase: string) {
    quickPhrases.value = quickPhrases.value.filter(x => x !== phrase)
  }

  function updateQuickPhrase(index: number, phrase: string) {
    const p = phrase.trim()
    if (!p) return
    const arr = [...quickPhrases.value]
    if (index < 0 || index >= arr.length) return
    if (arr.includes(p) && arr[index] !== p) return
    arr[index] = p
    quickPhrases.value = arr
  }

  function addPendingToolCall(
    data: { id: string; toolName: string; args: Record<string, unknown> }
  ) {
    pendingToolCalls.value.push({
      id: data.id,
      toolName: data.toolName,
      args: data.args,
      status: 'pending'
    })
  }

  function approveToolCall(id: string) {
    const t = pendingToolCalls.value.find(p => p.id === id)
    if (t && t.status === 'pending') t.status = 'approved'
  }

  function rejectToolCall(id: string) {
    const t = pendingToolCalls.value.find(p => p.id === id)
    if (t && t.status === 'pending') t.status = 'rejected'
  }

  function setToolCallStatus(id: string, status: ToolCallStatus, result?: string) {
    const t = pendingToolCalls.value.find(p => p.id === id)
    if (t) {
      t.status = status
      if (result !== undefined) t.result = result
    }
  }

  function getEffectiveAgent(
    topicAgentSnapshots: Record<string, AgentConfig> | undefined,
    agentId?: string
  ): AgentConfig | null {
    const aid = agentId || currentAgentId.value
    if (topicAgentSnapshots?.[aid]) return topicAgentSnapshots[aid]
    if (agentId) return agents.value.find(a => a.id === agentId) || null
    return currentAgent.value
  }

  return {
    agents, currentAgentId, currentAgent,
    permissionLevel, permissionConfig, permissionRules, quickPhrases,
    agentTools, pendingToolCalls, toolRegistry,
    webSearchEnabled, webSearchEngine,
    getMainAgent,
    getSubAgents,
    createMainAgent,
    createSubAgent,
    switchAgent,
    deleteAgent,
    updateAgent,
    initDefaultAgent,
    transitionAgentState,
    setPermission,
    setPermissionCategory,
    checkPermission,
    rememberPermission,
    clearPermissionRules,
    togglePermission,
    toggleWebSearch,
    setWebSearchEnabled,
    setSearchEngine,
    addQuickPhrase,
    removeQuickPhrase,
    updateQuickPhrase,
    addPendingToolCall,
    approveToolCall,
    rejectToolCall,
    setToolCallStatus,
    registerTool,
    unregisterTool,
    getToolSchemas,
    executeToolCall,
    getEffectiveAgent,
  }
}, {
  persist: {
    pick: ['permissionLevel', 'permissionConfig', 'quickPhrases', 'agents', 'currentAgentId'] as const
  }
})
