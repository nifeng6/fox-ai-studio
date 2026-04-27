<template>
  <div class="assistants-page">
    <header class="page-header">
      <h1 class="title">{{ t('assistant.title') }}</h1>
      <p class="subtitle">{{ t('pageUi.categoryFilter') }}</p>
    </header>

    <div class="toolbar">
      <el-input
        v-model="searchQuery"
        :placeholder="t('assistant.search')"
        clearable
        class="search-input"
      >
        <template #prefix>
          <el-icon><Search /></el-icon>
        </template>
      </el-input>
      <el-button type="primary" class="create-btn" @click="openCreate">
        <el-icon class="m-r-1"><Plus /></el-icon>
        {{ t('assistant.create') }}
      </el-button>
    </div>

    <div class="filter-row" role="tablist" :aria-label="t('pageUi.categoryFilter')">
      <button
        v-for="cat in categories"
        :key="cat"
        type="button"
        class="pill"
        :class="{ active: selectedCategory === cat }"
        @click="selectedCategory = cat"
      >
        {{ cat }}
      </button>
    </div>

    <div v-if="!filteredAssistants.length" class="empty">
      {{ t('pageUi.noResults') }}
    </div>
    <div v-else class="card-grid">
      <article v-for="a in filteredAssistants" :key="a.id" class="card" @click="openDetail(a)">
        <div class="card-top">
          <span class="cat-tag">{{ a.category }}</span>
          <div class="card-title">{{ a.name }}</div>
          <p class="card-desc">{{ a.description }}</p>
        </div>
        <div class="card-footer">
          <div v-if="a.skillIds?.length" class="card-skills">
            <span v-for="sid in a.skillIds.slice(0, 3)" :key="sid" class="skill-chip">
              {{ getSkillName(sid) }}
            </span>
            <span v-if="a.skillIds.length > 3" class="skill-chip skill-chip--more">
              +{{ a.skillIds.length - 3 }}
            </span>
          </div>
          <div class="card-actions">
            <el-button size="small" round @click.stop="goToChatWith(a)">{{ t('pageUi.use') }}</el-button>
            <el-button size="small" text @click.stop="openEdit(a)">{{ t('common.edit') }}</el-button>
          </div>
        </div>
      </article>
    </div>

    <!-- Detail Dialog -->
    <el-dialog
      v-model="detailVisible"
      :show-close="true"
      width="680px"
      class="assistant-detail-dialog"
      destroy-on-close
      align-center
    >
      <template #header>
        <div class="detail-header">
          <div class="detail-avatar">
            {{ detailAssistant?.name?.charAt(0) || '?' }}
          </div>
          <div class="detail-meta">
            <h2 class="detail-name">{{ detailAssistant?.name }}</h2>
            <p class="detail-desc">{{ detailAssistant?.description }}</p>
          </div>
        </div>
      </template>
      <div v-if="detailAssistant" class="detail-body">
        <div class="detail-section">
          <div class="detail-section__title">关于</div>
          <div class="detail-section__content">
            {{ detailAssistant.systemPrompt || '暂无系统提示词' }}
          </div>
        </div>

        <div class="detail-section">
          <div class="detail-section__title">技能</div>
          <div class="detail-section__content">
            <div v-if="detailSkills.length" class="detail-skills">
              <span v-for="sk in detailSkills" :key="sk.id" class="detail-skill-tag">
                {{ sk.name }}
              </span>
            </div>
            <span v-else class="detail-empty-hint">暂无绑定技能</span>
          </div>
        </div>

        <div class="detail-section">
          <div class="detail-section__title">示例</div>
          <div class="detail-section__content">
            <div v-if="detailAssistant.examples?.length" class="detail-examples">
              <div
                v-for="(ex, i) in detailAssistant.examples"
                :key="i"
                class="detail-example"
                @click="goToChatWithExample(detailAssistant, ex)"
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" /></svg>
                <span>{{ ex }}</span>
              </div>
            </div>
            <span v-else class="detail-empty-hint">暂无示例</span>
          </div>
        </div>

        <div class="detail-section">
          <div class="detail-section__title">参数</div>
          <div class="detail-section__content detail-params">
            <div class="detail-param">
              <span class="detail-param__label">Temperature</span>
              <span class="detail-param__value">{{ detailAssistant.temperature }}</span>
            </div>
            <div class="detail-param">
              <span class="detail-param__label">Max Tokens</span>
              <span class="detail-param__value">{{ detailAssistant.maxTokens }}</span>
            </div>
          </div>
        </div>
      </div>

      <template #footer>
        <el-button @click="detailVisible = false">关闭</el-button>
        <el-button @click="openEditFromDetail">{{ t('common.edit') }}</el-button>
        <el-button type="primary" @click="goToChatWith(detailAssistant!)">{{ t('pageUi.use') }}</el-button>
      </template>
    </el-dialog>

    <!-- Create / Edit Dialog -->
    <el-dialog
      v-model="dialogVisible"
      :title="editingId ? t('assistant.edit') : t('pageUi.createAssistantTitle')"
      width="560px"
      class="assistant-edit-dialog"
      destroy-on-close
      align-center
    >
      <el-form label-position="top" class="form">
        <el-form-item :label="t('assistant.name')">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item :label="t('assistant.description')">
          <el-input v-model="form.description" type="textarea" :rows="2" />
        </el-form-item>
        <el-form-item :label="t('assistant.category')">
          <el-input v-model="form.category" />
        </el-form-item>
        <el-form-item :label="t('assistant.systemPrompt')">
          <el-input v-model="form.systemPrompt" type="textarea" :rows="5" />
        </el-form-item>
        <el-form-item label="绑定技能">
          <el-select
            v-model="form.skillIds"
            multiple
            filterable
            placeholder="选择技能"
            class="w-full"
          >
            <el-option
              v-for="sk in allSkills"
              :key="sk.id"
              :label="sk.name"
              :value="sk.id"
            >
              <div class="skill-option">
                <span>{{ sk.name }}</span>
                <span class="skill-option__desc">{{ sk.description }}</span>
              </div>
            </el-option>
          </el-select>
        </el-form-item>
        <el-form-item label="示例提示词">
          <div class="examples-editor">
            <div
              v-for="(ex, i) in form.examples"
              :key="i"
              class="example-row"
            >
              <el-input
                v-model="form.examples[i]"
                placeholder="输入示例提示词..."
                size="small"
              />
              <el-button
                type="danger"
                link
                size="small"
                @click="form.examples.splice(i, 1)"
              >删除</el-button>
            </div>
            <el-button size="small" @click="form.examples.push('')">
              + 添加示例
            </el-button>
          </div>
        </el-form-item>
        <el-form-item :label="t('assistant.temperature')">
          <el-slider v-model="form.temperature" :min="0" :max="2" :step="0.05" show-input />
        </el-form-item>
        <el-form-item :label="t('assistant.maxTokens')">
          <el-input-number v-model="form.maxTokens" :min="256" :max="128000" :step="256" class="w-full" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="saveAssistant">{{ t('common.save') }}</el-button>
      </template>
    </el-dialog>

    <!-- Use Dialog -->
    <el-dialog
      v-model="useDialogVisible"
      title="使用助手"
      width="440px"
      class="assistant-use-dialog"
      destroy-on-close
      align-center
    >
      <div v-if="useTarget" class="use-dialog">
        <div class="use-dialog__info">
          <div class="use-dialog__avatar">{{ useTarget.name?.charAt(0) }}</div>
          <div class="use-dialog__name">{{ useTarget.name }}</div>
        </div>
        <el-radio-group v-model="useMode" class="use-dialog__mode">
          <el-radio value="new">创建新对话</el-radio>
          <el-radio value="existing">添加到已有对话（作为子 Agent）</el-radio>
        </el-radio-group>
        <div v-if="useMode === 'existing'" class="use-dialog__topic-list">
          <el-select
            v-model="useTargetTopicId"
            placeholder="选择对话"
            class="w-full"
            filterable
          >
            <el-option
              v-for="tp in existingTopics"
              :key="tp.id"
              :label="tp.title"
              :value="tp.id"
            />
          </el-select>
        </div>
      </div>
      <template #footer>
        <el-button @click="useDialogVisible = false">取消</el-button>
        <el-button
          type="primary"
          :disabled="useMode === 'existing' && !useTargetTopicId"
          @click="confirmUseAssistant"
        >确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { Search, Plus } from '@element-plus/icons-vue'
import { useAssistantStore } from '@/stores/assistant'
import { useSkillStore } from '@/stores/skill'
import { useChatStore } from '@/stores/chat'
import { useAgentStore } from '@/stores/agent'
import type { Assistant } from '@/types'
import { storeToRefs } from 'pinia'

const { t } = useI18n()
const router = useRouter()
const assistantStore = useAssistantStore()
const skillStore = useSkillStore()
const chatStore = useChatStore()
const agentStore = useAgentStore()
const { searchQuery, selectedCategory, categories, filteredAssistants } = storeToRefs(assistantStore)

onMounted(() => {
  assistantStore.initPresets()
  skillStore.ensurePresets()
})

const allSkills = computed(() => skillStore.skills)

function getSkillName(id: string): string {
  return skillStore.skills.find(s => s.id === id)?.name || id.slice(0, 6)
}

const detailVisible = ref(false)
const detailAssistant = ref<Assistant | null>(null)

const detailSkills = computed(() => {
  if (!detailAssistant.value?.skillIds?.length) return []
  return detailAssistant.value.skillIds
    .map(id => skillStore.skills.find(s => s.id === id))
    .filter(Boolean) as Array<{ id: string; name: string; description: string }>
})

function openDetail(a: Assistant) {
  detailAssistant.value = a
  detailVisible.value = true
}

function openEditFromDetail() {
  if (detailAssistant.value) {
    openEdit(detailAssistant.value)
    detailVisible.value = false
  }
}

const dialogVisible = ref(false)
const editingId = ref<string | null>(null)
const form = reactive({
  name: '',
  description: '',
  category: '自定义',
  systemPrompt: '',
  temperature: 0.7,
  maxTokens: 4096,
  skillIds: [] as string[],
  examples: [] as string[]
})

function openCreate() {
  editingId.value = null
  Object.assign(form, {
    name: '',
    description: '',
    category: '自定义',
    systemPrompt: '',
    temperature: 0.7,
    maxTokens: 4096,
    skillIds: [],
    examples: []
  })
  dialogVisible.value = true
}

function openEdit(a: Assistant) {
  editingId.value = a.id
  Object.assign(form, {
    name: a.name,
    description: a.description,
    category: a.category,
    systemPrompt: a.systemPrompt,
    temperature: a.temperature,
    maxTokens: a.maxTokens,
    skillIds: [...(a.skillIds || [])],
    examples: [...(a.examples || [])]
  })
  dialogVisible.value = true
}

function saveAssistant() {
  const cleanExamples = form.examples.filter(e => e.trim())
  if (editingId.value) {
    assistantStore.updateAssistant(editingId.value, {
      ...form,
      examples: cleanExamples,
      skillIds: [...form.skillIds]
    })
  } else {
    assistantStore.createAssistant({
      ...form,
      examples: cleanExamples,
      skillIds: [...form.skillIds]
    })
  }
  dialogVisible.value = false
}

const useDialogVisible = ref(false)
const useTarget = ref<Assistant | null>(null)
const useMode = ref<'new' | 'existing'>('new')
const useTargetTopicId = ref('')
const usePrompt = ref('')

const existingTopics = computed(() => chatStore.topics)

function goToChatWith(a: Assistant) {
  useTarget.value = a
  useMode.value = 'new'
  useTargetTopicId.value = ''
  usePrompt.value = ''
  useDialogVisible.value = true
}

function goToChatWithExample(a: Assistant, example: string) {
  useTarget.value = a
  useMode.value = 'new'
  useTargetTopicId.value = ''
  usePrompt.value = example
  useDialogVisible.value = true
  detailVisible.value = false
}

function confirmUseAssistant() {
  const a = useTarget.value
  if (!a) return

  if (useMode.value === 'new') {
    assistantStore.currentAssistantId = a.id
    const mainAgent = agentStore.getMainAgent() || agentStore.createMainAgent()
    const allAgentIds = agentStore.agents.map(ag => ag.id)
    chatStore.createTopic({
      title: a.name,
      assistantId: a.id,
      participantAgentIds: allAgentIds,
      activeTab: mainAgent.id
    })
    agentStore.switchAgent(mainAgent.id)
    if (a.systemPrompt) {
      agentStore.updateAgent(mainAgent.id, { systemPrompt: a.systemPrompt })
    }
    if (a.skillIds?.length) {
      agentStore.updateAgent(mainAgent.id, { skillIds: [...a.skillIds] })
    }
    const query: Record<string, string> = { assistant: a.id }
    if (usePrompt.value) query.prompt = usePrompt.value
    router.push({ path: '/chat', query })
  } else {
    if (!useTargetTopicId.value) return
    const mainAgent = agentStore.getMainAgent()
    if (!mainAgent) return
    const sub = agentStore.createSubAgent(mainAgent.id, {
      name: a.name,
      systemPrompt: a.systemPrompt || '',
      skillIds: a.skillIds || []
    })
    if (sub) {
      const topic = chatStore.topics.find(tp => tp.id === useTargetTopicId.value)
      if (topic) {
        const pids = topic.participantAgentIds || []
        if (!pids.includes(sub.id)) {
          topic.participantAgentIds = [...pids, sub.id]
        }
      }
      agentStore.switchAgent(sub.id)
      chatStore.currentTopicId = useTargetTopicId.value
      chatStore.setActiveTab(useTargetTopicId.value, sub.id)
    }
    router.push({ path: '/chat' })
  }

  useDialogVisible.value = false
  detailVisible.value = false
}
</script>

<style lang="scss" scoped>
.assistants-page {
  height: 100%;
  overflow-y: auto;
  padding: 24px 28px 40px;
  background: var(--color-background);
  color: var(--color-text-1);
}
.assistants-page::-webkit-scrollbar { width: 5px; }
.assistants-page::-webkit-scrollbar-thumb { background: var(--color-border); border-radius: 4px; }

.page-header {
  margin-bottom: 20px;
  .title {
    margin: 0 0 6px;
    font-size: 1.5rem;
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .subtitle {
    margin: 0;
    color: var(--color-text-3);
    font-size: 0.9rem;
  }
}

.toolbar {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
  flex-wrap: wrap;
  align-items: center;
  .search-input {
    flex: 1;
    min-width: 200px;
    max-width: 420px;
  }
  :deep(.el-input__wrapper) {
    border-radius: var(--fox-radius-md);
    background: var(--color-background-soft);
    box-shadow: 0 0 0 1px var(--color-border);
  }
}

.m-r-1 { margin-right: 4px; }

.create-btn {
  border-radius: var(--fox-radius-sm);
  background: var(--fox-accent-fg) !important;
  border-color: var(--fox-accent-border) !important;
  color: var(--fox-accent-on) !important;
  &:hover { filter: brightness(1.08); }
}

.filter-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 20px;
}

.pill {
  border: 1px solid var(--color-border);
  background: var(--color-background-soft);
  color: var(--color-text-2);
  padding: 6px 14px;
  border-radius: 999px;
  font-size: 0.85rem;
  cursor: pointer;
  transition: background 0.15s, color 0.15s, border-color 0.15s;
  &:hover { background: var(--color-hover); }
  &.active {
    background: var(--color-hover);
    color: var(--color-text-1);
    border-color: var(--fox-accent-border);
    box-shadow: 0 0 0 1px var(--fox-accent-border);
  }
}

.card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 16px;
}

.card {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 18px;
  border-radius: var(--fox-radius-lg);
  background: var(--color-background-soft);
  border: 1px solid var(--color-border);
  box-shadow: var(--shadow-sm);
  min-height: 180px;
  cursor: pointer;
  transition: box-shadow 0.2s, transform 0.2s;
  &:hover {
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }
}

.card-top { margin-bottom: 12px; }

.cat-tag {
  display: inline-block;
  font-size: 0.7rem;
  padding: 2px 8px;
  border-radius: var(--fox-radius-xs);
  background: var(--color-primary-mute);
  color: var(--color-text-2);
  margin-bottom: 8px;
}

.card-title {
  font-weight: 600;
  font-size: 1.05rem;
  margin-bottom: 8px;
}

.card-desc {
  margin: 0;
  font-size: 0.88rem;
  color: var(--color-text-2);
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.card-footer {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.card-skills {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.skill-chip {
  display: inline-block;
  font-size: 0.72rem;
  padding: 2px 8px;
  border-radius: var(--fox-radius-xs);
  background: var(--color-background-mute);
  color: var(--color-text-2);
  border: 1px solid var(--color-border);
}
.skill-chip--more {
  color: var(--color-text-3);
  background: transparent;
  border-style: dashed;
}

.card-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.empty {
  text-align: center;
  padding: 48px 16px;
  color: var(--color-text-3);
  border-radius: var(--fox-radius-md);
  border: 1px dashed var(--color-border);
}

/* Detail Dialog */
.detail-header {
  display: flex;
  align-items: center;
  gap: 14px;
}
.detail-avatar {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  background: var(--fox-accent-fg);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.3rem;
  font-weight: 700;
  flex-shrink: 0;
}
.detail-meta { min-width: 0; }
.detail-name {
  margin: 0;
  font-size: 1.15rem;
  font-weight: 600;
  color: var(--color-text-1);
}
.detail-desc {
  margin: 4px 0 0;
  font-size: 0.85rem;
  color: var(--color-text-3);
}
.detail-body {
  display: flex;
  flex-direction: column;
  gap: 20px;
  max-height: 55vh;
  overflow-y: auto;
  padding-right: 4px;
}
.detail-body::-webkit-scrollbar { width: 4px; }
.detail-body::-webkit-scrollbar-thumb { background: var(--color-border); border-radius: 3px; }
.detail-section__title {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--color-text-2);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  margin-bottom: 8px;
}
.detail-section__content {
  font-size: 0.9rem;
  color: var(--color-text-1);
  line-height: 1.6;
  background: var(--color-background-soft);
  border-radius: var(--fox-radius-md);
  padding: 12px 14px;
  border: 1px solid var(--color-border);
}
.detail-skills {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.detail-skill-tag {
  display: inline-block;
  padding: 4px 12px;
  border-radius: var(--fox-radius-sm);
  background: var(--color-background-mute);
  color: var(--color-text-1);
  font-size: 0.82rem;
  border: 1px solid var(--color-border);
}
.detail-empty-hint {
  color: var(--color-text-3);
  font-size: 0.85rem;
}
.detail-examples {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.detail-example {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 8px 10px;
  border-radius: var(--fox-radius-sm);
  cursor: pointer;
  transition: background 0.15s;
  color: var(--color-text-1);
  &:hover {
    background: var(--color-hover);
  }
  svg {
    flex-shrink: 0;
    margin-top: 2px;
    color: var(--color-text-3);
  }
  span {
    line-height: 1.5;
  }
}
.detail-params {
  display: flex;
  gap: 20px;
}
.detail-param {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.detail-param__label {
  font-size: 0.78rem;
  color: var(--color-text-3);
}
.detail-param__value {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--color-text-1);
}

/* Edit Dialog */
.form {
  max-height: 65vh;
  overflow-y: auto;
  padding-right: 4px;
  :deep(.el-form-item__label) {
    color: var(--color-text-2);
  }
}
.w-full { width: 100%; }
.skill-option {
  display: flex;
  align-items: center;
  gap: 8px;
}
.skill-option__desc {
  font-size: 0.78rem;
  color: var(--color-text-3);
}
.examples-editor {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.example-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* Use Dialog */
.use-dialog {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.use-dialog__info {
  display: flex;
  align-items: center;
  gap: 10px;
}
.use-dialog__avatar {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: var(--fox-accent-fg);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1rem;
  font-weight: 700;
  flex-shrink: 0;
}
.use-dialog__name {
  font-size: 1rem;
  font-weight: 600;
  color: var(--color-text-1);
}
.use-dialog__mode {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.use-dialog__topic-list {
  padding-left: 24px;
}
</style>

<style lang="scss">
.assistant-detail-dialog .el-dialog,
.assistant-edit-dialog .el-dialog,
.assistant-use-dialog .el-dialog {
  border-radius: var(--fox-radius-lg) !important;
  background: var(--modal-background, var(--color-background)) !important;
  border: 1px solid var(--color-border) !important;
}
</style>
