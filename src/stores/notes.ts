import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { v4 as uuidv4 } from 'uuid'
import type { Note, NoteFolder } from '@/types'

export const useNotesStore = defineStore('notes', () => {
  const notes = ref<Note[]>([])
  const folders = ref<NoteFolder[]>([])
  const currentId = ref<string | null>(null)
  const currentFolderId = ref<string | null>(null)
  const searchQuery = ref('')

  const sorted = computed(() => [...notes.value].sort((a, b) => b.updatedAt - a.updatedAt))

  const filteredNotes = computed(() => {
    let result = sorted.value
    if (currentFolderId.value) result = result.filter(n => n.folderId === currentFolderId.value)
    if (searchQuery.value.trim()) {
      const q = searchQuery.value.toLowerCase()
      result = result.filter(n => n.title.toLowerCase().includes(q) || n.content.toLowerCase().includes(q))
    }
    return result
  })

  const currentNote = computed(() => notes.value.find(n => n.id === currentId.value) ?? null)

  function createFolder(name: string): NoteFolder {
    const f: NoteFolder = { id: uuidv4(), name, createdAt: Date.now() }
    folders.value.push(f)
    currentFolderId.value = f.id
    return f
  }

  function createNote(partial: Partial<Note> = {}): Note {
    const n: Note = {
      id: uuidv4(), title: partial.title || '无标题笔记', content: partial.content || '',
      category: partial.category || '默认', folderId: currentFolderId.value || undefined,
      pinned: partial.pinned ?? false, starred: partial.starred ?? false,
      createdAt: Date.now(), updatedAt: Date.now()
    }
    notes.value.unshift(n)
    currentId.value = n.id
    return n
  }

  function updateNoteContent(id: string, content: string) {
    const n = notes.value.find(x => x.id === id)
    if (n) { n.content = content; n.updatedAt = Date.now() }
  }

  function updateNoteTitle(id: string, title: string) {
    const n = notes.value.find(x => x.id === id)
    if (n) { n.title = title; n.updatedAt = Date.now() }
  }

  function toggleStar(id: string) {
    const n = notes.value.find(x => x.id === id)
    if (n) n.starred = !n.starred
  }

  function deleteNote(id: string) {
    notes.value = notes.value.filter(x => x.id !== id)
    if (currentId.value === id) currentId.value = notes.value[0]?.id ?? null
  }

  function deleteFolder(id: string) {
    folders.value = folders.value.filter(f => f.id !== id)
    notes.value.filter(n => n.folderId === id).forEach(n => { n.folderId = undefined })
    if (currentFolderId.value === id) currentFolderId.value = null
  }

  function selectNote(id: string) { currentId.value = id }

  return {
    notes, folders, currentId, currentFolderId, searchQuery, sorted, filteredNotes, currentNote,
    createFolder, createNote, updateNoteContent, updateNoteTitle, toggleStar,
    deleteNote, deleteFolder, selectNote
  }
}, { persist: true })
