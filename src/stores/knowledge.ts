import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { KnowledgeBase } from '@/types'
import { knowledgeApi } from '@/utils/tauri-api'

export const useKnowledgeStore = defineStore('knowledge', () => {
  const bases = ref<KnowledgeBase[]>([])
  const currentBaseId = ref<string | null>(null)
  const loading = ref(false)

  async function loadBases() {
    loading.value = true
    try { bases.value = await knowledgeApi.getBases() } catch { /* noop */ }
    finally { loading.value = false }
  }

  async function createBase(data: Partial<KnowledgeBase>) {
    const result = await knowledgeApi.createBase(data)
    await loadBases()
    return result
  }

  async function deleteBase(id: string) {
    await knowledgeApi.deleteBase(id)
    await loadBases()
    if (currentBaseId.value === id) currentBaseId.value = null
  }

  async function addDocument(baseId: string, doc: any) {
    const result = await knowledgeApi.addDocument(baseId, doc)
    await loadBases()
    return result
  }

  async function search(baseId: string, query: string, topK = 5) {
    return await knowledgeApi.search(baseId, query, topK)
  }

  return { bases, currentBaseId, loading, loadBases, createBase, deleteBase, addDocument, search }
})
