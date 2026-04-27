import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useAppStore = defineStore('app', () => {
  const currentRoute = ref('chat')
  const globalSearchVisible = ref(false)
  const isLoading = ref(false)
  const platform = ref('win32')

  function setCurrentRoute(route: string) {
    currentRoute.value = route
  }

  function toggleGlobalSearch() {
    globalSearchVisible.value = !globalSearchVisible.value
  }

  return { currentRoute, globalSearchVisible, isLoading, platform, setCurrentRoute, toggleGlobalSearch }
})
