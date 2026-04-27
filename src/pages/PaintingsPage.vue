<template>
  <div class="paintings-page">
    <h1 class="title">{{ t('paintings.title') }}</h1>

    <div class="composer card">
      <el-input
        v-model="prompt"
        type="textarea"
        :rows="3"
        :placeholder="t('paintings.prompt')"
        class="prompt-input"
      />
      <div class="row">
        <div class="field">
          <span class="label">{{ t('paintings.size') }}</span>
          <el-select v-model="size" class="w-full">
            <el-option :label="t('pageUi.size1024')" value="1024x1024" />
            <el-option :label="t('pageUi.size1792')" value="1792x1024" />
            <el-option label="1024×1792" value="1024x1792" />
          </el-select>
        </div>
        <div class="field">
          <span class="label">{{ t('paintings.quality') }}</span>
          <el-select v-model="quality" class="w-full">
            <el-option :label="t('pageUi.paintingNormal')" value="normal" />
            <el-option :label="t('pageUi.paintingHD')" value="hd" />
          </el-select>
        </div>
        <div class="field">
          <span class="label">{{ t('paintings.style') }}</span>
          <el-select v-model="style" class="w-full">
            <el-option :label="t('pageUi.styleRealistic')" value="realistic" />
            <el-option :label="t('pageUi.styleAnime')" value="anime" />
            <el-option :label="t('pageUi.styleOil')" value="oil" />
            <el-option :label="t('pageUi.styleCyber')" value="cyber" />
          </el-select>
        </div>
        <el-button
          class="go-btn"
          type="primary"
          :loading="generating"
          :disabled="!prompt.trim()"
          @click="generate"
        >
          {{ t('paintings.generate') }}
        </el-button>
      </div>
    </div>

    <p class="subhint">{{ t('pageUi.paintingGenerated') }}</p>

    <div v-if="!gallery.length" class="empty-guide card">
      <h2 class="guide-title">{{ t('pageUi.paintingGuideTitle') }}</h2>
      <ol class="guide-list">
        <li>{{ t('pageUi.paintingGuide1') }}</li>
        <li>{{ t('pageUi.paintingGuide2') }}</li>
      </ol>
    </div>
    <div v-else class="gallery">
      <figure
        v-for="item in gallery"
        :key="item.id"
        class="g-item"
      >
        <div class="thumb" :style="{ backgroundImage: `url(${item.url})` }" />
        <figcaption class="cap">{{ item.prompt.slice(0, 80) }}{{ item.prompt.length > 80 ? '…' : '' }}</figcaption>
      </figure>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { v4 as uuidv4 } from 'uuid'
import { ElMessage } from 'element-plus'

const { t } = useI18n()

const prompt = ref('')
const size = ref('1024x1024')
const quality = ref('normal')
const style = ref('realistic')
const generating = ref(false)

interface GalleryItem {
  id: string
  url: string
  prompt: string
  createdAt: number
}

const gallery = ref<GalleryItem[]>([])

function makePlaceholderDataUrl(w: number, h: number, label: string) {
  const svg = encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}"><defs><linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="100%">
    <stop offset="0%" style="stop-color:#2a2a2a"/><stop offset="100%" style="stop-color:#111"/>
    </linearGradient></defs><rect width="100%" height="100%" fill="url(#g)"/>
    <text x="50%" y="50%" fill="rgba(255,255,255,0.4)" font-family="system-ui" font-size="14" text-anchor="middle" dominant-baseline="middle">${label}</text></svg>`
  )
  return `data:image/svg+xml,${svg}`
}

async function generate() {
  if (!prompt.value.trim()) return
  generating.value = true
  try {
    const { useProviderStore } = await import('@/stores/provider')
    const { useSettingsStore } = await import('@/stores/settings')
    const providerStore = useProviderStore()
    const settingsStore = useSettingsStore()
    const prov = providerStore.getProviderById(settingsStore.defaultProviderId)
      || providerStore.getEnabledProviders()[0]

    if (!prov?.apiKey || !prov?.apiEndpoint) {
      const [sw, sh] = size.value.split('x').map(Number)
      const url = makePlaceholderDataUrl(sw || 512, sh || 512, '请先配置模型提供商')
      gallery.value.unshift({ id: uuidv4(), url, prompt: prompt.value, createdAt: Date.now() })
      ElMessage.warning('请先在设置中配置模型提供商的 API Key')
      return
    }

    const endpoint = prov.apiEndpoint.replace(/\/+$/, '')
    const resp = await fetch(`${endpoint}/v1/images/generations`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${prov.apiKey}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        model: 'dall-e-3',
        prompt: prompt.value,
        n: 1,
        size: size.value,
        quality: quality.value === 'hd' ? 'hd' : 'standard'
      })
    })

    if (!resp.ok) {
      const errText = await resp.text()
      throw new Error(`API ${resp.status}: ${errText.slice(0, 200)}`)
    }

    const data = await resp.json()
    const imgUrl = data?.data?.[0]?.url || data?.data?.[0]?.b64_json
    if (imgUrl) {
      const finalUrl = imgUrl.startsWith('http') ? imgUrl : `data:image/png;base64,${imgUrl}`
      gallery.value.unshift({ id: uuidv4(), url: finalUrl, prompt: prompt.value, createdAt: Date.now() })
      ElMessage.success(t('common.success'))
    } else {
      throw new Error('No image in response')
    }
  } catch (e: any) {
    ElMessage.error(e?.message || '生成失败')
    const [sw, sh] = size.value.split('x').map(Number)
    const url = makePlaceholderDataUrl(sw || 512, sh || 512, '生成失败')
    gallery.value.unshift({ id: uuidv4(), url, prompt: prompt.value, createdAt: Date.now() })
  } finally {
    generating.value = false
  }
}
</script>

<style lang="scss" scoped>
.paintings-page {
  min-height: 100%;
  padding: 20px 24px 40px;
  background: var(--color-background);
  color: var(--color-text-1);
}

.title {
  margin: 0 0 20px;
  font-size: 1.5rem;
  font-weight: 600;
}

.card {
  border-radius: var(--fox-radius-lg);
  border: 1px solid var(--color-border);
  background: var(--color-background-soft);
  padding: 20px;
}

.composer {
  max-width: 900px;
}

.prompt-input {
  margin-bottom: 16px;
  :deep(.el-textarea__inner) {
    border-radius: var(--fox-radius-md);
    background: var(--color-background-mute);
    color: var(--color-text-1);
    box-shadow: 0 0 0 1px var(--color-border);
  }
}

.row {
  display: flex;
  flex-wrap: wrap;
  gap: 12px 16px;
  align-items: flex-end;
}

.field {
  flex: 1;
  min-width: 160px;
  .label {
    display: block;
    font-size: 0.8rem;
    color: var(--color-text-3);
    margin-bottom: 6px;
  }
  :deep(.el-select) {
    width: 100%;
  }
  :deep(.el-select__wrapper) {
    border-radius: var(--fox-radius-sm);
  }
}

.go-btn {
  background: var(--fox-accent-fg) !important;
  border-color: var(--fox-accent-border) !important;
  color: var(--fox-accent-on) !important;
  border-radius: var(--fox-radius-sm);
  padding: 20px 24px;
  height: auto;
  &:hover {
    filter: brightness(1.06);
  }
}

.subhint {
  color: var(--color-text-3);
  font-size: 0.88rem;
  margin: 16px 0 20px;
}

.empty-guide {
  max-width: 700px;
  .guide-title {
    margin: 0 0 12px;
    font-size: 1.1rem;
  }
  .guide-list {
    margin: 0;
    padding-left: 1.2rem;
    color: var(--color-text-2);
    line-height: 1.6;
  }
}

.gallery {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 16px;
}

.g-item {
  margin: 0;
  border-radius: var(--fox-radius-md);
  overflow: hidden;
  border: 1px solid var(--color-border);
  background: var(--color-background-soft);
  box-shadow: var(--shadow-sm);
}

.thumb {
  width: 100%;
  padding-bottom: 100%;
  background-size: cover;
  background-position: center;
}

.cap {
  padding: 8px 10px;
  font-size: 0.8rem;
  color: var(--color-text-2);
  line-height: 1.4;
}
</style>
