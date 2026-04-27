<template>
  <div class="notes-page">
    <aside class="n-sidebar">
      <div class="side-tools">
        <el-input
          v-model="searchQuery"
          :placeholder="t('notes.search')"
          clearable
          size="small"
        />
        <div class="side-btns">
          <el-button size="small" @click="newFolder">{{ t('notes.newFolder') }}</el-button>
          <el-button size="small" type="primary" class="btn-accent" @click="addNote">
            {{ t('notes.create') }}
          </el-button>
        </div>
      </div>
      <div class="folder-block">
        <div
          :class="['tree-row', { on: !currentFolderId }]"
          @click="setFolder(null)"
        >
          {{ t('notes.breadcrumbAll') }}
        </div>
        <div
          v-for="f in folders"
          :key="f.id"
          :class="['tree-row', { on: currentFolderId === f.id }]"
          @click="setFolder(f.id)"
        >
          <span class="fi">📁</span> {{ f.name }}
          <el-button
            type="danger"
            text
            size="small"
            class="f-del"
            @click.stop="removeFolder(f.id)"
          >×</el-button>
        </div>
      </div>
      <ul class="note-list">
        <li
          v-for="n in filteredNotes"
          :key="n.id"
          :class="['nl-item', { active: n.id === currentId }]"
          @click="select(n.id)"
        >
          <span class="n-title">{{ n.title }}</span>
          <span class="n-t">{{ formatTime(n.updatedAt) }}</span>
        </li>
      </ul>
    </aside>

    <div class="n-main" v-if="currentNote">
      <header class="n-head">
        <nav class="crumbs" aria-label="Breadcrumb">
          <span>{{ t('notes.breadcrumbAll') }}</span>
          <span v-if="currentFolder" class="sep">/</span>
          <span v-if="currentFolder">{{ currentFolderName }}</span>
          <span class="sep">/</span>
          <el-input
            v-model="titleEdit"
            class="title-input"
            :placeholder="t('notes.create')"
            @change="onTitleChange"
          />
        </nav>
        <el-button
          size="small"
          type="danger"
          text
          @click="removeNote"
        >{{ t('notes.delete') }}</el-button>
      </header>

        <div
        v-if="editor"
        class="editor-wrap"
      >
        <div class="toolbar" role="toolbar" :aria-label="t('pageUi.notesToolbar')">
          <el-button-group size="small">
            <el-button @click="runCh(e => e.chain().focus().toggleBold().run())">B</el-button>
            <el-button @click="runCh(e => e.chain().focus().toggleItalic().run())"><i>I</i></el-button>
            <el-button @click="runCh(e => e.chain().focus().toggleUnderline().run())"><u>U</u></el-button>
            <el-button @click="runCh(e => e.chain().focus().toggleStrike().run())"><s>S</s></el-button>
            <el-button @click="runCh(e => e.chain().focus().toggleCode().run())">Code</el-button>
          </el-button-group>
          <el-color-picker
            v-model="colorHex"
            size="small"
            class="m-l"
            @change="onColor"
          />
          <el-button-group size="small" class="m-l">
            <el-button @click="runCh(e => e.chain().focus().setHeading({ level: 1 }).run())">H1</el-button>
            <el-button @click="runCh(e => e.chain().focus().setHeading({ level: 2 }).run())">H2</el-button>
            <el-button @click="runCh(e => e.chain().focus().setHeading({ level: 3 }).run())">H3</el-button>
          </el-button-group>
          <el-button-group size="small" class="m-l">
            <el-button @click="runCh(e => e.chain().focus().toggleBulletList().run())">•</el-button>
            <el-button @click="runCh(e => e.chain().focus().toggleOrderedList().run())">1.</el-button>
            <el-button @click="runCh(e => e.chain().focus().toggleTaskList().run())">{{ t('notes.taskList') }}</el-button>
          </el-button-group>
          <el-button-group size="small" class="m-l">
            <el-button @click="runCh(e => e.chain().focus().toggleCodeBlock().run())">{{ t('notes.codeBlock') }}</el-button>
            <el-button @click="runCh(e => e.chain().focus().toggleBlockquote().run())">»</el-button>
          </el-button-group>
          <el-button-group size="small" class="m-l">
            <el-button @click="align('left')">{{ t('notes.alignLeft') }}</el-button>
            <el-button @click="align('center')">{{ t('notes.alignCenter') }}</el-button>
            <el-button @click="align('right')">{{ t('notes.alignRight') }}</el-button>
          </el-button-group>
          <el-button-group size="small" class="m-l">
            <el-button @click="addTable">Table</el-button>
            <el-button @click="setLink">Link</el-button>
            <el-button @click="insertImage">{{ t('notes.image') || 'Img' }}</el-button>
            <el-button @click="runCh(e => e.chain().focus().undo().run())">↺</el-button>
            <el-button @click="runCh(e => e.chain().focus().redo().run())">↻</el-button>
          </el-button-group>
        </div>
        <div :class="['body-row', { split: showPreview }]" @click="onBodyRowClick">
          <div class="editor-box" @click="onEditorAreaClick">
            <EditorContent v-if="editor" :editor="editor" class="editor-content" />
          </div>
          <div
            v-if="showPreview"
            class="preview-box"
            v-html="previewHtml"
          />
        </div>
      </div>
      <div v-else class="loading-ed">{{ t('common.loading') }}</div>

      <footer class="n-foot">
        <span class="cc">{{ t('notes.charCount') }}: {{ charCount }}</span>
        <el-switch v-model="showPreview" :active-text="t('notes.preview')" />
      </footer>
    </div>

    <div v-else class="n-empty">
      <p>{{ t('pageUi.noteSelectOrCreate') }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessageBox } from 'element-plus'
import { useEditor, EditorContent } from '@tiptap/vue-3'
import type { Editor, JSONContent } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import Underline from '@tiptap/extension-underline'
import Link from '@tiptap/extension-link'
import TaskList from '@tiptap/extension-task-list'
import TaskItem from '@tiptap/extension-task-item'
import { Table, TableRow, TableHeader, TableCell } from '@tiptap/extension-table'
import { CodeBlockLowlight } from '@tiptap/extension-code-block-lowlight'
import Placeholder from '@tiptap/extension-placeholder'
import { TextStyle } from '@tiptap/extension-text-style'
import { Color } from '@tiptap/extension-color'
import TextAlign from '@tiptap/extension-text-align'
import Highlight from '@tiptap/extension-highlight'
import Image from '@tiptap/extension-image'
import { all, createLowlight } from 'lowlight'
import { useNotesStore } from '@/stores/notes'
import { fileApi } from '@/utils/tauri-api'
import { storeToRefs } from 'pinia'

const lowlight = createLowlight(all)
const { t } = useI18n()
const store = useNotesStore()
const {
  notes, folders, currentId, currentFolderId, searchQuery, filteredNotes, currentNote
} = storeToRefs(store)

const titleEdit = ref('')
const showPreview = ref(false)
const colorHex = ref('#1a1a1a')
const previewHtml = ref('')
const charCount = ref(0)

const currentFolder = computed(() =>
  currentFolderId.value ? folders.value.find(f => f.id === currentFolderId.value) : null
)
const currentFolderName = computed(() => currentFolder.value?.name || '')

let saveTimer: ReturnType<typeof setTimeout> | null = null

function parseToDocJSON(s: string): JSONContent {
  if (!s) return { type: 'doc', content: [] }
  const t0 = s.trim()
  if (t0.startsWith('{')) {
    try {
      const o = JSON.parse(s) as JSONContent
      if (o && (o as { type?: string }).type === 'doc') return o
    } catch { /* continue */ }
  }
  return {
    type: 'doc' as const,
    content: [
      { type: 'paragraph' as const, content: t0 ? [{ type: 'text' as const, text: s }] : [] }
    ]
  }
}

const editor = useEditor({
  extensions: [
    StarterKit.configure({ codeBlock: false, heading: { levels: [1, 2, 3] } }),
    CodeBlockLowlight.configure({ lowlight }),
    Underline,
    TextStyle,
    Color,
    TextAlign.configure({ types: ['heading', 'paragraph', 'blockquote'] }),
    Link.configure({ openOnClick: false, autolink: true, HTMLAttributes: { rel: 'noopener', class: 'tiptap-link' } }),
    TaskList,
    TaskItem.configure({ nested: true }),
    Table.configure({ resizable: true }),
    TableRow,
    TableHeader,
    TableCell,
    Highlight.configure({ multicolor: true }),
    Image.configure({ inline: false, allowBase64: true }),
    Placeholder.configure({ placeholder: t('pageUi.noteContent') })
  ],
  content: { type: 'doc', content: [] },
  onUpdate: ({ editor: ed }) => {
    charCount.value = ed.getText().length
    try {
      previewHtml.value = ed.getHTML()
    } catch { previewHtml.value = '' }
    if (!currentId.value) return
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = setTimeout(() => {
      if (currentId.value) {
        const json = ed.getJSON()
        store.updateNoteContent(currentId.value, JSON.stringify(json))
      }
    }, 400)
  },
  editorProps: {
    attributes: {
      class: 'tiptap-root'
    }
  }
})

function runCh(fn: (e: Editor) => void) {
  const e = editor.value
  if (!e) return
  try { fn(e) } catch { /* empty */ }
}

function align(align: 'left' | 'center' | 'right') {
  const ed = editor.value
  if (!ed) return
  ed.chain().focus().setTextAlign(align).run()
}

function onColor() {
  const ed = editor.value
  if (!ed || !colorHex.value) return
  ed.chain().focus().setColor(colorHex.value).run()
}

function addTable() {
  const ed = editor.value
  if (!ed) return
  ed.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run()
}

function setLink() {
  const ed = editor.value
  if (!ed) return
  const prev = window.prompt(t('notes.linkUrl') || 'URL', 'https://')
  if (prev) ed.chain().focus().extendMarkRange('link').setLink({ href: prev }).run()
}

function onEditorAreaClick() {
  const ed = editor.value
  if (!ed) return
  if (!ed.isFocused) {
    ed.commands.focus('end')
  }
}

function onBodyRowClick(e: MouseEvent) {
  if ((e.target as HTMLElement).closest('.preview-box')) return
  onEditorAreaClick()
}

async function insertImage() {
  const ed = editor.value
  if (!ed) return
  try {
    const res = await fileApi.openDialog({
      properties: ['openFile'],
      title: 'Select Image',
      filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'] }]
    })
    if (res.canceled || !res.filePaths?.length) return
    const path = res.filePaths[0]
    ed.chain().focus().setImage({ src: `file://${path}` }).run()
  } catch {
    const url = window.prompt('Image URL', 'https://')
    if (url) ed.chain().focus().setImage({ src: url }).run()
  }
}

watch([currentNote, editor], () => {
  const n = currentNote.value
  const ed = editor.value
  if (!n) {
    titleEdit.value = ''
    return
  }
  titleEdit.value = n.title
  if (!ed) return
  const doc = parseToDocJSON(n.content)
  ed.commands.setContent(doc, false)
  charCount.value = ed.getText().length
  try { previewHtml.value = ed.getHTML() } catch { previewHtml.value = '' }
})

function select(id: string) {
  store.selectNote(id)
}

function setFolder(id: string | null) {
  store.currentFolderId = id
}

function addNote() {
  store.createNote()
}

function newFolder() {
  void ElMessageBox.prompt(t('notes.newFolder'), t('common.confirm'), { inputValue: t('notes.untitledFolder') })
    .then(({ value }) => {
      if (value) store.createFolder(value)
    })
    .catch(() => { /* cancel */ })
}

function removeFolder(id: string) {
  store.deleteFolder(id)
}

function onTitleChange() {
  if (currentId.value) store.updateNoteTitle(currentId.value, titleEdit.value)
}

function removeNote() {
  if (currentId.value) store.deleteNote(currentId.value)
}

function formatTime(tms: number) {
  return new Date(tms).toLocaleDateString()
}

onBeforeUnmount(() => {
  if (saveTimer) clearTimeout(saveTimer)
})
</script>

<style lang="scss" scoped>
.notes-page {
  display: grid;
  grid-template-columns: 240px 1fr;
  height: 100%;
  min-height: 0;
  background: var(--color-background);
  color: var(--color-text-1);
  overflow: hidden;
}

.n-sidebar {
  border-right: 1px solid var(--color-border);
  padding: 12px 10px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
  background: var(--color-background-mute);
}

.side-tools {
  display: flex;
  flex-direction: column;
  gap: 8px;
  :deep(.el-input__wrapper) {
    border-radius: var(--fox-radius-sm);
  }
}

.side-btns {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.btn-accent {
  background: var(--fox-accent-fg) !important;
  border-color: var(--fox-accent-border) !important;
  color: var(--fox-accent-on) !important;
}

.folder-block {
  max-height: 120px;
  overflow: auto;
  font-size: 0.88rem;
}
.tree-row {
  padding: 6px 8px;
  border-radius: var(--fox-radius-sm);
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--color-text-2);
  &:hover { background: var(--color-hover); }
  &.on { color: var(--color-text-1); background: var(--color-hover); }
  .f-del { margin-left: auto; padding: 0 4px; }
}
.fi { opacity: 0.8; }

.note-list {
  list-style: none;
  margin: 0;
  padding: 0;
  flex: 1;
  min-height: 0;
  overflow: auto;
}
.nl-item {
  padding: 8px 10px;
  border-radius: var(--fox-radius-sm);
  cursor: pointer;
  margin-bottom: 2px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: 0.88rem;
  border: 1px solid transparent;
  &:hover { background: var(--color-hover); }
  &.active {
    background: var(--color-hover);
    border-color: var(--fox-accent-border);
  }
  .n-title { font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .n-t { font-size: 0.7rem; color: var(--color-text-3); }
}

.n-main {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  flex: 1;
}
.n-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px 8px;
  border-bottom: 1px solid var(--color-border);
  gap: 8px;
}
.crumbs {
  display: flex;
  align-items: center;
  flex: 1;
  min-width: 0;
  gap: 6px;
  font-size: 0.8rem;
  color: var(--color-text-3);
  .sep { opacity: 0.6; }
  .title-input { flex: 1; min-width: 0; }
  :deep(.el-input__wrapper) { border-radius: var(--fox-radius-sm); }
}
.loading-ed, .n-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-3);
  padding: 40px;
}
.editor-wrap { flex: 1; min-height: 0; display: flex; flex-direction: column; }
.toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  padding: 8px 12px 6px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-background-soft);
  .m-l { margin-left: 4px; }
  :deep(.el-button) { min-width: 32px; }
}
.body-row {
  display: flex;
  flex: 1;
  min-height: 220px;
  overflow: auto;
  &.split {
    .editor-box, .preview-box { flex: 1; min-width: 0; }
    .preview-box {
      border-left: 1px solid var(--color-border);
      background: var(--color-background-mute);
    }
  }
}
.editor-box {
  min-width: 0;
  flex: 1;
  cursor: text;
}
.editor-content { min-height: 100%; }
:deep(.tiptap-root) {
  padding: 16px 20px 24px;
  min-height: 240px;
  outline: none;
  font-size: var(--fox-fs, 14px);
  line-height: 1.6;
  color: var(--color-text-1);
  p { margin: 0.5em 0; }
  h1, h2, h3 { line-height: 1.3; font-weight: 600; }
  a.tiptap-link { color: var(--fox-accent-fg); }
  pre {
    background: var(--color-background-mute);
    border-radius: var(--fox-radius-sm);
    padding: 8px 10px;
  }
  table {
    border-collapse: collapse;
    width: 100%;
    th, td { border: 1px solid var(--color-border); padding: 4px 6px; }
  }
  ul[data-type='taskList'] { list-style: none; padding-left: 0.5rem; }
}
.preview-box {
  padding: 16px 20px;
  font-size: var(--fox-fs, 14px);
  line-height: 1.6;
  overflow: auto;
  :deep(p) { margin: 0.5em 0; }
}
.n-foot {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 16px 12px;
  border-top: 1px solid var(--color-border);
  font-size: 0.85rem;
  color: var(--color-text-2);
  .cc { font-feature-settings: 'tnum', 'lnum'; }
}
</style>
