import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { v4 as uuidv4 } from 'uuid'

export interface ScheduledTask {
  id: string
  name: string
  agentId: string
  prompt: string
  cron: string
  enabled: boolean
  lastRunAt: number | null
  nextRunAt: number | null
  runCount: number
  createdAt: number
}

function parseCronToMs(cron: string): number | null {
  const m = cron.match(/^every\s+(\d+)\s*(m|min|h|hour|d|day)s?$/i)
  if (!m) return null
  const val = parseInt(m[1])
  const unit = m[2].toLowerCase()
  if (unit === 'm' || unit === 'min') return val * 60_000
  if (unit === 'h' || unit === 'hour') return val * 3_600_000
  if (unit === 'd' || unit === 'day') return val * 86_400_000
  return null
}

export const useScheduleStore = defineStore('schedule', () => {
  const tasks = ref<ScheduledTask[]>([])
  const timers = new Map<string, ReturnType<typeof setInterval>>()

  const enabledTasks = computed(() => tasks.value.filter(t => t.enabled))

  function createTask(data: {
    name: string; agentId: string; prompt: string; cron: string
  }): ScheduledTask {
    const task: ScheduledTask = {
      id: uuidv4(),
      name: data.name.trim() || '定时任务',
      agentId: data.agentId,
      prompt: data.prompt,
      cron: data.cron,
      enabled: true,
      lastRunAt: null,
      nextRunAt: null,
      runCount: 0,
      createdAt: Date.now()
    }
    tasks.value = [...tasks.value, task]
    scheduleTask(task)
    return task
  }

  function updateTask(id: string, data: Partial<ScheduledTask>) {
    const t = tasks.value.find(x => x.id === id)
    if (!t) return
    if (data.name !== undefined) t.name = data.name
    if (data.agentId !== undefined) t.agentId = data.agentId
    if (data.prompt !== undefined) t.prompt = data.prompt
    if (data.cron !== undefined) t.cron = data.cron
    if (data.enabled !== undefined) {
      t.enabled = data.enabled
      if (data.enabled) scheduleTask(t)
      else unscheduleTask(t.id)
    }
  }

  function deleteTask(id: string) {
    unscheduleTask(id)
    tasks.value = tasks.value.filter(t => t.id !== id)
  }

  function scheduleTask(task: ScheduledTask) {
    unscheduleTask(task.id)
    if (!task.enabled) return
    const intervalMs = parseCronToMs(task.cron)
    if (!intervalMs) return
    task.nextRunAt = Date.now() + intervalMs
    const timer = setInterval(() => {
      runTask(task.id)
    }, intervalMs)
    timers.set(task.id, timer)
  }

  function unscheduleTask(id: string) {
    const timer = timers.get(id)
    if (timer) { clearInterval(timer); timers.delete(id) }
  }

  async function runTask(id: string) {
    const task = tasks.value.find(t => t.id === id)
    if (!task || !task.enabled) return
    task.lastRunAt = Date.now()
    task.runCount++
    const intervalMs = parseCronToMs(task.cron)
    if (intervalMs) task.nextRunAt = Date.now() + intervalMs

    try {
      const { useAgentStore } = await import('@/stores/agent')
      const { useChatStore } = await import('@/stores/chat')
      const { sendChatMessage } = await import('@/utils/tauri-api')
      const agentStore = useAgentStore()
      const chatStore = useChatStore()

      const agent = agentStore.agents.find(a => a.id === task.agentId)
      if (!agent?.languageModel?.providerId || !agent.languageModel.modelId) return

      let topicId = chatStore.currentTopicId
      if (!topicId) {
        const topic = chatStore.createTopic({ title: `定时任务: ${task.name}` })
        topicId = topic.id
      }

      chatStore.addMessage(topicId, {
        role: 'system', content: `[定时任务 "${task.name}" 自动触发]`, status: 'complete'
      })
      const userMsg = chatStore.addMessage(topicId, {
        role: 'user', content: task.prompt, status: 'complete', agentId: task.agentId
      })

      const msgId = `sched-${task.id}-${Date.now()}`
      const assistantMsg = chatStore.addMessage(topicId, {
        role: 'assistant', content: '', status: 'streaming', agentId: task.agentId, agentName: agent.name
      })

      const { onStreamChunk, onStreamEnd, onStreamError } = await import('@/utils/tauri-api')
      const unChunk = await onStreamChunk((p) => {
        if (p.messageId === msgId) chatStore.appendToMessage(assistantMsg.id, p.chunk)
      })
      const unEnd = await onStreamEnd((p) => {
        if (p.messageId === msgId) {
          chatStore.updateMessage(assistantMsg.id, { status: 'complete' })
          unChunk(); unEnd(); unErr()
        }
      })
      const unErr = await onStreamError((p) => {
        if (p.messageId === msgId) {
          chatStore.updateMessage(assistantMsg.id, { status: 'error', error: p.error })
          unChunk(); unEnd(); unErr()
        }
      })

      await sendChatMessage({
        providerId: agent.languageModel.providerId,
        modelId: agent.languageModel.modelId,
        messageId: msgId,
        messages: [
          { role: 'system', content: agent.systemPrompt || 'You are a helpful assistant.' },
          { role: 'user', content: task.prompt }
        ]
      })
    } catch (e) {
      console.error('[Schedule] Task run error:', e)
    }
  }

  function initTimers() {
    for (const task of tasks.value) {
      if (task.enabled) scheduleTask(task)
    }
  }

  function stopAll() {
    for (const [id] of timers) unscheduleTask(id)
  }

  return {
    tasks, enabledTasks,
    createTask, updateTask, deleteTask,
    runTask, initTimers, stopAll
  }
}, {
  persist: { pick: ['tasks'] as const }
})
