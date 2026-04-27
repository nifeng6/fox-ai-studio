<script setup lang="ts">
import { computed } from 'vue'
import { useAgentStore } from '@/stores/agent'
import { storeToRefs } from 'pinia'

const agent = useAgentStore()
const { permissionLevel } = storeToRefs(agent)

const label = computed(() =>
  permissionLevel.value === 'default'
    ? '默认权限 - 工具执行需要确认'
    : '完全权限 - 工具自动执行'
)

function onClick() {
  agent.togglePermission()
}
</script>

<template>
  <button
    type="button"
    class="permission-banner"
    :title="label"
    :aria-pressed="permissionLevel === 'full'"
    @click="onClick"
  >
    {{ label }}
  </button>
</template>

<style lang="scss" scoped>
.permission-banner {
  display: inline-flex;
  align-items: center;
  align-self: flex-start;
  max-width: 100%;
  margin: 0 0 6px 0;
  padding: 4px 10px;
  font-size: 11px;
  line-height: 1.3;
  color: var(--color-text-2);
  background: var(--color-background-mute);
  border: 1px solid var(--color-border);
  border-radius: 999px;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
  text-align: left;
  word-break: break-all;

  &:hover {
    background: var(--color-hover);
    color: var(--color-text-1);
    border-color: var(--color-text-3);
  }
}
</style>
