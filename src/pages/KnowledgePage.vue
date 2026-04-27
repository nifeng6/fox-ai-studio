<template>
  <div class="knowledge-page">
    <h1 class="title">{{ t('knowledge.title') }}</h1>
    <div class="split">
      <aside class="panel left">
        <div class="panel-head">
          <el-button type="primary" class="btn-accent" @click="openNewBase">
            <el-icon class="m-r-1"><FolderAdd /></el-icon>
            {{ t('knowledge.create') }}
          </el-button>
        </div>
        <div v-if="loading" class="muted">{{ t('common.loading') }}</div>
        <ul v-else class="base-list" role="list">
          <li
            v-for="b in bases"
            :key="b.id"
            :class="['base-item', { active: currentBaseId === b.id }]"
            @click="selectBase(b.id)"
          >
            <div class="base-name-row">
              <span class="base-name">{{ b.name }}</span>
              <el-button
                size="small"
                text
                type="danger"
                @click.stop="confirmDelete(b)"
              >
                {{ t('common.delete') }}
              </el-button>
            </div>
            <div class="base-meta">
              {{ b.documentCount }} {{ t('pageUi.docCount') }}
            </div>
          </li>
        </ul>
        <p v-if="!loading && !bases.length" class="empty-side">{{ t('knowledge.empty') }}</p>
      </aside>

      <main class="panel right">
        <template v-if="currentBase">
          <div class="right-head">
            <h2>{{ currentBase.name }}</h2>
            <p v-if="currentBase.description" class="desc">{{ currentBase.description }}</p>
          </div>

          <div class="row gap">
            <el-button class="btn-accent" @click="addDocOpen = true">
              {{ t('knowledge.addDocument') }}
            </el-button>
          </div>

          <div class="search-block">
            <el-input
              v-model="searchText"
              :placeholder="t('knowledge.search')"
              clearable
              @keydown.enter="runSearch"
            >
              <template #append>
                <el-button :loading="searchLoading" @click="runSearch">
                  {{ t('pageUi.searchInKb') }}
                </el-button>
              </template>
            </el-input>
            <p class="hint">{{ t('pageUi.selectKb') }}</p>
          </div>

          <div v-if="searchHits.length" class="hit-list">
            <div v-for="(h, i) in searchHits" :key="i" class="hit-card">
              <pre class="hit-pre">{{ String(h.content || h.text || JSON.stringify(h, null, 2)) }}</pre>
            </div>
          </div>
          <div v-else-if="searchRan && !searchLoading" class="muted pad">{{ t('pageUi.noResults') }}</div>

          <h3 class="h3">{{ t('knowledge.documents') }}</h3>
          <ul v-if="docsForBase.length" class="doc-list">
            <li v-for="d in docsForBase" :key="d.id" class="doc-item">
              <span class="doc-name">{{ d.name }}</span>
              <span class="doc-type">{{ d.type }}</span>
            </li>
          </ul>
          <p v-else class="muted pad">{{ t('knowledge.empty') }}</p>
        </template>
        <div v-else class="empty-main">
          {{ t('pageUi.selectKb') }}
        </div>
      </main>
    </div>

    <el-dialog
      v-model="newBaseOpen"
      :title="t('knowledge.create')"
      width="440px"
      align-center
      destroy-on-close
    >
      <el-form label-position="top">
        <el-form-item :label="t('assistant.name')">
          <el-input v-model="newBase.name" />
        </el-form-item>
        <el-form-item :label="t('assistant.description')">
          <el-input v-model="newBase.description" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="newBaseOpen = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="savingBase" @click="submitBase">{{ t('common.save') }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="addDocOpen"
      :title="t('pageUi.addDocTitle')"
      width="500px"
      align-center
      destroy-on-close
    >
      <el-form label-position="top">
        <el-form-item :label="t('pageUi.docName')">
          <el-input v-model="newDoc.name" />
        </el-form-item>
        <el-form-item :label="t('pageUi.docType')">
          <el-input v-model="newDoc.type" placeholder="md / txt" />
        </el-form-item>
        <el-form-item :label="t('pageUi.docContent')">
          <el-input v-model="newDoc.content" type="textarea" :rows="6" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="addDocOpen = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="addDocLoading" @click="submitDocument">{{ t('common.save') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { FolderAdd } from '@element-plus/icons-vue'
import { v4 as uuidv4 } from 'uuid'
import { useKnowledgeStore } from '@/stores/knowledge'
import { storeToRefs } from 'pinia'

const { t } = useI18n()
const knowledge = useKnowledgeStore()
const { bases, currentBaseId, loading } = storeToRefs(knowledge)

const docsByBase = ref<Record<string, { id: string; name: string; type: string; createdAt: number }[]>>({})

const currentBase = computed(() => bases.value.find(b => b.id === currentBaseId.value) ?? null)
const docsForBase = computed(() => (currentBaseId.value && docsByBase.value[currentBaseId.value]) || [])

const newBaseOpen = ref(false)
const savingBase = ref(false)
const newBase = reactive({ name: '', description: '' })

const addDocOpen = ref(false)
const addDocLoading = ref(false)
const newDoc = reactive({ name: '', type: 'md', content: '' })

const searchText = ref('')
const searchHits = ref<any[]>([])
const searchLoading = ref(false)
const searchRan = ref(false)

onMounted(() => {
  void knowledge.loadBases()
})

function selectBase(id: string) {
  knowledge.currentBaseId = id
  searchHits.value = []
  searchRan.value = false
}

function openNewBase() {
  newBase.name = ''
  newBase.description = ''
  newBaseOpen.value = true
}

async function submitBase() {
  if (!newBase.name.trim()) return
  savingBase.value = true
  try {
    await knowledge.createBase({ name: newBase.name, description: newBase.description })
    newBaseOpen.value = false
    ElMessage.success(t('common.success'))
  } catch {
    ElMessage.error(t('common.error'))
  } finally {
    savingBase.value = false
  }
}

async function confirmDelete(b: { id: string; name: string }) {
  try {
    await ElMessageBox.confirm(
      t('pageUi.knowledgeDeleteConfirm'),
      { type: 'warning' }
    )
    await knowledge.deleteBase(b.id)
    docsByBase.value = { ...docsByBase.value, [b.id]: [] }
  } catch { /* user cancel */ }
}

async function submitDocument() {
  if (!currentBaseId.value || !newDoc.name.trim()) return
  addDocLoading.value = true
  try {
    await knowledge.addDocument(currentBaseId.value, { ...newDoc })
    const list = docsByBase.value[currentBaseId.value!] || []
    docsByBase.value = {
      ...docsByBase.value,
      [currentBaseId.value!]: [
        { id: uuidv4(), name: newDoc.name, type: newDoc.type, createdAt: Date.now() },
        ...list
      ]
    }
    addDocOpen.value = false
    newDoc.name = ''
    newDoc.content = ''
    ElMessage.success(t('common.success'))
  } catch {
    ElMessage.error(t('common.error'))
  } finally {
    addDocLoading.value = false
  }
}

async function runSearch() {
  if (!currentBaseId.value || !searchText.value.trim()) {
    searchHits.value = []
    return
  }
  searchLoading.value = true
  searchRan.value = true
  try {
    const res = await knowledge.search(currentBaseId.value, searchText.value, 8)
    searchHits.value = Array.isArray(res) ? res : []
  } catch {
    searchHits.value = []
    ElMessage.error(t('common.error'))
  } finally {
    searchLoading.value = false
  }
}
</script>

<style lang="scss" scoped>
.knowledge-page {
  height: 100%;
  min-height: 0;
  padding: 20px 24px 24px;
  background: var(--color-background);
  color: var(--color-text-1);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.title {
  margin: 0 0 20px;
  font-size: 1.5rem;
  font-weight: 600;
}

.split {
  display: grid;
  grid-template-columns: 260px 1fr;
  gap: 16px;
  flex: 1;
  min-height: 0;
}

.panel {
  border-radius: var(--fox-radius-lg);
  border: 1px solid var(--color-border);
  background: var(--color-background-soft);
  padding: 16px;
}

.left {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.panel-head {
  display: flex;
  justify-content: flex-start;
}

.btn-accent {
  background: var(--fox-accent-fg) !important;
  border-color: var(--fox-accent-border) !important;
  color: var(--fox-accent-on) !important;
  border-radius: var(--fox-radius-sm);
  &:hover {
    filter: brightness(1.05);
  }
}

.m-r-1 {
  margin-right: 4px;
}

.base-list {
  list-style: none;
  margin: 0;
  padding: 0;
  flex: 1;
  overflow: auto;
}

.base-item {
  padding: 12px 12px;
  border-radius: var(--fox-radius-sm);
  cursor: pointer;
  border: 1px solid transparent;
  margin-bottom: 6px;
  transition: background 0.2s, border-color 0.2s;
  &:hover {
    background: var(--color-hover);
  }
  &.active {
    background: var(--color-hover);
    border-color: var(--fox-accent-border);
  }
}

.base-name-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.base-name {
  font-weight: 600;
  font-size: 0.95rem;
}

.base-meta {
  font-size: 0.8rem;
  color: var(--color-text-3);
  margin-top: 4px;
}

.empty-side,
.empty-main {
  color: var(--color-text-3);
  text-align: center;
  padding: 24px 8px;
}

.right-head {
  margin-bottom: 16px;
  h2 {
    margin: 0 0 6px;
    font-size: 1.2rem;
  }
  .desc {
    margin: 0;
    color: var(--color-text-2);
    font-size: 0.9rem;
  }
}

.row.gap {
  margin-bottom: 16px;
}

.search-block {
  margin-bottom: 20px;
  .hint {
    font-size: 0.8rem;
    color: var(--color-text-3);
    margin: 8px 0 0;
  }
}

.h3 {
  font-size: 1rem;
  margin: 20px 0 10px;
}

.doc-list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.doc-item {
  display: flex;
  justify-content: space-between;
  padding: 10px 12px;
  border-radius: var(--fox-radius-sm);
  border: 1px solid var(--color-border);
  margin-bottom: 8px;
  font-size: 0.9rem;
}
.doc-type {
  color: var(--color-text-3);
  font-size: 0.8rem;
}

.hit-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 20px;
  max-height: 240px;
  overflow: auto;
}

.hit-card {
  padding: 10px 12px;
  border-radius: var(--fox-radius-sm);
  background: var(--color-background-mute);
  border: 1px solid var(--color-border);
}

.hit-pre {
  margin: 0;
  font-size: 0.8rem;
  color: var(--color-text-2);
  white-space: pre-wrap;
  word-break: break-word;
  font-family: ui-monospace, monospace;
}

.muted {
  color: var(--color-text-3);
  font-size: 0.9rem;
}
.pad {
  padding: 12px 0;
}
</style>
