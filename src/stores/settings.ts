import { defineStore } from 'pinia'
import { ref } from 'vue'

export type NavBarPosition = 'left' | 'top'
export type TopicPosition = 'left' | 'right'
export type ModelIconType = 'model' | 'emoji' | 'avatar' | 'none'
export type SelectionMethod = 'select' | 'ctrl' | 'shortcut'

export interface WebSearchEngineConfig {
  apiKey: string
  apiUrl: string
}

export const useSettingsStore = defineStore('settings', () => {
  const language = ref('zh-CN')
  const sendKey = ref<'Enter' | 'Shift+Enter'>('Enter')
  const proxyUrl = ref('')
  const proxyEnabled = ref(false)
  const launchAtStartup = ref(false)
  const minimizeToTray = ref(true)
  const showInMenuBar = ref(true)
  const enableMemory = ref(true)
  const enableWebSearch = ref(false)
  const defaultProviderId = ref('')
  const defaultModelId = ref('')
  const defaultAssistantId = ref('')
  const quickModelProviderId = ref('')
  const quickModelId = ref('')
  const translateModelProviderId = ref('')
  const translateModelId = ref('')
  const webdavUrl = ref('')
  const webdavUsername = ref('')
  const webdavPassword = ref('')
  const webdavRemotePath = ref('/fox-ai-backup')
  const autoBackup = ref(false)
  const autoBackupInterval = ref(24)
  const leanBackup = ref(false)

  const navBarPosition = ref<NavBarPosition>('left')
  const zoom = ref(100)
  const globalFont = ref('')
  const codeFont = ref('')
  const topicPosition = ref<TopicPosition>('left')
  const autoSwitchTopic = ref(true)
  const showTopicTime = ref(false)
  const pinTopicTop = ref(false)
  const modelIconType = ref<ModelIconType>('model')
  const customCss = ref('')
  const storagePath = ref('')

  const selectionAssistantEnabled = ref(true)
  const selectionMethod = ref<SelectionMethod>('select')
  const brushMode = ref(false)
  const selToolbarAutoShow = ref(true)
  const selRememberSize = ref(false)
  const selAutoClose = ref(false)
  const selAutoTop = ref(false)
  const selOpacity = ref(100)
  const selActions = ref<string[]>(['translate', 'explain', 'summarize', 'search', 'copy'])

  const ocrProvider = ref('system')
  const ocrLanguages = ref<string[]>(['en'])
  const docProvider = ref('mineru')
  const mineruApiKey = ref('')
  const mineruApiUrl = ref('https://mineru.net')

  const apiServerEnabled = ref(false)
  const apiServerPort = ref(23333)
  const apiServerKey = ref('')

  const channels = ref<Array<{
    id: string; name: string; platformId: string; webhookUrl: string; secret?: string;
    enabled: boolean; createdAt: number; notifyOnReply: boolean; messageTemplate?: string;
  }>>([])

  const shortcuts = ref<Array<{
    id: string; labelKey: string; keys: string; enabled: boolean; isGlobal: boolean
  }>>([
    { id: 'zoomIn', labelKey: 'shortcuts.zoomIn', keys: 'CommandOrControl+=', enabled: true, isGlobal: false },
    { id: 'zoomOut', labelKey: 'shortcuts.zoomOut', keys: 'CommandOrControl+-', enabled: true, isGlobal: false },
    { id: 'zoomReset', labelKey: 'shortcuts.zoomReset', keys: 'CommandOrControl+0', enabled: true, isGlobal: false },
    { id: 'openSettings', labelKey: 'shortcuts.openSettings', keys: 'CommandOrControl+,', enabled: true, isGlobal: false },
    { id: 'toggleApp', labelKey: 'shortcuts.toggleApp', keys: 'CommandOrControl+Shift+S', enabled: false, isGlobal: true },
    { id: 'newChat', labelKey: 'shortcuts.newChat', keys: 'CommandOrControl+N', enabled: true, isGlobal: false },
  ])

  const harnessEnabled = ref(true)
  const harnessPlanningGate = ref(true)
  const harnessOutputConstraints = ref(true)
  const harnessFeedbackLoop = ref(true)
  const harnessMaxRetries = ref(2)

  const webSearchEngineSettings = ref<Record<string, WebSearchEngineConfig>>({})

  function getWebSearchConfig(engineId: string): WebSearchEngineConfig {
    const cur = webSearchEngineSettings.value[engineId]
    if (cur) return { apiKey: cur.apiKey, apiUrl: cur.apiUrl }
    return { apiKey: '', apiUrl: '' }
  }

  function setWebSearchConfig(engineId: string, partial: Partial<WebSearchEngineConfig>) {
    const prev = getWebSearchConfig(engineId)
    webSearchEngineSettings.value = {
      ...webSearchEngineSettings.value,
      [engineId]: { apiKey: partial.apiKey ?? prev.apiKey, apiUrl: partial.apiUrl ?? prev.apiUrl }
    }
  }

  function setLanguage(lang: string) {
    language.value = lang
  }

  return {
    language, sendKey, proxyUrl, proxyEnabled, launchAtStartup, minimizeToTray,
    showInMenuBar, enableMemory, enableWebSearch, defaultProviderId, defaultModelId,
    defaultAssistantId, quickModelProviderId, quickModelId, translateModelProviderId, translateModelId,
    webdavUrl, webdavUsername, webdavPassword, webdavRemotePath,
    autoBackup, autoBackupInterval, leanBackup, setLanguage,
    navBarPosition, zoom, globalFont, codeFont, topicPosition, autoSwitchTopic, showTopicTime,
    pinTopicTop, modelIconType, customCss, storagePath,
    selectionAssistantEnabled, selectionMethod, brushMode, selToolbarAutoShow, selRememberSize,
    selAutoClose, selAutoTop, selOpacity, selActions,
    ocrProvider, ocrLanguages, docProvider, mineruApiKey, mineruApiUrl,
    apiServerEnabled, apiServerPort, apiServerKey,
    channels, shortcuts,
    harnessEnabled, harnessPlanningGate, harnessOutputConstraints, harnessFeedbackLoop, harnessMaxRetries,
    webSearchEngineSettings, getWebSearchConfig, setWebSearchConfig
  }
}, { persist: true })
