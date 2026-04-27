<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useProviderStore } from '@/stores/provider'
import type { Provider } from '@/types'

const props = defineProps<{
  providerId: string
  modelId: string
}>()

const emit = defineEmits<{
  (e: 'update:providerId', v: string): void
  (e: 'update:modelId', v: string): void
}>()

const { t } = useI18n()
const providerStore = useProviderStore()

const enabledProviders = computed(() => providerStore.getEnabledProviders())

const groupOptions = computed(() => {
  const out: { provider: Provider; models: string[] }[] = []
  for (const p of enabledProviders.value) {
    if (p.models?.length) out.push({ provider: p, models: p.models })
  }
  return out
})

const selectValue = computed({
  get: () => (props.providerId && props.modelId ? `${props.providerId}:::${props.modelId}` : ''),
  set: (v: string) => {
    if (!v) {
      emit('update:providerId', '')
      emit('update:modelId', '')
      return
    }
    const s = v.indexOf(':::')
    if (s === -1) {
      // Custom typed value (allow-create): use current provider or first enabled
      const pid = props.providerId || enabledProviders.value[0]?.id || ''
      if (pid) {
        emit('update:providerId', pid)
        emit('update:modelId', v)
      }
      return
    }
    emit('update:providerId', v.slice(0, s))
    emit('update:modelId', v.slice(s + 3))
  }
})

function ensureValidSelection() {
  if (!groupOptions.value.length) return
  for (const g of groupOptions.value) {
    for (const m of g.models) {
      if (g.provider.id === props.providerId && m === props.modelId) return
    }
  }
  const first = groupOptions.value[0]
  const fm = first.models[0]
  if (first && fm) {
    emit('update:providerId', first.provider.id)
    emit('update:modelId', fm)
  }
}

watch([() => props.providerId, () => props.modelId, groupOptions], ensureValidSelection, { immediate: true })

onMounted(() => {
  if (!providerStore.providers.length) {
    void providerStore.loadProviders()
  }
})
</script>

<template>
  <el-select
    v-model="selectValue"
    class="model-selector"
    :placeholder="t('chat.selectModel')"
    :disabled="!groupOptions.length"
    filterable
    allow-create
    default-first-option
    size="default"
  >
    <el-option
      v-if="!groupOptions.length"
      key="__empty__"
      value=""
      :label="t('chat.noProviders')"
      disabled
    />
    <el-option-group
      v-for="g in groupOptions"
      :key="g.provider.id"
      :label="g.provider.name"
    >
      <el-option
        v-for="m in g.models"
        :key="`${g.provider.id}:${m}`"
        :value="`${g.provider.id}:::${m}`"
        :label="`${g.provider.name} / ${m}`"
      />
    </el-option-group>
  </el-select>
</template>

<style lang="scss" scoped>
.model-selector {
  min-width: 200px;
  max-width: 320px;
  --el-select-border-color: var(--color-border);
  --el-text-color-primary: var(--color-text-1);
  --el-text-color-secondary: var(--color-text-2);
  --el-border-radius-base: var(--fox-radius-sm);
  --el-font-size-base: 13px;
}

:deep(.el-input__wrapper) {
  background: var(--color-background-mute) !important;
  border: 1px solid var(--color-border) !important;
  border-radius: var(--fox-radius-sm) !important;
  box-shadow: var(--shadow-sm) !important;
  transition: border-color 0.2s var(--fox-ease), box-shadow 0.2s var(--fox-ease);
}

:deep(.el-input__wrapper:hover) {
  border-color: var(--color-text-3) !important;
  background: var(--color-hover) !important;
}

:deep(.el-input__wrapper.is-focus) {
  border-color: var(--fox-accent-border) !important;
  box-shadow: 0 0 0 2px var(--color-primary-mute) !important;
}
</style>
