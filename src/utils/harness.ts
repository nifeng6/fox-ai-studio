/**
 * Harness Engineering Layer
 *
 * Enhances instruction-following for all models (especially smaller ones)
 * through structured constraints, progressive context, and feedback loops.
 *
 * Based on: OpenAI Codex harness report (Feb 2026), HumanLayer R.P.I.,
 * LangChain Terminal-Bench improvements, grammar-constrained generation concepts.
 */

export interface HarnessConfig {
  maxSystemTokens: number
  maxToolTokens: number
  maxHistoryRatio: number
  reserveRatio: number
  enablePlanningGate: boolean
  enableOutputConstraints: boolean
  enableFeedbackLoop: boolean
  maxRetries: number
}

export const DEFAULT_HARNESS_CONFIG: HarnessConfig = {
  maxSystemTokens: 512,
  maxToolTokens: 256,
  maxHistoryRatio: 0.65,
  reserveRatio: 0.2,
  enablePlanningGate: true,
  enableOutputConstraints: true,
  enableFeedbackLoop: true,
  maxRetries: 2
}

interface ChatMsg {
  role: string
  content: string
}

function estimateTokens(text: string): number {
  return Math.ceil(text.length / 3.5)
}

function truncateToTokens(text: string, maxTokens: number): string {
  const maxChars = Math.floor(maxTokens * 3.5)
  if (text.length <= maxChars) return text
  return text.slice(0, maxChars) + '\n...[truncated]'
}

/**
 * Core harness: wraps system messages with structured constraints
 * that dramatically improve instruction following for weaker models.
 */
export function applyHarnessConstraints(
  systemMessages: ChatMsg[],
  userText: string,
  config: HarnessConfig = DEFAULT_HARNESS_CONFIG
): ChatMsg[] {
  const result: ChatMsg[] = []

  const structuredPreamble = buildStructuredPreamble()
  result.push({ role: 'system', content: structuredPreamble })

  for (const msg of systemMessages) {
    const truncated = truncateToTokens(msg.content, config.maxSystemTokens)
    result.push({ role: msg.role, content: truncated })
  }

  if (config.enableOutputConstraints) {
    result.push({
      role: 'system',
      content: buildOutputConstraints(userText)
    })
  }

  return result
}

function buildStructuredPreamble(): string {
  return `<HARNESS_RULES>
你是一个精确遵循指令的AI助手。请严格遵守以下规则：

1. 【精确性】只回答用户问的问题，不添加无关内容
2. 【格式遵循】如果用户要求特定格式（JSON/列表/代码等），严格按该格式输出
3. 【拒绝幻觉】如果你不确定，明确说"我不确定"而不是编造
4. 【步骤化】对于复杂任务，先列出步骤再执行
5. 【自我验证】回答后检查是否完全满足了用户的每一个要求
6. 【语言匹配】用户使用什么语言提问，就用什么语言回答
</HARNESS_RULES>`
}

function buildOutputConstraints(userText: string): string {
  const hints: string[] = []

  if (userText.match(/json|JSON|格式/i)) {
    hints.push('用户可能需要结构化输出。如果是JSON请输出有效的JSON格式。')
  }

  if (userText.match(/代码|code|function|函数|class|类/i)) {
    hints.push('代码请放在```代码块中，标明语言类型。')
  }

  if (userText.match(/翻译|translate/i)) {
    hints.push('翻译任务请直接给出翻译结果，不需要解释原文。')
  }

  if (userText.match(/总结|summarize|summary/i)) {
    hints.push('总结请用要点列表形式，控制在3-5个要点。')
  }

  if (userText.match(/对比|compare|vs/i)) {
    hints.push('对比请使用表格或并列形式，清晰展示差异。')
  }

  if (userText.match(/步骤|step|how to|怎么|如何/i)) {
    hints.push('请用编号步骤形式回答，每步简洁明了。')
  }

  if (!hints.length) return ''
  return `<OUTPUT_HINTS>\n${hints.join('\n')}\n</OUTPUT_HINTS>`
}

/**
 * Manages context budget - ensures total context doesn't exceed model limits.
 * Key insight from harness engineering: most failures come from context overflow.
 */
export function budgetContext(
  systemMsgs: ChatMsg[],
  historyMsgs: ChatMsg[],
  contextWindow: number = 128000,
  config: HarnessConfig = DEFAULT_HARNESS_CONFIG
): { system: ChatMsg[]; history: ChatMsg[] } {
  const reserveTokens = Math.floor(contextWindow * config.reserveRatio)
  const availableTokens = contextWindow - reserveTokens

  let systemTokens = 0
  const budgetedSystem: ChatMsg[] = []
  for (const msg of systemMsgs) {
    const t = estimateTokens(msg.content)
    if (systemTokens + t > config.maxSystemTokens * 4) {
      budgetedSystem.push({
        role: msg.role,
        content: truncateToTokens(msg.content, config.maxSystemTokens)
      })
    } else {
      budgetedSystem.push(msg)
    }
    systemTokens += Math.min(t, config.maxSystemTokens)
  }

  const historyBudget = Math.floor(availableTokens * config.maxHistoryRatio)
  let historyTokens = 0
  const budgetedHistory: ChatMsg[] = []

  for (let i = historyMsgs.length - 1; i >= 0; i--) {
    const msg = historyMsgs[i]
    const t = estimateTokens(msg.content)
    if (historyTokens + t > historyBudget) break
    budgetedHistory.unshift(msg)
    historyTokens += t
  }

  return { system: budgetedSystem, history: budgetedHistory }
}

/**
 * Planning gate: for complex tasks, forces the model to plan before executing.
 * This is the single most impactful harness technique per research.
 */
export function shouldUsePlanningGate(userText: string): boolean {
  const complexIndicators = [
    /\b(实现|重构|创建|设计|开发|构建)\b/,
    /\b(implement|refactor|create|design|develop|build)\b/i,
    /\b(步骤|方案|计划|架构)\b/,
    /\b(step|plan|architecture|workflow)\b/i,
    /.{200,}/
  ]
  return complexIndicators.some(r => r.test(userText))
}

export function buildPlanningPrompt(userText: string): string {
  return `<PLANNING_GATE>
在执行之前，请先按以下格式输出计划：

## 分析
- 用户需求的核心是什么？
- 涉及哪些关键步骤？

## 执行计划
1. [步骤1]
2. [步骤2]
...

## 验证方式
- 如何确认每步完成？

现在开始执行计划。
</PLANNING_GATE>

用户请求: ${userText}`
}

/**
 * Feedback loop: when a previous response failed or was unsatisfactory,
 * inject error context to help the model self-correct.
 */
export function buildFeedbackContext(
  previousResponse: string,
  errorDescription: string
): ChatMsg {
  return {
    role: 'system',
    content: `<FEEDBACK_LOOP>
上一次回答存在问题：${errorDescription}

上一次回答（部分）：
${previousResponse.slice(0, 500)}

请纠正以上问题，重新生成回答。注意避免重复同样的错误。
</FEEDBACK_LOOP>`
  }
}

/**
 * Tool definition compression: for small models, keep tool schemas minimal.
 * Research shows max 4 tools per turn, 256 tokens per tool definition.
 */
export function compressToolSchemas(
  tools: Array<{ name: string; description: string; parameters?: Record<string, any> }>,
  maxTools: number = 4
): string {
  const selected = tools.slice(0, maxTools)
  const lines = selected.map(t => {
    const params = t.parameters
      ? Object.entries(t.parameters)
          .map(([k, v]) => `${k}:${(v as any).type || 'string'}`)
          .join(', ')
      : ''
    return `- ${t.name}(${params}): ${t.description.slice(0, 80)}`
  })
  return lines.join('\n')
}

/**
 * Progressive disclosure: only include context categories relevant to the query.
 * Prevents the "dumb zone" where too many instructions degrade performance.
 */
export function selectRelevantContext(
  userText: string,
  availableContexts: Array<{ category: string; content: string; keywords: string[] }>
): string[] {
  const q = userText.toLowerCase()
  return availableContexts
    .filter(ctx => {
      if (ctx.keywords.some(k => q.includes(k.toLowerCase()))) return true
      if (ctx.content.length < 200) return true
      return false
    })
    .map(ctx => ctx.content)
}

/**
 * Multi-role review: for important outputs, simulate sequential expert review.
 * E.g., UX designer → architect → devil's advocate.
 */
export function buildMultiRoleReviewPrompt(task: string): string {
  return `请从以下三个角色依次审视你的回答：

1. **用户体验专家**：这个回答对用户是否友好、清晰？
2. **技术架构师**：技术方案是否合理、可扩展？
3. **质疑者**：有哪些潜在问题或遗漏？

任务: ${task}

请综合三个角色的观点给出最终回答。`
}


// ============================================================
// Computer Use Harness — 专用约束层
// ============================================================

export interface ComputerUseHarnessConfig {
  maxStepsPerSession: number
  maxActionsPerSecond: number
  screenshotHistoryLimit: number
  maxToolsPerTurn: number
  forbiddenPatterns: string[]
  dangerousKeywords: string[]
  coordinateBoundsCheck: boolean
  screenWidth: number
  screenHeight: number
}

export const DEFAULT_COMPUTER_USE_HARNESS: ComputerUseHarnessConfig = {
  maxStepsPerSession: 50,
  maxActionsPerSecond: 2,
  screenshotHistoryLimit: 3,
  maxToolsPerTurn: 8,
  forbiddenPatterns: [
    'password', 'passwd', 'credential', 'bank', 'payment',
    'credit card', '密码', '支付', '银行', '信用卡',
    'System32', 'registry', '注册表', 'format', 'rmdir /s',
    'rm -rf', 'del /f /s /q'
  ],
  dangerousKeywords: [
    'admin', 'sudo', 'regedit', 'taskmgr', 'cmd.exe',
    'powershell', 'format', 'diskpart'
  ],
  coordinateBoundsCheck: true,
  screenWidth: 0,
  screenHeight: 0
}

export async function getActualScreenSize(): Promise<{ width: number; height: number; logicalWidth: number; logicalHeight: number; scaleFactor: number }> {
  try {
    const { desktopApi } = await import('@/utils/tauri-api')
    const info = await desktopApi.getScreenSize()
    return {
      width: info.physicalWidth || info.width,
      height: info.physicalHeight || info.height,
      logicalWidth: info.width,
      logicalHeight: info.height,
      scaleFactor: info.scaleFactor || 1,
    }
  } catch {
    return { width: 0, height: 0, logicalWidth: 0, logicalHeight: 0, scaleFactor: 1 }
  }
}

export function buildComputerUseSystemPrompt(
  goal: string,
  config: ComputerUseHarnessConfig = DEFAULT_COMPUTER_USE_HARNESS
): string {
  return `<COMPUTER_USE_HARNESS>
You are an AI agent that controls a computer through screenshots and input simulation.

COORDINATE SYSTEM:
- When you call screenshot, you receive an image — the tool result tells you the exact image dimensions
- Use coordinates as pixel positions IN THAT IMAGE (not screen coordinates)
- Origin (0,0) is the top-left corner of the image
- The system automatically scales your image coordinates to the actual screen — you do NOT need to do any scaling yourself
- All coordinates must be non-negative integers within the image dimensions

MANDATORY WORKFLOW (every step):
1. SCREENSHOT: Always take a screenshot first to see the current state
2. OBSERVE: Carefully examine the screenshot to identify all visible UI elements
3. LOCATE: Determine the precise pixel coordinates (x,y) of the target element's CENTER
4. EXECUTE: Call mouse_click(x=..., y=...) to click that position. For typing, use keyboard_type.
5. VERIFY: Take another screenshot to confirm the action worked

HOW TO CLICK (IMPORTANT):
- The screenshot result tells you the image dimensions (display_width x display_height)
- All coordinates must be within those dimensions
- To click an element: estimate its CENTER position in the screenshot, then call mouse_click(x=..., y=...)
- For small icons (~32px): aim for the exact center of the icon
- For buttons/text: aim for the center of the element
- Prefer keyboard shortcuts and app launcher over clicking small targets

SAFETY CONSTRAINTS:
- NEVER interact with: ${config.forbiddenPatterns.join(', ')}
- NEVER type passwords or sensitive information unless explicitly instructed
- If something goes wrong, take a new screenshot and re-assess

ERROR RECOVERY:
- If a click missed: the element may have moved. Take a NEW screenshot and re-locate
- If text didn't appear: click the target field first, then type
- After 3 failed attempts at the same action: report failure and suggest alternative

CURRENT GOAL: ${goal}
</COMPUTER_USE_HARNESS>`
}

export interface CoordinateValidation {
  valid: boolean
  reason?: string
  suggested?: { x: number; y: number }
}

export function validateCoordinates(
  x: number,
  y: number,
  _config: ComputerUseHarnessConfig = DEFAULT_COMPUTER_USE_HARNESS
): CoordinateValidation {
  if (!_config.coordinateBoundsCheck) return { valid: true }

  if (x < 0 || y < 0) {
    return {
      valid: false,
      reason: `Negative coordinates (${x}, ${y})`,
      suggested: { x: Math.max(0, x), y: Math.max(0, y) }
    }
  }

  return { valid: true }
}

export function checkActionSafety(
  actionName: string,
  args: Record<string, unknown>,
  config: ComputerUseHarnessConfig = DEFAULT_COMPUTER_USE_HARNESS
): { safe: boolean; reason?: string } {
  if (actionName === 'keyboard_type') {
    const text = (args.text as string) || ''
    const lower = text.toLowerCase()
    for (const pattern of config.forbiddenPatterns) {
      if (lower.includes(pattern.toLowerCase())) {
        return { safe: false, reason: `Text contains forbidden pattern: "${pattern}"` }
      }
    }
  }

  if (actionName === 'keyboard_hotkey') {
    const keys = (args.keys as string[]) || []
    const combo = keys.map(k => k.toLowerCase()).join('+')
    const destructive = ['alt+f4', 'ctrl+alt+delete', 'ctrl+shift+escape']
    if (destructive.includes(combo)) {
      return { safe: false, reason: `Potentially destructive hotkey: ${combo}` }
    }
  }

  if (['mouse_click', 'mouse_double_click', 'mouse_move', 'mouse_drag'].includes(actionName)) {
    const x = (args.x ?? args.from_x ?? 0) as number
    const y = (args.y ?? args.from_y ?? 0) as number
    const validation = validateCoordinates(x, y, config)
    if (!validation.valid) {
      return { safe: false, reason: validation.reason }
    }
  }

  return { safe: true }
}

let lastActionTimestamp = 0

export function checkRateLimit(
  config: ComputerUseHarnessConfig = DEFAULT_COMPUTER_USE_HARNESS
): { allowed: boolean; waitMs: number } {
  const now = Date.now()
  const minInterval = 1000 / config.maxActionsPerSecond
  const elapsed = now - lastActionTimestamp

  if (elapsed < minInterval) {
    return { allowed: false, waitMs: minInterval - elapsed }
  }

  lastActionTimestamp = now
  return { allowed: true, waitMs: 0 }
}

export function manageScreenshotHistory(
  history: Array<{ step: number; base64: string }>,
  config: ComputerUseHarnessConfig = DEFAULT_COMPUTER_USE_HARNESS
): Array<{ step: number; base64: string }> {
  if (history.length <= config.screenshotHistoryLimit) return history
  return history.slice(history.length - config.screenshotHistoryLimit)
}

export function buildComputerUseOutputConstraints(): string {
  return `<COMPUTER_USE_OUTPUT_RULES>
- Coordinates must be integers within the screenshot image dimensions
- Use the display_width_px and display_height_px from the screenshot tool result as bounds
- The system scales coordinates to actual screen resolution automatically
- Always take a screenshot before any mouse action to see the current state
</COMPUTER_USE_OUTPUT_RULES>`
}
