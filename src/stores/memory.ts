import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { v4 as uuidv4 } from 'uuid'
import type { MemoryEntry, MemoryEntryCategory, UserProfile } from '@/types'

const emptyProfile = (): UserProfile => ({
  name: '',
  preferences: {},
  notes: ''
})

function matchesQuery(text: string, q: string): boolean {
  const t = text.toLowerCase()
  const words = q
    .toLowerCase()
    .split(/\s+/)
    .map(w => w.trim())
    .filter(Boolean)
  if (!words.length) return true
  return words.every(w => t.includes(w))
}

export const useMemoryStore = defineStore('memory', () => {
  const memories = ref<MemoryEntry[]>([])
  const userProfile = ref<UserProfile>(emptyProfile())

  const memoryCount = computed(() => memories.value.length)

  function addMemory(data: {
    content: string
    category: MemoryEntryCategory
    source: MemoryEntry['source']
  }): MemoryEntry {
    const now = Date.now()
    const e: MemoryEntry = {
      id: uuidv4(),
      content: data.content.trim(),
      category: data.category,
      source: data.source,
      createdAt: now,
      updatedAt: now
    }
    memories.value = [e, ...memories.value]
    return e
  }

  function updateMemory(id: string, partial: Partial<Pick<MemoryEntry, 'content' | 'category'>>) {
    const m = memories.value.find(x => x.id === id)
    if (!m) return
    if (partial.content !== undefined) m.content = partial.content
    if (partial.category !== undefined) m.category = partial.category
    m.updatedAt = Date.now()
  }

  function deleteMemory(id: string) {
    memories.value = memories.value.filter(m => m.id !== id)
  }

  function searchMemories(query: string): MemoryEntry[] {
    const q = query.trim()
    if (!q) return [...memories.value]
    return memories.value.filter(m => matchesQuery(m.content, q))
  }

  function getByCategory(category: MemoryEntryCategory): MemoryEntry[] {
    return memories.value.filter(m => m.category === category)
  }

  function updateUserProfile(partial: Partial<UserProfile>) {
    const p = userProfile.value
    if (partial.name !== undefined) p.name = partial.name
    if (partial.preferences !== undefined) p.preferences = { ...p.preferences, ...partial.preferences }
    if (partial.notes !== undefined) p.notes = partial.notes
  }

  const sessionSummaries = ref<{ topicId: string; summary: string; createdAt: number }[]>([])
  const memoryStats = ref({
    totalRecalls: 0,
    lastConsolidation: 0
  })

  function addSessionSummary(topicId: string, summary: string) {
    sessionSummaries.value.push({ topicId, summary, createdAt: Date.now() })
    if (sessionSummaries.value.length > 50) {
      sessionSummaries.value = sessionSummaries.value.slice(-50)
    }
  }

  function getSessionSummary(topicId: string): string | null {
    const s = sessionSummaries.value.find(x => x.topicId === topicId)
    return s?.summary || null
  }

  function consolidateMemories() {
    const seen = new Map<string, MemoryEntry>()
    const toRemove: string[] = []

    for (const m of memories.value) {
      const key = m.content.trim().toLowerCase()
      if (seen.has(key)) {
        toRemove.push(m.id)
      } else {
        seen.set(key, m)
      }
    }

    if (toRemove.length) {
      memories.value = memories.value.filter(m => !toRemove.includes(m.id))
    }

    memoryStats.value.lastConsolidation = Date.now()
    return toRemove.length
  }

  function getRelevantContext(query: string, limit: number = 10): string[] {
    const results: string[] = []

    const mems = searchMemories(query).slice(0, limit)
    for (const m of mems) {
      results.push(`[${m.category}] ${m.content}`)
    }
    memoryStats.value.totalRecalls++

    const up = userProfile.value
    if (up.name?.trim()) results.unshift(`User: ${up.name}`)

    return results
  }

  function decayOldMemories(maxAgeDays: number = 90) {
    const cutoff = Date.now() - maxAgeDays * 24 * 60 * 60 * 1000
    const before = memories.value.length
    memories.value = memories.value.filter(m => m.updatedAt > cutoff)
    return before - memories.value.length
  }

  return {
    memories,
    userProfile,
    memoryCount,
    sessionSummaries,
    memoryStats,
    addMemory,
    updateMemory,
    deleteMemory,
    searchMemories,
    getByCategory,
    updateUserProfile,
    addSessionSummary,
    getSessionSummary,
    consolidateMemories,
    getRelevantContext,
    decayOldMemories
  }
}, {
  persist: {
    pick: ['memories', 'userProfile', 'sessionSummaries', 'memoryStats'] as const
  }
})
