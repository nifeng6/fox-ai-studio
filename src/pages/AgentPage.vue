<template>
  <div class="agent-page">
    <div class="hero">
      <div class="icon-wrap" aria-hidden="true">
        <el-icon :size="44" class="hero-icon">
          <Cpu />
        </el-icon>
      </div>
      <h1 class="title">{{ t('agents.title') }}</h1>
      <p class="sub">{{ t('pageUi.agentSubtitle') }}</p>
      <p class="coming">{{ t('agents.comingSoon') }}</p>
      <p class="desc">{{ t('agents.description') }}</p>
    </div>

    <div class="card-grid">
      <article
        v-for="(text, i) in featureTexts"
        :key="i"
        class="feature-card"
      >
        <p class="card-text">{{ text }}</p>
      </article>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Cpu } from '@element-plus/icons-vue'

const { t } = useI18n()

const featureTexts = computed(() => [
  t('pageUi.agentFeature1'),
  t('pageUi.agentFeature2'),
  t('pageUi.agentFeature3'),
  t('pageUi.agentFeature4')
])
</script>

<style lang="scss" scoped>
.agent-page {
  min-height: 100%;
  padding: 28px 24px 48px;
  background: var(--color-background);
  color: var(--color-text-1);
}

.hero {
  text-align: center;
  max-width: 640px;
  margin: 0 auto 40px;
}

.icon-wrap {
  display: inline-flex;
  width: 88px;
  height: 88px;
  border-radius: var(--fox-radius-lg);
  background: var(--color-background-soft);
  border: 1px solid var(--color-border);
  align-items: center;
  justify-content: center;
  margin-bottom: 16px;
  box-shadow: var(--shadow-sm);
}

.hero-icon {
  color: var(--fox-accent-fg);
}

.title {
  margin: 0 0 8px;
  font-size: 1.75rem;
  font-weight: 700;
  letter-spacing: -0.02em;
}

.sub {
  margin: 0 0 12px;
  color: var(--color-text-2);
  font-size: 1.05rem;
}

.coming {
  display: inline-block;
  margin: 0 0 16px;
  padding: 6px 14px;
  border-radius: 999px;
  background: var(--color-primary-mute);
  color: var(--color-text-1);
  font-size: 0.9rem;
  font-weight: 500;
  border: 1px solid var(--color-border);
}

.desc {
  margin: 0;
  color: var(--color-text-3);
  line-height: 1.6;
  font-size: 0.95rem;
}

.card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 16px;
  max-width: 1000px;
  margin: 0 auto;
}

.feature-card {
  padding: 20px 20px 22px;
  border-radius: var(--fox-radius-lg);
  background: var(--color-background-soft);
  border: 1px solid var(--color-border);
  box-shadow: var(--shadow-sm);
  transition: transform 0.2s, box-shadow 0.2s;
  &:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow-md);
  }
}

.card-text {
  margin: 0;
  font-size: 0.9rem;
  line-height: 1.6;
  color: var(--color-text-2);
}
</style>
