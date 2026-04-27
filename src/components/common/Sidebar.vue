<script setup lang="ts">
import { computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'

const router = useRouter()
const route = useRoute()
const { t } = useI18n()

interface NavItem { key: string; icon: string; label: string; path: string }

const topNavItems: NavItem[] = [
  { key: 'chat', icon: 'chat', label: 'nav.chat', path: '/chat' },
  { key: 'assistants', icon: 'user', label: 'nav.assistants', path: '/assistants' },
  { key: 'paintings', icon: 'picture', label: 'nav.paintings', path: '/paintings' },
  { key: 'translate', icon: 'translate', label: 'nav.translate', path: '/translate' },
  { key: 'knowledge', icon: 'folder', label: 'nav.knowledge', path: '/knowledge' },
  { key: 'miniprogram', icon: 'grid', label: 'nav.miniprogram', path: '/miniprogram' },
  { key: 'notes', icon: 'document', label: 'nav.notes', path: '/notes' },
  { key: 'files', icon: 'files', label: 'nav.files', path: '/files' }
]

const bottomNavItems: NavItem[] = [
  { key: 'settings', icon: 'setting', label: 'nav.settings', path: '/settings' }
]

const activeKey = computed(() => {
  const path = route.path
  return topNavItems.find(n => n.path === path)?.key || bottomNavItems.find(n => n.path === path)?.key || 'chat'
})

function navigate(item: NavItem) { router.push(item.path) }

const iconMap: Record<string, string> = {
  chat: 'M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2z',
  user: 'M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z',
  folder: 'M10 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z',
  picture: 'M21 19V5c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2zM8.5 13.5l2.5 3.01L14.5 12l4.5 6H5l3.5-4.5z',
  translate: 'M12.87 15.07l-2.54-2.51.03-.03A17.52 17.52 0 0014.07 6H17V4h-7V2H8v2H1v2h11.17C11.5 7.92 10.44 9.75 9 11.35 8.07 10.32 7.3 9.19 6.69 8h-2c.73 1.63 1.73 3.17 2.98 4.56l-5.09 5.02L4 19l5-5 3.11 3.11.76-2.04zM18.5 10h-2L12 22h2l1.12-3h4.75L21 22h2l-4.5-12zm-2.62 7l1.62-4.33L19.12 17h-3.24z',
  grid: 'M4 4h7v7H4V4zm9 0h7v7h-7V4zm-9 9h7v7H4v-7zm9 0h7v7h-7v-7z',
  document: 'M14 2H6c-1.1 0-2 .9-2 2v16c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z',
  cpu: 'M15 9H9v6h6V9zm-2 4h-2v-2h2v2zm8-2V9h-2V7c0-1.1-.9-2-2-2h-2V3h-2v2h-2V3H9v2H7c-1.1 0-2 .9-2 2v2H3v2h2v2H3v2h2v2c0 1.1.9 2 2 2h2v2h2v-2h2v2h2v-2h2c1.1 0 2-.9 2-2v-2h2v-2h-2v-2h2zm-4 6H7V7h10v10z',
  setting: 'M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58a.49.49 0 00.12-.61l-1.92-3.32a.49.49 0 00-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54a.484.484 0 00-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.07.62-.07.94s.02.64.07.94l-2.03 1.58a.49.49 0 00-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z',
  files: 'M20 6h-8l-2-2H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm0 12H4V8h16v10z',
  monitor: 'M21 2H3c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h7v2H8v2h8v-2h-2v-2h7c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm0 14H3V4h18v12z'
}
</script>

<template>
  <nav class="sidebar">
    <div class="sidebar-top">
      <el-tooltip v-for="item in topNavItems" :key="item.key" :content="t(item.label)" placement="right" :show-after="400" :offset="8">
        <button class="nav-btn" :class="{ active: activeKey === item.key }" @click="navigate(item)">
          <svg width="18" height="18" viewBox="0 0 24 24"><path :d="iconMap[item.icon]" fill="currentColor"/></svg>
        </button>
      </el-tooltip>
    </div>
    <div class="sidebar-bottom">
      <el-tooltip v-for="item in bottomNavItems" :key="item.key" :content="t(item.label)" placement="right" :show-after="400" :offset="8">
        <button class="nav-btn" :class="{ active: activeKey === item.key }" @click="navigate(item)">
          <svg width="18" height="18" viewBox="0 0 24 24"><path :d="iconMap[item.icon]" fill="currentColor"/></svg>
        </button>
      </el-tooltip>
    </div>
  </nav>
</template>

<style lang="scss" scoped>
.sidebar {
  width: var(--fox-sidebar-w);
  background: var(--navbar-background);
  backdrop-filter: blur(var(--glass-blur-light));
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 6px 0;
  flex-shrink: 0;
  z-index: 100;
  position: relative;
  transition: background-color 0.12s;
}

.sidebar-top {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  width: 100%;
  padding: 0 7px;
}

.sidebar-bottom {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  width: 100%;
  padding: 0 7px 6px;
}

.nav-btn {
  width: 36px;
  height: 36px;
  border: none;
  background: transparent;
  color: var(--color-icon);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--fox-radius-sm);
  transition: all 0.1s var(--fox-ease);
  position: relative;

  &:hover {
    background: var(--color-hover);
    color: var(--color-icon-white);
  }

  &.active {
    background: var(--color-active);
    color: var(--color-icon-white);

    &::before {
      content: '';
      position: absolute;
      left: -7px;
      top: 50%;
      transform: translateY(-50%);
      width: 3px;
      height: 16px;
      background: var(--color-text-1);
      border-radius: 0 3px 3px 0;
    }
  }
}
</style>
