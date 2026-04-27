<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { windowApi } from '@/utils/tauri-api'

const isMaximized = ref(false)
let cleanup: (() => void) | null = null

onMounted(async () => {
  isMaximized.value = await windowApi.isMaximized()
  const unlisten = await windowApi.onMaximizedChanged((m) => { isMaximized.value = m })
  cleanup = unlisten
})

onUnmounted(() => { cleanup?.() })
</script>

<template>
  <header class="titlebar" data-tauri-drag-region>
    <div class="titlebar-spacer" data-tauri-drag-region />
    <div class="titlebar-controls nodrag">
      <button class="ctl" @click="windowApi.minimize()" title="最小化">
        <svg width="10" height="1" viewBox="0 0 10 1"><rect width="10" height="1" rx="0.5" fill="currentColor"/></svg>
      </button>
      <button class="ctl" @click="windowApi.maximize()" :title="isMaximized ? '还原' : '最大化'">
        <svg v-if="!isMaximized" width="10" height="10" viewBox="0 0 10 10">
          <rect x="0.5" y="0.5" width="9" height="9" rx="1.5" stroke="currentColor" stroke-width="1" fill="none"/>
        </svg>
        <svg v-else width="10" height="10" viewBox="0 0 10 10">
          <rect x="2" y="0" width="8" height="8" rx="1" stroke="currentColor" stroke-width="1" fill="none"/>
          <rect x="0" y="2" width="8" height="8" rx="1" stroke="currentColor" stroke-width="1" fill="var(--color-background)"/>
        </svg>
      </button>
      <button class="ctl ctl-close" @click="windowApi.close()" title="关闭">
        <svg width="10" height="10" viewBox="0 0 10 10">
          <path d="M1 1L9 9M9 1L1 9" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
        </svg>
      </button>
    </div>
  </header>
</template>

<style lang="scss" scoped>
.titlebar {
  height: var(--navbar-height);
  background: var(--color-background);
  display: flex;
  align-items: center;
  padding: 0;
  user-select: none;
  flex-shrink: 0;
  z-index: 5;
  position: relative;
  transition: background-color 0.12s;
}

.titlebar-spacer { flex: 1; }

.titlebar-controls { display: flex; align-items: center; }

.ctl {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 46px;
  height: var(--navbar-height);
  border: none;
  background: transparent;
  color: var(--color-text-2);
  cursor: pointer;
  outline: none;
  padding: 0;
  -webkit-app-region: no-drag;
  app-region: no-drag;
  transition: background 0.08s, color 0.08s;

  &:hover { background: var(--color-hover); }
  &:active { background: var(--color-active); }
  &.ctl-close:hover { background: #e81123; color: #fff; }
  &.ctl-close:active { background: #c50e1f; color: #fff; }
  svg { pointer-events: none; }
}
</style>
