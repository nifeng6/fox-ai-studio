import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface CacheEntry {
  key: string
  value: string
  createdAt: number
  ttl: number
  hits: number
}

export const useCacheStore = defineStore('cache', () => {
  const entries = ref<Map<string, CacheEntry>>(new Map())
  const maxSize = ref(200)

  function generateKey(toolName: string, args: Record<string, unknown>): string {
    const sorted = JSON.stringify(args, Object.keys(args).sort())
    return `${toolName}::${sorted}`
  }

  function get(key: string): string | null {
    const e = entries.value.get(key)
    if (!e) return null
    if (e.ttl > 0 && Date.now() - e.createdAt > e.ttl) {
      entries.value.delete(key)
      return null
    }
    e.hits++
    return e.value
  }

  function set(key: string, value: string, ttl: number = 5 * 60 * 1000) {
    if (entries.value.size >= maxSize.value) {
      let oldestKey = ''
      let oldestTime = Infinity
      for (const [k, v] of entries.value) {
        if (v.createdAt < oldestTime) {
          oldestTime = v.createdAt
          oldestKey = k
        }
      }
      if (oldestKey) entries.value.delete(oldestKey)
    }
    entries.value.set(key, { key, value, createdAt: Date.now(), ttl, hits: 0 })
  }

  function invalidate(pattern?: string) {
    if (!pattern) {
      entries.value.clear()
      return
    }
    for (const key of entries.value.keys()) {
      if (key.includes(pattern)) entries.value.delete(key)
    }
  }

  function getStats() {
    let totalHits = 0
    let totalEntries = 0
    for (const e of entries.value.values()) {
      totalHits += e.hits
      totalEntries++
    }
    return { totalEntries, totalHits, maxSize: maxSize.value }
  }

  return { entries, maxSize, generateKey, get, set, invalidate, getStats }
}, {
  persist: {
    serializer: {
      serialize: (state: any) => {
        const s = { ...state }
        s.entries = Object.fromEntries(state.entries)
        return JSON.stringify(s)
      },
      deserialize: (str: string) => {
        const s = JSON.parse(str)
        s.entries = new Map(Object.entries(s.entries || {}))
        return s
      }
    }
  }
})
