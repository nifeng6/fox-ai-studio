/**
 * Auxiliary Task Router — Hermes-style per-task model routing
 *
 * Runs a **separate, isolated** LLM call for side-tasks like vision analysis.
 * Uses a unique messageId and dedicated event listeners that are registered
 * BEFORE the request is sent, avoiding any race conditions with the main
 * agent stream.
 */
import type { AgentConfig } from '@/types'

export interface AuxiliaryTaskConfig {
  provider: string
  model: string
  baseUrl?: string
  apiKey?: string
  timeout?: number
}

export interface AuxiliaryCallOptions {
  task: string
  messages: any[]
  agent?: AgentConfig | null
  provider?: string
  model?: string
  timeout?: number
}

interface ResolvedRoute {
  providerId: string
  modelId: string
}

function resolveRoute(opts: AuxiliaryCallOptions): ResolvedRoute {
  const agent = opts.agent

  if (opts.provider && opts.model) {
    return { providerId: opts.provider, modelId: opts.model }
  }

  const auxCfg = (agent as any)?.auxiliaryConfig?.[opts.task] as AuxiliaryTaskConfig | undefined
  if (auxCfg?.provider && auxCfg.provider !== 'auto' && auxCfg.model) {
    return { providerId: auxCfg.provider, modelId: auxCfg.model }
  }

  if (opts.task === 'vision' && agent?.visionModel?.providerId && agent.visionModel.modelId) {
    return { providerId: agent.visionModel.providerId, modelId: agent.visionModel.modelId }
  }
  if (agent?.toolModel?.providerId && agent.toolModel.modelId) {
    return { providerId: agent.toolModel.providerId, modelId: agent.toolModel.modelId }
  }

  if (agent?.languageModel?.providerId && agent.languageModel.modelId) {
    return { providerId: agent.languageModel.providerId, modelId: agent.languageModel.modelId }
  }

  return { providerId: '', modelId: '' }
}

let auxCallCounter = 0

/**
 * Call an auxiliary LLM for a side task. This creates isolated event listeners
 * that are registered synchronously BEFORE the request fires, ensuring no
 * events are missed. The unique msgId prevents cross-talk with the main stream.
 */
export async function callAuxiliaryLLM(opts: AuxiliaryCallOptions): Promise<string> {
  const route = resolveRoute(opts)
  if (!route.providerId || !route.modelId) {
    throw new Error(`No model configured for auxiliary task: ${opts.task}. Please configure a vision model in agent settings.`)
  }

  const timeout = opts.timeout ?? 30000
  auxCallCounter++
  const msgId = `__aux_${opts.task}_${auxCallCounter}_${Date.now()}`
  let result = ''

  const { listen } = await import('@tauri-apps/api/event')

  // Register listeners SYNCHRONOUSLY before sending the request
  const chunkUnlisten = await listen<{ messageId: string; chunk: string }>('chat:stream-chunk', (ev) => {
    if (ev.payload.messageId === msgId) {
      result += ev.payload.chunk
    }
  })

  const endPromise = new Promise<string>((resolve) => {
    let endUnlisten: (() => void) | null = null
    let errUnlisten: (() => void) | null = null
    const timer = setTimeout(() => {
      endUnlisten?.()
      errUnlisten?.()
      resolve(result || `[${opts.task}: timed out after ${timeout / 1000}s]`)
    }, timeout)

    listen<{ messageId: string; hasToolCalls: boolean }>('chat:stream-end', (ev) => {
      if (ev.payload.messageId === msgId) {
        clearTimeout(timer)
        endUnlisten?.()
        errUnlisten?.()
        resolve(result)
      }
    }).then(fn => { endUnlisten = fn })

    listen<{ messageId: string; error: string }>('chat:stream-error', (ev) => {
      if (ev.payload.messageId === msgId) {
        clearTimeout(timer)
        endUnlisten?.()
        errUnlisten?.()
        resolve(result || `[${opts.task} error: ${ev.payload.error}]`)
      }
    }).then(fn => { errUnlisten = fn })
  })

  // Now send the request — listeners are already registered
  const { sendChatMessage } = await import('@/utils/tauri-api')
  try {
    await sendChatMessage({
      providerId: route.providerId,
      modelId: route.modelId,
      messageId: msgId,
      messages: opts.messages,
    })
  } catch (e: any) {
    chunkUnlisten()
    throw new Error(`Auxiliary ${opts.task} request failed: ${e?.message || e}`)
  }

  // Wait for the stream to complete
  const response = await endPromise
  chunkUnlisten()
  return response
}

/**
 * Analyze an image using the vision auxiliary route.
 * Handles both data URLs (base64) and file paths.
 */
export async function analyzeImageWithVision(
  imageUrl: string,
  question: string,
  agent?: AgentConfig | null
): Promise<string> {
  // If it's a local file path, read it as base64 first
  let finalUrl = imageUrl
  if (imageUrl && !imageUrl.startsWith('data:') && !imageUrl.startsWith('http')) {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const b64: string = await invoke('read_file_base64', { path: imageUrl })
      const ext = imageUrl.split('.').pop()?.toLowerCase() || 'png'
      const mime = ext === 'jpg' ? 'image/jpeg' : `image/${ext}`
      finalUrl = `data:${mime};base64,${b64}`
    } catch (e: any) {
      return `Failed to read image file: ${e?.message || e}`
    }
  }

  if (!finalUrl || finalUrl.length < 50) {
    return 'No valid image provided for analysis.'
  }

  const result = await callAuxiliaryLLM({
    task: 'vision',
    agent,
    timeout: 45000,
    messages: [{
      role: 'user',
      content: [
        { type: 'text', text: question || 'Describe what you see in this image in detail.' },
        { type: 'image_url', image_url: { url: finalUrl, detail: 'high' } },
      ]
    }],
  })

  return result || 'Vision analysis returned empty result.'
}

/**
 * Compress conversation history using an auxiliary LLM to create a summary.
 * Called when messages exceed a threshold to prevent context overflow.
 */
export async function compressContextWithLLM(
  messages: any[],
  agent?: AgentConfig | null,
): Promise<string> {
  const textParts: string[] = []
  for (const m of messages) {
    const content = typeof m.content === 'string' ? m.content : JSON.stringify(m.content)
    const truncated = content.length > 500 ? content.slice(0, 500) + '...' : content
    textParts.push(`[${m.role}]: ${truncated}`)
  }
  const conversationText = textParts.join('\n')

  const result = await callAuxiliaryLLM({
    task: 'compress',
    agent,
    timeout: 30000,
    messages: [{
      role: 'user',
      content: `Summarize the following conversation history concisely. Preserve all important: user goals, decisions made, key findings, tool results, and action items. Keep technical details like file paths, commands, and error messages. Output a structured summary in 200-400 words:\n\n${conversationText}`,
    }],
  })

  return result || '[Context compression failed]'
}
