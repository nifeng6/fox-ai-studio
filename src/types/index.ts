export interface ModelSettings {
  supportsVision?: boolean
}

export interface Provider {
  id: string
  name: string
  channelType: number
  apiKey: string
  apiEndpoint: string
  models: string[]
  enabled: boolean
  createdAt: number
  updatedAt: number
  supportsVision?: boolean
  modelSettings?: Record<string, ModelSettings>
}

export interface Assistant {
  id: string
  name: string
  description: string
  avatar: string
  systemPrompt: string
  providerId: string
  modelId: string
  temperature: number
  maxTokens: number
  topP: number
  category: string
  tags: string[]
  isPreset: boolean
  createdAt: number
  updatedAt: number
  /** Bound skill IDs */
  skillIds?: string[]
  /** Example prompts/conversations */
  examples?: string[]
}

export interface Topic {
  id: string
  title: string
  assistantId?: string
  providerId: string
  modelId: string
  messageCount: number
  pinned: boolean
  createdAt: number
  updatedAt: number
  isGroupChat?: boolean
  participantAgentIds?: string[]
  /** Currently active agent tab: 'group' for group chat, or an agent ID for 1-on-1 */
  activeTab?: string
  /** Bound channel ID for this conversation */
  channelId?: string | null
  /** Per-topic agent config snapshots keyed by agent ID */
  agentSnapshots?: Record<string, AgentConfig>
}

export interface Message {
  id: string
  topicId: string
  role: 'system' | 'user' | 'assistant' | 'tool'
  content: string
  model?: string
  provider?: string
  tokens?: number
  status: 'pending' | 'streaming' | 'complete' | 'error'
  error?: string
  attachments?: Attachment[]
  parentId?: string
  createdAt: number
  agentId?: string
  agentName?: string
  mentions?: string[]
  parts?: MessagePart[]
  /** For tool role messages: the tool_call_id this result belongs to */
  toolCallId?: string
  /** For assistant messages: the raw tool_calls made in this response */
  toolCallsRaw?: Array<{ id: string; type: string; function: { name: string; arguments: string } }>
}

export type MessagePartType = 'text' | 'tool_call' | 'tool_result' | 'reasoning' | 'file' | 'step' | 'error'

export interface MessagePart {
  id: string
  type: MessagePartType
  content?: string
  // tool_call fields
  toolName?: string
  toolArgs?: Record<string, unknown> | string
  toolCallId?: string
  toolStatus?: 'pending' | 'approved' | 'rejected' | 'running' | 'completed' | 'done' | 'error'
  toolResult?: string
  // file fields
  fileName?: string
  filePath?: string
  language?: string
  // step fields
  stepTitle?: string
  stepStatus?: 'running' | 'completed' | 'error'
  // timing
  startedAt?: number
  completedAt?: number
}

export type PermissionAction = 'once' | 'always' | 'reject'
export type PermissionCategory = 'shell' | 'file_read' | 'file_write' | 'web_browse' | 'code_execute' | 'screen_capture' | 'mouse_control' | 'keyboard_input' | 'app_launch'

export interface ContentPart {
  type: 'text' | 'image_url' | 'image'
  text?: string
  image_url?: { url: string; detail?: string }
  source?: { type: string; media_type: string; data: string }
}

export type ComputerUsePermissionMode = 'supervised' | 'semi-auto' | 'full-auto'

export interface ComputerUseSession {
  sessionId: string
  goal: string
  running: boolean
  currentStep: number
  maxSteps: number
  permissionMode: ComputerUsePermissionMode
  steps: ComputerUseStep[]
}

export interface ComputerUseStep {
  step: number
  screenshotBase64: string
  actionDescription: string
  toolCalls: ComputerUseToolCall[]
  status: 'pending' | 'executing' | 'completed' | 'error'
  timestamp: number
}

export interface ComputerUseToolCall {
  name: string
  arguments: Record<string, unknown>
}

export interface ComputerUseActionRequest {
  sessionId: string
  step: number
  action: ComputerUseToolCall
  needsApproval: boolean
}

export interface WindowInfo {
  id: number
  name: string
  x: number
  y: number
  width: number
  height: number
  isMinimized: boolean
}

export interface ScreenSize {
  width: number
  height: number
}

export interface CursorPos {
  x: number
  y: number
}

export interface PermissionRequest {
  id: string
  category: PermissionCategory
  toolName: string
  description: string
  args?: Record<string, unknown>
  status: 'pending' | 'allowed' | 'rejected'
  action?: PermissionAction
  createdAt: number
}

export interface Attachment {
  id: string
  name: string
  type: string
  size: number
  url: string
}

export interface KnowledgeBase {
  id: string
  name: string
  description: string
  documentCount: number
  createdAt: number
  updatedAt: number
}

export interface Memory {
  id: string
  content: string
  source: string
  tags: string[]
  importance: number
  createdAt: number
  updatedAt: number
}

export interface MCPServer {
  id: string
  name: string
  command: string
  args: string[]
  env?: Record<string, string>
  enabled: boolean
  status: 'stopped' | 'running' | 'error'
  tools: MCPTool[]
  createdAt: number
}

export interface MCPTool {
  name: string
  description: string
  inputSchema: any
}

export interface Note {
  id: string
  title: string
  content: string
  category: string
  folderId?: string
  pinned: boolean
  starred: boolean
  createdAt: number
  updatedAt: number
}

export interface NoteFolder {
  id: string
  name: string
  parentId?: string
  createdAt: number
}

export interface MiniProgram {
  id: string
  name: string
  url: string
  icon: string
  sortOrder: number
}

export type ChannelPlatform = 'feishu' | 'dingtalk' | 'telegram' | 'discord' | 'slack' | 'webhook'

export interface Channel {
  id: string
  platformId: ChannelPlatform
  name: string
  webhookUrl: string
  secret?: string
  enabled: boolean
  createdAt: number
  /** When true, sends a notification when AI completes a reply */
  notifyOnReply: boolean
  /** Custom message template. Use {content} for AI reply, {model} for model name, {topic} for topic title */
  messageTemplate?: string
}

export interface ShortcutBinding {
  id: string
  label: string
  keys: string
  enabled: boolean
  isGlobal: boolean
}

export type ThemeMode = 'light' | 'dark' | 'system'

export interface ModelSlot {
  providerId: string
  modelId: string
}

export interface Personality {
  id: string
  name: string
  avatar: string
  description: string
  systemPrompt: string
  isDefault: boolean
  createdAt: number
}

export type MemoryEntryCategory = 'fact' | 'preference' | 'instruction' | 'context'
export type MemoryEntrySource = 'user' | 'agent'

export interface MemoryEntry {
  id: string
  content: string
  category: MemoryEntryCategory
  source: MemoryEntrySource
  createdAt: number
  updatedAt: number
}

export interface UserProfile {
  name: string
  preferences: Record<string, string>
  notes: string
}

export interface Skill {
  id: string
  name: string
  description: string
  trigger: string
  instructions: string
  enabled: boolean
  usageCount: number
  lastUsed: number | null
  createdAt: number
  version?: string
  category?: string
  filePath?: string
}

export type AgentState =
  | 'idle'
  | 'thinking'
  | 'awaiting_tool_approval'
  | 'tool_running'
  | 'streaming'
  | 'paused'
  | 'error'
  | 'completed'

export interface AgentConfig {
  id: string
  name: string
  type: 'main' | 'sub'
  parentId: string | null
  systemPrompt: string
  languageModel: ModelSlot
  visionModel: ModelSlot | null
  toolModel: ModelSlot | null
  tools: string[]
  maxIterations: number
  delegateDepth: number
  status: AgentState
  createdAt: number
  /** Hermes-style persona (SOUL.md equivalent) */
  personalityId: string | null
  memoryEnabled: boolean
  skillIds: string[]
  sessionSearchEnabled: boolean
  /** Bound channel ID for this agent */
  channelId?: string | null
  /** Hermes-style auxiliary task routing config */
  auxiliaryConfig?: {
    vision?: AuxiliaryTaskConfig
    webExtract?: AuxiliaryTaskConfig
    compression?: AuxiliaryTaskConfig
  }
}

export interface AuxiliaryTaskConfig {
  provider: string
  model: string
  baseUrl?: string
  apiKey?: string
  timeout?: number
}
