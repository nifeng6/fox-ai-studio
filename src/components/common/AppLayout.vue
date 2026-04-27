<script setup lang="ts">
import Titlebar from './Titlebar.vue'
import Sidebar from './Sidebar.vue'
import SearchDialog from './SearchDialog.vue'
import { useAppStore } from '@/stores/app'

const appStore = useAppStore()
</script>

<template>
  <div class="app-shell">
    <div class="app-body">
      <Sidebar />
      <div class="app-main">
        <Titlebar />
        <div id="content-container" class="app-content">
          <router-view v-slot="{ Component }">
            <keep-alive>
              <component :is="Component" />
            </keep-alive>
          </router-view>
        </div>
      </div>
    </div>
    <SearchDialog v-if="appStore.globalSearchVisible" />
  </div>
</template>

<style lang="scss" scoped>
.app-shell {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.app-body {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.app-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.app-content {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  background-color: var(--color-background);
  border-top: 1px solid var(--color-border);
  border-top-left-radius: 14px;
  border-left: 1px solid var(--color-border);
  transition: background-color 0.12s;
}
</style>
