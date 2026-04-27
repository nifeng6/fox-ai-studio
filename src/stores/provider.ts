import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Provider } from '@/types'
import { providerApi } from '@/utils/tauri-api'

export const useProviderStore = defineStore('provider', () => {
  const providers = ref<Provider[]>([])
  const loading = ref(false)

  async function loadProviders() {
    loading.value = true
    try { providers.value = await providerApi.getProviders() } catch { /* noop */ }
    finally { loading.value = false }
  }

  async function addProvider(provider: Partial<Provider>) {
    const result = await providerApi.addProvider(provider)
    await loadProviders()
    return result
  }

  async function updateProvider(id: string, data: Partial<Provider>) {
    await providerApi.updateProvider(id, data)
    await loadProviders()
  }

  async function removeProvider(id: string) {
    await providerApi.removeProvider(id)
    await loadProviders()
  }

  async function testConnection(id: string) {
    return await providerApi.testConnection(id)
  }

  async function getModels(id: string) {
    return await providerApi.getModels(id)
  }

  function getEnabledProviders(): Provider[] {
    return providers.value.filter(p => p.enabled && p.apiKey)
  }

  function getProviderById(id: string): Provider | undefined {
    return providers.value.find(p => p.id === id)
  }

  return {
    providers, loading, loadProviders, addProvider, updateProvider, removeProvider,
    testConnection, getModels, getEnabledProviders, getProviderById
  }
})
