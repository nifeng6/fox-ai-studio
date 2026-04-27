<template>
  <div class="files-page">
    <div class="layout">
      <!-- Left: conversation + agent tree -->
      <aside class="sidebar">
        <h2 class="side-title">工作空间</h2>

        <div class="tree-section">
          <div class="tree-section__label">对话</div>
          <div
            v-for="topic in topics"
            :key="topic.id"
            class="tree-node tree-node--topic"
            :class="{ active: selectedTopicId === topic.id }"
            @click="selectTopic(topic.id)"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>
            <span class="tree-node__text">{{ topic.title }}</span>
          </div>
          <div v-if="!topics.length" class="tree-empty">暂无对话</div>
        </div>

        <template v-if="selectedTopicId">
          <div class="tree-section">
            <div class="tree-section__label">Agents</div>
            <div
              v-for="ag in topicAgents"
              :key="ag.id"
              class="tree-node tree-node--agent"
              :class="{ active: selectedAgentId === ag.id }"
              @click="selectAgent(ag.id)"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>
              <span class="tree-node__text">{{ ag.name }}</span>
              <span class="tree-node__badge">{{ ag.type === 'main' ? '主' : '子' }}</span>
            </div>
          </div>

          <template v-if="selectedAgentId">
            <div class="tree-section">
              <div class="tree-section__label">目录</div>
              <div
                v-for="dir in dirTabs"
                :key="dir.id"
                class="tree-node tree-node--dir"
                :class="{ active: activeDir === dir.id }"
                @click="activeDir = dir.id"
              >
                <component :is="dir.icon" />
                <span class="tree-node__text">{{ dir.label }}</span>
                <span class="tree-node__count">{{ dirCounts[dir.id] || 0 }}</span>
              </div>
            </div>
          </template>
        </template>
      </aside>

      <!-- Right: content panel -->
      <main class="main">
        <template v-if="!selectedTopicId">
          <div class="empty-state">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" class="empty-icon">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
            </svg>
            <p>选择左侧对话查看 Agent 工作目录</p>
          </div>
        </template>

        <template v-else-if="!selectedAgentId">
          <div class="empty-state">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" class="empty-icon">
              <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>
            </svg>
            <p>选择一个 Agent 查看其工作目录</p>
          </div>
        </template>

        <template v-else>
          <div class="content-header">
            <div class="content-header__path">
              <span class="path-seg" @click="selectedAgentId = ''">{{ selectedTopicTitle }}</span>
              <span class="path-sep">/</span>
              <span class="path-seg" @click="activeDir = 'files'">{{ selectedAgentName }}</span>
              <span class="path-sep">/</span>
              <span class="path-seg path-seg--current">{{ activeDirLabel }}</span>
            </div>
            <div class="content-header__actions">
              <button class="action-btn" @click="onCreateItem" title="新建">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
              </button>
              <button class="action-btn" @click="onRefresh" title="刷新">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>
              </button>
              <button class="action-btn" @click="onOpenFolder" title="打开文件夹">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
              </button>
            </div>
          </div>

          <div class="content-body">
            <!-- Skills -->
            <template v-if="activeDir === 'skills'">
              <div v-if="!agentSkills.length" class="dir-empty">
                <p>暂无技能文件</p>
                <p class="dir-empty__hint">Agent 的技能定义文件将存放在此处</p>
              </div>
              <div v-else class="item-list">
                <div v-for="s in agentSkills" :key="s.id" class="item-card">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="item-card__icon"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>
                  <div class="item-card__info">
                    <div class="item-card__name">{{ s.name }}</div>
                    <div class="item-card__desc">{{ s.description || '无描述' }}</div>
                  </div>
                </div>
              </div>
            </template>

            <!-- Files -->
            <template v-if="activeDir === 'files'">
              <div v-if="!agentFiles.length" class="dir-empty">
                <p>暂无文件</p>
                <p class="dir-empty__hint">对话中产生的附件和文件将存放在此处</p>
              </div>
              <div v-else class="item-list">
                <div v-for="f in agentFiles" :key="f.id" class="item-card">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="item-card__icon"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>
                  <div class="item-card__info">
                    <div class="item-card__name">{{ f.name }}</div>
                    <div class="item-card__desc">{{ f.size }}</div>
                  </div>
                </div>
              </div>
            </template>

            <!-- Knowledge -->
            <template v-if="activeDir === 'knowledge'">
              <div v-if="!agentKnowledge.length" class="dir-empty">
                <p>暂无知识库</p>
                <p class="dir-empty__hint">Agent 的知识库文件将存放在此处</p>
              </div>
              <div v-else class="item-list">
                <div v-for="k in agentKnowledge" :key="k.id" class="item-card">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="item-card__icon"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/></svg>
                  <div class="item-card__info">
                    <div class="item-card__name">{{ k.name }}</div>
                    <div class="item-card__desc">{{ k.description || '无描述' }}</div>
                  </div>
                </div>
              </div>
            </template>

            <!-- Memory -->
            <template v-if="activeDir === 'memory'">
              <div v-if="!agentMemories.length" class="dir-empty">
                <p>暂无记忆条目</p>
                <p class="dir-empty__hint">Agent 的持久记忆将存放在此处</p>
              </div>
              <div v-else class="item-list">
                <div v-for="m in agentMemories" :key="m.id" class="item-card">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="item-card__icon"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>
                  <div class="item-card__info">
                    <div class="item-card__name">[{{ m.category }}] {{ m.content.slice(0, 60) }}</div>
                    <div class="item-card__desc">{{ new Date(m.updatedAt).toLocaleDateString() }}</div>
                  </div>
                </div>
              </div>
            </template>
          </div>
        </template>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, h } from 'vue'
import { useChatStore } from '@/stores/chat'
import { useAgentStore } from '@/stores/agent'
import { useSkillStore } from '@/stores/skill'
import { useMemoryStore } from '@/stores/memory'
import { ElMessage } from 'element-plus'

const chatStore = useChatStore()
const agentStore = useAgentStore()
const skillStore = useSkillStore()
const memoryStore = useMemoryStore()

const selectedTopicId = ref('')
const selectedAgentId = ref('')
type DirType = 'skills' | 'files' | 'knowledge' | 'memory'
const activeDir = ref<DirType>('files')

const topics = computed(() =>
  [...chatStore.topics].sort((a, b) => b.updatedAt - a.updatedAt)
)

const selectedTopicTitle = computed(() =>
  chatStore.topics.find(t => t.id === selectedTopicId.value)?.title || ''
)

const topicAgents = computed(() => {
  const topic = chatStore.topics.find(t => t.id === selectedTopicId.value)
  if (!topic) return []
  const ids = topic.participantAgentIds || []
  if (ids.length) {
    return agentStore.agents.filter(a => ids.includes(a.id))
  }
  return agentStore.agents
})

const selectedAgentName = computed(() => {
  return agentStore.agents.find(a => a.id === selectedAgentId.value)?.name || ''
})

function IconBolt() { return h('svg', { width: 14, height: 14, viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '1.8', innerHTML: '<polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>' }) }
function IconFile() { return h('svg', { width: 14, height: 14, viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '1.8', innerHTML: '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/>' }) }
function IconBook() { return h('svg', { width: 14, height: 14, viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '1.8', innerHTML: '<path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/>' }) }
function IconClock() { return h('svg', { width: 14, height: 14, viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '1.8', innerHTML: '<circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/>' }) }

const dirTabs = [
  { id: 'files' as const, label: '文件', icon: IconFile },
  { id: 'skills' as const, label: '技能', icon: IconBolt },
  { id: 'knowledge' as const, label: '知识库', icon: IconBook },
  { id: 'memory' as const, label: '记忆', icon: IconClock },
]

const activeDirLabel = computed(() =>
  dirTabs.find(d => d.id === activeDir.value)?.label || ''
)

const selectedAgent = computed(() =>
  agentStore.agents.find(a => a.id === selectedAgentId.value)
)

const agentSkills = computed(() => {
  const ag = selectedAgent.value
  if (!ag?.skillIds?.length) return []
  return skillStore.skills.filter(s => ag.skillIds.includes(s.id))
})

const agentFiles = computed(() => {
  if (!selectedTopicId.value || !selectedAgentId.value) return []
  const msgs = chatStore.getTopicMessages(selectedTopicId.value)
  const items: Array<{ id: string; name: string; size: string }> = []
  for (const m of msgs) {
    if (m.agentId && m.agentId !== selectedAgentId.value) continue
    if (m.attachments?.length) {
      for (const att of m.attachments) {
        items.push({
          id: att.id || att.name,
          name: att.name || 'file',
          size: att.size ? formatSize(att.size) : '-'
        })
      }
    }
  }
  return items
})

const agentKnowledge = computed(() => {
  return [] as Array<{ id: string; name: string; description: string }>
})

const agentMemories = computed(() => {
  return memoryStore.memories.slice(0, 50)
})

const dirCounts = computed(() => ({
  files: agentFiles.value.length,
  skills: agentSkills.value.length,
  knowledge: agentKnowledge.value.length,
  memory: agentMemories.value.length,
}))

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

function selectTopic(id: string) {
  selectedTopicId.value = id
  selectedAgentId.value = ''
  activeDir.value = 'files'
}

function selectAgent(id: string) {
  selectedAgentId.value = id
  activeDir.value = 'files'
}

function onCreateItem() {
  ElMessage.info(`在 ${activeDirLabel.value} 中新建项目（待实现）`)
}

function onRefresh() {
  ElMessage.success('已刷新')
}

async function onOpenFolder() {
  try {
    const { open } = await import('@tauri-apps/plugin-shell')
    await open('.')
  } catch {
    ElMessage.info('打开文件夹功能需要在 Tauri 环境中使用')
  }
}
</script>

<style lang="scss" scoped>
.files-page {
  height: 100%;
  background: var(--color-background);
  color: var(--color-text-1);
}

.layout {
  display: grid;
  grid-template-columns: 220px 1fr;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.sidebar {
  border-right: 1px solid var(--color-border);
  padding: 16px 12px;
  background: var(--color-background-soft);
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.side-title {
  margin: 0 0 12px 4px;
  font-size: 15px;
  font-weight: 600;
  color: var(--color-text-1);
}

.tree-section {
  margin-bottom: 8px;
}
.tree-section__label {
  font-size: 11px;
  font-weight: 600;
  color: var(--color-text-3);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding: 4px 8px 4px;
  margin-bottom: 2px;
}

.tree-node {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  border-radius: var(--fox-radius-sm);
  cursor: pointer;
  font-size: 13px;
  color: var(--color-text-2);
  border: 1px solid transparent;
  transition: background 0.1s, color 0.1s;
  &:hover { background: var(--color-hover); color: var(--color-text-1); }
  &.active {
    background: var(--color-primary-mute);
    color: var(--color-text-1);
    border-color: var(--color-border);
  }
}
.tree-node--agent { padding-left: 20px; }
.tree-node--dir { padding-left: 30px; }
.tree-node__text {
  flex: 1; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.tree-node__badge {
  font-size: 10px; padding: 1px 5px;
  border-radius: 4px; background: var(--color-background-mute);
  color: var(--color-text-3); border: 1px solid var(--color-border);
  flex-shrink: 0;
}
.tree-node__count {
  font-size: 11px; color: var(--color-text-3); flex-shrink: 0;
}
.tree-empty {
  font-size: 12px; color: var(--color-text-3);
  padding: 8px 12px; text-align: center;
}

.main {
  display: flex; flex-direction: column;
  min-width: 0; min-height: 0; overflow: hidden;
}

.empty-state {
  flex: 1; display: flex; flex-direction: column;
  align-items: center; justify-content: center;
  gap: 12px; color: var(--color-text-3);
  .empty-icon { opacity: 0.3; }
  p { font-size: 14px; }
}

.content-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 12px 20px; border-bottom: 1px solid var(--color-border);
  background: var(--color-background-soft); flex-shrink: 0;
}
.content-header__path {
  display: flex; align-items: center; gap: 4px;
  font-size: 13px; min-width: 0; overflow: hidden;
}
.path-seg {
  color: var(--color-text-2); cursor: pointer;
  max-width: 140px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  &:hover { color: var(--color-text-1); }
}
.path-seg--current { color: var(--color-text-1); font-weight: 600; cursor: default; }
.path-sep { color: var(--color-text-3); flex-shrink: 0; }

.content-header__actions {
  display: flex; gap: 6px; flex-shrink: 0;
}
.action-btn {
  width: 30px; height: 30px;
  display: flex; align-items: center; justify-content: center;
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-sm);
  background: var(--color-background-mute);
  color: var(--color-text-2); cursor: pointer;
  transition: all 0.12s;
  &:hover { color: var(--color-text-1); background: var(--color-hover); border-color: var(--color-text-3); }
}

.content-body {
  flex: 1; overflow-y: auto; padding: 16px 20px;
}

.dir-empty {
  text-align: center; padding: 48px 16px; color: var(--color-text-3);
  p { margin: 0 0 4px; font-size: 14px; }
  .dir-empty__hint { font-size: 12px; opacity: 0.7; }
}

.item-list {
  display: flex; flex-direction: column; gap: 6px;
}
.item-card {
  display: flex; align-items: flex-start; gap: 12px;
  padding: 12px 14px; border-radius: var(--fox-radius-sm);
  border: 1px solid var(--color-border);
  background: var(--color-background-soft);
  transition: background 0.12s, border-color 0.12s;
  &:hover { background: var(--color-hover); border-color: var(--color-text-3); }
}
.item-card__icon {
  flex-shrink: 0; color: var(--color-icon); margin-top: 2px;
}
.item-card__info { flex: 1; min-width: 0; }
.item-card__name {
  font-size: 13px; font-weight: 500; color: var(--color-text-1);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.item-card__desc {
  font-size: 11px; color: var(--color-text-3); margin-top: 2px;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
</style>
