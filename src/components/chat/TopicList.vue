<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useChatStore } from '@/stores/chat'
import { useSettingsStore } from '@/stores/settings'
import { useProviderStore } from '@/stores/provider'
import { ElMessageBox, ElMessage } from 'element-plus'
import { Plus, Search, MoreFilled, Location, UserFilled } from '@element-plus/icons-vue'
import type { Topic, AgentConfig } from '@/types'
import { useAgentStore } from '@/stores/agent'

const { t } = useI18n()
const chat = useChatStore()
const settings = useSettingsStore()
const provider = useProviderStore()
const agent = useAgentStore()

const search = ref('')

const displayTopics = computed(() => {
  const q = search.value.trim().toLowerCase()
  let list = !q
    ? [...chat.topics]
    : chat.topics.filter(x => x.title.toLowerCase().includes(q))
  list = list.sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1
    return b.updatedAt - a.updatedAt
  })
  return list
})

onMounted(() => {
  if (!provider.providers.length) void provider.loadProviders()
})

function defaultTopicModel() {
  const fp = settings.defaultProviderId
  const fm = settings.defaultModelId
  const p = provider.getProviderById(fp)
  const hasModel = p?.models?.some(m => m === fm) ?? false
  return {
    providerId: fp && p ? fp : (provider.getEnabledProviders()[0]?.id || ''),
    modelId: hasModel ? fm : (provider.getEnabledProviders()[0]?.models?.[0] || '')
  }
}

function onNew() {
  const m = defaultTopicModel()
  const mainAgent = agent.getMainAgent() || agent.createMainAgent()
  const freshMain: AgentConfig = JSON.parse(JSON.stringify(mainAgent))
  const topic = chat.createTopic({
    title: t('chat.newTitle') as string,
    providerId: m.providerId,
    modelId: m.modelId,
    participantAgentIds: [freshMain.id],
    activeTab: freshMain.id
  })
  chat.setTopicAgentSnapshots(topic.id, [freshMain])
  agent.switchAgent(freshMain.id)
}


async function onRename(tpc: Topic) {
  const { value } = await ElMessageBox.prompt(
    t('chat.topicTitle') as string,
    t('chat.rename') as string,
    {
      inputValue: tpc.title,
      confirmButtonText: t('common.save') as string,
      cancelButtonText: t('common.cancel') as string
    }
  )
  if (value?.trim()) {
    chat.updateTopicTitle(tpc.id, value.trim())
  }
}

function onPin(tpc: Topic) {
  chat.togglePinTopic(tpc.id)
}

async function onDelete(tpc: Topic) {
  try {
    await ElMessageBox.confirm(
      t('chat.deleteTopicConfirm') as string,
      t('chat.deleteTopic') as string,
      { type: 'warning', confirmButtonText: t('common.delete') as string, cancelButtonText: t('common.cancel') as string }
    )
    chat.deleteTopic(tpc.id)
  } catch {
    /* user cancelled */
  }
}

async function onClearAll() {
  if (!chat.topics.length) return
  try {
    await ElMessageBox.confirm(
      t('chat.clearConfirm') as string,
      t('chat.clearAll') as string,
      { type: 'warning', confirmButtonText: t('common.delete') as string, cancelButtonText: t('common.cancel') as string }
    )
    chat.clearAllTopics()
  } catch {
    /* user cancelled */
  }
}

function select(tpc: Topic) {
  chat.selectTopic(tpc.id)
  // Sync currentAgent to this topic's activeTab
  const tab = tpc.activeTab
  if (tab && tab !== 'group') {
    agent.switchAgent(tab)
  } else {
    const main = agent.getMainAgent()
    if (main) agent.switchAgent(main.id)
  }
}

async function onExport(tpc: Topic) {
  const jsonData = chat.exportTopic(tpc.id)
  if (!jsonData) return
  const filename = `${tpc.title.replace(/[<>:"/\\|?*]/g, '_')}_${new Date().toISOString().slice(0, 10)}.json`
  await doExportFile(filename, jsonData)
}

async function onExportText(tpc: Topic) {
  const text = chat.exportTopicAsText(tpc.id)
  if (!text) return
  const filename = `${tpc.title.replace(/[<>:"/\\|?*]/g, '_')}_${new Date().toISOString().slice(0, 10)}.md`
  await doExportFile(filename, text)
}

async function doExportFile(defaultName: string, content: string) {
  try {
    const { save } = await import('@tauri-apps/plugin-dialog')
    const filePath = await save({
      defaultPath: defaultName,
      filters: [
        { name: '所有文件', extensions: ['*'] },
        { name: 'JSON', extensions: ['json'] },
        { name: 'Markdown', extensions: ['md'] },
      ]
    })
    if (!filePath) return
    const { fileApi } = await import('@/utils/tauri-api')
    await fileApi.writeFile(filePath, content)
    ElMessage.success(`已导出到: ${filePath}`)
  } catch (e: any) {
    if (typeof document !== 'undefined') {
      const blob = new Blob([content], { type: 'text/plain' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = defaultName
      a.click()
      URL.revokeObjectURL(url)
      ElMessage.success('对话已导出')
    }
  }
}

function menuCommand(tpc: Topic, cmd: 'rename' | 'pin' | 'delete' | 'export-json' | 'export-md') {
  if (cmd === 'rename') void onRename(tpc)
  else if (cmd === 'pin') onPin(tpc)
  else if (cmd === 'delete') void onDelete(tpc)
  else if (cmd === 'export-json') onExport(tpc)
  else if (cmd === 'export-md') onExportText(tpc)
}
</script>

<template>
  <div class="topic-list">
    <div class="topic-list__search">
      <el-input
        v-model="search"
        :placeholder="t('chat.searchPlaceholder')"
        clearable
        :prefix-icon="Search"
        size="default"
        class="topic-search"
      />
    </div>
    <div class="topic-list__actions">
      <el-button
        class="new-btn"
        type="primary"
        :icon="Plus"
        @click="onNew"
      >
        {{ t('chat.newTopic') }}
      </el-button>
    </div>
    <div class="topic-list__scroll" role="navigation" :aria-label="t('nav.chat')">
      <ul class="topic-ul">
        <li
          v-for="tpc in displayTopics"
          :key="tpc.id"
          class="topic-item"
          :class="{ 'topic-item--active': chat.currentTopicId === tpc.id }"
          @click="select(tpc)"
        >
          <span v-if="tpc.pinned" class="topic-item__pin" :title="t('chat.unpin')">
            <el-icon :size="12"><Location /></el-icon>
          </span>
          <span
            v-if="tpc.isGroupChat"
            class="topic-item__group"
            :title="(t('chat.groupChat') as string)"
            aria-hidden="true"
          >
            <el-icon :size="14"><UserFilled /></el-icon>
          </span>
          <span class="topic-item__title" :title="tpc.title">{{ tpc.title }}</span>
          <el-dropdown
            trigger="click"
            @click.stop
            @command="(c: 'rename' | 'pin' | 'delete' | 'export-json' | 'export-md') => menuCommand(tpc, c)"
          >
            <button type="button" class="topic-item__more" :aria-label="t('chat.topicMenu')">
              <el-icon :size="16"><MoreFilled /></el-icon>
            </button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item command="rename">{{ t('chat.rename') }}</el-dropdown-item>
                <el-dropdown-item command="pin">{{ tpc.pinned ? t('chat.unpin') : t('chat.pin') }}</el-dropdown-item>
                <el-dropdown-item command="export-json" divided>导出 JSON</el-dropdown-item>
                <el-dropdown-item command="export-md">导出 Markdown</el-dropdown-item>
                <el-dropdown-item command="delete" divided>{{ t('chat.deleteTopic') }}</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </li>
      </ul>
      <p v-if="!displayTopics.length" class="topic-list__empty">{{ t('common.noData') }}</p>
    </div>
    <div class="topic-list__footer">
      <el-button class="clear-all-btn" :disabled="!chat.topics.length" @click="onClearAll">
        {{ t('chat.clearAll') }}
      </el-button>
    </div>

  </div>
</template>

<style lang="scss" scoped>
.topic-list {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-width: 0;
  background: var(--color-background-soft);
  border-right: 1px solid var(--color-border);
}

.topic-list__search {
  padding: 12px 12px 8px;
  flex-shrink: 0;
}

.topic-search {
  width: 100%;
  --el-input-border-color: var(--color-border);
  --el-text-color-primary: var(--color-text-1);
  --el-text-color-placeholder: var(--color-text-3);
  --el-border-radius-base: var(--fox-radius-sm);
}

:deep(.el-input__wrapper) {
  background: var(--color-background-mute) !important;
  box-shadow: none !important;
  border: 1px solid var(--color-border) !important;
  transition: border-color 0.2s, box-shadow 0.2s;
}
:deep(.el-input__wrapper:hover) {
  border-color: var(--color-text-3) !important;
}
:deep(.el-input__wrapper.is-focus) {
  border-color: var(--fox-accent-border) !important;
  box-shadow: 0 0 0 2px var(--color-primary-mute) !important;
}

.topic-list__actions {
  padding: 0 12px 10px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.new-btn,
.group-btn {
  width: 100%;
  --el-color-primary: var(--fox-accent-fg);
  border: 1px solid var(--color-border) !important;
  border-radius: var(--fox-radius-sm) !important;
  background: var(--color-background-mute) !important;
  color: var(--color-text-1) !important;
  font-weight: 500;
  box-shadow: var(--shadow-sm) !important;
  transition: background 0.2s, border-color 0.2s, transform 0.1s;
}

.new-btn:hover {
  background: var(--color-hover) !important;
  border-color: var(--color-text-3) !important;
  transform: translateY(-1px);
  box-shadow: var(--shadow-md) !important;
}
.new-btn:active {
  background: var(--color-active) !important;
  transform: translateY(0);
}

.group-btn {
  --el-color-primary: var(--fox-accent-fg);
  border: 1px solid var(--color-border) !important;
  border-radius: var(--fox-radius-sm) !important;
  background: var(--color-background) !important;
  color: var(--color-text-1) !important;
  font-weight: 500;
  box-shadow: var(--shadow-sm) !important;
  transition: background 0.2s, border-color 0.2s, transform 0.1s;
}

.group-btn:hover {
  background: var(--color-hover) !important;
  border-color: var(--color-text-3) !important;
  transform: translateY(-1px);
}
.group-btn:active {
  background: var(--color-active) !important;
  transform: translateY(0);
}

.group-dialog-hint {
  margin: 0 0 12px;
  font-size: 12px;
  color: var(--color-text-3);
  line-height: 1.5;
}

.group-dialog-checks {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
}

.topic-list__footer {
  flex-shrink: 0;
  padding: 8px 12px 12px;
  border-top: 1px solid var(--color-border);
  background: var(--color-background-soft);
}

.clear-all-btn {
  width: 100%;
  --el-button-border-color: var(--color-border);
  --el-button-bg-color: var(--color-background-mute);
  --el-button-text-color: var(--color-text-2);
  border-radius: var(--fox-radius-sm);
  font-size: 12px;
}
.clear-all-btn:hover:not(:disabled) {
  --el-button-border-color: var(--color-text-3);
  --el-button-bg-color: var(--color-hover);
  --el-button-text-color: var(--color-text-1);
}

.topic-list__scroll {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 0 8px 12px;
}

.topic-ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.topic-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 8px 8px 10px;
  border-radius: var(--fox-radius-sm);
  border: 1px solid transparent;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s, box-shadow 0.15s;
  color: var(--color-text-2);
}

.topic-item:hover {
  background: var(--color-hover);
  color: var(--color-text-1);
}

.topic-item--active {
  background: var(--color-primary-mute) !important;
  border: 1px solid var(--color-border) !important;
  color: var(--color-text-1) !important;
  box-shadow: var(--shadow-sm);
}

.topic-item__pin {
  color: var(--color-warning);
  flex-shrink: 0;
  display: flex;
  align-items: center;
}

.topic-item__group {
  color: var(--color-text-3);
  flex-shrink: 0;
  display: flex;
  align-items: center;
}

.topic-item__title {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  line-height: 1.35;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.topic-item__more {
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid transparent;
  border-radius: var(--fox-radius-sm);
  background: transparent;
  color: var(--color-text-3);
  cursor: pointer;
  transition: color 0.15s, background 0.15s, border-color 0.15s;
}

.topic-item__more:hover {
  color: var(--color-text-1);
  background: var(--color-hover);
  border-color: var(--color-border);
}
.topic-item__more:active {
  background: var(--color-active);
}

.topic-list__empty {
  margin: 20px 8px 0;
  text-align: center;
  font-size: 12px;
  color: var(--color-text-3);
}
</style>
