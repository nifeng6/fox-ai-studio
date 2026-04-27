<template>
  <div class="settings-page">
    <div class="settings-layout">
      <aside class="settings-nav" aria-label="Settings navigation">
        <button
          v-for="item in navItems"
          :key="item.key"
          type="button"
          class="settings-nav__btn"
          :class="{ 'is-active': active === item.key }"
          @click="active = item.key"
        >
          <svg class="settings-nav__icon" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <path :d="item.icon" />
          </svg>
          <span class="settings-nav__label">{{ t(item.labelKey) }}</span>
        </button>
      </aside>

      <div class="settings-main">
        <div class="settings-main__scroll">
          <div class="settings-main__inner">
            <div v-show="active === 'model'" class="settings-panel">
              <div class="settings-card settings-card--flush">
                <div class="settings-card__title">{{ t('settings.sectionModel') }}</div>
                <p class="settings-card__intro">{{ t('pageUi.settingsProviderHint') }}</p>
                <div class="settings-split settings-split--model">
                  <div class="settings-split__mid">
                    <div class="prov-filter-bar">
                      <el-input v-model="providerQuery" clearable size="small" :placeholder="t('settings.providerSearch')" class="prov-filter-input">
                        <template #prefix>
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8" /><path d="M21 21l-4.35-4.35" /></svg>
                        </template>
                      </el-input>
                      <el-tooltip content="按启用状态筛选" placement="top" :show-after="400">
                        <button type="button" class="prov-filter-btn" :class="{ 'is-on': provFilterEnabled }" @click="provFilterEnabled = !provFilterEnabled">
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3" /></svg>
                        </button>
                      </el-tooltip>
                    </div>
                    <div v-loading="provider.loading" class="prov-list">
                      <button
                        v-for="p in filteredProviders"
                        :key="p.id"
                        type="button"
                        class="prov-card"
                        :class="{ 'is-pick': selectedProvId === p.id }"
                        @click="selectedProvId = p.id"
                      >
                        <span class="prov-card__icon" :style="{ background: getProviderColor(p.name) }">{{ p.name.charAt(0).toUpperCase() }}</span>
                        <span class="prov-card__name">{{ p.name }}</span>
                        <span v-if="p.enabled" class="prov-card__badge prov-card__badge--on">ON</span>
                        <el-switch
                          :model-value="p.enabled"
                          class="prov-card__sw"
                          size="small"
                          @click.stop
                          @change="(v: boolean) => toggleProvider(p.id, v)"
                        />
                      </button>
                      <p v-if="!filteredProviders.length" class="t-sm m-t">{{ t('common.noData') }}</p>
                    </div>
                    <el-button type="primary" class="btn-accent w-full m-t" :loading="provider.loading" @click="openProvider()">
                      + {{ t('provider.add') }}
                    </el-button>
                  </div>
                  <div class="settings-split__detail">
                    <template v-if="selectedProvId && detailProv">
                      <div class="detail-head">
                        <div class="detail-head__left">
                          <span class="detail-head__icon" :style="{ background: getProviderColor(detailProv.name) }">{{ detailProv.name.charAt(0).toUpperCase() }}</span>
                          <div>
                            <h3 class="detail-title">{{ detailProv.name }}</h3>
                            <span class="detail-head__status" :class="detailProv.enabled ? 'is-on' : 'is-off'">
                              {{ detailProv.enabled ? '已启用' : '已禁用' }}
                            </span>
                          </div>
                        </div>
                        <div class="detail-head__actions">
                          <el-tooltip content="编辑" placement="top" :show-after="400">
                            <el-button size="small" circle @click="openProvider(selectedProvId)">
                              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" /><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" /></svg>
                            </el-button>
                          </el-tooltip>
                          <el-tooltip content="删除" placement="top" :show-after="400">
                            <el-button size="small" circle type="danger" @click="removeProv(selectedProvId)">
                              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6" /><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" /></svg>
                            </el-button>
                          </el-tooltip>
                        </div>
                      </div>
                      <el-form label-position="top" class="m-t">
                        <el-form-item label="API 密钥">
                          <div class="api-key-row">
                            <el-input v-model="provDetail.apiKey" type="password" show-password @change="saveProvDetail" placeholder="sk-..." />
                            <el-button :loading="testingProvider" @click="testProv(selectedProvId)">{{ testingProvider ? modelFetchMsg : '检测' }}</el-button>
                          </div>
                          <p class="field-hint">多个密钥使用逗号分隔</p>
                        </el-form-item>
                        <el-form-item label="API 地址">
                          <el-input v-model="provDetail.apiEndpoint" @change="saveProvDetail" placeholder="https://api.example.com/v1" />
                          <p v-if="provDetail.apiEndpoint" class="url-preview">
                            预览: {{ buildPreviewUrl(provDetail.apiEndpoint, provDetail.channelType) }}
                          </p>
                        </el-form-item>
                        <div class="model-section">
                          <div class="model-section__header">
                            <span class="model-section__title">模型</span>
                            <span class="model-section__count">{{ (detailProv.models || []).length }}</span>
                            <div class="model-section__toolbar">
                              <el-tooltip content="搜索模型" placement="top" :show-after="400">
                                <button type="button" class="model-toolbar-btn" @click="modelSearchVisible = !modelSearchVisible">
                                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8" /><path d="M21 21l-4.35-4.35" /></svg>
                                </button>
                              </el-tooltip>
                              <el-tooltip content="获取模型列表" placement="top" :show-after="400">
                                <button type="button" class="model-toolbar-btn" @click="loadModelsForSelected">
                                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10" /><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" /></svg>
                                </button>
                              </el-tooltip>
                              <el-tooltip content="添加" placement="top" :show-after="400">
                                <button type="button" class="model-toolbar-btn" @click="modelAddVisible = !modelAddVisible">
                                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" /></svg>
                                </button>
                              </el-tooltip>
                            </div>
                          </div>
                          <div class="model-section__actions">
                            <el-button size="small" @click="loadModelsForSelected">获取模型列表</el-button>
                            <el-button size="small" @click="loadPresetModels">加载预置模型</el-button>
                            <el-button size="small" :loading="batchTestingModels" @click="testAllModels">全部检测</el-button>
                            <el-button size="small" type="danger" text :disabled="!(detailProv.models || []).length" @click="clearAllModels">清空</el-button>
                          </div>
                          <el-collapse-transition>
                            <div v-if="modelSearchVisible" class="model-search-bar">
                              <el-input v-model="modelSearchQuery" clearable size="small" placeholder="搜索模型...">
                                <template #prefix>
                                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8" /><path d="M21 21l-4.35-4.35" /></svg>
                                </template>
                              </el-input>
                            </div>
                          </el-collapse-transition>
                          <el-collapse-transition>
                            <div v-if="modelAddVisible" class="prov-model-add">
                              <el-input v-model="newModelId" size="small" placeholder="手动输入模型 ID，如 gpt-4o" style="flex:1" @keyup.enter="addModelManually" />
                              <el-button size="small" type="primary" :disabled="!newModelId.trim()" @click="addModelManually">添加</el-button>
                            </div>
                          </el-collapse-transition>
                          <div class="model-list-wrap">
                            <div
                              v-for="group in filteredModelGroups"
                              :key="group.label"
                              class="model-group"
                            >
                              <div v-if="group.label" class="model-group__label">
                                <span>{{ group.label }}</span>
                              </div>
                              <div
                                v-for="m in group.models"
                                :key="m.id"
                                class="model-row"
                              >
                                <span class="model-row__icon">
                                  <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><circle cx="12" cy="12" r="4" /></svg>
                                </span>
                                <span class="model-row__id" :title="m.id">{{ m.id }}</span>
                                <span v-if="modelTestResults[m.id] === 'ok'" class="model-row__status model-row__status--ok">
                                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#22c55e" stroke-width="3"><polyline points="20 6 9 17 4 12" /></svg>
                                </span>
                                <span v-else-if="modelTestResults[m.id] === 'fail'" class="model-row__status model-row__status--fail">
                                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#ef4444" stroke-width="3"><line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" /></svg>
                                </span>
                                <span v-else-if="modelTestResults[m.id] === 'testing'" class="model-test-spinner" />
                                <el-tooltip :content="getModelVision(m.id) ? '已启用视觉' : '启用视觉'" placement="top" :show-after="400">
                                  <button
                                    type="button"
                                    class="model-vision-btn"
                                    :class="{ 'is-on': getModelVision(m.id) }"
                                    @click="toggleModelVision(m.id)"
                                  >
                                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle cx="12" cy="12" r="3" /></svg>
                                  </button>
                                </el-tooltip>
                                <div class="model-row__actions">
                                  <el-tooltip content="检测" placement="top" :show-after="400">
                                    <button type="button" class="model-act-btn" @click="testSingleModel(m.id)">
                                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" /><polyline points="22 4 12 14.01 9 11.01" /></svg>
                                    </button>
                                  </el-tooltip>
                                  <el-tooltip content="删除" placement="top" :show-after="400">
                                    <button type="button" class="model-act-btn model-act-btn--danger" @click="removeModelFromProvider(m.id)">—</button>
                                  </el-tooltip>
                                </div>
                              </div>
                            </div>
                            <p v-if="!filteredModelGroups.some(g => g.models.length)" class="model-empty">暂无模型，点击上方「获取模型列表」或手动添加</p>
                          </div>
                          <p v-if="modelFetchMsg" class="settings-card__foot">{{ modelFetchMsg }}</p>
                        </div>
                        <el-button type="primary" class="btn-accent m-t" @click="saveProvDetail">{{ t('common.save') }}</el-button>
                      </el-form>
                    </template>
                    <div v-else class="empty-detail">
                      <p class="t-sm">{{ t('settings.noProviderSelected') }}</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div v-show="active === 'defaultModel'" class="settings-panel">
              <div class="settings-card">
                <div class="settings-card__title">{{ t('settings.sectionDefaultModel') }}</div>
                <p class="settings-card__intro">{{ t('settings.defaultModelHint') }}</p>
                <div class="model-pick-row">
                  <div class="model-pick-row__text">
                    <div class="setting-row__label">{{ t('settings.modelCardAssistant') }}</div>
                    <div class="setting-row__desc" />
                  </div>
                  <el-select v-model="defaultModelSlot" filterable class="w-full" :placeholder="t('chat.selectModel')">
                    <el-option v-for="o in modelOptions" :key="o.value" :label="o.label" :value="o.value" />
                  </el-select>
                  <el-button class="icon-btn" @click="gearTarget = 'default'; gearOpen = true">
                    <el-icon><Setting /></el-icon>
                  </el-button>
                </div>
                <p class="setting-row__desc m-b">{{ t('pageUi.settingsDefaultAssistant') }}</p>
                <el-select v-model="settings.defaultAssistantId" filterable clearable class="w-full m-b">
                  <el-option v-for="a in assistantList" :key="a.id" :label="a.name" :value="a.id" />
                </el-select>
                <div class="model-pick-row">
                  <div class="model-pick-row__text">
                    <div class="setting-row__label">{{ t('settings.modelCardQuick') }}</div>
                    <div class="setting-row__desc">{{ t('settings.quickModelDesc') }}</div>
                  </div>
                  <el-select v-model="quickModelSlot" filterable class="w-full" :placeholder="t('chat.selectModel')">
                    <el-option v-for="o in modelOptions" :key="o.value" :label="o.label" :value="o.value" />
                  </el-select>
                  <el-button class="icon-btn" @click="gearTarget = 'quick'; gearOpen = true">
                    <el-icon><Setting /></el-icon>
                  </el-button>
                </div>
                <div class="model-pick-row m-t">
                  <div class="model-pick-row__text">
                    <div class="setting-row__label">{{ t('settings.modelCardTranslate') }}</div>
                    <div class="setting-row__desc">{{ t('settings.translateModelDesc') }}</div>
                  </div>
                  <el-select v-model="translateModelSlot" filterable class="w-full" :placeholder="t('chat.selectModel')">
                    <el-option v-for="o in modelOptions" :key="o.value" :label="o.label" :value="o.value" />
                  </el-select>
                  <el-button class="icon-btn" @click="gearTarget = 'translate'; gearOpen = true">
                    <el-icon><Setting /></el-icon>
                  </el-button>
                </div>
              </div>
            </div>

            <!-- 甯歌 -->
            <div v-show="active === 'general'" class="settings-panel">
              <div class="settings-card">
                <div class="settings-card__title">{{ t('settings.sectionGeneral') }}</div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.language') }}</div>
                    <div class="setting-row__desc" />
                  </div>
                  <div class="setting-row__control">
                    <el-select v-model="settings.language" class="w-full" @change="(v: string) => setLocale(v)">
                      <el-option label="中文" value="zh-CN" />
                      <el-option label="English" value="en-US" />
                    </el-select>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('pageUi.settingsSendKey') }}</div>
                    <div class="setting-row__desc" />
                  </div>
                  <div class="setting-row__control">
                    <el-radio-group v-model="settings.sendKey">
                      <el-radio value="Enter">{{ t('pageUi.settingsSendKeyEnter') }}</el-radio>
                      <el-radio value="Shift+Enter">{{ t('pageUi.settingsSendKeyShift') }}</el-radio>
                    </el-radio-group>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('pageUi.settingsLaunchAtStartup') }}</div>
                    <div class="setting-row__desc" />
                  </div>
                  <div class="setting-row__control">
                    <el-switch v-model="settings.launchAtStartup" />
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('pageUi.settingsMinimizeTray') }}</div>
                    <div class="setting-row__desc" />
                  </div>
                  <div class="setting-row__control">
                    <el-switch v-model="settings.minimizeToTray" />
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('pageUi.settingsShowMenuBar') }}</div>
                    <div class="setting-row__desc" />
                  </div>
                  <div class="setting-row__control">
                    <el-switch v-model="settings.showInMenuBar" />
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('pageUi.settingsProxyEnabled') }}</div>
                    <div class="setting-row__desc" />
                  </div>
                  <div class="setting-row__control">
                    <el-switch v-model="settings.proxyEnabled" />
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('pageUi.settingsProxyUrl') }}</div>
                    <div class="setting-row__desc" />
                  </div>
                  <div class="setting-row__control">
                    <el-input v-model="settings.proxyUrl" :disabled="!settings.proxyEnabled" class="w-full" />
                  </div>
                </div>
              </div>
            </div>

            <!-- 鏄剧ず -->
            <div v-show="active === 'display'" class="settings-panel">
              <div class="settings-card">
                <div class="settings-card__title">{{ t('settings.sectionDisplay') }}</div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('pageUi.settingsThemeMode') }}</div>
                    <div class="setting-row__desc" />
                  </div>
                  <div class="setting-row__control">
                    <div class="pills">
                      <button
                        v-for="m in (['light', 'dark', 'system'] as const)"
                        :key="m"
                        type="button"
                        class="pill"
                        :class="{ on: theme.mode === m }"
                        @click="theme.setMode(m)"
                      >{{ t('settings.' + (m === 'light' ? 'themeLight' : m === 'dark' ? 'themeDark' : 'themeSystem')) }}</button>
                    </div>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('pageUi.settingsAccent') }}</div>
                    <div class="setting-row__desc" />
                  </div>
                  <div class="setting-row__control control-row">
                    <el-color-picker v-model="accentProxy" @change="onAccent" />
                    <el-input v-model="accentProxy" class="m-l-input" @change="onAccent" />
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('pageUi.settingsFontSize') }}</div>
                    <div class="setting-row__desc" />
                  </div>
                  <div class="setting-row__control control-col">
                    <el-slider
                      v-model="theme.fontSize"
                      :min="12"
                      :max="20"
                      :step="1"
                      show-stops
                      :show-tooltip="true"
                    />
                    <p class="slider-hint">{{ theme.fontSize }} px</p>
                  </div>
                </div>
                <div class="subhead">{{ t('settings.navBarSettings') }}</div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.navBarPosition') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-radio-group v-model="settings.navBarPosition">
                      <el-radio value="left">{{ t('settings.navBarLeft') }}</el-radio>
                      <el-radio value="top">{{ t('settings.navBarTop') }}</el-radio>
                    </el-radio-group>
                  </div>
                </div>
                <div class="subhead">{{ t('settings.zoomSettings') }}</div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.zoom') }}</div>
                  </div>
                  <div class="setting-row__control control-col stretch">
                    <div class="zoom-bar">
                      <el-button size="small" @click="settings.zoom = Math.max(50, settings.zoom - 5)">−</el-button>
                      <el-slider v-model="settings.zoom" :min="50" :max="200" :step="5" class="flex-1" />
                      <el-button size="small" @click="settings.zoom = Math.min(200, settings.zoom + 5)">+</el-button>
                      <span class="zoom-pct">{{ settings.zoom }}%</span>
                    </div>
                    <el-button size="small" class="m-t" @click="settings.zoom = 100">{{ t('settings.zoomReset') }}</el-button>
                  </div>
                </div>
                <div class="subhead">{{ t('settings.fontSettings') }}</div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.globalFont') }}</div>
                  </div>
                  <div class="setting-row__control control-row">
                    <el-select v-model="settings.globalFont" clearable :placeholder="t('settings.fontDefault')" class="w-full" filterable allow-create>
                      <el-option :label="t('settings.fontDefault')" value="" />
                      <el-option label="Inter, system-ui" value="Inter, system-ui, sans-serif" />
                      <el-option label="Microsoft YaHei" value="Microsoft YaHei, sans-serif" />
                    </el-select>
                    <el-button @click="settings.globalFont = ''">{{ t('settings.zoomReset') }}</el-button>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.codeFont') }}</div>
                  </div>
                  <div class="setting-row__control control-row">
                    <el-select v-model="settings.codeFont" clearable :placeholder="t('settings.fontDefault')" class="w-full" filterable allow-create>
                      <el-option :label="t('settings.fontDefault')" value="" />
                      <el-option label="JetBrains Mono" value="JetBrains Mono, monospace" />
                      <el-option label="Consolas" value="Consolas, monospace" />
                    </el-select>
                    <el-button @click="settings.codeFont = ''">{{ t('settings.zoomReset') }}</el-button>
                  </div>
                </div>
                <div class="subhead">{{ t('settings.topicSettings') }}</div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.topicPosition') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-radio-group v-model="settings.topicPosition">
                      <el-radio value="left">{{ t('settings.topicLeft') }}</el-radio>
                      <el-radio value="right">{{ t('settings.topicRight') }}</el-radio>
                    </el-radio-group>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.autoSwitchTopic') }}</div>
                  </div>
                  <div class="setting-row__control"><el-switch v-model="settings.autoSwitchTopic" /></div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.showTopicTime') }}</div>
                  </div>
                  <div class="setting-row__control"><el-switch v-model="settings.showTopicTime" /></div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.pinTopicTop') }}</div>
                  </div>
                  <div class="setting-row__control"><el-switch v-model="settings.pinTopicTop" /></div>
                </div>
                <div class="subhead">{{ t('settings.assistantSettings') }}</div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.modelIconType') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-radio-group v-model="settings.modelIconType">
                      <el-radio value="model">{{ t('settings.iconTypeModel') }}</el-radio>
                      <el-radio value="emoji">{{ t('settings.iconTypeEmoji') }}</el-radio>
                      <el-radio value="avatar">{{ t('settings.iconTypeAvatar') }}</el-radio>
                      <el-radio value="none">{{ t('settings.iconTypeNone') }}</el-radio>
                    </el-radio-group>
                  </div>
                </div>
                <div class="subhead">{{ t('settings.customCssSection') }}</div>
                <p class="settings-card__intro">
                  <el-link type="primary" href="https://cherrystudio.com" target="_blank" rel="noopener">
                    {{ t('settings.customCssLink') }}
                  </el-link>
                  — {{ t('settings.customCssHint') }}
                </p>
                <el-input v-model="settings.customCss" type="textarea" :rows="4" :placeholder="t('settings.customCssSection')" class="w-full" />
              </div>
            </div>

            <div v-show="active === 'data'" class="settings-panel">
              <div class="settings-split settings-split--data">
                <nav class="data-subnav" aria-label="Data">
                  <button
                    v-for="d in dataNav"
                    :key="d.k"
                    type="button"
                    class="data-subnav__btn"
                    :class="{ on: dataSub === d.k }"
                    @click="dataSub = d.k"
                  >
                    {{ t(d.lk) }}
                  </button>
                </nav>
                <div class="data-sub__main">
                  <div v-show="dataSub === 'basic'" class="settings-card">
                    <div class="settings-card__title">{{ t('settings.dataDirLabel') }}</div>
                    <p class="t-sm m-b">./app-data ({{ t('settings.appDataPath') }})</p>
                    <div class="settings-card__actions">
                      <el-button @click="exportHint">{{ t('pageUi.settingsDataExport') }}</el-button>
                      <el-button @click="importHint">{{ t('pageUi.settingsDataImport') }}</el-button>
                      <el-button @click="dataOpenDirHint">{{ t('settings.openDir') }}</el-button>
                    </div>
                    <p class="t-sm m-t m-b">./logs ({{ t('settings.appLogPath') }})</p>
                    <el-button @click="dataOpenLogsHint">{{ t('settings.openLogs') }}</el-button>
                    <p class="t-sm m-t m-b">{{ t('settings.knowledgeFiles') }}</p>
                    <el-button @click="dataEditKbHint">{{ t('settings.editFiles') }}</el-button>
                    <p class="t-sm m-t m-b">{{ t('settings.clearCache') }} ({{ t('settings.cacheSizeHint') }}: {{ cacheSizeHint }})</p>
                    <el-button @click="clearCacheHint">{{ t('settings.clearCache') }}</el-button>
                    <el-divider />
                    <el-button type="danger" @click="resetDataHint2">{{ t('settings.resetData') }}</el-button>
                  </div>
                  <div v-show="dataSub === 'full'" class="settings-card">
                    <div class="settings-card__title">{{ t('settings.backupAndRestore') }}</div>
                    <el-button @click="exportHint">{{ t('settings.backup') }}</el-button>
                    <el-button @click="importHint">{{ t('settings.restore') }}</el-button>
                    <div class="setting-row m-t">
                      <div class="setting-row__info">
                        <div class="setting-row__label">{{ t('settings.leanBackup') }}</div>
                      </div>
                      <div class="setting-row__control"><el-switch v-model="settings.leanBackup" /></div>
                    </div>
                    <el-divider />
                    <div class="subhead">{{ t('pageUi.settingsWebdav') }}</div>
                    <div class="setting-row">
                      <div class="setting-row__info">
                        <div class="setting-row__label">{{ t('pageUi.settingsWebdavPath') }}</div>
                      </div>
                      <div class="setting-row__control">
                        <el-input v-model="settings.webdavRemotePath" class="w-full" />
                      </div>
                    </div>
                    <div class="setting-row">
                      <div class="setting-row__info">
                        <div class="setting-row__label">{{ t('pageUi.settingsWebdavUser') }}</div>
                      </div>
                      <div class="setting-row__control">
                        <el-input v-model="settings.webdavUsername" class="w-full" />
                      </div>
                    </div>
                    <div class="setting-row">
                      <div class="setting-row__info">
                        <div class="setting-row__label">{{ t('pageUi.settingsWebdavPass') }}</div>
                      </div>
                      <div class="setting-row__control">
                        <el-input v-model="settings.webdavPassword" type="password" show-password class="w-full" />
                      </div>
                    </div>
                    <div class="setting-row">
                      <div class="setting-row__info">
                        <div class="setting-row__label">{{ t('pageUi.settingsAutoBackup') }}</div>
                      </div>
                      <div class="setting-row__control control-row">
                        <el-switch v-model="settings.autoBackup" class="m-r" />
                        <el-input-number v-model="settings.autoBackupInterval" :min="1" :max="168" />
                        <span class="m-l-small">{{ t('pageUi.settingsIntervalHours') }}</span>
                      </div>
                    </div>
                    <p class="t-sm m-t">{{ t('settings.dataLocalBackup') }} / {{ t('settings.dataJianguoyun') }} / {{ t('settings.dataS3') }}</p>
                  </div>
                  <div v-show="dataSub === 'import'" class="settings-card">
                    <div class="settings-card__title">{{ t('settings.dataImportApp') }}</div>
                    <p class="settings-card__intro">{{ t('pageUi.settingsDataImport') }}</p>
                    <el-button @click="importHint">{{ t('pageUi.settingsDataImport') }}</el-button>
                  </div>
                  <div v-show="dataSub === 'export'" class="settings-card">
                    <div class="settings-card__title">{{ t('settings.exportToPhone') }}</div>
                    <div class="setting-row">
                      <div class="setting-row__info">
                        <div class="setting-row__label">{{ t('settings.lanTransfer') }}</div>
                      </div>
                      <div class="setting-row__control">
                        <el-button @click="exportHint">{{ t('settings.startTransfer') }}</el-button>
                      </div>
                    </div>
                    <div class="setting-row">
                      <div class="setting-row__info">
                        <div class="setting-row__label">{{ t('settings.exportToFile') }}</div>
                      </div>
                      <div class="setting-row__control">
                        <el-button @click="exportHint">{{ t('settings.exportFileBtn') }}</el-button>
                      </div>
                    </div>
                    <p class="t-sm m-t">{{ t('settings.dataExportOrder') }} / {{ t('settings.dataExportMd') }}</p>
                  </div>
                  <div v-show="dataSub === 'third'" class="settings-card">
                    <div class="settings-card__title">{{ t('pageUi.settingsWebdav') }}</div>
                    <p class="t-sm m-b">Notion / {{ t('dataHuoye') }} / Joplin / Obsidian / {{ t('dataSiyuan') }}</p>
                    <el-input :placeholder="t('dataNotion')" class="m-b" />
                    <el-input :placeholder="t('dataJoplin')" class="m-b" />
                    <el-input :placeholder="t('dataObsidian')" />
                  </div>
                </div>
              </div>
            </div>

            <div v-show="active === 'mcp'" class="settings-panel">
              <div class="settings-card settings-card--flush">
                <div class="settings-card__title">{{ t('settings.sectionMcp') }}</div>
                <div class="settings-split settings-split--mcp">
                  <div class="mcp-left">
                    <p class="subhead">{{ t('pageUi.add') }} MCP</p>
                    <p class="t-sm m-b">{{ t('settings.mcpBuiltin') }} / {{ t('settings.mcpMarket') }}</p>
                    <ul class="mcp-left__list">
                      <li
                        v-for="r in mcpRecList"
                        :key="r.id"
                        class="mcp-left__item"
                        :class="{ on: mcpPick === r.id }"
                        @click="mcpPick = r.id"
                      >
                        {{ t(r.lk) }}
                      </li>
                      <li
                        v-for="s in mcpServers"
                        :key="s.id"
                        class="mcp-left__item"
                        :class="{ on: mcpPick === s.id }"
                        @click="mcpPick = s.id"
                      >
                        {{ s.name }}
                      </li>
                    </ul>
                    <el-button class="btn-accent w-full m-t" @click="mcpAddOpen = true">{{ t('mcp.add') }}</el-button>
                  </div>
                  <div class="mcp-detail">
                    <div v-if="mcpPick" class="detail-head">
                      <h3 class="detail-title">{{ mcpDetailTitle }}</h3>
                      <div>
                        <el-button size="small" @click="mcpAddOpen = true">{{ t('common.add') }}</el-button>
                        <el-button size="small" @click="mcpAddOpen = true">{{ t('common.edit') }}</el-button>
                      </div>
                    </div>
                    <p v-if="!mcpServerRow" class="t-sm empty-detail">{{ t('mcp.add') }} — {{ mcpDetailTitle }}</p>
                    <el-table
                      v-else
                      v-loading="mcpLoading"
                      :data="[mcpServerRow]"
                      class="settings-table m-t"
                      size="small"
                      border
                    >
                      <el-table-column :label="t('mcp.name')" prop="name" />
                      <el-table-column :label="t('mcp.tools')" min-width="80">
                        <template #default="{ row }">
                          <span class="t-sm">{{ (row.tools || []).length || '—' }}</span>
                        </template>
                      </el-table-column>
                      <el-table-column :label="t('mcp.start')" width="200">
                        <template #default="{ row }">
                          <el-button
                            v-if="row.status !== 'running'"
                            size="small"
                            :loading="mcpBusyId === row.id"
                            @click="startMcp(row.id)"
                          >{{ t('mcp.start') }}</el-button>
                          <el-button
                            v-else
                            size="small"
                            :loading="mcpBusyId === row.id"
                            @click="stopMcp(row.id)"
                          >{{ t('mcp.stop') }}</el-button>
                        </template>
                      </el-table-column>
                    </el-table>
                  </div>
                </div>
              </div>
            </div>

            <div v-show="active === 'skills'" class="settings-panel">
              <div class="settings-card settings-card--flush">
                <div class="settings-card__title">{{ t('settings.sectionSkills') }}</div>
                <div class="settings-split settings-split--skills">
                  <div class="skills-left">
                    <div class="skills-head">
                      <span class="t-sm">{{ t('settings.skillCount', { count: skillStore.skills.length }) }}</span>
                      <el-input v-model="skillQuery" size="small" clearable :placeholder="t('common.search')" class="skills-search" />
                    </div>
                    <div class="prov-list">
                      <button
                        v-for="s in filteredSkills"
                        :key="s.id"
                        type="button"
                        class="prov-card"
                        :class="{ 'is-pick': selectedSkillId === s.id }"
                        @click="selectedSkillId = s.id"
                      >
                        <span class="prov-card__name">{{ s.name }}</span>
                        <el-button size="small" text @click.stop="openSkillDialog(s)">{{ t('common.edit') }}</el-button>
                        <el-button size="small" text type="danger" @click.stop="removeSkill(s.id)">{{ t('common.delete') }}</el-button>
                      </button>
                    </div>
                  </div>
                  <div class="mcp-detail">
                    <template v-if="currentSkill">
                      <h3 class="detail-title">{{ currentSkill.name }}</h3>
                      <p class="t-sm m-b">{{ currentSkill.description }}</p>
                      <p class="t-sm code-preview">{{ currentSkill.instructions?.slice(0, 200) }}{{ (currentSkill.instructions?.length || 0) > 200 ? '…' : '' }}</p>
                    </template>
                    <div v-else class="empty-detail">
                      <p class="t-sm">{{ t('settings.skillNotSelected') }}</p>
                      <p class="t-sm m-t">{{ t('settings.skillInstallZip') }} / {{ t('settings.skillInstallFolder') }}</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div v-show="active === 'webSearch'" class="settings-panel">
              <div class="settings-card settings-card--flush">
                <div class="settings-card__title">{{ t('settings.sectionWebSearch') }}</div>
                <p class="settings-card__intro">{{ t('settings.webSearchHint') }}</p>
                <div class="settings-split settings-split--ws">
                  <div class="mcp-left">
                    <div class="setting-row">
                      <div class="setting-row__info">
                        <div class="setting-row__label">{{ t('settings.sectionWebSearch') }}</div>
                      </div>
                      <div class="setting-row__control">
                        <el-switch
                          :model-value="agent.webSearchEnabled"
                          @change="(v: boolean) => agent.setWebSearchEnabled(v)"
                        />
                      </div>
                    </div>
                    <p class="subhead">{{ t('settings.webSearchApiEngines') }}</p>
                    <ul class="mcp-left__list">
                      <li
                        v-for="e in webApiEngines"
                        :key="e"
                        class="mcp-left__item"
                        :class="{ on: webEnginePick === e }"
                        @click="webEnginePick = e; agent.setSearchEngine(e as WebSearchEngine)"
                      >
                        {{ e }}
                        <el-tag v-if="engineIsDefault(e)" size="small" class="m-l-small">{{ t('settings.defaultBadge') }}</el-tag>
                      </li>
                    </ul>
                    <p class="subhead m-t">{{ t('settings.webSearchLocalEngines') }}</p>
                    <ul class="mcp-left__list">
                      <li
                        v-for="e in webLocalEngines"
                        :key="e"
                        class="mcp-left__item"
                        :class="{ on: webEnginePick === e }"
                        @click="webEnginePick = e; agent.setSearchEngine(e as WebSearchEngine)"
                      >
                        {{ e }}
                        <el-tag v-if="e === 'bing'" size="small" class="m-l-small">{{ t('settings.defaultBadge') }}</el-tag>
                      </li>
                    </ul>
                  </div>
                  <div class="mcp-detail">
                    <h3 class="detail-title">{{ webEnginePick }}</h3>
                    <el-form label-position="top" class="m-t">
                      <el-form-item :label="t('provider.apiKey')">
                        <el-input v-model="webCfgApiKey" type="password" show-password />
                      </el-form-item>
                      <el-form-item :label="t('provider.endpoint')">
                        <el-input v-model="webCfgUrl" type="url" />
                      </el-form-item>
                      <el-button class="btn-accent" @click="setWebDefault">{{ t('settings.setAsDefault') }}</el-button>
                    </el-form>
                  </div>
                </div>
              </div>
            </div>

            <div v-show="active === 'memory'" class="settings-panel">
              <div class="settings-card">
                <div class="mem-head">
                  <div>
                    <span class="settings-card__title">{{ t('settings.sectionGlobalMemory') }}</span>
                    <el-tag class="m-l-small" size="small" type="info">{{ t('settings.memoryBeta') }}</el-tag>
                  </div>
                  <div>
                    <el-switch v-model="settings.enableMemory" class="m-r" />
                    <el-button size="small" text @click="memSettingsOpen = !memSettingsOpen">
                      <el-icon><Setting /></el-icon>
                    </el-button>
                  </div>
                </div>
                <p v-if="memSettingsOpen" class="t-sm m-b">{{ t('settings.memoryHint') }}</p>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.memoryUserManage') }}</div>
                  </div>
                  <div class="setting-row__control control-row">
                    <el-icon class="m-r" aria-hidden="true"><User /></el-icon>
                    <el-select v-model="memoryUserId" :placeholder="t('settings.defaultUser')">
                      <el-option :label="t('settings.defaultUser')" value="default" />
                    </el-select>
                  </div>
                </div>
                <div v-if="settings.enableMemory" class="m-t mem-toolbar">
                  <el-input v-model="memoryQuery" :placeholder="t('memory.search')" clearable class="flex-1" />
                  <el-button class="btn-accent" @click="openMemAdd">{{ t('memory.add') }}</el-button>
                  <el-dropdown>
                    <el-button>{{ t('settings.memoryMore') }} ▾</el-button>
                    <template #dropdown>
                      <el-dropdown-menu>
                        <el-dropdown-item @click="exportHint">{{ t('pageUi.settingsDataExport') }}</el-dropdown-item>
                      </el-dropdown-menu>
                    </template>
                  </el-dropdown>
                </div>
                <ul v-if="displayMemories.length" class="mem-list m-t">
                  <li v-for="m in displayMemories" :key="m.id" class="mem-list__it">
                    <p>{{ m.content }}</p>
                    <el-button size="small" text type="danger" @click="removeMem(m.id)">{{ t('common.delete') }}</el-button>
                  </li>
                </ul>
                <div v-else class="empty-detail m-t">
                  <p class="t-sm">{{ t('memory.empty') }}</p>
                  <el-button class="m-t" @click="openMemAdd">{{ t('settings.memoryAddFirst') }}</el-button>
                </div>
              </div>
            </div>

            <div v-show="active === 'api'" class="settings-panel">
              <div class="settings-card">
                <div class="detail-head m-b">
                  <div>
                    <div class="settings-card__title">{{ t('settings.apiServerTitle') }}</div>
                    <p class="t-sm m-t">{{ t('settings.apiServerDesc') }}</p>
                  </div>
                </div>

                <!-- Status & Controls -->
                <div class="settings-card m-b" style="padding: 14px 16px; border: 1px solid var(--color-border);">
                  <div class="setting-row">
                    <div class="setting-row__info">
                      <div class="setting-row__label">{{ t('settings.apiStatus') }}</div>
                      <div class="setting-row__desc" style="font-family: monospace;">{{ apiServerUrlDisplay }}</div>
                    </div>
                    <div class="setting-row__control control-row">
                      <el-tag :type="proxyServerRunning ? 'success' : 'info'" effect="dark" round>
                        <span v-if="proxyServerRunning" style="display:inline-block;width:6px;height:6px;border-radius:50%;background:#52c41a;margin-right:6px;vertical-align:middle;" />
                        <span v-else style="display:inline-block;width:6px;height:6px;border-radius:50%;background:#999;margin-right:6px;vertical-align:middle;" />
                        {{ proxyServerRunning ? t('settings.apiRunning') : t('settings.apiStopped') }}
                      </el-tag>
                      <el-button size="small" @click="copyText(apiServerUrlDisplay)">{{ t('settings.copyUrl') }}</el-button>
                      <el-button
                        v-if="proxyServerRunning"
                        size="small"
                        type="danger"
                        :loading="proxyLoading"
                        @click="handleStopProxy"
                      >{{ t('settings.stopServer') }}</el-button>
                      <el-button
                        v-else
                        size="small"
                        type="primary"
                        :loading="proxyLoading"
                        @click="handleStartProxy"
                      >{{ t('settings.startServer') }}</el-button>
                    </div>
                  </div>
                </div>

                <!-- Config -->
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.apiPort') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-input-number v-model="settings.apiServerPort" :min="1024" :max="65535" :disabled="proxyServerRunning" />
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.apiKey') }}</div>
                    <div class="setting-row__desc">{{ t('settings.authHeader') }}: Bearer {{ skDisplay }}</div>
                  </div>
                  <div class="setting-row__control control-row">
                    <el-input v-model="settings.apiServerKey" type="password" show-password class="w-full" :disabled="proxyServerRunning" />
                    <el-button size="small" @click="copyText(settings.apiServerKey)">{{ t('settings.copyKey') }}</el-button>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.apiDefaultProvider') }}</div>
                    <div class="setting-row__desc">{{ t('settings.defaultModelHint') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-select v-model="settings.defaultProviderId" :disabled="proxyServerRunning" style="width:220px">
                      <el-option
                        v-for="p in enabledProvidersList"
                        :key="p.id"
                        :label="p.name"
                        :value="p.id"
                      />
                    </el-select>
                  </div>
                </div>
              </div>

              <!-- Endpoints Reference -->
              <div class="settings-card m-t">
                <div class="settings-card__title">{{ t('settings.apiUsageHint') }}</div>

                <div class="proxy-endpoints m-t">
                  <div class="proxy-ep">
                    <span class="proxy-ep__badge proxy-ep__badge--openai">OpenAI</span>
                    <code class="proxy-ep__url">{{ apiServerUrlDisplay }}/v1/chat/completions</code>
                    <el-button size="small" text @click="copyText(`${apiServerUrlDisplay}/v1/chat/completions`)">{{ t('common.copy') }}</el-button>
                  </div>
                  <div class="proxy-ep">
                    <span class="proxy-ep__badge proxy-ep__badge--anthropic">Anthropic</span>
                    <code class="proxy-ep__url">{{ apiServerUrlDisplay }}/v1/messages</code>
                    <el-button size="small" text @click="copyText(`${apiServerUrlDisplay}/v1/messages`)">{{ t('common.copy') }}</el-button>
                  </div>
                  <div class="proxy-ep">
                    <span class="proxy-ep__badge proxy-ep__badge--models">Models</span>
                    <code class="proxy-ep__url">{{ apiServerUrlDisplay }}/v1/models</code>
                    <el-button size="small" text @click="copyText(`${apiServerUrlDisplay}/v1/models`)">{{ t('common.copy') }}</el-button>
                  </div>
                  <div class="proxy-ep">
                    <span class="proxy-ep__badge proxy-ep__badge--health">Health</span>
                    <code class="proxy-ep__url">{{ apiServerUrlDisplay }}/health</code>
                    <el-button size="small" text @click="copyText(`${apiServerUrlDisplay}/health`)">{{ t('common.copy') }}</el-button>
                  </div>
                </div>

                <!-- Usage examples -->
                <div class="proxy-examples m-t">
                  <div class="proxy-example">
                    <div class="proxy-example__title">{{ t('settings.apiCursorExample') }}</div>
                    <pre class="proxy-example__code">OPENAI_API_BASE={{ apiServerUrlDisplay }}/v1
OPENAI_API_KEY={{ settings.apiServerKey || 'your-key' }}</pre>
                  </div>
                  <div class="proxy-example">
                    <div class="proxy-example__title">{{ t('settings.apiClaudeCodeExample') }}</div>
                    <pre class="proxy-example__code">ANTHROPIC_BASE_URL={{ apiServerUrlDisplay }}
ANTHROPIC_API_KEY={{ settings.apiServerKey || 'your-key' }}</pre>
                  </div>
                </div>
              </div>
            </div>

            <div v-show="active === 'channels'" class="settings-panel">
              <div class="settings-card settings-card--flush">
                <div class="settings-card__title">通知渠道</div>
                <p class="settings-card__intro">配置外部 IM 平台的 Webhook 地址，当 AI 回复完成后自动推送通知。支持飞书、钉钉、Telegram、Discord、Slack 和自定义 Webhook。</p>
                <div class="settings-split settings-split--ch">
                  <div class="mcp-left">
                    <ul class="mcp-left__list">
                      <li
                        v-for="p in platformOpts"
                        :key="p.id"
                        class="mcp-left__item"
                        :class="{ on: chPlatform === p.id }"
                        @click="chPlatform = p.id"
                      >
                        <span class="ch-ico" aria-hidden="true">{{ p.icon }}</span> {{ p.name }}
                        <span v-if="channelCountByPlat(p.id)" class="ch-count-badge">{{ channelCountByPlat(p.id) }}</span>
                      </li>
                    </ul>
                  </div>
                  <div class="mcp-detail">
                    <div class="detail-head">
                      <h3 class="detail-title">{{ platformName(chPlatform) }}</h3>
                      <el-button class="btn-accent" @click="addChannelPlat">添加渠道</el-button>
                    </div>
                    <p v-if="!channelsForPlat.length" class="ch-empty-hint">
                      <span class="ch-empty-icon">📭</span>
                      <span>暂未配置 {{ platformName(chPlatform) }} 通知渠道</span>
                      <span class="t-sm">点击「添加渠道」开始配置 Webhook 地址</span>
                    </p>
                    <div v-else class="m-t ch-rows">
                      <div v-for="row in channelsForPlat" :key="row.id" class="ch-card">
                        <div class="ch-card__head">
                          <el-input v-model="row.name" size="small" :placeholder="t('pageUi.channelName')" class="ch-card__name" />
                          <el-switch v-model="row.enabled" />
                        </div>
                        <el-input
                          v-model="row.webhookUrl"
                          size="small"
                          :placeholder="getWebhookPlaceholder(chPlatform)"
                          class="m-t-xs"
                        />
                        <el-input
                          v-if="chPlatform === 'dingtalk'"
                          v-model="row.secret"
                          size="small"
                          placeholder="签名密钥 (Secret)，可选"
                          class="m-t-xs"
                          show-password
                        />
                        <el-input
                          v-model="row.messageTemplate"
                          size="small"
                          placeholder="自定义模板 (可选)，可用 {content} {model} {topic}"
                          class="m-t-xs"
                        />
                        <div class="ch-card__row">
                          <el-checkbox v-model="row.notifyOnReply" label="AI 回复完成后自动推送" />
                        </div>
                        <div class="ch-card__actions">
                          <el-button size="small" @click="testChannelWebhook(row)" :loading="chTestingId === row.id">
                            {{ chTestingId === row.id ? '测试中...' : '测试连接' }}
                          </el-button>
                          <el-button size="small" type="danger" text @click="removeChRow(row)">删除</el-button>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div v-show="active === 'sched'" class="settings-panel">
              <div class="settings-split settings-split--sched">
                <div class="settings-card mcp-left">
                  <div class="detail-head">
                    <div class="settings-card__title">{{ t('settings.sectionSched') }}</div>
                    <el-button size="small" @click="schedAddHint">+ 添加</el-button>
                  </div>
                  <p v-if="!schedTasks.length" class="t-sm m-t">{{ t('settings.schedEmpty') }}</p>
                  <div v-else class="sched-list">
                    <div v-for="x in schedTasks" :key="x.id" class="sched-item">
                      <div class="sched-item__info">
                        <div class="sched-item__name">{{ x.name }}</div>
                        <div class="sched-item__meta">{{ x.cron }} · 已执行 {{ x.runCount }} 次</div>
                      </div>
                      <div class="sched-item__actions">
                        <el-switch :model-value="x.enabled" size="small" @change="schedToggle(x.id)" />
                        <el-button link size="small" @click="schedRun(x.id)">运行</el-button>
                        <el-button link size="small" @click="schedEdit(x)">编辑</el-button>
                        <el-button link size="small" type="danger" @click="schedDelete(x.id)">删除</el-button>
                      </div>
                    </div>
                  </div>
                  <p class="t-sm m-t">{{ t('settings.schedInfo') }}</p>
                </div>
                <div class="settings-card mcp-detail">
                  <div v-if="schedTasks.length">
                    <p class="t-sm">已配置 {{ schedTasks.length }} 个定时任务，{{ scheduleStore.enabledTasks.length }} 个启用中。</p>
                    <p class="t-sm m-t">支持格式: every 5 min, every 1 hour, every 1 day</p>
                  </div>
                  <p v-else class="t-sm">{{ t('settings.schedEmpty') }}</p>
                </div>
              </div>

              <el-dialog v-model="schedDialogVisible" :title="schedEditId ? '编辑定时任务' : '添加定时任务'" width="460px" align-center destroy-on-close>
                <el-form label-position="top">
                  <el-form-item label="任务名称">
                    <el-input v-model="schedForm.name" placeholder="例如：每日总结" />
                  </el-form-item>
                  <el-form-item label="Agent">
                    <el-select v-model="schedForm.agentId" placeholder="选择 Agent" style="width:100%">
                      <el-option v-for="ag in agent.agents" :key="ag.id" :label="ag.name" :value="ag.id" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="提示词">
                    <el-input v-model="schedForm.prompt" type="textarea" :rows="4" placeholder="任务执行时发送给 Agent 的提示词" />
                  </el-form-item>
                  <el-form-item label="执行周期">
                    <el-input v-model="schedForm.cron" placeholder="every 30 min / every 1 hour / every 1 day" />
                  </el-form-item>
                </el-form>
                <template #footer>
                  <el-button @click="schedDialogVisible = false">取消</el-button>
                  <el-button type="primary" @click="schedSave" :disabled="!schedForm.agentId || !schedForm.prompt">保存</el-button>
                </template>
              </el-dialog>
            </div>

            <div v-show="active === 'doc'" class="settings-panel">
              <div class="settings-card">
                <div class="settings-card__title">{{ t('settings.ocrService') }}</div>
                <el-form label-position="top">
                  <el-form-item :label="t('settings.ocrService')">
                    <el-select v-model="settings.ocrProvider" class="w-full">
                      <el-option label="System OCR" value="system" />
                    </el-select>
                  </el-form-item>
                  <el-form-item :label="t('settings.ocrLangs')">
                    <el-select v-model="settings.ocrLanguages" multiple class="w-full">
                      <el-option label="English" value="en" />
                      <el-option label="中文" value="zh" />
                      <el-option label="日本語" value="ja" />
                    </el-select>
                  </el-form-item>
                </el-form>
              </div>
              <div class="settings-card m-t">
                <div class="settings-card__title">{{ t('settings.docService') }}</div>
                <el-form label-position="top">
                  <el-form-item>
                    <el-select v-model="settings.docProvider" class="w-full">
                      <el-option :label="t('settings.docMinerU')" value="mineru" />
                    </el-select>
                  </el-form-item>
                  <el-form-item>
                    <el-link type="primary" href="https://mineru.net" target="_blank">{{ t('settings.mineruLink') }}</el-link>
                  </el-form-item>
                  <el-form-item :label="t('provider.apiKey')">
                    <el-input v-model="settings.mineruApiKey" type="password" show-password />
                  </el-form-item>
                  <el-form-item :label="t('provider.endpoint')">
                    <el-input v-model="settings.mineruApiUrl" type="url" />
                  </el-form-item>
                </el-form>
              </div>
            </div>

            <div v-show="active === 'harness'" class="settings-panel">
              <div class="settings-card">
                <div class="detail-head m-b">
                  <div>
                    <div class="settings-card__title">{{ t('settings.harnessTitle') }}</div>
                    <p class="t-sm m-t">{{ t('settings.harnessDesc') }}</p>
                  </div>
                </div>

                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.harnessEnable') }}</div>
                    <div class="setting-row__desc">{{ t('settings.harnessEnableDesc') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-switch v-model="settings.harnessEnabled" />
                  </div>
                </div>

                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.harnessPlanningGate') }}</div>
                    <div class="setting-row__desc">{{ t('settings.harnessPlanningGateDesc') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-switch v-model="settings.harnessPlanningGate" :disabled="!settings.harnessEnabled" />
                  </div>
                </div>

                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.harnessOutputConstraints') }}</div>
                    <div class="setting-row__desc">{{ t('settings.harnessOutputConstraintsDesc') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-switch v-model="settings.harnessOutputConstraints" :disabled="!settings.harnessEnabled" />
                  </div>
                </div>

                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.harnessFeedbackLoop') }}</div>
                    <div class="setting-row__desc">{{ t('settings.harnessFeedbackLoopDesc') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-switch v-model="settings.harnessFeedbackLoop" :disabled="!settings.harnessEnabled" />
                  </div>
                </div>

                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.harnessMaxRetries') }}</div>
                    <div class="setting-row__desc">{{ t('settings.harnessMaxRetriesDesc') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-input-number
                      v-model="settings.harnessMaxRetries"
                      :min="0"
                      :max="5"
                      :disabled="!settings.harnessEnabled || !settings.harnessFeedbackLoop"
                    />
                  </div>
                </div>

                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.harnessContextBudget') }}</div>
                    <div class="setting-row__desc">{{ t('settings.harnessContextBudgetDesc') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-tag type="success" effect="plain">{{ t('common.enabled') }}</el-tag>
                  </div>
                </div>
              </div>
            </div>

            <div v-show="active === 'phrases'" class="settings-panel">
              <div class="settings-card">
                <div class="detail-head m-b">
                  <div class="settings-card__title">{{ t('settings.sectionPhrases') }}</div>
                  <el-button class="btn-accent" @click="openPhraseEdit(-1)">+</el-button>
                </div>
                <ul class="phrase-list">
                  <li v-for="(ph, idx) in agent.quickPhrases" :key="idx" class="phrase-list__it">
                    <span class="phrase-ico" aria-hidden="true">💬</span>
                    <div class="phrase-body">
                      <div class="phrase-title">{{ ph.slice(0, 24) }}{{ ph.length > 24 ? '…' : '' }}</div>
                      <div class="t-sm phrase-prev">{{ ph }}</div>
                    </div>
                    <el-button size="small" text @click="openPhraseEdit(idx)">{{ t('common.edit') }}</el-button>
                    <el-button size="small" text type="danger" @click="removePhraseAt(idx)">{{ t('common.delete') }}</el-button>
                  </li>
                </ul>
              </div>
            </div>

            <!-- 蹇嵎閿?-->
            <div v-show="active === 'hotkeys'" class="settings-panel">
              <div class="settings-card">
                <div class="settings-card__title">{{ t('settings.sectionHotkeys') }}</div>
                <p class="settings-card__intro">点击快捷键栏位并按下新的组合键来自定义，修改后立即生效</p>
                <el-table :data="shortcutRows" class="settings-table m-t" size="small" border>
                  <el-table-column :label="t('shortcuts.title')" min-width="160">
                    <template #default="{ row }">
                      {{ t(row.labelKey) }}
                      <el-tag v-if="row.isGlobal" size="small" type="info" style="margin-left:6px">全局</el-tag>
                    </template>
                  </el-table-column>
                  <el-table-column :label="t('shortcuts.pressKey')" width="200">
                    <template #default="{ row }">
                      <div
                        class="shortcut-recorder"
                        :class="{ recording: row.editing }"
                        tabindex="0"
                        @focus="row.editing = true"
                        @blur="row.editing = false; syncShortcuts()"
                        @keydown.prevent.stop="(e: KeyboardEvent) => onKeyRec(row, e)"
                      >
                        <span class="shortcut-keys">{{ formatKeys(row.keys) }}</span>
                        <span v-if="row.editing" class="shortcut-hint">按下快捷键...</span>
                      </div>
                    </template>
                  </el-table-column>
                  <el-table-column :label="t('pageUi.enabled')" width="80" align="center">
                    <template #default="{ row }">
                      <el-switch v-model="row.enabled" @change="syncShortcuts()" />
                    </template>
                  </el-table-column>
                  <el-table-column width="80" align="center">
                    <template #default="{ row }">
                      <el-button size="small" text @click="resetShortcut(row)">{{ t('shortcuts.reset') }}</el-button>
                    </template>
                  </el-table-column>
                </el-table>
              </div>
            </div>

            <div v-show="active === 'quickAsst'" class="settings-panel">
              <div class="settings-card">
                <div class="detail-head m-b">
                  <div>
                    <div class="settings-card__title">{{ t('settings.sectionQuickAsst') }}</div>
                    <el-link type="primary" class="m-t" @click.prevent="faqHint()">{{ t('settings.quickAsstFaq') }}</el-link>
                  </div>
                  <el-switch v-model="qa.enabled" />
                </div>
                <p class="subhead">{{ t('settings.selectToolbar') }}</p>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.pickBySelect') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-radio-group v-model="qa.method">
                      <el-radio value="select">{{ t('settings.pickBySelect') }}</el-radio>
                      <el-radio value="ctrl">{{ t('settings.pickByCtrl') }}</el-radio>
                      <el-radio value="shortcut">{{ t('settings.pickByShortcut') }}</el-radio>
                    </el-radio-group>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.brushMode') }}</div>
                  </div>
                  <div class="setting-row__control"><el-switch v-model="qa.brush" /></div>
                </div>
                <p class="subhead">{{ t('settings.selectWindow') }}</p>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.followMouseToolbar') }}</div>
                  </div>
                  <div class="setting-row__control"><el-switch v-model="qa.toolbar" /></div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.rememberSize') }}</div>
                  </div>
                  <div class="setting-row__control"><el-switch v-model="qa.remember" /></div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.autoClose') }}</div>
                  </div>
                  <div class="setting-row__control"><el-switch v-model="qa.autoclose" /></div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.autoTop') }}</div>
                  </div>
                  <div class="setting-row__control"><el-switch v-model="qa.autotop" /></div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.opacity') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-slider v-model="qa.opacity" :min="20" :max="100" />
                  </div>
                </div>
              </div>
            </div>

            <div v-show="active === 'selection'" class="settings-panel">
              <div class="settings-card">
                <div class="detail-head m-b">
                  <div>
                    <div class="settings-card__title">{{ t('settings.sectionSelection') }}</div>
                    <el-link type="primary" class="m-t" @click.prevent="faqHint()">{{ t('settings.selectionFaq') }}</el-link>
                  </div>
                  <el-switch v-model="settings.selectionAssistantEnabled" />
                </div>
                <p class="subhead">{{ t('settings.selectToolbar') }}</p>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.selectionHint') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-radio-group v-model="settings.selectionMethod">
                      <el-radio value="select">{{ t('settings.pickBySelect') }}</el-radio>
                      <el-radio value="ctrl">{{ t('settings.pickByCtrl') }}</el-radio>
                      <el-radio value="shortcut">{{ t('settings.pickByShortcut') }}</el-radio>
                    </el-radio-group>
                  </div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.brushMode') }}</div>
                  </div>
                  <div class="setting-row__control"><el-switch v-model="settings.brushMode" /></div>
                </div>
                <p class="subhead">{{ t('settings.selectWindow') }}</p>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.followMouseToolbar') }}</div>
                  </div>
                  <div class="setting-row__control"><el-switch v-model="settings.selToolbarAutoShow" /></div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.rememberSize') }}</div>
                  </div>
                  <div class="setting-row__control"><el-switch v-model="settings.selRememberSize" /></div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.autoClose') }}</div>
                  </div>
                  <div class="setting-row__control"><el-switch v-model="settings.selAutoClose" /></div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.autoTop') }}</div>
                  </div>
                  <div class="setting-row__control"><el-switch v-model="settings.selAutoTop" /></div>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('settings.opacity') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-slider v-model="settings.selOpacity" :min="20" :max="100" />
                  </div>
                </div>
                <p class="subhead">{{ t('settings.selectActions') }}</p>
                <div class="sel-actions">
                  <button
                    v-for="a in selActionDefs"
                    :key="a.id"
                    type="button"
                    class="sel-act"
                    :class="{ off: !settings.selActions.includes(a.id) }"
                    @click="toggleSelAction(a.id)"
                  >
                    {{ t(a.lk) }}
                  </button>
                  <el-button size="small" @click="selCustomHint">{{ t('settings.customAction') }}</el-button>
                </div>
              </div>
            </div>

            <!-- 鍏充簬 -->
            <div v-show="active === 'about'" class="settings-panel">
              <div class="settings-card about-block">
                <div class="settings-card__title">{{ t('settings.sectionAbout') }}</div>
                <div class="about-card">
                  <div class="app-name">{{ APP_CONFIG.displayName }}</div>
                  <p class="ver">{{ t('pageUi.settingsVersion') }}: {{ APP_CONFIG.version }}</p>
                  <p class="desc">{{ t('pageUi.settingsAboutDesc') }}</p>
                  <p class="links">{{ t('pageUi.settingsLinks') }} —</p>
                </div>
                <div class="setting-row">
                  <div class="setting-row__info">
                    <div class="setting-row__label">{{ t('pageUi.settingsClear') }}</div>
                    <div class="setting-row__desc">{{ t('pageUi.settingsClearDesc') }}</div>
                  </div>
                  <div class="setting-row__control">
                    <el-button type="danger" plain @click="clearDataHint">{{ t('pageUi.settingsClear') }}</el-button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <el-dialog
      v-model="provOpen"
      :title="editProvId ? t('provider.edit') : provAddStep === 'pick' ? '选择模型提供商' : `配置 ${provForm.name}`"
      :width="provAddStep === 'pick' && !editProvId ? '680px' : '520px'"
      align-center
      destroy-on-close
    >
      <template v-if="provAddStep === 'pick' && !editProvId">
        <el-input
          v-model="provPresetSearch"
          clearable
          size="small"
          placeholder="搜索提供商..."
          class="m-b"
        />
        <div class="prov-preset-grid">
          <button
            v-for="preset in filteredPresets"
            :key="preset.id"
            type="button"
            class="prov-preset-card"
            @click="pickPreset(preset)"
          >
            <span class="prov-preset-card__name">{{ preset.name }}</span>
            <span class="prov-preset-card__desc">{{ preset.desc }}</span>
            <span class="prov-preset-card__count">{{ preset.models.length ? preset.models.length + ' 模型' : '自定义' }}</span>
          </button>
        </div>
      </template>

      <template v-else>
        <div v-if="provSelectedPreset && !editProvId" class="prov-config-header">
          <div>
            <div class="prov-config-header__name">{{ provSelectedPreset.name }}</div>
            <div class="prov-config-header__hint">{{ provSelectedPreset.desc }}</div>
          </div>
        </div>

        <el-form label-position="top" class="prov-config-form">
          <el-form-item label="API Key" required>
            <el-input v-model="provForm.apiKey" type="password" show-password placeholder="sk-..." />
          </el-form-item>
          <el-form-item label="API 地址" :class="{ 'is-dimmed': provSelectedPreset && provForm.apiEndpoint === provSelectedPreset.apiEndpoint }">
            <el-input v-model="provForm.apiEndpoint" :placeholder="provForm.channelType === 8 ? 'https://your-api.com/v1 (输入到版本路径即可)' : 'https://api.example.com'" />
            <p v-if="provSelectedPreset && provForm.apiEndpoint === provSelectedPreset.apiEndpoint" class="prov-config-auto-hint">已自动填入默认地址</p>
            <p v-if="provForm.apiEndpoint" class="url-preview">
              预览: {{ buildPreviewUrl(provForm.apiEndpoint, provForm.channelType) }}
            </p>
            <p v-if="provForm.channelType === 8" class="prov-config-auto-hint" style="color:var(--color-text-3);margin-top:4px">
              填写 base URL 即可，例如 https://api.example.com/v2<br/>
              系统会自动拼接 /chat/completions 路径。<br/>
              模型 ID 需要在下方手动添加。
            </p>
          </el-form-item>
          <el-collapse-transition>
            <div class="prov-config-advanced">
              <el-form-item :label="t('provider.name')">
                <el-input v-model="provForm.name" />
              </el-form-item>
              <el-form-item label="渠道类型">
                <el-select v-model="provForm.channelType" style="width:100%">
                  <el-option label="OpenAI" :value="1" />
                  <el-option label="Azure OpenAI" :value="3" />
                  <el-option label="Ollama" :value="4" />
                  <el-option label="Custom/兼容" :value="8" />
                  <el-option label="Anthropic" :value="14" />
                  <el-option label="百度文心" :value="15" />
                  <el-option label="智谱 AI" :value="16" />
                  <el-option label="阿里通义" :value="17" />
                  <el-option label="OpenRouter" :value="20" />
                  <el-option label="腾讯混元" :value="23" />
                  <el-option label="Google Gemini" :value="24" />
                  <el-option label="Moonshot" :value="25" />
                  <el-option label="智谱 GLM-4 V4" :value="26" />
                  <el-option label="Perplexity" :value="27" />
                  <el-option label="零一万物" :value="31" />
                  <el-option label="Cohere" :value="34" />
                  <el-option label="MiniMax" :value="35" />
                  <el-option label="SiliconFlow" :value="40" />
                  <el-option label="Mistral" :value="42" />
                  <el-option label="DeepSeek" :value="43" />
                  <el-option label="火山引擎" :value="45" />
                  <el-option label="xAI/Grok" :value="48" />
                  <el-option label="Coze" :value="49" />
                </el-select>
              </el-form-item>
              <el-form-item :label="t('pageUi.enabled')">
                <el-switch v-model="provForm.enabled" />
              </el-form-item>
            </div>
          </el-collapse-transition>
          <div v-if="provSelectedPreset && provSelectedPreset.models.length && !editProvId" class="prov-config-models">
            <p class="prov-config-models__title">将自动添加 {{ provSelectedPreset.models.length }} 个模型（保存后会尝试从 API 自动获取最新列表）</p>
            <div class="prov-config-models__list">
              <span v-for="m in provSelectedPreset.models" :key="m" class="prov-config-models__tag">{{ m }}</span>
            </div>
          </div>
        </el-form>
      </template>

      <template #footer>
        <el-button v-if="provAddStep === 'config' && !editProvId" @click="provAddStep = 'pick'">← 返回选择</el-button>
        <el-button @click="provOpen = false">{{ t('common.cancel') }}</el-button>
        <el-button
          v-if="provAddStep === 'config'"
          type="primary"
          :loading="provSaving"
          :disabled="!provForm.apiKey && provForm.apiEndpoint !== 'http://localhost:11434/v1' && provForm.apiEndpoint !== 'http://localhost:1234/v1'"
          @click="saveProvider"
        >{{ t('common.save') }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="mcpAddOpen"
      :title="t('pageUi.settingsMcpAddTitle')"
      width="480px"
      align-center
      destroy-on-close
    >
      <el-form label-position="top">
        <el-form-item :label="t('pageUi.settingsMcpName')">
          <el-input v-model="mcpForm.name" />
        </el-form-item>
        <el-form-item :label="t('pageUi.settingsMcpCommand')">
          <el-input v-model="mcpForm.command" />
        </el-form-item>
        <el-form-item :label="t('pageUi.settingsMcpArgs')">
          <el-input v-model="mcpForm.argsStr" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="mcpAddOpen = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="mcpSaving" @click="submitMcp">{{ t('common.save') }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="gearOpen" :title="t('common.edit')" width="480px" align-center destroy-on-close>
      <el-select v-model="gearSlot" filterable class="w-full" :placeholder="t('chat.selectModel')">
        <el-option v-for="o in modelOptions" :key="o.value" :label="o.label" :value="o.value" />
      </el-select>
      <template #footer>
        <el-button @click="gearOpen = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="applyGearSlot">{{ t('common.save') }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="phraseOpen" :title="t('settings.phraseAdd')" width="480px" align-center destroy-on-close>
      <el-input v-model="phraseText" type="textarea" :rows="4" :placeholder="t('settings.phraseContent')" />
      <template #footer>
        <el-button @click="phraseOpen = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="savePhrase">{{ t('common.save') }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="memAddOpen" :title="t('memory.add')" width="480px" align-center destroy-on-close>
      <el-input v-model="memText" type="textarea" :rows="4" :placeholder="t('memory.content')" />
      <template #footer>
        <el-button @click="memAddOpen = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="saveMem">{{ t('common.save') }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="skillOpen" :title="t('skill.title')" width="520px" align-center destroy-on-close>
      <el-form label-position="top">
        <el-form-item :label="t('common.edit') + ' / ' + t('provider.name')">
          <el-input v-model="skillForm.name" />
        </el-form-item>
        <el-form-item :label="t('skill.description')">
          <el-input v-model="skillForm.description" type="textarea" :rows="2" />
        </el-form-item>
        <el-form-item :label="t('skill.trigger')">
          <el-input v-model="skillForm.trigger" />
        </el-form-item>
        <el-form-item :label="t('skill.instructions')">
          <el-input v-model="skillForm.instructions" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="skillOpen = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="saveSkillForm">{{ t('common.save') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { Setting, User } from '@element-plus/icons-vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { v4 as uuidv4 } from 'uuid'
import { useSettingsStore } from '@/stores/settings'
import { useThemeStore } from '@/stores/theme'
import { useProviderStore } from '@/stores/provider'
import { useAssistantStore } from '@/stores/assistant'
import { useAgentStore, type WebSearchEngine } from '@/stores/agent'
import { useMemoryStore } from '@/stores/memory'
import { useSkillStore } from '@/stores/skill'
import { storeToRefs } from 'pinia'
import { APP_CONFIG } from '@/config/app.config'
import { mcpApi } from '@/utils/tauri-api'
import { useScheduleStore } from '@/stores/schedule'
import type { Provider, Skill } from '@/types'

const { t, locale: i18nLocale } = useI18n()

type Section =
  'model' | 'defaultModel' | 'general' | 'display' | 'data' | 'mcp' | 'skills' | 'webSearch' | 'memory' |
  'api' | 'channels' | 'sched' | 'doc' | 'harness' | 'phrases' | 'hotkeys' | 'quickAsst' | 'selection' | 'about'

const active = ref<Section>('model')

const settings = useSettingsStore()
const theme = useThemeStore()
const provider = useProviderStore()
const assistant = useAssistantStore()
const agent = useAgentStore()
const memory = useMemoryStore()
const skillStore = useSkillStore()
const { assistants: assistantList } = storeToRefs(assistant)

const accentProxy = ref(theme.accentColor)
watch(() => theme.accentColor, v => { accentProxy.value = v }, { immediate: true })

const wideSection = computed(() => ['model', 'mcp', 'data', 'webSearch', 'channels', 'memory', 'skills'].includes(active.value))

const providerQuery = ref('')
const filteredProviders = computed(() => {
  const q = providerQuery.value.trim().toLowerCase()
  return provider.providers.filter(p => {
    if (q && !p.name.toLowerCase().includes(q)) return false
    if (provFilterEnabled.value && !p.enabled) return false
    return true
  })
})
const selectedProvId = ref<string | null>(null)
const provDetail = reactive({ name: '', channelType: 1 as number, apiKey: '', apiEndpoint: '', enabled: true, supportsVision: false })

const detailProv = computed(() => (selectedProvId.value ? provider.getProviderById(selectedProvId.value) : undefined))
watch(
  [selectedProvId, () => provider.providers],
  () => {
    const list = provider.providers
    if (list.length && (!selectedProvId.value || !list.some(p => p.id === selectedProvId.value))) {
      selectedProvId.value = list[0].id
    }
    const p = selectedProvId.value ? provider.getProviderById(selectedProvId.value) : null
    if (p) {
      provDetail.name = p.name
      provDetail.channelType = p.channelType
      provDetail.apiKey = p.apiKey
      provDetail.apiEndpoint = p.apiEndpoint
      provDetail.enabled = p.enabled
      provDetail.supportsVision = p.supportsVision ?? false
    }
  },
  { immediate: true, deep: true }
)

function toggleProvider(id: string, on: boolean) {
  const p = provider.getProviderById(id)
  if (p) void provider.updateProvider(id, { enabled: on } as Partial<Provider>)
}

function buildPreviewUrl(baseUrl: string, channelType: number): string {
  if (!baseUrl) return ''
  const base = baseUrl.replace(/\/+$/, '')
  const SPECIAL: Record<number, (b: string) => string> = {
    14: b => `${b}/v1/messages`,
    35: b => `${b}/v1/text/chatcompletion_v2`,
    24: b => {
      const v = b.includes('/v1beta') ? '' : '/v1beta'
      return `${b}${v}/models/{model}:streamGenerateContent?alt=sse&key=***`
    },
    3: b => `${b}/openai/deployments/{model}/chat/completions?api-version=2024-06-01`,
    17: b => `${b}/compatible-mode/v1/chat/completions`,
    34: b => `${b}/compatibility/v1/chat/completions`,
    45: b => `${b}/api/v3/chat/completions`,
    26: b => `${b}/chat/completions`,
  }
  if (SPECIAL[channelType]) return SPECIAL[channelType](base)
  if (base.includes('/chat/completions')) return base
  const lastSeg = base.split('/').pop() || ''
  const hasVer = lastSeg.length >= 2 && lastSeg.startsWith('v') && /^\d+$/.test(lastSeg.slice(1))
  return hasVer ? `${base}/chat/completions` : `${base}/v1/chat/completions`
}
async function saveProvDetail() {
  if (!selectedProvId.value) return
  await provider.updateProvider(selectedProvId.value, {
    name: provDetail.name,
    channelType: provDetail.channelType,
    apiKey: provDetail.apiKey,
    apiEndpoint: provDetail.apiEndpoint,
    enabled: provDetail.enabled,
    supportsVision: provDetail.supportsVision,
  } as Partial<Provider>)
  ElMessage.success(t('common.success'))
}
const newModelId = ref('')

function loadModelsForSelected() {
  if (selectedProvId.value) void loadModels({ id: selectedProvId.value })
}

async function addModelManually() {
  const mid = newModelId.value.trim()
  if (!mid || !selectedProvId.value) return
  const p = provider.getProviderById(selectedProvId.value)
  if (!p) return
  const models = [...(p.models || [])]
  if (models.includes(mid)) {
    ElMessage.warning('该模型已存在')
    return
  }
  models.push(mid)
  await provider.updateProvider(selectedProvId.value, { models } as Partial<Provider>)
  newModelId.value = ''
  ElMessage.success(`已添加模型: ${mid}`)
}

async function removeModelFromProvider(modelId: string) {
  if (!selectedProvId.value) return
  const p = provider.getProviderById(selectedProvId.value)
  if (!p) return
  const models = (p.models || []).filter(m => m !== modelId)
  await provider.updateProvider(selectedProvId.value, { models } as Partial<Provider>)
}

function getModelVision(modelId: string): boolean {
  const p = detailProv.value
  if (!p) return false
  return p.modelSettings?.[modelId]?.supportsVision ?? p.supportsVision ?? false
}

async function toggleModelVision(modelId: string) {
  if (!selectedProvId.value) return
  const p = provider.getProviderById(selectedProvId.value)
  if (!p) return
  const ms = { ...(p.modelSettings || {}) }
  const cur = ms[modelId]?.supportsVision ?? p.supportsVision ?? false
  ms[modelId] = { ...(ms[modelId] || {}), supportsVision: !cur }
  await provider.updateProvider(selectedProvId.value, { modelSettings: ms } as Partial<Provider>)
}

async function clearAllModels() {
  if (!selectedProvId.value) return
  await provider.updateProvider(selectedProvId.value, { models: [] } as Partial<Provider>)
  ElMessage.success('已清空全部模型')
}

const modelTestResults = reactive<Record<string, 'ok' | 'fail' | 'testing'>>({})
const batchTestingModels = ref(false)
const modelSearchVisible = ref(false)
const modelAddVisible = ref(true)
const modelSearchQuery = ref('')
const provFilterEnabled = ref(false)

function getProviderColor(name: string): string {
  const colors = [
    '#3b82f6', '#8b5cf6', '#ec4899', '#f59e0b', '#10b981',
    '#06b6d4', '#f43f5e', '#6366f1', '#14b8a6', '#a855f7',
    '#e11d48', '#0ea5e9', '#84cc16', '#7c3aed'
  ]
  let hash = 0
  for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash)
  return colors[Math.abs(hash) % colors.length]
}

const filteredModelGroups = computed(() => {
  const models = detailProv.value?.models || []
  const q = modelSearchQuery.value.trim().toLowerCase()
  const list = q
    ? models.filter(m => m.toLowerCase().includes(q))
    : models

  const groups: Map<string, string[]> = new Map()
  for (const m of list) {
    const slash = m.indexOf('/')
    const label = slash > 0 ? m.slice(0, slash) : ''
    if (!groups.has(label)) groups.set(label, [])
    groups.get(label)!.push(m)
  }
  return Array.from(groups.entries()).map(([label, ids]) => ({
    label,
    models: ids.map(id => ({ id }))
  }))
})

async function testSingleModel(modelId: string) {
  if (!selectedProvId.value) return
  const p = provider.getProviderById(selectedProvId.value)
  if (!p) return
  modelTestResults[modelId] = 'testing'
  try {
    const { sendChatMessage, onStreamChunk, onStreamEnd, onStreamError } = await import('@/utils/tauri-api')
    const msgId = `test_${Date.now()}_${modelId}`
    let gotReply = false
    const unsubs: (() => void)[] = []
    const result = await new Promise<boolean>((resolve) => {
      const timeout = setTimeout(() => resolve(gotReply), 60000)
      void Promise.all([
        onStreamChunk(ev => {
          if (ev.messageId === msgId && ev.chunk) gotReply = true
        }),
        onStreamEnd(ev => {
          if (ev.messageId === msgId) { clearTimeout(timeout); resolve(gotReply) }
        }),
        onStreamError(ev => {
          if (ev.messageId === msgId) { clearTimeout(timeout); resolve(false) }
        }),
      ]).then(fns => unsubs.push(...fns))
      sendChatMessage({
        providerId: p.id,
        modelId,
        messageId: msgId,
        messages: [{ role: 'user', content: 'Hi' }],
      }).catch(() => { clearTimeout(timeout); resolve(false) })
    })
    for (const u of unsubs) u()
    modelTestResults[modelId] = result ? 'ok' : 'fail'
  } catch {
    modelTestResults[modelId] = 'fail'
  }
}

async function testAllModels() {
  if (!selectedProvId.value) return
  const p = provider.getProviderById(selectedProvId.value)
  if (!p?.models?.length) return
  batchTestingModels.value = true
  for (const m of p.models) {
    await testSingleModel(m)
  }
  batchTestingModels.value = false
  const ok = p.models.filter(m => modelTestResults[m] === 'ok').length
  const fail = p.models.filter(m => modelTestResults[m] === 'fail').length
  ElMessage.info(`检测完成: ${ok} 可用, ${fail} 不可用`)
}
async function loadPresetModels() {
  if (!selectedProvId.value) return
  const p = provider.getProviderById(selectedProvId.value)
  if (!p) return
  const preset = PROVIDER_PRESETS.find(pr => pr.name === p.name || pr.apiEndpoint === p.apiEndpoint)
  if (preset?.models?.length) {
    const models = presetModelsToStrings(preset.models)
    await provider.updateProvider(selectedProvId.value, { models } as Partial<Provider>)
    modelFetchMsg.value = `已加载 ${models.length} 个预置模型`
    ElMessage.success(modelFetchMsg.value)
  } else {
    ElMessage.warning('该提供商没有预置模型列表')
  }
}

const modelOptions = computed(() => {
  const o: { label: string; value: string }[] = []
  for (const p of provider.getEnabledProviders()) {
    for (const m of p.models || []) {
      o.push({ label: `${m} | ${p.name}`, value: `${p.id}||${m}` })
    }
  }
  return o
})
function useSlot(
  getP: () => string,
  getM: () => string,
  setPair: (pid: string, mid: string) => void
) {
  return computed({
    get: () => {
      const a = getP()
      const b = getM()
      return a && b ? `${a}||${b}` : ''
    },
    set: (v: string) => {
      const [a, b] = (v || '').split('||')
      setPair(a || '', b || '')
    }
  })
}
const defaultModelSlot = useSlot(
  () => settings.defaultProviderId,
  () => settings.defaultModelId,
  (pid, mid) => { settings.defaultProviderId = pid; settings.defaultModelId = mid }
)
const quickModelSlot = useSlot(
  () => settings.quickModelProviderId,
  () => settings.quickModelId,
  (pid, mid) => { settings.quickModelProviderId = pid; settings.quickModelId = mid }
)
const translateModelSlot = useSlot(
  () => settings.translateModelProviderId,
  () => settings.translateModelId,
  (pid, mid) => { settings.translateModelProviderId = pid; settings.translateModelId = mid }
)
const gearTarget = ref<'default' | 'quick' | 'translate'>('default')
const gearOpen = ref(false)
const gearSlot = ref('')
function applyGearSlot() {
  if (gearTarget.value === 'default') defaultModelSlot.value = gearSlot.value
  else if (gearTarget.value === 'quick') quickModelSlot.value = gearSlot.value
  else translateModelSlot.value = gearSlot.value
  gearOpen.value = false
}
watch(gearOpen, o => { if (o) { if (gearTarget.value === 'default') gearSlot.value = defaultModelSlot.value; else if (gearTarget.value === 'quick') gearSlot.value = quickModelSlot.value; else gearSlot.value = translateModelSlot.value } })

const dataSub = ref<'basic' | 'full' | 'import' | 'export' | 'third'>('basic')
const dataNav = [
  { k: 'basic' as const, lk: 'settings.dataNavBasic' },
  { k: 'full' as const, lk: 'settings.dataNavBackup' },
  { k: 'import' as const, lk: 'settings.dataNavInput' },
  { k: 'export' as const, lk: 'settings.dataNavExport' },
  { k: 'third' as const, lk: 'settings.dataNavThird' }
]
const cacheSizeHint = ref('—')
async function dataOpenDirHint() {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const { appDataDir } = await import('@tauri-apps/api/path')
    const dir = await appDataDir()
    await invoke('plugin:shell|open', { path: dir })
  } catch {
    ElMessage.info(t('settings.openDir'))
  }
}
async function dataOpenLogsHint() {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const { appLogDir } = await import('@tauri-apps/api/path')
    const dir = await appLogDir()
    await invoke('plugin:shell|open', { path: dir })
  } catch {
    ElMessage.info(t('settings.openLogs'))
  }
}
function dataEditKbHint() { ElMessage.info(t('settings.editFiles')) }
function clearCacheHint() {
  try {
    localStorage.removeItem('fox-ai-cache')
    sessionStorage.clear()
    ElMessage.success('缓存已清除')
  } catch {
    ElMessage.success(t('settings.clearCache'))
  }
}
function resetDataHint2() {
  void ElMessageBox.confirm(t('settings.clearConfirm'), { type: 'warning' })
    .then(() => {
      localStorage.clear()
      ElMessage.success('数据已重置，即将重新加载...')
      setTimeout(() => window.location.reload(), 1000)
    }).catch(() => {})
}

const mcpRecList = [
  { id: 'aliyun', lk: 'settings.mcpAlibaba' as const },
  { id: 'ms', lk: 'settings.mcpModelScope' as const },
  { id: 'tf', lk: 'settings.mcpTokenFlux' as const },
  { id: 'lb', lk: 'settings.mcpLanbiao' as const },
  { id: 'i302', lk: 'settings.mcp302' as const },
  { id: 'router', lk: 'settings.mcpRouter' as const }
]
const mcpPick = ref('')
const mcpServerRow = computed(() => mcpServers.value.find(s => s.id === mcpPick.value) || null)
const mcpDetailTitle = computed(() => {
  const s = mcpServerRow.value
  if (s) return s.name
  const r = mcpRecList.find(x => x.id === mcpPick.value)
  return r ? t(r.lk) : mcpPick.value || ''
})

const skillQuery = ref('')
const selectedSkillId = ref<string | null>(null)
const filteredSkills = computed(() => skillStore.searchSkills(skillQuery.value))
const currentSkill = computed<Skill | null>(() =>
  selectedSkillId.value ? skillStore.skills.find(s => s.id === selectedSkillId.value) || null : null
)
watch(filteredSkills, list => {
  if (list.length && (!selectedSkillId.value || !list.some(s => s.id === selectedSkillId.value))) {
    selectedSkillId.value = list[0].id
  } else if (!list.length) selectedSkillId.value = null
}, { immediate: true })
const skillOpen = ref(false)
const skillForm = reactive({ id: '' as string, name: '', description: '', trigger: '', instructions: '' })
function openSkillDialog(s: Skill) {
  skillForm.id = s.id; skillForm.name = s.name; skillForm.description = s.description; skillForm.trigger = s.trigger; skillForm.instructions = s.instructions
  skillOpen.value = true
}
function saveSkillForm() {
  if (skillForm.id) {
    skillStore.updateSkill(skillForm.id, { name: skillForm.name, description: skillForm.description, trigger: skillForm.trigger, instructions: skillForm.instructions })
  }
  skillOpen.value = false
  ElMessage.success(t('common.success'))
}
function removeSkill(id: string) {
  void ElMessageBox.confirm(t('skill.deleteConfirm'), { type: 'warning' }).then(() => {
    skillStore.deleteSkill(id)
    if (selectedSkillId.value === id) selectedSkillId.value = null
  }).catch(() => {})
}

const webApiEngines: WebSearchEngine[] = ['zhipu', 'tavily', 'searxng', 'exa', 'examcp', 'bocha', 'querit']
const webLocalEngines: WebSearchEngine[] = ['google', 'bing', 'baidu']
const webEnginePick = ref<WebSearchEngine>(agent.webSearchEngine)
watch(() => agent.webSearchEngine, v => { webEnginePick.value = v }, { immediate: true })
const webCfgApiKey = computed({
  get: () => settings.getWebSearchConfig(webEnginePick.value).apiKey,
  set: (v: string) => settings.setWebSearchConfig(webEnginePick.value, { apiKey: v })
})
const webCfgUrl = computed({
  get: () => settings.getWebSearchConfig(webEnginePick.value).apiUrl,
  set: (v: string) => settings.setWebSearchConfig(webEnginePick.value, { apiUrl: v })
})
function setWebDefault() { agent.setSearchEngine(webEnginePick.value) }
function engineIsDefault(e: string) { return agent.webSearchEngine === e }

const memSettingsOpen = ref(false)
const memoryUserId = ref('default')
const memoryQuery = ref('')
const displayMemories = computed(() => memory.searchMemories(memoryQuery.value))
const memAddOpen = ref(false)
const memText = ref('')
function openMemAdd() { memText.value = ''; memAddOpen.value = true }
function saveMem() {
  if (!memText.value.trim()) return
  memory.addMemory({ content: memText.value, category: 'fact', source: 'user' })
  memAddOpen.value = false
  ElMessage.success(t('common.success'))
}
function removeMem(id: string) {
  void ElMessageBox.confirm(t('memory.deleteConfirm'), { type: 'warning' })
    .then(() => { memory.deleteMemory(id) })
    .catch(() => {})
}

const apiServerUrlDisplay = computed(() => `http://127.0.0.1:${settings.apiServerPort}`)
const skDisplay = computed(() => (settings.apiServerKey ? `${settings.apiServerKey.slice(0, 8)}…` : '—'))
const proxyServerRunning = ref(false)
const proxyLoading = ref(false)
const enabledProvidersList = computed(() => provider.providers.filter(p => p.enabled && p.apiKey))
async function copyText(s: string) {
  try {
    await navigator.clipboard.writeText(s)
    ElMessage.success(t('common.copied'))
  } catch { ElMessage.error(t('common.error')) }
}

async function handleStartProxy() {
  proxyLoading.value = true
  try {
    const { proxyApi } = await import('@/utils/tauri-api')
    const result = await proxyApi.startServer(
      settings.apiServerPort,
      settings.apiServerKey,
      settings.defaultProviderId
    )
    proxyServerRunning.value = result?.running ?? false
    settings.apiServerEnabled = proxyServerRunning.value
    if (proxyServerRunning.value) {
      ElMessage.success(`${t('settings.apiRunning')} - ${result.url}`)
    }
  } catch (e: any) {
    ElMessage.error(e?.message || e?.toString() || t('common.error'))
  } finally {
    proxyLoading.value = false
  }
}

async function handleStopProxy() {
  proxyLoading.value = true
  try {
    const { proxyApi } = await import('@/utils/tauri-api')
    await proxyApi.stopServer()
    proxyServerRunning.value = false
    settings.apiServerEnabled = false
    ElMessage.success(t('settings.apiStopped'))
  } catch (e: any) {
    ElMessage.error(e?.message || e?.toString() || t('common.error'))
  } finally {
    proxyLoading.value = false
  }
}

async function refreshProxyStatus() {
  try {
    const { proxyApi } = await import('@/utils/tauri-api')
    const status = await proxyApi.getStatus()
    proxyServerRunning.value = status?.running ?? false
  } catch { /* noop */ }
}
refreshProxyStatus()

const chPlatform = ref('feishu')
const chPlatformOpts = [
  { id: 'feishu', name: '飞书', icon: '💬' },
  { id: 'dingtalk', name: '钉钉', icon: '🔔' },
  { id: 'telegram', name: 'Telegram', icon: '✈️' },
  { id: 'discord', name: 'Discord', icon: '🎮' },
  { id: 'slack', name: 'Slack', icon: '💼' },
  { id: 'webhook', name: '自定义 Webhook', icon: '🔗' },
]
const platformOpts = chPlatformOpts
function platformName(id: string) { return chPlatformOpts.find(p => p.id === id)?.name || id }
const channelsForPlat = computed(() => settings.channels.filter(c => c.platformId === chPlatform.value))
function channelCountByPlat(platId: string) { return settings.channels.filter(c => c.platformId === platId).length }
const chTestingId = ref('')

function getWebhookPlaceholder(platId: string): string {
  const map: Record<string, string> = {
    feishu: 'https://open.feishu.cn/open-apis/bot/v2/hook/xxx',
    dingtalk: 'https://oapi.dingtalk.com/robot/send?access_token=xxx',
    telegram: 'https://api.telegram.org/botTOKEN/CHAT_ID',
    discord: 'https://discord.com/api/webhooks/xxx/xxx',
    slack: 'https://hooks.slack.com/services/xxx/xxx/xxx',
    webhook: 'https://your-service.com/webhook',
  }
  return map[platId] || 'Webhook URL'
}

function addChannelPlat() {
  settings.channels.push({
    id: uuidv4(),
    name: `${platformName(chPlatform.value)} 通知`,
    platformId: chPlatform.value,
    webhookUrl: '',
    secret: '',
    enabled: true,
    createdAt: Date.now(),
    notifyOnReply: true,
    messageTemplate: '',
  })
}
function removeChRow(row: { id: string }) { settings.channels = settings.channels.filter(c => c.id !== row.id) }

async function testChannelWebhook(row: { id: string; platformId: string; webhookUrl: string; secret?: string }) {
  if (!row.webhookUrl) {
    ElMessage.warning('请先填写 Webhook 地址')
    return
  }
  chTestingId.value = row.id
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('test_channel_webhook', {
      platformId: row.platformId,
      webhookUrl: row.webhookUrl,
      secret: row.secret || null,
    })
    ElMessage.success('测试消息发送成功！')
  } catch (err: any) {
    ElMessage.error(`发送失败: ${err?.toString() || '未知错误'}`)
  } finally {
    chTestingId.value = ''
  }
}

const scheduleStore = useScheduleStore()
const schedTasks = computed(() => scheduleStore.tasks)
const schedDialogVisible = ref(false)
const schedForm = reactive({ name: '', agentId: '', prompt: '', cron: 'every 30 min' })
const schedEditId = ref<string | null>(null)

function schedAddHint() {
  schedEditId.value = null
  Object.assign(schedForm, { name: '', agentId: '', prompt: '', cron: 'every 30 min' })
  schedDialogVisible.value = true
}

function schedEdit(task: { id: string }) {
  const t = scheduleStore.tasks.find(x => x.id === task.id)
  if (!t) return
  schedEditId.value = t.id
  Object.assign(schedForm, { name: t.name, agentId: t.agentId, prompt: t.prompt, cron: t.cron })
  schedDialogVisible.value = true
}

function schedSave() {
  if (schedEditId.value) {
    scheduleStore.updateTask(schedEditId.value, { ...schedForm })
  } else {
    scheduleStore.createTask({ ...schedForm })
  }
  schedDialogVisible.value = false
}

function schedDelete(id: string) {
  scheduleStore.deleteTask(id)
}

function schedToggle(id: string) {
  const t = scheduleStore.tasks.find(x => x.id === id)
  if (t) scheduleStore.updateTask(id, { enabled: !t.enabled })
}

function schedRun(id: string) {
  scheduleStore.runTask(id)
  ElMessage.success('任务已触发')
}

const phraseOpen = ref(false)
const phraseText = ref('')
const phraseEditIdx = ref<number | null>(null)
function openPhraseEdit(idx: number) {
  phraseEditIdx.value = idx
  if (idx < 0) { phraseText.value = ''; phraseOpen.value = true; return }
  phraseText.value = agent.quickPhrases[idx] || ''
  phraseOpen.value = true
}
function savePhrase() {
  const p = phraseText.value.trim()
  if (!p) return
  if (phraseEditIdx.value != null && phraseEditIdx.value >= 0) {
    agent.updateQuickPhrase(phraseEditIdx.value, p)
  } else { agent.addQuickPhrase(p) }
  phraseOpen.value = false
}
function removePhraseAt(idx: number) { agent.removeQuickPhrase(agent.quickPhrases[idx]!) }

const qa = reactive({
  enabled: true,
  method: 'select' as 'select' | 'ctrl' | 'shortcut',
  brush: false,
  toolbar: true,
  remember: false,
  autoclose: false,
  autotop: false,
  opacity: 100
})
function faqHint() { ElMessage.info(t('pageUi.settingsAboutDesc')) }
function selCustomHint() { ElMessage.info(t('common.add')) }
const selActionDefs: { id: string; lk: string }[] = [
  { id: 'translate', lk: 'translate.title' },
  { id: 'explain', lk: 'settings.selActExplain' },
  { id: 'summarize', lk: 'settings.selActSummarize' },
  { id: 'search', lk: 'chat.webSearch' },
  { id: 'copy', lk: 'common.copy' }
]
function toggleSelAction(id: string) {
  const s = new Set(settings.selActions)
  if (s.has(id)) s.delete(id); else s.add(id)
  settings.$patch({ selActions: Array.from(s) })
}

/** Sidebar: keys match `Section`, labels via i18n, icons as single SVG path `d` (24×24) */
const navItems: { key: Section; labelKey: string; icon: string }[] = [
  { key: 'model', labelKey: 'settings.sectionModel', icon: 'M5.25 3A2.25 2.25 0 0 0 3 5.25v2.5A2.25 2.25 0 0 0 5.25 10h2.5A2.25 2.25 0 0 0 10 7.75v-2.5A2.25 2.25 0 0 0 7.75 3h-2.5zm9 0A2.25 2.25 0 0 0 12 5.25v2.5A2.25 2.25 0 0 0 14.25 10h2.5A2.25 2.25 0 0 0 19 7.75v-2.5A2.25 2.25 0 0 0 16.75 3h-2.5zm-9 9A2.25 2.25 0 0 0 3 14.25v2.5A2.25 2.25 0 0 0 5.25 19h2.5A2.25 2.25 0 0 0 10 16.75v-2.5A2.25 2.25 0 0 0 7.75 12h-2.5zm9 0A2.25 2.25 0 0 0 12 14.25v2.5A2.25 2.25 0 0 0 14.25 19h2.5A2.25 2.25 0 0 0 19 16.75v-2.5A2.25 2.25 0 0 0 16.75 12h-2.5z' },
  { key: 'defaultModel', labelKey: 'settings.sectionDefaultModel', icon: 'M11.48 3.499a.75.75 0 0 1 1.04 0l6.5 5.999a.75.75 0 0 1-.28 1.28l-2.02 1.12v3.5a.75.75 0 0 1-1.5 0V13.5L12 10.5l-4.22 2.1v2.1a.75.75 0 0 1-1.5 0V11.89l-2.02-1.12a.75.75 0 0 1-.28-1.28l6.5-5.999Z' },
  { key: 'general', labelKey: 'settings.sectionGeneral', icon: 'M12 1.5a.75.75 0 0 1 .66.4l.28.56a.75.75 0 0 0 .8.4 5.2 5.2 0 0 1 1.1.35.75.75 0 0 0 .95-.2l.4-.4a.75.75 0 0 1 1.06 0l.85.85a.75.75 0 0 1 0 1.06l-.4.4a.75.75 0 0 0-.2.95c.16.35.28.72.35 1.1a.75.75 0 0 0 .4.8l.56.28a.75.75 0 0 1 .4.66v1.2a.75.75 0 0 1-.4.66l-.56.28a.75.75 0 0 0-.4.8c-.07.38-.19.75-.35 1.1a.75.75 0 0 0 .2.95l.4.4a.75.75 0 0 1 0 1.06l-.85.85a.75.75 0 0 1-1.06 0l-.4-.4a.75.75 0 0 0-.95-.2c-.35.16-.72.28-1.1.35a.75.75 0 0 0-.8.4l-.28.56a.75.75 0 0 1-.66.4h-1.2a.75.75 0 0 1-.66-.4l-.28-.56a.75.75 0 0 0-.8-.4A5.2 5.2 0 0 1 7.1 20.1a.75.75 0 0 0-.95.2l-.4.4a.75.75 0 0 1-1.06 0l-.85-.85a.75.75 0 0 1 0-1.06l.4-.4a.75.75 0 0 0 .2-.95 5.2 5.2 0 0 1-.35-1.1.75.75 0 0 0-.4-.8l-.56-.28a.75.75 0 0 1-.4-.66v-1.2a.75.75 0 0 1 .4-.66l.56-.28a.75.75 0 0 0 .4-.8A5.2 5.2 0 0 1 3.8 7.1a.75.75 0 0 0-.95-.2l-.4-.4a.75.75 0 0 1 0-1.06l.85-.85a.75.75 0 0 1 1.06 0l.4.4a.75.75 0 0 0 .95.2c.35-.16.72-.28 1.1-.35a.75.75 0 0 0 .8-.4l.28-.56A.75.75 0 0 1 10.8 1.5h1.2Z' },
  { key: 'display', labelKey: 'settings.sectionDisplay', icon: 'M12 3a.75.75 0 0 1 .75.75v.75h1.5A2.25 2.25 0 0 1 16.5 6.75v6a2.25 2.25 0 0 1-2.25 2.25H9.75A2.25 2.25 0 0 1 7.5 12.75v-6A2.25 2.25 0 0 1 9.75 4.5h1.5v-.75A.75.75 0 0 1 12 3Zm-1.5 4.5H9a.75.75 0 0 0-.75.75V12c0 .414.336.75.75.75h1.5V7.5Zm3 0V12h1a.75.75 0 0 0 .75-.75V8.25A.75.75 0 0 0 15 7.5h-1.5ZM8.25 19.5a.75.75 0 0 1 .75-.75h6a.75.75 0 0 1 0 1.5h-6a.75.75 0 0 1-.75-.75Z' },
  { key: 'data', labelKey: 'settings.sectionData', icon: 'M4.5 6.75a3 3 0 0 1 3-3h2.25A3.75 3.75 0 0 1 16.5 3h.75A3.75 3.75 0 0 1 21 6.75V9a.75.75 0 0 1-.75.75H4.5A.75.75 0 0 1 3.75 9V6.75A3.3 3.3 0 0 1 4.5 6.75Zm0 4.5a.75.75 0 0 0-.75.75V18a1.5 1.5 0 0 0 1.5 1.5H19.5A1.5 1.5 0 0 0 21 18V12a.75.75 0 0 0-.75-.75H4.5Z' },
  { key: 'mcp', labelKey: 'settings.sectionMcp', icon: 'M13.5 1.5a1.5 1.5 0 0 0-3 0v1.5H9A2.25 2.25 0 0 0 6.75 5.25V7.5H18V5.25A2.25 2.25 0 0 0 15.75 3H15V1.5Zm-1.5 6v12a1.5 1.5 0 0 0 1.5 1.5h3a1.5 1.5 0 0 0 1.5-1.5V7.5h-6Zm-4.5 0H3.75A1.5 1.5 0 0 0 2.25 9v6a1.5 1.5 0 0 0 1.5 1.5h3.75V7.5Z' },
  { key: 'skills', labelKey: 'settings.sectionSkills', icon: 'M3.75 2.25A2.25 2.25 0 0 0 1.5 4.5v3.75A2.25 2.25 0 0 0 3.75 10.5h3A2.25 2.25 0 0 0 9 8.25V4.5A2.25 2.25 0 0 0 6.75 2.25h-3zm12 0A2.25 2.25 0 0 0 13.5 4.5v3.75A2.25 2.25 0 0 0 15.75 10.5h3A2.25 2.25 0 0 0 21 8.25V4.5A2.25 2.25 0 0 0 18.75 2.25h-3zM3.75 12A2.25 2.25 0 0 0 1.5 14.25v3.75A2.25 2.25 0 0 0 3.75 20.25h3A2.25 2.25 0 0 0 9 18v-3.75A2.25 2.25 0 0 0 6.75 12h-3zm8.25 0A2.25 2.25 0 0 0 9.75 14.25V18A2.25 2.25 0 0 0 12 20.25h6A2.25 2.25 0 0 0 20.25 18v-3.75A2.25 2.25 0 0 0 18 12h-6z' },
  { key: 'webSearch', labelKey: 'settings.sectionWebSearch', icon: 'M10.5 3.75A6.75 6.75 0 0 0 3.75 10.5c0 1.37.4 2.64 1.1 3.7l-2.1 2.1a.75.75 0 0 0 1.06 1.06l2.1-2.1A6.74 6.74 0 0 0 10.5 17.25a6.75 6.75 0 0 0 0-13.5Zm0 1.5a5.25 5.25 0 0 1 3.2 9.3l.25-.2a.75.75 0 0 0-.2-1.2 4.2 4.2 0 0 0-2.1-.6 4.2 4.2 0 0 0-4.2 4.2.75.75 0 0 0 1.5 0A2.7 2.7 0 0 1 10.2 9a.75.75 0 0 0-.45-.6 5.2 5.2 0 0 0-.3 4.1Z' },
  { key: 'memory', labelKey: 'settings.sectionGlobalMemory', icon: 'M12 2.25a.75.75 0 0 1 .68.45l.35.78.84.1a.75.75 0 0 1 .42 1.3l-.6.5.18.84a.75.75 0 0 1-1.1.8L12 6.1l-.77.4a.75.75 0 0 1-1.1-.8l.18-.84-.6-.5a.75.75 0 0 1 .42-1.3l.84-.1.35-.78a.75.75 0 0 1 .68-.45Zm-6.6 4.1a.75.75 0 0 1 1.02-.3l.45.3a.75.75 0 0 0 .8 0l.45-.3a.75.75 0 0 1 1.02.3l.2.4a.75.75 0 0 0 .4.4l.45.2a.75.75 0 0 1 0 1.35l-.45.2a.75.75 0 0 0-.4.4l-.2.45a.75.75 0 0 1-1.35 0L8.4 8.8a.75.75 0 0 0-.4-.4l-.45-.2a.75.75 0 0 1 0-1.35l.45-.2a.75.75 0 0 0 .4-.4l.1-.2Zm.6 6.3a.75.75 0 0 1 1.5 0V18a.75.75 0 0 0 1.5 0v-5.4a.75.75 0 0 1 1.5 0V18A2.25 2.25 0 0 1 8.1 20.25H6.75A2.25 2.25 0 0 1 4.5 18v-1.2a.75.75 0 0 1 .75-.75H6a.75.75 0 0 0 .75-.75v-1.1Z' },
  { key: 'api', labelKey: 'settings.sectionApi', icon: 'M14.25 1.5a.75.75 0 0 1 .75.75V4.5H18a.75.75 0 0 1 0 1.5H15V9a.75.75 0 0 1-1.5 0V6H9.75A2.25 2.25 0 0 0 7.5 8.25V18a.75.75 0 0 1-1.5 0V8.25A3.75 3.75 0 0 1 9.75 4.5H12V2.25a.75.75 0 0 1 .75-.75h1.5Z' },
  { key: 'channels', labelKey: 'settings.sectionChannels', icon: 'M6 2.25A2.25 2.25 0 0 0 3.75 4.5v3.75A2.25 2.25 0 0 0 6 10.5h.75V12a.75.75 0 0 0 1.28.53L10.2 9.6A3 3 0 0 1 12.75 9H18A2.25 2.25 0 0 0 20.25 6.75V4.5A2.25 2.25 0 0 0 18 2.25H6Zm.75 12a.75.75 0 0 0-1.5 0V18A2.25 2.25 0 0 0 7.5 20.25h9A2.25 2.25 0 0 0 18.75 18v-1.5a.75.75 0 0 0-1.5 0V18a.75.75 0 0 1-.75.75H9a.75.75 0 0 0-.75.75.75.75 0 0 1-1.5 0V14.25Z' },
  { key: 'sched', labelKey: 'settings.sectionSched', icon: 'M12 1.5A10.5 10.5 0 1 0 12 22.5a10.5 10.5 0 0 0 0-21ZM12.75 5.5a.75.75 0 0 0-1.5V12c0 .2.1.4.2.5l3 2.2a.75.75 0 0 0 .8-1.2l-2.5-1.9V5.5Z' },
  { key: 'doc', labelKey: 'settings.sectionDoc', icon: 'M4.5 2.25A2.25 2.25 0 0 0 2.25 4.5v12A2.25 2.25 0 0 0 4.5 18.75h2.1l2.1 2.1a.75.75 0 0 0 1.1 0l2.1-2.1H19.5A2.25 2.25 0 0 0 21.75 16.5v-12A2.25 2.25 0 0 0 19.5 2.25h-15Zm.75 3.75H18a.75.75 0 0 1 .75.75V15a.75.75 0 0 1-.75.75H5.25A.75.75 0 0 1 4.5 15V6.75A.75.75 0 0 1 5.25 6Z' },
  { key: 'harness', labelKey: 'settings.sectionHarness', icon: 'M12 2.25a.75.75 0 0 1 .53.22l3.75 3.75a.75.75 0 0 1-1.06 1.06L12 4.06 8.78 7.28a.75.75 0 0 1-1.06-1.06l3.75-3.75A.75.75 0 0 1 12 2.25ZM3.75 9A.75.75 0 0 1 4.5 8.25h15a.75.75 0 0 1 0 1.5h-15A.75.75 0 0 1 3.75 9Zm0 6a.75.75 0 0 1 .75-.75h15a.75.75 0 0 1 0 1.5h-15a.75.75 0 0 1-.75-.75Zm8.25 6.75a.75.75 0 0 1-.53-.22l-3.75-3.75a.75.75 0 0 1 1.06-1.06L12 19.94l3.22-3.22a.75.75 0 0 1 1.06 1.06l-3.75 3.75a.75.75 0 0 1-.53.22Z' },
  { key: 'phrases', labelKey: 'settings.sectionPhrases', icon: 'M4.5 3.75A2.25 2.25 0 0 0 2.25 6v6.6a2.25 2.25 0 0 0 1.1 1.9l2.1 1.2V20.2a.75.75 0 0 0 1.1.7l2.1-1.1 2.1 1.1a.75.75 0 0 0 1.1-.7v-4.5l2.1-1.2A2.25 2.25 0 0 0 21.75 12.6V6A2.25 2.25 0 0 0 19.5 3.75h-15Z' },
  { key: 'hotkeys', labelKey: 'settings.sectionHotkeys', icon: 'M2.25 4.5A1.5 1.5 0 0 0 .75 6v6A1.5 1.5 0 0 0 2.25 13.5H4.5V15a.75.75 0 0 0 1.5 0v-1.5H18V15a.75.75 0 0 0 1.5 0v-1.5h2.25A1.5 1.5 0 0 0 23.25 12V6A1.5 1.5 0 0 0 21.75 4.5H2.25Z' },
  { key: 'quickAsst', labelKey: 'settings.sectionQuickAsst', icon: 'M9.4 1.2a.75.75 0 0 1 1.2 0l1.1 1.4a.75.75 0 0 0 .5.2h1.6a.75.75 0 0 1 .4 1.3l-1.2 1a.75.75 0 0 0-.2.5v1.2a.75.75 0 0 1-1.2.5l-1.1-.6a.75.75 0 0 0-.5 0l-1.1.6A.75.75 0 0 1 6.8 6.1V4.6a.75.75 0 0 0-.2-.5l-1.2-1A.75.75 0 0 1 5.7 1.8h1.6a.75.75 0 0 0 .5-.2l1.1-1.4ZM12 9a.75.75 0 0 0-.7.4l-3 5.4a.75.75 0 0 0 .65 1.1H15a.75.75 0 0 0 .7-1.1l-3-5.4A.75.75 0 0 0 12 9Zm-3 8.2a.75.75 0 0 0-1.5 0V20a.75.75 0 0 0 1.5 0v-2.8Z' },
  { key: 'selection', labelKey: 'settings.sectionSelection', icon: 'M3.3 2.1a.75.75 0 0 1 1.05-.1L18 12.1a.75.75 0 0 1 0 1.2L4.35 20.1a.75.75 0 0 1-1.05-.1.75.75 0 0 1-.1-1.05l4.1-4.1a.75.75 0 0 0 0-1.05L3.2 3.1a.75.75 0 0 1 .1-1.05Z' },
  { key: 'about', labelKey: 'settings.sectionAbout', icon: 'M12 1.5A10.5 10.5 0 1 0 12 22.5 10.5 10.5 0 0 0 12 1.5ZM12 6a.75.75 0 0 1 .75.75V12a.75.75 0 0 1-1.5 0V6.75A.75.75 0 0 1 12 6Zm0 7.5a1.1 1.1 0 1 0 0 2.2 1.1 1.1 0 0 0 0-2.2Z' }
]

onMounted(() => {
  i18nLocale.value = (settings.language as 'zh-CN' | 'en-US') || 'zh-CN'
  void provider.loadProviders()
  void loadMcp().then(() => {
    if (mcpServers.value[0]?.id) mcpPick.value = mcpServers.value[0].id
    else mcpPick.value = mcpRecList[0].id
  })
  void assistant.initPresets()
  skillStore.ensurePresets()
  webEnginePick.value = agent.webSearchEngine
  accentProxy.value = theme.accentColor
  mcpPick.value = mcpRecList[0].id
})

function setLocale(code: string) {
  settings.setLanguage(code)
  i18nLocale.value = (code as 'zh-CN' | 'en-US') || 'zh-CN'
}

function onAccent() {
  if (accentProxy.value) theme.setAccentColor(accentProxy.value)
}

interface ProviderPreset {
  id: string
  name: string
  channelType: number
  apiEndpoint: string
  desc: string
  models: string[]
}

const PROVIDER_PRESETS: ProviderPreset[] = [
  { id: 'openai', name: 'OpenAI', channelType: 1, apiEndpoint: 'https://api.openai.com', desc: 'GPT-4o / o1 / o3 系列',
    models: ['gpt-4o', 'gpt-4o-mini', 'gpt-4o-2024-11-20', 'gpt-4-turbo', 'gpt-4', 'gpt-3.5-turbo', 'o1', 'o1-mini', 'o1-preview', 'o3-mini'] },
  { id: 'anthropic', name: 'Anthropic', channelType: 14, apiEndpoint: 'https://api.anthropic.com', desc: 'Claude Sonnet 4 / Opus / Haiku',
    models: ['claude-sonnet-4-20250514', 'claude-3-7-sonnet-20250219', 'claude-3-5-sonnet-20241022', 'claude-3-5-haiku-20241022', 'claude-3-opus-20240229'] },
  { id: 'gemini', name: 'Google Gemini', channelType: 24, apiEndpoint: 'https://generativelanguage.googleapis.com', desc: 'Gemini 2.5 Pro / Flash',
    models: ['gemini-2.5-pro-preview-05-06', 'gemini-2.5-flash-preview-04-17', 'gemini-2.0-flash', 'gemini-2.0-flash-lite', 'gemini-1.5-pro', 'gemini-1.5-flash'] },
  { id: 'deepseek', name: 'DeepSeek', channelType: 43, apiEndpoint: 'https://api.deepseek.com', desc: 'DeepSeek-Chat / Reasoner',
    models: ['deepseek-chat', 'deepseek-reasoner'] },
  { id: 'openrouter', name: 'OpenRouter', channelType: 20, apiEndpoint: 'https://openrouter.ai/api', desc: '100+ 模型聚合，按量计费',
    models: ['openai/gpt-4o', 'anthropic/claude-sonnet-4-20250514', 'google/gemini-2.5-pro-preview', 'meta-llama/llama-3.1-405b-instruct', 'deepseek/deepseek-chat'] },
  { id: 'azure', name: 'Azure OpenAI', channelType: 3, apiEndpoint: '', desc: '微软 Azure 托管 OpenAI 模型',
    models: ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo', 'gpt-4', 'gpt-35-turbo'] },
  { id: 'ali', name: '阿里通义 DashScope', channelType: 17, apiEndpoint: 'https://dashscope.aliyuncs.com', desc: 'Qwen 系列大模型',
    models: ['qwen-max', 'qwen-plus', 'qwen-turbo', 'qwen-long', 'qwen-vl-max', 'qwen-vl-plus', 'qwen2.5-72b-instruct', 'qwen2.5-32b-instruct'] },
  { id: 'zhipu', name: '智谱 AI', channelType: 16, apiEndpoint: 'https://open.bigmodel.cn', desc: 'GLM-4 系列',
    models: ['glm-4-plus', 'glm-4-long', 'glm-4-air', 'glm-4-airx', 'glm-4-flash', 'glm-4-flashx', 'glm-4v-plus', 'glm-4v'] },
  { id: 'zhipu-v4', name: '智谱 GLM-4 (V4 API)', channelType: 26, apiEndpoint: 'https://open.bigmodel.cn/api/paas/v4', desc: 'GLM-4 V4 接口',
    models: ['glm-4-plus', 'glm-4-long', 'glm-4-air', 'glm-4-airx', 'glm-4-flash', 'glm-4-flashx', 'glm-4v-plus'] },
  { id: 'minimax', name: 'MiniMax', channelType: 35, apiEndpoint: 'https://api.minimax.chat', desc: 'MiniMax M2.7 / M2.5 / abab 系列',
    models: ['MiniMax-M2.7', 'MiniMax-M2.7-highspeed', 'MiniMax-M2.5', 'MiniMax-M2.5-highspeed', 'MiniMax-M2.1', 'MiniMax-M2.1-highspeed', 'MiniMax-M2', 'MiniMax-Text-01', 'abab6.5-chat', 'abab6.5s-chat', 'abab6-chat', 'abab5.5-chat', 'abab5.5s-chat'] },
  { id: 'moonshot', name: 'Moonshot / 月之暗面', channelType: 25, apiEndpoint: 'https://api.moonshot.cn', desc: 'Moonshot-v1 系列',
    models: ['moonshot-v1-8k', 'moonshot-v1-32k', 'moonshot-v1-128k'] },
  { id: 'siliconflow', name: 'SiliconFlow / 硅基流动', channelType: 40, apiEndpoint: 'https://api.siliconflow.cn', desc: '国内中转 DeepSeek/Qwen',
    models: ['deepseek-ai/DeepSeek-V3', 'deepseek-ai/DeepSeek-R1', 'Qwen/Qwen2.5-72B-Instruct', 'Pro/Qwen/Qwen2.5-7B-Instruct', 'THUDM/glm-4-9b-chat'] },
  { id: 'xai', name: 'xAI / Grok', channelType: 48, apiEndpoint: 'https://api.x.ai', desc: 'Grok-3 / Grok-2 系列',
    models: ['grok-3', 'grok-3-mini', 'grok-2', 'grok-2-vision'] },
  { id: 'mistral', name: 'Mistral AI', channelType: 42, apiEndpoint: 'https://api.mistral.ai', desc: 'Mistral Large / Codestral',
    models: ['mistral-large-latest', 'mistral-medium-latest', 'mistral-small-latest', 'open-mixtral-8x22b', 'codestral-latest'] },
  { id: 'perplexity', name: 'Perplexity', channelType: 27, apiEndpoint: 'https://api.perplexity.ai', desc: 'Sonar 联网搜索模型',
    models: ['sonar-pro', 'sonar', 'sonar-reasoning-pro', 'sonar-reasoning', 'sonar-deep-research'] },
  { id: 'cohere', name: 'Cohere', channelType: 34, apiEndpoint: 'https://api.cohere.ai', desc: 'Command R+ 系列',
    models: ['command-r-plus', 'command-r', 'command-light'] },
  { id: 'lingyi', name: '零一万物', channelType: 31, apiEndpoint: 'https://api.lingyiwanwu.com', desc: 'Yi-Lightning / Large',
    models: ['yi-lightning', 'yi-large', 'yi-large-turbo', 'yi-medium', 'yi-spark'] },
  { id: 'baidu', name: '百度文心', channelType: 15, apiEndpoint: 'https://aip.baidubce.com', desc: 'ERNIE 系列',
    models: ['ernie-4.0-8k', 'ernie-4.0-turbo-8k', 'ernie-3.5-8k', 'ernie-speed-128k', 'ernie-lite-8k'] },
  { id: 'tencent', name: '腾讯混元', channelType: 23, apiEndpoint: 'https://hunyuan.tencentcloudapi.com', desc: '混元大模型',
    models: ['hunyuan-pro', 'hunyuan-standard', 'hunyuan-lite', 'hunyuan-turbo', 'hunyuan-vision'] },
  { id: 'volcengine', name: '火山引擎 / 豆包', channelType: 45, apiEndpoint: 'https://ark.cn-beijing.volces.com', desc: '字节 Doubao 系列',
    models: ['doubao-pro-32k', 'doubao-pro-128k', 'doubao-lite-32k'] },
  { id: 'coze', name: 'Coze / 扣子', channelType: 49, apiEndpoint: 'https://api.coze.cn', desc: '字节扣子 Bot API',
    models: [] },
  { id: 'ollama', name: 'Ollama (本地)', channelType: 4, apiEndpoint: 'http://localhost:11434', desc: '本地模型，无需 API Key',
    models: [] },
  { id: 'custom', name: '自定义 (兼容接口)', channelType: 8, apiEndpoint: '', desc: '手动配置任意兼容端点',
    models: [] },
]

const provOpen = ref(false)
const editProvId = ref<string | null>(null)
const provSaving = ref(false)
const provAddStep = ref<'pick' | 'config'>('pick')
const provSelectedPreset = ref<ProviderPreset | null>(null)
const provForm = reactive({
  name: '', channelType: 1, apiKey: '', apiEndpoint: '', enabled: true
})
const provPresetSearch = ref('')
const filteredPresets = computed(() => {
  const q = provPresetSearch.value.trim().toLowerCase()
  return q ? PROVIDER_PRESETS.filter(p => p.name.toLowerCase().includes(q) || p.id.includes(q)) : PROVIDER_PRESETS
})
const modelFetchMsg = ref('')
const testingProvider = ref(false)

const mcpServers = ref<any[]>([])
const mcpLoading = ref(false)
const mcpBusyId = ref<string | null>(null)
const mcpAddOpen = ref(false)
const mcpSaving = ref(false)
const mcpForm = reactive({ name: '', command: '', argsStr: '' })

const DEFAULT_SHORTCUT_KEYS: Record<string, string> = {
  zoomIn: 'CommandOrControl+=',
  zoomOut: 'CommandOrControl+-',
  zoomReset: 'CommandOrControl+0',
  openSettings: 'CommandOrControl+,',
  toggleApp: 'CommandOrControl+Shift+S',
  newChat: 'CommandOrControl+N',
}

const shortcutRows = computed(() =>
  settings.shortcuts.map(s => ({
    ...s,
    editing: shortcutEditingId.value === s.id,
  }))
)
const shortcutEditingId = ref('')

function onKeyRec(row: { id: string; keys: string; editing: boolean }, e: KeyboardEvent) {
  const parts: string[] = []
  if (e.ctrlKey || e.metaKey) parts.push('CommandOrControl')
  if (e.shiftKey) parts.push('Shift')
  if (e.altKey) parts.push('Alt')
  if (e.key && !['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) {
    parts.push(e.key.length === 1 ? e.key.toUpperCase() : e.key)
  }
  if (parts.length >= 2) {
    const combo = parts.filter((v, i, a) => a.indexOf(v) === i).join('+')
    const sc = settings.shortcuts.find(s => s.id === row.id)
    if (sc) {
      sc.keys = combo
      shortcutEditingId.value = ''
      syncShortcuts()
    }
  }
}

function resetShortcut(row: { id: string }) {
  const sc = settings.shortcuts.find(s => s.id === row.id)
  if (sc && DEFAULT_SHORTCUT_KEYS[sc.id]) {
    sc.keys = DEFAULT_SHORTCUT_KEYS[sc.id]
    syncShortcuts()
  }
}

function formatKeys(keys: string): string {
  return keys
    .replace(/CommandOrControl/g, navigator.platform.includes('Mac') ? '\u2318' : 'Ctrl')
    .replace(/Shift/g, navigator.platform.includes('Mac') ? '\u21E7' : 'Shift')
    .replace(/Alt/g, navigator.platform.includes('Mac') ? '\u2325' : 'Alt')
}

function syncShortcuts() {
  unregisterAllShortcuts()
  registerAllShortcuts()
}

async function unregisterAllShortcuts() {
  try {
    const { unregisterAll } = await import('@tauri-apps/plugin-global-shortcut')
    await unregisterAll()
  } catch {}
}

async function registerAllShortcuts() {
  try {
    const { register } = await import('@tauri-apps/plugin-global-shortcut')
    for (const sc of settings.shortcuts) {
      if (!sc.enabled || !sc.isGlobal) continue
      try {
        await register(sc.keys, () => executeShortcutAction(sc.id))
      } catch (err) {
        console.warn(`Failed to register shortcut ${sc.keys}:`, err)
      }
    }
  } catch {}
}

function executeShortcutAction(actionId: string) {
  switch (actionId) {
    case 'toggleApp':
      import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
        const win = getCurrentWindow()
        win.isVisible().then(visible => {
          if (visible) { win.hide() } else { win.show(); win.setFocus() }
        })
      })
      break
  }
}

async function loadMcp() {
  mcpLoading.value = true
  try {
    mcpServers.value = await mcpApi.getServers()
  } catch { mcpServers.value = [] }
  finally { mcpLoading.value = false }
}

async function startMcp(id: string) {
  mcpBusyId.value = id
  try {
    await mcpApi.startServer(id)
    await loadMcp()
    ElMessage.success(t('common.success'))
  } catch {
    ElMessage.error(t('common.error'))
  } finally { mcpBusyId.value = null }
}

async function stopMcp(id: string) {
  mcpBusyId.value = id
  try {
    await mcpApi.stopServer(id)
    await loadMcp()
  } catch { ElMessage.error(t('common.error')) }
  finally { mcpBusyId.value = null }
}

async function submitMcp() {
  if (!mcpForm.name.trim() || !mcpForm.command.trim()) return
  mcpSaving.value = true
  try {
    const args = mcpForm.argsStr.split(/\s+/).filter(Boolean)
    await mcpApi.addServer({
      name: mcpForm.name,
      command: mcpForm.command,
      args,
      env: {},
      enabled: true,
      status: 'stopped',
      tools: [],
      createdAt: Date.now()
    })
    mcpAddOpen.value = false
    mcpForm.name = ''
    mcpForm.command = ''
    mcpForm.argsStr = ''
    await loadMcp()
    ElMessage.success(t('common.success'))
  } catch { ElMessage.error(t('common.error')) }
  finally { mcpSaving.value = false }
}

function openProvider(id?: string) {
  editProvId.value = id || null
  provPresetSearch.value = ''
  if (id) {
    const p = provider.getProviderById(id)
    if (p) {
      provForm.name = p.name
      provForm.channelType = p.channelType
      provForm.apiKey = p.apiKey
      provForm.apiEndpoint = p.apiEndpoint
      provForm.enabled = p.enabled
    }
    provAddStep.value = 'config'
    provSelectedPreset.value = null
  } else {
    Object.assign(provForm, { name: '', channelType: 1, apiKey: '', apiEndpoint: '', enabled: true })
    provAddStep.value = 'pick'
    provSelectedPreset.value = null
  }
  provOpen.value = true
}

function pickPreset(preset: ProviderPreset) {
  provSelectedPreset.value = preset
  provForm.name = preset.name
  provForm.channelType = preset.channelType
  provForm.apiEndpoint = preset.apiEndpoint
  provForm.apiKey = ''
  provForm.enabled = true
  provAddStep.value = 'config'
}

function presetModelsToStrings(ids: string[]): string[] {
  return [...ids]
}

async function saveProvider() {
  provSaving.value = true
  try {
    const presetIds = provSelectedPreset?.value?.models ?? []
    const now = Date.now()
    if (editProvId.value) {
      await provider.updateProvider(editProvId.value, {
        name: provForm.name,
        channelType: provForm.channelType,
        apiKey: provForm.apiKey,
        apiEndpoint: provForm.apiEndpoint,
        enabled: provForm.enabled,
      } as Partial<Provider>)
    } else {
      const newProv: Provider = {
        id: crypto.randomUUID(),
        name: provForm.name,
        channelType: provForm.channelType,
        apiKey: provForm.apiKey,
        apiEndpoint: provForm.apiEndpoint,
        enabled: provForm.enabled,
        models: presetIds.length ? presetModelsToStrings(presetIds) : [],
        createdAt: now,
        updatedAt: now,
      }
      const saved = await provider.addProvider(newProv)
      if (saved?.id && provForm.apiKey && provForm.apiEndpoint) {
        try {
          const list = await provider.getModels(saved.id)
          if (list?.length) {
            await provider.updateProvider(saved.id, { models: list } as Partial<Provider>)
            ElMessage.info(`已从 API 加载 ${list.length} 个模型`)
          }
        } catch { /* fallback to preset models */ }
      }
    }
    provOpen.value = false
    ElMessage.success(t('common.success'))
  } catch (e: any) {
    console.error('saveProvider error:', e)
    ElMessage.error(e?.message || t('common.error'))
  } finally {
    provSaving.value = false
  }
}

async function removeProv(id: string) {
  try {
    await ElMessageBox.confirm(t('settings.clearConfirm'), { type: 'warning' })
  } catch { return }
  try {
    await provider.removeProvider(id)
    ElMessage.success(t('common.success'))
  } catch { ElMessage.error(t('common.error')) }
}

async function testProv(id: string) {
  testingProvider.value = true
  const startTime = Date.now()
  const timer = setInterval(() => {
    const elapsed = ((Date.now() - startTime) / 1000).toFixed(1)
    modelFetchMsg.value = `检测中... ${elapsed}s`
  }, 200)
  try {
    const r = await provider.testConnection(id)
    const elapsed = ((Date.now() - startTime) / 1000).toFixed(1)
    const serverTime = r?.elapsed_ms ? `${r.elapsed_ms}ms` : `${elapsed}s`
    modelFetchMsg.value = r?.message || (r?.success ? `连接成功 (${serverTime})` : `连接失败 (${serverTime})`)
    ElMessage[r?.success ? 'success' : 'error'](modelFetchMsg.value)
  } catch (e: any) {
    const elapsed = ((Date.now() - startTime) / 1000).toFixed(1)
    modelFetchMsg.value = `连接失败 (${elapsed}s): ${e?.message || '未知错误'}`
    ElMessage.error(modelFetchMsg.value)
  } finally {
    clearInterval(timer)
    testingProvider.value = false
  }
}

async function loadModels(row: { id: string }) {
  modelFetchMsg.value = '加载中...'
  try {
    const list = await provider.getModels(row.id)
    if (list?.length) {
      await provider.updateProvider(row.id, { models: list } as Partial<Provider>)
      modelFetchMsg.value = `已加载 ${list.length} 个模型`
      ElMessage.success(`已加载 ${list.length} 个模型`)
    } else {
      const p = provider.getProviderById(row.id)
      const preset = PROVIDER_PRESETS.find(pr => pr.name === p?.name || pr.apiEndpoint === p?.apiEndpoint)
      if (preset?.models?.length) {
        const models = presetModelsToStrings(preset.models)
        await provider.updateProvider(row.id, { models } as Partial<Provider>)
        modelFetchMsg.value = `已加载预置 ${models.length} 个模型`
        ElMessage.success(`已加载预置 ${models.length} 个模型`)
      } else {
        modelFetchMsg.value = '未获取到模型，请检查 API Key 和地址'
        ElMessage.warning('未获取到模型')
      }
    }
  } catch {
    modelFetchMsg.value = '加载失败，请检查连接'
    ElMessage.error(t('common.error'))
  }
}

async function exportHint() {
  try {
    const allData: Record<string, any> = {}
    const keys = Object.keys(localStorage)
    for (const key of keys) {
      try { allData[key] = JSON.parse(localStorage.getItem(key) || 'null') } catch { allData[key] = localStorage.getItem(key) }
    }
    const blob = new Blob([JSON.stringify(allData, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `fox-ai-backup-${new Date().toISOString().slice(0, 10)}.json`
    a.click()
    URL.revokeObjectURL(url)
    ElMessage.success('导出成功')
  } catch (e: any) {
    ElMessage.error(e?.message || '导出失败')
  }
}

async function importHint() {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.json'
  input.onchange = async () => {
    const file = input.files?.[0]
    if (!file) return
    try {
      const text = await file.text()
      const data = JSON.parse(text)
      if (typeof data !== 'object') throw new Error('Invalid backup file')
      for (const [key, val] of Object.entries(data)) {
        localStorage.setItem(key, typeof val === 'string' ? val : JSON.stringify(val))
      }
      ElMessage.success('导入成功，请刷新页面')
    } catch (e: any) {
      ElMessage.error(e?.message || '导入失败')
    }
  }
  input.click()
}

function clearDataHint() {
  void ElMessageBox.confirm(t('settings.clearConfirm'), { type: 'warning' })
    .then(() => {
      localStorage.clear()
      ElMessage.success('数据已清除，请刷新页面')
    })
    .catch(() => { /* no-op */ })
}
</script>

<style lang="scss" scoped>
.settings-page {
  height: 100%;
  min-height: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--color-background);
  color: var(--color-text-1);
  overflow: hidden;
}

.settings-layout {
  display: flex;
  min-height: 0;
  flex: 1;
  align-items: stretch;
  overflow: hidden;
}

/* ---- Sidebar: plain buttons, no el-menu / el-scrollbar ---- */
.settings-nav {
  width: 180px;
  flex-shrink: 0;
  flex-grow: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 12px 10px 16px;
  border-right: 1px solid var(--color-border);
  background: var(--color-background-mute);
  box-shadow: var(--shadow-sm);
  overflow-y: auto;
  overflow-x: hidden;
  height: 100%;
  min-height: 0;
  scrollbar-width: thin;
  scrollbar-color: var(--color-border) transparent;

  &::-webkit-scrollbar { width: 4px; }
  &::-webkit-scrollbar-track { background: transparent; }
  &::-webkit-scrollbar-thumb { background: var(--color-border); border-radius: 4px; }
  &::-webkit-scrollbar-thumb:hover { background: var(--color-text-3); }
}

.settings-nav__btn {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  margin: 0;
  padding: 8px 10px;
  border: 1px solid transparent;
  border-radius: var(--fox-radius-sm);
  background: transparent;
  color: var(--color-text-2);
  font-size: 0.8125rem;
  line-height: 1.3;
  text-align: left;
  cursor: pointer;
  transition:
    background 0.08s ease,
    color 0.08s ease,
    border-color 0.08s ease,
    box-shadow 0.08s ease;

  &:hover {
    background: var(--color-hover);
    color: var(--color-text-1);
  }

  &:focus-visible {
    outline: 2px solid var(--fox-accent-border);
    outline-offset: 1px;
  }

  &.is-active {
    background: var(--color-primary-mute);
    color: var(--color-text-1);
    border-color: var(--color-border);
    box-shadow: var(--shadow-sm);
  }

  &:active:not(.is-active) {
    background: var(--color-active);
  }
}

.settings-nav__icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  opacity: 0.9;
  color: var(--color-text-2);

  .settings-nav__btn.is-active &,
  .settings-nav__btn:hover & {
    color: var(--color-text-1);
  }
}

.settings-nav__label {
  flex: 1;
  min-width: 0;
}

/* ---- Main: scrolls independently with custom scrollbar ---- */
.settings-main {
  min-width: 0;
  min-height: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
}

.settings-main__scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  scrollbar-gutter: stable;
  scrollbar-width: thin;
  scrollbar-color: var(--color-border) var(--color-background-mute);

  &::-webkit-scrollbar {
    width: 8px;
  }
  &::-webkit-scrollbar-track {
    background: var(--color-background-mute);
    border-radius: var(--fox-radius-sm);
  }
  &::-webkit-scrollbar-thumb {
    background: var(--color-border);
    border-radius: var(--fox-radius-sm);
  }
  &::-webkit-scrollbar-thumb:hover {
    background: var(--color-text-3);
  }
}

.settings-main__inner {
  padding: 20px 24px 40px;
  width: 100%;
  box-sizing: border-box;
}
.subhead {
  font-size: 0.8rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--color-text-3);
  margin: 16px 0 8px;
}
.settings-card--flush { padding-bottom: 12px; }
.settings-split {
  display: grid;
  gap: 16px;
  margin-top: 12px;
}
.settings-split--model { grid-template-columns: 280px 1fr; }
.settings-split--data { grid-template-columns: 200px 1fr; }
.settings-split--mcp,
.settings-split--skills,
.settings-split--ws,
.settings-split--ch { grid-template-columns: 240px 1fr; }
.settings-split--sched { grid-template-columns: 1fr 1fr; }
.sched-list { display: flex; flex-direction: column; gap: 6px; margin-top: 8px; }
.sched-item {
  display: flex; align-items: center; justify-content: space-between; gap: 8px;
  padding: 8px 10px; border-radius: var(--fox-radius-sm);
  border: 1px solid var(--color-border); background: var(--color-background);
}
.sched-item__info { min-width: 0; flex: 1; }
.sched-item__name { font-size: 13px; font-weight: 500; color: var(--color-text-1); }
.sched-item__meta { font-size: 11px; color: var(--color-text-3); margin-top: 2px; }
.sched-item__actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
.prov-filter-bar {
  display: flex; align-items: center; gap: 6px; margin-bottom: 8px;
}
.prov-filter-input { flex: 1; }
.prov-filter-btn {
  width: 32px; height: 32px; flex-shrink: 0;
  display: flex; align-items: center; justify-content: center;
  border: 1px solid var(--color-border); border-radius: var(--fox-radius-sm);
  background: var(--color-background); color: var(--color-text-3);
  cursor: pointer; transition: all 0.15s;
  &:hover { color: var(--color-text-1); border-color: var(--color-text-3); }
  &.is-on { color: var(--fox-accent-fg, #3b82f6); border-color: var(--fox-accent-fg, #3b82f6); background: var(--color-primary-mute); }
}
.prov-list { display: flex; flex-direction: column; gap: 4px; max-height: 420px; overflow-y: auto; }
.prov-card {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-sm);
  background: var(--color-background);
  cursor: pointer;
  text-align: left;
  color: var(--color-text-1);
  transition: border-color 0.15s ease, box-shadow 0.15s ease, background 0.15s ease;
  &:hover { border-color: var(--fox-accent-border); background: var(--color-hover); }
  &.is-pick {
    border-color: var(--fox-accent-border);
    box-shadow: var(--shadow-sm);
    background: var(--color-primary-mute);
  }
}
.prov-card__icon {
  width: 28px; height: 28px; flex-shrink: 0;
  display: flex; align-items: center; justify-content: center;
  border-radius: 8px; font-size: 13px; font-weight: 700;
  color: #fff; text-transform: uppercase;
}
.prov-card__name { flex: 1; min-width: 0; font-size: 13px; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.prov-card__badge {
  flex-shrink: 0; font-size: 10px; font-weight: 700; padding: 1px 6px;
  border-radius: 4px; letter-spacing: 0.3px; text-transform: uppercase;
}
.prov-card__badge--on { background: #22c55e22; color: #22c55e; }
.prov-card__sw { flex-shrink: 0; }

.prov-preset-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 10px;
  max-height: 440px;
  overflow-y: auto;
  padding: 2px;
}
.prov-preset-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 16px 10px 12px;
  border: 1px solid var(--color-border);
  border-radius: 12px;
  background: var(--color-background);
  cursor: pointer;
  text-align: center;
  color: var(--color-text-1);
  transition: border-color 0.15s, box-shadow 0.15s, transform 0.1s;
}
.prov-preset-card:hover {
  border-color: var(--fox-accent-border);
  box-shadow: 0 2px 12px rgba(0,0,0,0.08);
  transform: translateY(-1px);
}
.prov-preset-card__name {
  font-size: 13px; font-weight: 600; line-height: 1.2;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  max-width: 100%;
}
.prov-preset-card__desc {
  font-size: 11px; color: var(--color-text-3); line-height: 1.3;
  overflow: hidden; text-overflow: ellipsis;
  display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical;
}
.prov-preset-card__count {
  font-size: 10px; color: var(--color-text-4);
  background: var(--color-background-mute);
  padding: 1px 6px; border-radius: 8px;
}

.prov-config-header {
  display: flex; align-items: center; gap: 12px;
  padding: 12px 14px; margin-bottom: 16px;
  border-radius: 10px; background: var(--color-background-soft);
  border: 1px solid var(--color-border);
}
.prov-config-header__name { font-size: 15px; font-weight: 600; color: var(--color-text-1); }
.prov-config-header__hint { font-size: 12px; color: var(--color-text-3); margin-top: 2px; }

.prov-config-form .is-dimmed .el-form-item__label { opacity: 0.6; }
.prov-config-auto-hint { font-size: 11px; color: var(--color-text-3); margin-top: 4px; }

.url-preview {
  font-size: 11.5px;
  color: var(--color-text-3);
  margin-top: 4px;
  font-family: ui-monospace, 'Cascadia Code', 'Source Code Pro', monospace;
  word-break: break-all;
  line-height: 1.5;
  padding: 4px 8px;
  border-radius: 4px;
  background: var(--color-background-mute);
  border: 1px dashed var(--color-border);
}

.field-hint {
  font-size: 11px;
  color: var(--color-text-4, var(--color-text-3));
  margin-top: 4px;
  opacity: 0.8;
}

.prov-config-advanced { margin-top: 4px; }

.prov-config-models {
  margin-top: 12px; padding: 10px 12px;
  border-radius: 8px; background: var(--color-background-soft);
  border: 1px solid var(--color-border);
}
.prov-config-models__title { font-size: 12px; font-weight: 500; color: var(--color-text-2); margin-bottom: 8px; }
.prov-config-models__list { display: flex; flex-wrap: wrap; gap: 6px; }
.prov-config-models__tag {
  display: inline-flex; align-items: center; gap: 3px;
  padding: 3px 8px; font-size: 11px; font-weight: 500;
  border-radius: 6px; background: var(--color-background);
  border: 1px solid var(--color-border); color: var(--color-text-2);
}
.prov-config-models__vis { font-size: 10px; }

.settings-split__mid, .settings-split__detail, .mcp-left, .mcp-detail, .skills-left { min-width: 0; }
.detail-head {
  display: flex; align-items: center; justify-content: space-between; gap: 12px;
  padding-bottom: 12px; margin-bottom: 4px;
  border-bottom: 1px solid var(--color-border);
}
.detail-head__left { display: flex; align-items: center; gap: 10px; min-width: 0; }
.detail-head__icon {
  width: 36px; height: 36px; flex-shrink: 0;
  display: flex; align-items: center; justify-content: center;
  border-radius: 10px; font-size: 16px; font-weight: 700; color: #fff;
}
.detail-title { margin: 0; font-size: 1rem; font-weight: 600; color: var(--color-text-1); line-height: 1.3; }
.detail-head__status {
  font-size: 11px; font-weight: 500; margin-top: 1px;
  &.is-on { color: #22c55e; }
  &.is-off { color: var(--color-text-3); }
}
.detail-head__actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
.api-key-row { display: flex; gap: 8px; width: 100%; align-items: center; }
.api-key-row .el-input { flex: 1; }
.prov-model-add { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; }

.model-section {
  margin-top: 4px;
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-md, 10px);
  background: var(--color-background);
  overflow: hidden;
}
.model-section__header {
  display: flex; align-items: center; gap: 8px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-background-soft);
}
.model-section__title { font-size: 14px; font-weight: 600; color: var(--color-text-1); }
.model-section__count {
  font-size: 11px; font-weight: 600; padding: 1px 7px;
  border-radius: 10px; background: var(--color-primary-mute);
  color: var(--fox-accent-fg, #3b82f6);
}
.model-section__toolbar {
  margin-left: auto; display: flex; align-items: center; gap: 4px;
}
.model-toolbar-btn {
  width: 26px; height: 26px; display: flex; align-items: center; justify-content: center;
  border: 1px solid var(--color-border); border-radius: 6px;
  background: transparent; color: var(--color-text-3);
  cursor: pointer; transition: all 0.12s;
  &:hover { color: var(--color-text-1); background: var(--color-hover); border-color: var(--color-text-3); }
}
.model-section__actions {
  display: flex; align-items: center; gap: 6px; flex-wrap: wrap;
  padding: 8px 14px;
  border-bottom: 1px solid var(--color-border);
}
.model-search-bar { padding: 6px 14px; border-bottom: 1px solid var(--color-border); }
.model-list-wrap {
  max-height: 340px; overflow-y: auto;
  scrollbar-width: thin; scrollbar-color: var(--color-border) transparent;
}
.model-group__label {
  padding: 6px 14px 4px; font-size: 11px; font-weight: 600;
  color: var(--color-text-3); text-transform: none; letter-spacing: 0.02em;
  background: var(--color-background-mute); border-bottom: 1px solid var(--color-border);
  position: sticky; top: 0; z-index: 1;
}
.model-row {
  display: flex; align-items: center; gap: 6px;
  padding: 6px 14px; border-bottom: 1px solid var(--color-border);
  transition: background 0.1s;
  &:hover { background: var(--color-hover); }
  &:last-child { border-bottom: none; }
}
.model-row__icon { flex-shrink: 0; color: var(--color-text-4, var(--color-text-3)); display: flex; align-items: center; }
.model-row__id {
  flex: 1; min-width: 0; font-size: 13px; font-weight: 500;
  color: var(--color-text-1); overflow: hidden; text-overflow: ellipsis;
  white-space: nowrap; font-family: ui-monospace, 'Cascadia Code', monospace;
}
.model-row__status { flex-shrink: 0; display: flex; align-items: center; }
.model-row__actions {
  display: flex; align-items: center; gap: 2px; flex-shrink: 0;
  opacity: 0; transition: opacity 0.1s;
}
.model-row:hover .model-row__actions { opacity: 1; }
.model-vision-btn {
  width: 22px; height: 22px; flex-shrink: 0;
  display: flex; align-items: center; justify-content: center;
  border: none; border-radius: 4px; background: transparent;
  color: var(--color-text-4, var(--color-text-3)); cursor: pointer;
  transition: all 0.12s; opacity: 0.5;
  &:hover { opacity: 1; color: var(--color-text-2); }
  &.is-on { opacity: 1; color: #8b5cf6; }
}
.model-act-btn {
  width: 22px; height: 22px; display: flex; align-items: center; justify-content: center;
  border: none; border-radius: 4px; background: transparent;
  color: var(--color-text-3); cursor: pointer; font-size: 13px; font-weight: 700;
  transition: all 0.1s;
  &:hover { color: var(--color-text-1); background: var(--color-hover); }
  &--danger:hover { color: #ef4444; background: #ef444415; }
}
.model-empty {
  padding: 24px 14px; text-align: center;
  font-size: 12px; color: var(--color-text-3);
}
.model-test-spinner {
  display: inline-block;
  width: 12px;
  height: 12px;
  border: 2px solid var(--color-border);
  border-top-color: var(--fox-accent-border, #3b82f6);
  border-radius: 50%;
  animation: model-spin 0.7s linear infinite;
}
@keyframes model-spin { to { transform: rotate(360deg); } }
.empty-detail { padding: 24px; text-align: center; color: var(--color-text-3); }
.data-subnav {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 4px;
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-sm);
  background: var(--color-background-mute);
  height: fit-content;
}
.data-subnav__btn {
  border: none;
  background: transparent;
  text-align: left;
  padding: 8px 10px;
  border-radius: var(--fox-radius-sm);
  font-size: 0.85rem;
  color: var(--color-text-2);
  cursor: pointer;
  &.on { background: var(--color-primary-mute); color: var(--color-text-1); }
  &:hover { background: var(--color-hover); }
}
.data-sub__main { min-width: 0; }
.mcp-left__list { list-style: none; margin: 0; padding: 0; }
.mcp-left__item {
  padding: 8px 10px;
  border-radius: var(--fox-radius-sm);
  cursor: pointer;
  font-size: 0.85rem;
  margin-bottom: 4px;
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  &.on { background: var(--color-primary-mute); }
  &:hover { background: var(--color-hover); }
}
.model-pick-row {
  display: grid;
  grid-template-columns: 1fr minmax(160px, 1fr) auto;
  gap: 8px;
  align-items: center;
  margin-bottom: 12px;
}
.model-pick-row__text { min-width: 0; }
.icon-btn { padding: 8px; }
.mem-head { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px; }
.mem-toolbar { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
.flex-1 { flex: 1; min-width: 0; }
.mem-list { list-style: none; margin: 0; padding: 0; }
.mem-list__it {
  display: flex; justify-content: space-between; gap: 8px; align-items: flex-start;
  padding: 10px 0; border-bottom: 1px solid var(--color-border);
  font-size: 0.9rem;
}
.settings-mono { font-family: ui-monospace, monospace; word-break: break-all; }

.proxy-endpoints { display: flex; flex-direction: column; gap: 10px; }
.proxy-ep {
  display: flex; align-items: center; gap: 10px; padding: 8px 12px;
  background: var(--color-background-soft); border-radius: 8px;
}
.proxy-ep__badge {
  font-size: 0.72rem; font-weight: 600; padding: 2px 8px; border-radius: 4px;
  text-transform: uppercase; white-space: nowrap; letter-spacing: 0.5px;
}
.proxy-ep__badge--openai { background: #10a37f22; color: #10a37f; }
.proxy-ep__badge--anthropic { background: #d4783422; color: #d47834; }
.proxy-ep__badge--models { background: #6366f122; color: #6366f1; }
.proxy-ep__badge--health { background: #22c55e22; color: #22c55e; }
.proxy-ep__url {
  flex: 1; font-family: ui-monospace, monospace; font-size: 0.82rem;
  word-break: break-all; color: var(--color-text-2);
}

.proxy-examples { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 12px; }
.proxy-example {
  background: var(--color-background-soft); border-radius: 8px; padding: 12px 14px;
  border: 1px solid var(--color-border);
}
.proxy-example__title {
  font-size: 0.82rem; font-weight: 600; margin-bottom: 8px; color: var(--color-text-2);
}
.proxy-example__code {
  font-family: ui-monospace, monospace; font-size: 0.78rem; line-height: 1.6;
  background: var(--color-background); border-radius: 6px; padding: 10px 12px;
  white-space: pre-wrap; word-break: break-all; margin: 0;
  color: var(--color-text-1); border: 1px solid var(--color-border);
}
.ch-ico { margin-right: 4px; }
.ch-rows { display: flex; flex-direction: column; gap: 12px; }
.ch-card {
  padding: 14px; border-radius: 8px; border: 1px solid var(--color-border);
  background: var(--color-bg-2); transition: border-color .15s;
  &:hover { border-color: var(--el-color-primary-light-5, #b3d8ff); }
}
.ch-card__head { display: flex; align-items: center; gap: 10px; }
.ch-card__name { flex: 1; }
.ch-card__row { margin-top: 8px; }
.ch-card__actions { display: flex; gap: 8px; margin-top: 10px; }
.m-t-xs { margin-top: 6px; }
.ch-empty-hint {
  display: flex; flex-direction: column; align-items: center; gap: 6px;
  padding: 32px 16px; color: var(--color-text-3); text-align: center;
}
.ch-empty-icon { font-size: 36px; }
.ch-count-badge {
  display: inline-flex; align-items: center; justify-content: center;
  min-width: 18px; height: 18px; padding: 0 5px;
  border-radius: 9px; font-size: 11px; font-weight: 600;
  background: var(--el-color-primary); color: #fff; margin-left: auto;
}
.ch-row { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
.m-l-s { margin-left: 8px; }
.m-b { margin-bottom: 8px; }
.phrase-list { list-style: none; margin: 0; padding: 0; }
.phrase-list__it {
  display: flex; align-items: flex-start; gap: 10px;
  padding: 12px 0; border-bottom: 1px solid var(--color-border);
}
.phrase-body { flex: 1; min-width: 0; }
.phrase-title { font-weight: 500; font-size: 0.9rem; }
.phrase-prev { margin-top: 4px; word-break: break-word; }
.phrase-ico { font-size: 1.2rem; }
.sel-actions { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin-top: 8px; }
.sel-act {
  border: 1px solid var(--color-border);
  background: var(--color-background);
  border-radius: 999px;
  padding: 6px 12px;
  font-size: 0.85rem;
  cursor: pointer;
  &.off { opacity: 0.45; }
}
.skills-head { display: flex; justify-content: space-between; align-items: center; gap: 8px; margin-bottom: 8px; }
.skills-search { max-width: 160px; }
.code-preview { white-space: pre-wrap; word-break: break-word; }
.zoom-bar { display: flex; align-items: center; gap: 8px; width: 100%; min-width: 220px; }
.zoom-pct { font-size: 0.85rem; color: var(--color-text-3); min-width: 44px; }
.stretch { align-items: stretch !important; }
.control-col.stretch .setting-row__control { align-items: stretch; }

.settings-panel {
  min-height: 0;
}

/* ---- Cards ---- */
.settings-card {
  border-radius: var(--fox-radius-md);
  border: 1px solid var(--color-border);
  background: var(--color-background-soft);
  padding: 16px;
  margin-bottom: 16px;
  box-shadow: var(--shadow-sm);
  transition: box-shadow 0.2s ease;

  &:last-child {
    margin-bottom: 0;
  }
}

.settings-card__title {
  font-size: 0.8rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  text-transform: uppercase;
  color: var(--color-text-3);
  margin: 0 0 4px;
}

.settings-card__intro {
  font-size: 0.875rem;
  line-height: 1.5;
  color: var(--color-text-2);
  margin: 0 0 12px;
}

.settings-card__actions {
  margin-bottom: 8px;
}

.settings-card__foot {
  font-size: 0.8rem;
  color: var(--color-text-3);
  margin: 12px 0 0;
}

/* ---- macOS-style rows: label left, control right ---- */
.setting-row {
  display: flex;
  align-items: center;
  gap: 16px;
  min-height: 48px;
  padding: 4px 0;
  border-bottom: 1px solid var(--color-border);

  &:last-child {
    border-bottom: none;
  }
}

.settings-card .setting-row:last-child {
  border-bottom: none;
}

/* When card has title/intro but no lead row: first row still gets top spacing */
.settings-card__title + .settings-card__intro + .setting-row,
.settings-card__intro + .setting-row {
  margin-top: 0;
  padding-top: 4px;
}

.setting-row__info {
  flex: 1;
  min-width: 0;
  padding-right: 8px;
}

.setting-row__label {
  font-size: 0.9rem;
  font-weight: 500;
  color: var(--color-text-1);
  line-height: 1.35;
}

.setting-row__desc {
  font-size: 0.75rem;
  color: var(--color-text-3);
  line-height: 1.4;
  margin-top: 2px;

  &:empty {
    display: none;
  }
}

.setting-row__control {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  min-width: 120px;
  max-width: 52%;
}

.control-row {
  flex-wrap: wrap;
  gap: 8px;
  max-width: 100%;
  justify-content: flex-end;
}

.control-col {
  flex-direction: column;
  align-items: stretch;
  min-width: 200px;
  max-width: 100%;
}

.slider-hint {
  margin: 4px 0 0;
  font-size: 0.8rem;
  color: var(--color-text-3);
  text-align: right;
  width: 100%;
}

/* ---- Tables in cards ---- */
.m-t { margin-top: 12px; }
.w-full { width: 100%; }
.t-sm { font-size: 0.8rem; color: var(--color-text-3); }
.settings-table {
  width: 100%;
}

:deep(.el-table) {
  background: var(--color-background);
  border-radius: var(--fox-radius-sm);
  overflow: hidden;
}

:deep(.el-table th.el-table__cell) {
  background: var(--color-background-mute);
}

/* ---- Pills, accent, misc ---- */
.pills { display: flex; flex-wrap: wrap; gap: 8px; justify-content: flex-end; }
.pill {
  border: 1px solid var(--color-border);
  background: var(--color-background);
  color: var(--color-text-2);
  padding: 6px 14px;
  border-radius: 999px;
  font-size: 0.85rem;
  cursor: pointer;
  transition: border-color 0.2s ease, color 0.2s ease, background 0.2s ease, box-shadow 0.2s ease;
  &:hover { background: var(--color-hover); }
  &.on {
    color: var(--fox-accent-on);
    background: var(--fox-accent-fg);
    border-color: var(--fox-accent-border);
    box-shadow: var(--shadow-sm);
  }
}
.mask { font-family: monospace; letter-spacing: 1px; }
.m-l-input { max-width: 200px; min-width: 120px; }
.m-r { margin-right: 8px; }
.m-l-small { margin-left: 8px; font-size: 0.85rem; color: var(--color-text-3); }
.btn-accent { background: var(--color-text-1) !important; border-color: var(--color-text-1) !important; color: var(--color-background) !important; }

/* ---- About block ---- */
.about-block .about-card {
  border-radius: var(--fox-radius-sm);
  border: 1px solid var(--color-border);
  background: var(--color-background);
  padding: 16px 18px;
  margin-bottom: 12px;
  .app-name { font-size: 1.15rem; font-weight: 600; color: var(--color-text-1); }
  .ver { color: var(--color-text-2); font-size: 0.9rem; margin: 8px 0 0; }
  .desc { color: var(--color-text-3); line-height: 1.5; font-size: 0.9rem; }
  .links { color: var(--color-text-3); font-size: 0.85rem; }
}

.shortcut-recorder {
  display: flex; align-items: center; gap: 6px;
  padding: 4px 10px; border-radius: 6px; cursor: pointer;
  border: 1px solid var(--color-border); background: var(--color-bg-2);
  min-height: 28px; transition: all .15s;
  &:focus, &.recording {
    border-color: var(--el-color-primary); box-shadow: 0 0 0 2px rgba(var(--el-color-primary-rgb, 64,158,255), .15);
    outline: none;
  }
}
.shortcut-keys {
  font-family: var(--user-code-font, monospace); font-size: 12px;
  color: var(--color-text-1); white-space: nowrap;
}
.shortcut-hint {
  font-size: 11px; color: var(--el-color-primary); animation: pulse-hint 1s infinite;
}
@keyframes pulse-hint { 0%, 100% { opacity: 1; } 50% { opacity: .5; } }

</style>
