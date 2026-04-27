/**
 * Tool Executor — Hermes-style tool system
 *
 * Bridges AI model tool_call responses to Rust backend execution.
 * Handles permission checks, execution, result feedback, and
 * system prompt assembly following Hermes Agent architecture.
 */
import { tauriInvoke } from '@/utils/tauri-api'
import type { PermissionConfig, PermissionLevel } from '@/stores/agent'

export interface ToolDefinition {
  type: 'function'
  function: {
    name: string
    description: string
    parameters: {
      type: 'object'
      properties: Record<string, { type: string; description: string }>
      required?: string[]
    }
  }
}

export interface ToolCallFromModel {
  id: string
  type: 'function'
  function: {
    name: string
    arguments: string
  }
}

export interface ToolExecutionResult {
  toolCallId: string
  toolName: string
  result: string
  success: boolean
  screenshot?: string
}

// ── Permission mapping ──

const TOOL_CATEGORY_MAP: Record<string, keyof PermissionConfig> = {
  terminal: 'shell',
  process: 'shell',
  read_file: 'file_read',
  write_file: 'file_write',
  patch: 'file_write',
  search_files: 'file_read',
  fetch_url: 'web_browse',
  web_extract: 'web_browse',
  web_search: 'web_browse',
  execute_code: 'code_execute',
  screenshot: 'screen_capture',
  mouse_click: 'mouse_control',
  mouse_move: 'mouse_control',
  mouse_double_click: 'mouse_control',
  mouse_drag: 'mouse_control',
  mouse_scroll: 'mouse_control',
  action_sequence: 'mouse_control',
  keyboard_type: 'keyboard_input',
  keyboard_key: 'keyboard_input',
  keyboard_hotkey: 'keyboard_input',
}

export function getToolCategory(toolName: string): keyof PermissionConfig | null {
  return TOOL_CATEGORY_MAP[toolName] ?? null
}

export function needsPermissionCheck(
  toolName: string,
  permLevel: PermissionLevel,
  permConfig: PermissionConfig
): 'allow' | 'ask' | 'deny' {
  if (permLevel === 'full') return 'allow'
  const cat = getToolCategory(toolName)
  if (!cat) return 'allow'
  return permConfig[cat]
}

export function isDangerousTool(toolName: string): boolean {
  const dangerous = new Set([
    'terminal', 'write_file', 'patch', 'keyboard_hotkey',
    'mouse_drag', 'execute_code', 'process',
  ])
  return dangerous.has(toolName)
}

const DANGEROUS_PATTERNS = [
  /\brm\s+(-[rf]+\s+)*\//i,
  /\brm\s+(-[rf]+\s+)*~/i,
  /\brm\s+-rf\b/i,
  /\bformat\s+[a-zA-Z]:/i,
  /\bdiskpart/i,
  /\bmkfs\b/i,
  /\bdd\s+.*of=/i,
  /\bdrop\s+(table|database|schema)\b/i,
  /\btruncate\s+table\b/i,
  /\bdelete\s+from\s+\w+\s*(;|$)/i,
  /\bgit\s+push.*--force\b/i,
  /\bgit\s+reset\s+--hard\b/i,
  /\b(shutdown|reboot|halt|poweroff)\b/i,
  /\breg\s+(delete|add)\b/i,
  /\bchmod\s+777\b/i,
  /\bcurl\s+.*\|\s*(ba)?sh\b/i,
  /\bwget\s+.*\|\s*(ba)?sh\b/i,
  /\bnew-item\s+.*-force\b/i,
  /\bremove-item\s+.*-recurse\b/i,
  /\bstop-process\b/i,
  /\bkill\s+-9\b/i,
  /\bkillall\b/i,
  /\b:>\s*\//,
  />\s*\/dev\/sd/i,
]

export function detectDangerousCommand(command: string): string | null {
  for (const pat of DANGEROUS_PATTERNS) {
    if (pat.test(command)) {
      return `匹配危险模式: ${pat.source}`
    }
  }
  return null
}

const PARALLELIZABLE_TOOLS = new Set([
  'read_file', 'search_files', 'fetch_url', 'web_extract',
  'web_search', 'skills_list', 'skill_view', 'session_search',
  'vision_analyze',
])

export function isParallelizableTool(name: string): boolean {
  return PARALLELIZABLE_TOOLS.has(name)
}

export function parseToolArgs(argsStr: string): Record<string, unknown> {
  try {
    return JSON.parse(argsStr)
  } catch {
    return {}
  }
}

// ── Frontend-intercepted tools (handled in ChatPage, not sent to Rust) ──

const FRONTEND_TOOLS = new Set([
  'skills_list', 'skill_view', 'skill_manage',
  'todo', 'vision_analyze', 'web_search', 'memory', 'session_search',
])

export function isFrontendTool(name: string): boolean {
  return FRONTEND_TOOLS.has(name)
}

// ── Tool execution via Rust backend ──

export async function executeTool(
  toolName: string,
  args: Record<string, unknown>
): Promise<ToolExecutionResult> {
  const callId = `tc-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
  try {
    const result = await tauriInvoke<{ success: boolean; result: string }>('execute_chat_tool', {
      name: toolName,
      arguments: JSON.stringify(args),
      toolCallId: callId,
      messageId: callId,
    })
    return {
      toolCallId: callId,
      toolName,
      result: result?.result ?? 'No result',
      success: result?.success ?? false,
      screenshot: toolName === 'screenshot' ? result?.result : undefined,
    }
  } catch (err) {
    return {
      toolCallId: callId,
      toolName,
      result: `Execution error: ${String(err)}`,
      success: false,
    }
  }
}

// ── System prompt assembly (Hermes-style layered) ──

const TOOL_USE_ENFORCEMENT = `# Tool-use enforcement
You MUST use your tools to take action. Do not describe what you would do — execute it immediately with tool calls.
Keep working until the task is actually complete. Every response should either (a) contain tool calls that make progress, or (b) deliver a final result.
Responses that only describe intentions without acting are not acceptable.`

const SKILLS_GUIDANCE = `# Self-evolution & Persistent Learning

## Memory System (MANDATORY)
- Use memory(action='add') to persist knowledge: user preferences, learned facts, useful techniques, corrections.
- Use memory(action='replace', old_content='...', content='...') to update existing memories.
- Use memory(action='remove', content='...') to delete outdated memories.
- Use memory(target='user') to update the user profile.
- When the user tells you about themselves (name, preferences, habits), IMMEDIATELY save it with memory.
- When you discover something new (a working solution, an API quirk, a system configuration), save it as an insight.
- When you make a mistake and correct it, save the correction so you never repeat it.

## Skill Creation (MANDATORY after complex tasks)
- After completing a task that required 3+ tool calls, save it as a reusable skill with skill_manage(action='create').
- A good skill includes: clear name, description of WHEN to use it, step-by-step instructions, and example triggers.
- Before starting complex tasks, check skills_list for existing solutions. Use skill_view to load full instructions.
- If a skill is outdated or incomplete after use, fix it immediately with skill_manage(action='patch').
- Skills are your accumulated expertise — creating them is how you improve over time.

## When to save memory vs create skill
- memory: facts, preferences, corrections, one-off insights (e.g., "user prefers dark theme", "this API requires header X")
- skill_manage: repeatable workflows and procedures (e.g., "how to deploy to production", "how to analyze images on desktop")`

const EXECUTION_WORKFLOW = `# 桌面控制工作流程

## ⚠️ 核心原则：用户要求的操作方式，你必须照做！
当用户说"双击XX"、"点击XX"、"打开桌面上的XX"，你必须：
1. 先 screenshot() 看到屏幕
2. 从截图分析中找到目标元素的中心坐标
3. 用 mouse_double_click 或 mouse_click 在该坐标上操作
4. 再 screenshot() 验证

绝对禁止：用 terminal 或 open_application 替代用户明确要求的桌面图形操作

## 操作步骤
1. screenshot() → 获取屏幕分析（所有元素的中心坐标）
2. 在分析结果中找到目标元素
3. 使用分析给出的中心坐标执行鼠标操作
4. screenshot() → 验证操作结果
5. 如果点击没有效果，可能坐标有小误差，微调±15像素后重试
6. 重复直到完成

## 关于坐标
- 截图分析返回的坐标是元素【可点击区域的正中心】
- 坐标是物理像素值，直接传给 mouse_click/mouse_double_click
- 绝不猜测坐标，必须来自截图分析
- 桌面图标的中心坐标：x 通常在 40-55（第一列），行间距约 85-100

## 工具列表
### 鼠标（用户要求桌面操作时必须用）
- mouse_click(x, y) — 单击
- mouse_double_click(x, y) — 双击打开图标/文件
- mouse_drag(from_x, from_y, to_x, to_y) — 拖拽
- mouse_scroll(x, y, direction, amount) — 滚动
- action_sequence(steps=[...]) — 多步连续操作

### 键盘
- keyboard_type(text="内容") — 输入文字
- keyboard_key(key="enter") — 按键
- keyboard_hotkey(keys=["ctrl","c"]) — 组合键

### 其他
- screenshot() — 截图+分析（操作前后必用）
- open_application(name) — 仅在用户没有要求特定操作方式时使用
- terminal(command) — 仅命令行任务，不替代图形操作
- read_file / write_file / search_files — 文件
- web_search / fetch_url — 网络
- task_complete(summary) — 完成`

const RULES = `# 核心规则
1. 先截图再操作：任何鼠标操作前必须先 screenshot()
2. 只用分析坐标：坐标必须来自截图分析结果，绝对不能猜测
3. 操作后验证：每次操作后 screenshot() 确认效果
4. 图形优先：用户说"双击/点击/打开桌面上的XX"，必须用鼠标操作，禁止用 terminal/open_application 替代
5. 不调 vision_analyze：截图已含分析
6. 坐标是中心点：分析坐标就是元素可点击区域的正中心，直接用
7. 输入前点击：先点击输入框再输入文字
8. 失败重试：点击没反应时，坐标可能有小误差，尝试微调±15像素后重新点击`

export interface SystemPromptOptions {
  personality?: string
  memory?: string
  skillsIndex?: string
  dateTime?: string
  environmentHints?: string
}

export function buildAgentSystemPrompt(opts: SystemPromptOptions = {}): string {
  const parts: string[] = []

  // Layer 1: Agent identity
  if (opts.personality) {
    parts.push(opts.personality)
  } else {
    parts.push('你是一个桌面自动化AI代理，可以通过鼠标和键盘操控用户的电脑。当用户要求你执行桌面操作（如点击、双击、拖拽、打开应用），你必须优先使用鼠标操作来完成，而不是用命令行替代。你需要先截图看到屏幕内容，然后根据分析结果精确操作。')
  }

  // Layer 2: Tool-use enforcement
  parts.push(TOOL_USE_ENFORCEMENT)

  // Layer 3: Execution workflow
  parts.push(EXECUTION_WORKFLOW)

  // Layer 4: Memory snapshot
  if (opts.memory) {
    parts.push(`# Memory\n${opts.memory}`)
  }

  // Layer 5: Skills index
  if (opts.skillsIndex) {
    parts.push(`# Available Skills\nBefore replying, scan the skills below. If one matches the task, use skill_view to load its instructions.\n${opts.skillsIndex}`)
  }

  // Layer 6: Skills guidance
  parts.push(SKILLS_GUIDANCE)

  // Layer 7: Date & time
  const dt = opts.dateTime || new Date().toLocaleString()
  parts.push(`# Current date & time\n${dt}`)

  // Layer 8: Environment hints
  if (opts.environmentHints) {
    parts.push(`# Environment\n${opts.environmentHints}`)
  }

  // Layer 9: Rules
  parts.push(RULES)

  return parts.filter(p => p.trim()).join('\n\n')
}

// Legacy alias for backward compatibility
export function buildComputerUseSystemPrompt(_hasVisionModel = false): string {
  return buildAgentSystemPrompt()
}

export function formatToolResultForChat(result: ToolExecutionResult): string {
  if (result.screenshot) {
    return `[Tool: ${result.toolName}] Screenshot captured.`
  }
  const status = result.success ? 'Success' : 'Error'
  const preview = result.result.length > 500
    ? result.result.slice(0, 500) + '...[truncated]'
    : result.result
  return `[Tool: ${result.toolName}] ${status}: ${preview}`
}

export function extractToolCallsFromContent(content: string): ToolCallFromModel[] {
  const calls: ToolCallFromModel[] = []
  const patterns = [
    /```tool_call\s*\n([\s\S]*?)```/g,
    /<tool_call>([\s\S]*?)<\/tool_call>/g,
    /\{"name"\s*:\s*"(\w+)"\s*,\s*"arguments"\s*:\s*(\{[\s\S]*?\})\s*\}/g
  ]

  for (const pat of patterns) {
    let m: RegExpExecArray | null
    while ((m = pat.exec(content)) !== null) {
      try {
        let parsed: { name?: string; arguments?: unknown }
        if (pat.source.includes('name')) {
          parsed = { name: m[1], arguments: JSON.parse(m[2]) }
        } else {
          parsed = JSON.parse(m[1])
        }
        if (parsed.name) {
          calls.push({
            id: `extracted-${Date.now()}-${calls.length}`,
            type: 'function',
            function: {
              name: parsed.name,
              arguments: typeof parsed.arguments === 'string'
                ? parsed.arguments
                : JSON.stringify(parsed.arguments || {})
            }
          })
        }
      } catch { /* skip malformed */ }
    }
  }

  return calls
}
