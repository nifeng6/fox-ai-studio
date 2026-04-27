<template>
  <div class="miniprogram-page">
    <header class="head">
      <h1 class="title">{{ t('miniprogram.title') }}</h1>
      <el-input
        v-model="query"
        clearable
        :placeholder="t('pageUi.searchPlaceholder')"
        class="search"
      >
        <template #prefix>
          <el-icon><Search /></el-icon>
        </template>
      </el-input>
      <el-button type="primary" class="btn-accent" @click="openAdd">
        <el-icon class="m-r-1"><Plus /></el-icon>
        {{ t('miniprogram.add') }}
      </el-button>
    </header>

    <div v-if="!filtered.length" class="empty">{{ t('miniprogram.empty') }}</div>
    <div v-else class="grid">
      <button
        v-for="p in filtered"
        :key="p.id"
        type="button"
        class="tile"
        @click="openUrl(p.url)"
        @contextmenu.prevent="openEdit(p)"
      >
        <span class="icon" aria-hidden="true">{{ p.icon }}</span>
        <span class="name">{{ p.name }}</span>
        <span class="actions">
          <el-button size="small" text @click.stop="openUrl(p.url)">{{ t('pageUi.miniprogramOpen') }}</el-button>
          <el-button size="small" text @click.stop="openEdit(p)">{{ t('pageUi.miniprogramEdit') }}</el-button>
        </span>
      </button>
    </div>

    <el-dialog
      v-model="dialogOpen"
      :title="editingId ? t('pageUi.miniprogramEdit') : t('pageUi.miniprogramAddTitle')"
      width="420px"
      align-center
      destroy-on-close
    >
      <el-form label-position="top">
        <el-form-item :label="t('pageUi.miniprogramIcon')">
          <el-input v-model="form.icon" maxlength="4" show-word-limit />
        </el-form-item>
        <el-form-item :label="t('miniprogram.name')">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item :label="t('miniprogram.url')">
          <el-input v-model="form.url" type="url" placeholder="https://" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button v-if="editingId" type="danger" text @click="removeCurrent">{{ t('common.delete') }}</el-button>
        <el-button @click="dialogOpen = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="save">{{ t('common.save') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Search, Plus } from '@element-plus/icons-vue'
import { useMiniprogramStore } from '@/stores/miniprogram'
import { storeToRefs } from 'pinia'
import type { MiniProgram } from '@/types'

const { t } = useI18n()
const store = useMiniprogramStore()
const { sorted } = storeToRefs(store)

onMounted(() => store.initPresets())

const query = ref('')
const filtered = computed(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return sorted.value
  return sorted.value.filter(p => p.name.toLowerCase().includes(q) || p.url.toLowerCase().includes(q))
})

const dialogOpen = ref(false)
const editingId = ref<string | null>(null)
const form = reactive({ name: '', url: '', icon: '🧩' })

function openAdd() {
  editingId.value = null
  form.name = ''
  form.url = ''
  form.icon = '🧩'
  dialogOpen.value = true
}

function openEdit(p: MiniProgram) {
  editingId.value = p.id
  form.name = p.name
  form.url = p.url
  form.icon = p.icon
  dialogOpen.value = true
}

function save() {
  if (!form.name.trim() || !form.url.trim()) return
  if (editingId.value) {
    store.updateProgram(editingId.value, { name: form.name, url: form.url, icon: form.icon })
  } else {
    store.addProgram({ name: form.name, url: form.url, icon: form.icon })
  }
  dialogOpen.value = false
}

function removeCurrent() {
  if (editingId.value) {
    store.removeProgram(editingId.value)
  }
  dialogOpen.value = false
}

function openUrl(url: string) {
  let u = url.trim()
  if (!/^https?:\/\//i.test(u)) u = `https://${u}`
  window.open(u, '_blank', 'noopener,noreferrer')
}
</script>

<style lang="scss" scoped>
.miniprogram-page {
  min-height: 100%;
  padding: 20px 24px 40px;
  background: var(--color-background);
  color: var(--color-text-1);
}

.head {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  align-items: center;
  margin-bottom: 24px;
  .title {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
    flex: 0 0 auto;
  }
  .search {
    flex: 1;
    min-width: 200px;
    max-width: 400px;
    :deep(.el-input__wrapper) {
      border-radius: var(--fox-radius-md);
    }
  }
}

.m-r-1 {
  margin-right: 4px;
}

.btn-accent {
  background: var(--fox-accent-fg) !important;
  border-color: var(--fox-accent-border) !important;
  color: var(--fox-accent-on) !important;
  border-radius: var(--fox-radius-sm);
}

.empty {
  text-align: center;
  padding: 40px 16px;
  color: var(--color-text-3);
  border: 1px dashed var(--color-border);
  border-radius: var(--fox-radius-md);
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 14px;
}

.tile {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 20px 12px 12px;
  border-radius: var(--fox-radius-lg);
  background: var(--color-background-soft);
  border: 1px solid var(--color-border);
  cursor: pointer;
  color: inherit;
  font: inherit;
  transition: box-shadow 0.2s, transform 0.2s;
  &:hover {
    box-shadow: var(--shadow-md);
    transform: translateY(-2px);
  }
  &:focus-visible {
    outline: 2px solid var(--fox-accent-border);
    outline-offset: 2px;
  }
}

.icon {
  font-size: 2rem;
  line-height: 1;
  margin-bottom: 10px;
}

.name {
  font-weight: 600;
  font-size: 0.95rem;
  margin-bottom: 8px;
  word-break: break-word;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 4px;
  opacity: 0.9;
  :deep(.el-button) {
    padding: 2px 6px;
  }
}
</style>
