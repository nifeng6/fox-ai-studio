import { createApp } from 'vue'
import { createPinia } from 'pinia'
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate'
import ElementPlus from 'element-plus'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import 'element-plus/dist/index.css'
import router from './router'
import i18n from './i18n'
import App from './App.vue'
import './assets/styles/global.scss'

function showFatalError(msg: string) {
  const el = document.getElementById('app')
  if (el) {
    el.innerHTML = `<pre style="padding:24px;color:#ff6b6b;background:#1a1a2e;overflow:auto;height:100vh;font-size:13px;white-space:pre-wrap;word-break:break-word;">${msg}</pre>`
  }
}

window.addEventListener('error', (e) => {
  if (e.message?.includes('ResizeObserver')) return
  console.error('[Global Error]', e.error)
  showFatalError(`[Uncaught Error]\n${e.message}\n\n${e.error?.stack || ''}`)
})

window.addEventListener('unhandledrejection', (e) => {
  console.error('[Unhandled Rejection]', e.reason)
})

const app = createApp(App)
const pinia = createPinia()
pinia.use(piniaPluginPersistedstate)

app.config.errorHandler = (err, _vm, info) => {
  console.error('[Vue Fatal]', err, info)
  showFatalError(`[Vue Error] ${info}\n\n${err}`)
}

app.use(pinia)
app.use(router)
app.use(i18n)
app.use(ElementPlus, { locale: zhCn })

async function showTauriWindow() {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    await getCurrentWindow().show()
  } catch {}
}

function dismissSplash() {
  const splash = document.getElementById('splash')
  if (splash) {
    splash.classList.add('fade-out')
    setTimeout(() => splash.remove(), 350)
  }
}

router.isReady().then(async () => {
  app.mount('#app')
  await showTauriWindow()
  requestAnimationFrame(() => {
    requestAnimationFrame(dismissSplash)
  })
}).catch(async (err) => {
  console.error('[Router Init Failed]', err)
  await showTauriWindow()
  dismissSplash()
  showFatalError(`[Router Init Failed]\n${err}`)
})
