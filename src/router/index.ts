import { createRouter, createWebHashHistory } from 'vue-router'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/chat' },
    { path: '/chat', name: 'Chat', component: () => import('@/pages/ChatPage.vue') },
    { path: '/assistants', name: 'Assistants', component: () => import('@/pages/AssistantsPage.vue') },
    { path: '/knowledge', name: 'Knowledge', component: () => import('@/pages/KnowledgePage.vue') },
    { path: '/paintings', name: 'Paintings', component: () => import('@/pages/PaintingsPage.vue') },
    { path: '/translate', name: 'Translate', component: () => import('@/pages/TranslatePage.vue') },
    { path: '/miniprogram', name: 'Miniprogram', component: () => import('@/pages/MiniprogramPage.vue') },
    { path: '/notes', name: 'Notes', component: () => import('@/pages/NotesPage.vue') },
    { path: '/files', name: 'Files', component: () => import('@/pages/FilesPage.vue') },
    { path: '/settings', name: 'Settings', component: () => import('@/pages/SettingsPage.vue') },
    {
      path: '/selection-popup',
      name: 'SelectionPopup',
      component: () => import('@/pages/SelectionPopupPage.vue'),
      meta: { isPopup: true }
    }
  ]
})

export default router
