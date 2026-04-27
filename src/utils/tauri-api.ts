import type { UnlistenFn } from '@tauri-apps/api/event'

const isTauri = !!(window as any).__TAURI_INTERNALS__

async function getInvoke() {
  if (!isTauri) return null
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke
}

async function getListen() {
  if (!isTauri) return null
  const { listen } = await import('@tauri-apps/api/event')
  return listen
}

async function getWindow() {
  if (!isTauri) return null
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  return getCurrentWindow()
}

export async function tauriInvoke<T = any>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const inv = await getInvoke()
  if (!inv) {
    console.warn(`[tauri-mock] ${cmd} called outside Tauri`)
    return undefined as T
  }
  try {
    return await inv<T>(cmd, args)
  } catch (e) {
    console.error(`[tauri-invoke] ${cmd} failed:`, e)
    throw e
  }
}

const noop = () => {}

export const windowApi = {
  minimize: async () => {
    try { const w = await getWindow(); await w?.minimize() }
    catch (e) { console.warn('[windowApi] minimize failed:', e) }
  },
  maximize: async () => {
    try { const w = await getWindow(); await w?.toggleMaximize() }
    catch (e) { console.warn('[windowApi] toggleMaximize failed:', e) }
  },
  close: async () => {
    try { const w = await getWindow(); await w?.close() }
    catch (e) { console.warn('[windowApi] close failed:', e) }
  },
  isMaximized: async () => {
    try { const w = await getWindow(); return w ? await w.isMaximized() : false }
    catch { return false }
  },
  onMaximizedChanged: async (cb: (maximized: boolean) => void): Promise<UnlistenFn> => {
    try {
      const w = await getWindow()
      if (!w) return noop
      return w.onResized(async () => { cb(await w.isMaximized()) })
    } catch { return noop }
  },
  startDragging: async () => {
    try { const w = await getWindow(); await w?.startDragging() }
    catch (e) { console.warn('[windowApi] startDragging failed:', e) }
  }
}

export async function sendChatMessage(args: {
  providerId: string
  modelId: string
  messages: Array<{ role: string; content: any; toolCallId?: string; toolCalls?: any[] }>
  messageId: string
  options?: Record<string, any>
}): Promise<void> {
  return tauriInvoke('send_chat_message', args)
}

export async function executeChatTool(args: {
  name: string
  arguments: Record<string, any>
  toolCallId: string
  messageId: string
}): Promise<{ toolCallId: string; name: string; result: string; success: boolean }> {
  return tauriInvoke('execute_chat_tool', args)
}

export async function abortChat(messageId: string): Promise<void> {
  return tauriInvoke('abort_chat', { messageId })
}

export async function onStreamChunk(cb: (payload: { messageId: string; chunk: string }) => void): Promise<UnlistenFn> {
  const l = await getListen()
  if (!l) return noop
  return l<{ messageId: string; chunk: string }>('chat:stream-chunk', (e) => cb(e.payload))
}

export async function onStreamThinking(cb: (payload: { messageId: string; chunk: string }) => void): Promise<UnlistenFn> {
  const l = await getListen()
  if (!l) return noop
  return l<{ messageId: string; chunk: string }>('chat:stream-thinking', (e) => cb(e.payload))
}

export async function onStreamToolCall(cb: (payload: { messageId: string; toolCallId: string; name: string; arguments: string }) => void): Promise<UnlistenFn> {
  const l = await getListen()
  if (!l) return noop
  return l<{ messageId: string; toolCallId: string; name: string; arguments: string }>('chat:stream-tool-call', (e) => cb(e.payload))
}

export async function onStreamEnd(cb: (payload: { messageId: string; hasToolCalls: boolean }) => void): Promise<UnlistenFn> {
  const l = await getListen()
  if (!l) return noop
  return l<{ messageId: string; hasToolCalls: boolean }>('chat:stream-end', (e) => cb(e.payload))
}

export async function onStreamError(cb: (payload: { messageId: string; error: string }) => void): Promise<UnlistenFn> {
  const l = await getListen()
  if (!l) return noop
  return l<{ messageId: string; error: string }>('chat:stream-error', (e) => cb(e.payload))
}

export async function onToolResult(cb: (payload: { messageId: string; toolCallId: string; name: string; result: string; success: boolean }) => void): Promise<UnlistenFn> {
  const l = await getListen()
  if (!l) return noop
  return l<any>('chat:tool-result', (e) => cb(e.payload))
}

export const providerApi = {
  getProviders: () => tauriInvoke<any[]>('get_providers').then(r => r || []),
  addProvider: (provider: any) => tauriInvoke('add_provider', { provider }),
  updateProvider: (id: string, data: any) => tauriInvoke('update_provider', { id, data }),
  removeProvider: (id: string) => tauriInvoke('remove_provider', { id }),
  testConnection: (id: string) => tauriInvoke<{ success: boolean; message: string }>('test_connection', { id }),
  getModels: (id: string) => tauriInvoke<any[]>('get_models', { id }).then(r => r || [])
}

export const knowledgeApi = {
  getBases: () => tauriInvoke<any[]>('get_knowledge_bases').then(r => r || []),
  createBase: (data: any) => tauriInvoke('create_knowledge_base', { data }),
  deleteBase: (id: string) => tauriInvoke('delete_knowledge_base', { id }),
  addDocument: (baseId: string, doc: any) => tauriInvoke('add_knowledge_document', { baseId, doc }),
  search: (baseId: string, query: string, topK: number) => tauriInvoke<any[]>('search_knowledge', { baseId, query, topK }).then(r => r || [])
}

export const mcpApi = {
  getServers: () => tauriInvoke<any[]>('get_mcp_servers').then(r => r || []),
  addServer: (server: any) => tauriInvoke('add_mcp_server', { server }),
  removeServer: (id: string) => tauriInvoke('remove_mcp_server', { id }),
  startServer: (id: string) => tauriInvoke<{ success: boolean; message: string }>('start_mcp_server', { id }),
  stopServer: (id: string) => tauriInvoke('stop_mcp_server', { id }),
  getTools: (id: string) => tauriInvoke<any[]>('get_mcp_tools', { id }).then(r => r || []),
  callTool: (serverId: string, toolName: string, args: any) => tauriInvoke('call_mcp_tool', { serverId, toolName, args })
}

export const fileApi = {
  openDialog: (options?: any) => tauriInvoke<{ canceled: boolean; filePaths: string[] }>('open_file_dialog', { options }).then(r => r || { canceled: true, filePaths: [] }),
  saveDialog: (options?: any) => tauriInvoke<{ canceled: boolean; filePath: string }>('save_file_dialog', { options }).then(r => r || { canceled: true, filePath: '' }),
  readFile: (path: string) => tauriInvoke<string>('read_file', { path }).then(r => r || ''),
  writeFile: (path: string, content: string) => tauriInvoke('write_file', { path, content }),
  getAppDataPath: () => tauriInvoke<string>('get_app_data_path').then(r => r || '')
}

export interface ApiServerStatus {
  running: boolean
  port: number
  url: string
}

export const desktopApi = {
  captureScreen: (monitorIndex?: number) =>
    tauriInvoke<string>('capture_screen', { monitorIndex: monitorIndex ?? null }),
  captureWindow: (windowId: number) =>
    tauriInvoke<string>('capture_window', { windowId }),
  listWindows: () =>
    tauriInvoke<any[]>('list_windows').then(r => r || []),
  getScreenSize: (monitorIndex?: number) =>
    tauriInvoke<{ width: number; height: number; scaleFactor: number; physicalWidth: number; physicalHeight: number }>('get_screen_size', { monitorIndex: monitorIndex ?? null }),
  getCursorPosition: () =>
    tauriInvoke<{ x: number; y: number }>('get_cursor_position'),
}

export const inputApi = {
  mouseMove: (x: number, y: number) =>
    tauriInvoke('mouse_move', { x, y }),
  mouseClick: (x: number, y: number, button?: string) =>
    tauriInvoke('mouse_click', { x, y, button: button ?? null }),
  mouseDoubleClick: (x: number, y: number) =>
    tauriInvoke('mouse_double_click', { x, y }),
  mouseDrag: (fromX: number, fromY: number, toX: number, toY: number) =>
    tauriInvoke('mouse_drag', { fromX, fromY, toX, toY }),
  mouseScroll: (x: number, y: number, direction: string, amount: number) =>
    tauriInvoke('mouse_scroll', { x, y, direction, amount }),
  keyboardType: (text: string) =>
    tauriInvoke('keyboard_type', { text }),
  keyboardKey: (key: string, modifiers?: string[]) =>
    tauriInvoke('keyboard_key', { key, modifiers: modifiers ?? null }),
  keyboardHotkey: (keys: string[]) =>
    tauriInvoke('keyboard_hotkey', { keys }),
}

export const computerUseApi = {
  start: (goal: string, providerId: string, modelId: string, maxSteps?: number, permissionMode?: string) =>
    tauriInvoke<string>('start_computer_use', {
      goal, providerId, modelId,
      maxSteps: maxSteps ?? null,
      permissionMode: permissionMode ?? null
    }),
  stop: (sessionId: string) =>
    tauriInvoke('stop_computer_use', { sessionId }),
  getStatus: (sessionId: string) =>
    tauriInvoke<{ sessionId: string; running: boolean; currentStep: number; maxSteps: number }>('get_computer_use_status', { sessionId }),
  approveAction: (sessionId: string, approved: boolean) =>
    tauriInvoke('approve_action', { sessionId, approved }),
}

export async function onComputerUseStep(cb: (payload: any) => void) {
  const l = await getListen()
  if (!l) return () => {}
  return l<any>('computer-use:step', (e) => cb(e.payload))
}

export async function onComputerUseAction(cb: (payload: any) => void) {
  const l = await getListen()
  if (!l) return () => {}
  return l<any>('computer-use:action', (e) => cb(e.payload))
}

export async function onComputerUseComplete(cb: (payload: any) => void) {
  const l = await getListen()
  if (!l) return () => {}
  return l<any>('computer-use:complete', (e) => cb(e.payload))
}

export async function onComputerUseError(cb: (payload: any) => void) {
  const l = await getListen()
  if (!l) return () => {}
  return l<any>('computer-use:error', (e) => cb(e.payload))
}

export const proxyApi = {
  startServer: (port: number, apiKey: string, defaultProviderId: string) =>
    tauriInvoke<ApiServerStatus>('start_api_server', { port, apiKey, defaultProviderId }),
  stopServer: () =>
    tauriInvoke<ApiServerStatus>('stop_api_server'),
  getStatus: () =>
    tauriInvoke<ApiServerStatus>('get_api_server_status').then(r => r || { running: false, port: 23333, url: '' }),
  updateProviders: () =>
    tauriInvoke('update_api_server_providers'),
}
