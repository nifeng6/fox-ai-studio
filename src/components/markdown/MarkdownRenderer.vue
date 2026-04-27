<script setup lang="ts">
import { computed, watch, onMounted, onUnmounted, nextTick, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import MarkdownIt from 'markdown-it'
import type { PluginSimple } from 'markdown-it'
import hljs from 'highlight.js'
import 'highlight.js/styles/github.min.css'
import 'highlight.js/styles/github-dark.min.css'
import katex from 'katex'
import 'katex/dist/katex.min.css'
import texmath from 'markdown-it-texmath'
import taskLists from 'markdown-it-task-lists'
import mermaid from 'mermaid'
import 'markdown-it-texmath/css/texmath.css'
import { open } from '@tauri-apps/plugin-shell'

const props = withDefaults(
  defineProps<{
    content: string
  }>(),
  { content: '' }
)

const { t } = useI18n()
const rootRef = ref<HTMLElement | null>(null)
let md: MarkdownIt | null = null
let mermaidInitTheme: string | null = null
let mermaidId = 0

function isExternal(href: string) {
  return /^https?:\/\//i.test(href) || href.startsWith('//')
}

async function openExternal(href: string) {
  const hasTauri = typeof window !== 'undefined' && (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
  if (hasTauri) {
    try {
      await open(href)
      return
    } catch (e) {
      console.error(e)
    }
  }
  window.open(href, '_blank', 'noopener,noreferrer')
}

const highlightPlugin: PluginSimple = (mdIt) => {
  const def = mdIt.options.highlight
  mdIt.set({
    highlight(str, lang, attrs) {
      const l = (lang || '').toLowerCase()
      if (l && hljs.getLanguage(l)) {
        try {
          return (
            '<pre class="hljs"><code' +
            (attrs ? ' ' + attrs : '') +
            '>' +
            hljs.highlight(str, { language: l, ignoreIllegals: true }).value +
            '</code></pre>'
          )
        } catch {
          /* fall through */
        }
      } else if (l && l !== 'mermaid' && l !== 'text' && l !== 'plaintext' && l !== 'txt') {
        const guess = hljs.highlightAuto(str)
        return (
          '<pre class="hljs"><code' +
          (attrs ? ' ' + attrs : '') +
          '>' +
          guess.value +
          '</code></pre>'
        )
      }
      if (def) {
        return def(str, lang, attrs)
      }
      return (
        '<pre class="hljs"><code' + (attrs ? ' ' + attrs : '') + '>' + mdIt.utils.escapeHtml(str) + '</code></pre>'
      )
    }
  })
}

function getMd(): MarkdownIt {
  if (md) return md
  md = new MarkdownIt({ html: false, linkify: true, typographer: true, breaks: true })
  highlightPlugin(md)
  md.use(texmath, {
    engine: katex,
    delimiters: 'dollars',
    katexOptions: { throwOnError: false, output: 'html' }
  })
  md.use(taskLists, { enabled: true, label: true, labelAfter: true } as any)

  const defLink =
    md.renderer.rules.link_open ||
    function (this: any, tokens: any[], i: number, o: any, e: any, s: any) {
      return s.renderToken(tokens, i, o)
    }
  md.renderer.rules.link_open = (tokens, idx, options, env, self) => {
    const tkn = tokens[idx]
    const href = tkn.attrGet('href') || ''
    if (isExternal(href)) {
      tkn.attrSet('rel', 'noopener noreferrer')
      tkn.attrSet('class', (tkn.attrGet('class') || '') + ' md-external-link'.trim())
      tkn.attrSet('data-external', '1')
    }
    return defLink.call(self, tokens, idx, options, env, self)
  }

  md.renderer.rules.fence = (tokens, idx) => {
    const token = tokens[idx]
    const info = (token.info || '').trim()
    const lang = (info.split(/\s+/g)[0] || '').trim() || 'text'
    const raw = token.content

    if (lang === 'mermaid') {
      mermaidId += 1
      const graphId = `mmd-${mermaidId}`
      const safe = raw.replace(/&/g, '&amp;').replace(/</g, '&lt;')
      return `<div class="md-mermaid-wrap"><pre class="mermaid" id="${graphId}">${safe}</pre></div>`
    }

    const id = `c-${Date.now()}-${idx}-${Math.random().toString(36).slice(2, 9)}`
    const l = (md as MarkdownIt).utils.escapeHtml(lang)
    let highlighted: string
    if (l !== 'text' && l && hljs.getLanguage(l)) {
      try {
        highlighted = hljs.highlight(raw, { language: l, ignoreIllegals: true }).value
      } catch {
        highlighted = (md as MarkdownIt).utils.escapeHtml(raw)
      }
    } else {
      try {
        highlighted = hljs.highlightAuto(raw).value
      } catch {
        highlighted = (md as MarkdownIt).utils.escapeHtml(raw)
      }
    }
    // After highlighting, add line numbers
    const lines = highlighted.split('\n')
    const lineCount = lines.length
    // Only show line numbers for blocks > 3 lines
    const showLineNums = lineCount > 3
    const lineNumsHtml = showLineNums
      ? `<div class="md-fence__lines">${lines.map((_, i) => `<span>${i + 1}</span>`).join('\n')}</div>`
      : ''
    const isLong = lineCount > 20
    const wrapStart = isLong
      ? `<details class="md-fence__collapse" open><summary class="md-fence__collapse-btn">展开 ${lineCount} 行</summary>`
      : ''
    const wrapEnd = isLong ? '</details>' : ''

    return `<div class="md-fence" data-fence data-lines="${lineCount}">
      <div class="md-fence__toolbar">
        <span class="md-fence__lang">${l}</span>
        <span class="md-fence__line-count">${lineCount} lines</span>
        <button type="button" class="md-fence__copy" data-md-copy data-copy-id="${id}">${(md as MarkdownIt).utils.escapeHtml(String(t('chat.copy')))}</button>
      </div>
      ${wrapStart}
      <div class="md-fence__body${showLineNums ? ' md-fence__body--numbered' : ''}">
        ${lineNumsHtml}
        <pre class="md-fence__pre"><code class="hljs language-${l}" id="${id}">${highlighted}</code></pre>
      </div>
      ${wrapEnd}
    </div>`
  }
  return md
}

const html = computed(() => {
  if (!props.content) return '<p class="md-empty-p"></p>'
  mermaidId = 0

  let content = props.content
  // Stream-safe: count backtick fences to handle unclosed blocks
  const fenceMatches = content.match(/^```/gm)
  if (fenceMatches && fenceMatches.length % 2 !== 0) {
    // Odd number of fences = unclosed block, close it
    content = content + '\n```'
  }

  return getMd().render(content)
})

function mermaidTheme(): 'default' | 'dark' {
  return document.documentElement.getAttribute('theme-mode') === 'dark' ? 'dark' : 'default'
}

async function runMermaid() {
  const root = rootRef.value
  if (!root) return
  const nodes = root.querySelectorAll<HTMLElement>('pre.mermaid')
  if (!nodes.length) return
  const th = mermaidTheme()
  if (mermaidInitTheme !== th) {
    mermaidInitTheme = th
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: 'loose',
      theme: th
    })
  }
  try {
    await mermaid.run({ nodes: [...nodes] })
  } catch (e) {
    console.error('[mermaid]', e)
  }
}

function onRootClick(e: MouseEvent) {
  const tEl = (e.target as HTMLElement).closest('[data-md-copy]')
  if (tEl) {
    const cId = tEl.getAttribute('data-copy-id')
    if (cId) {
      const el = document.getElementById(cId)
      if (el) {
        const text = el.textContent || ''
        void navigator.clipboard.writeText(text).then(
          () => ElMessage.success(t('common.copied')),
          () => ElMessage.error(t('common.error'))
        )
      }
    }
    return
  }
  const a = (e.target as HTMLElement).closest('a[href]') as HTMLAnchorElement | null
  if (a && a.getAttribute('data-external') === '1' && a.href) {
    e.preventDefault()
    e.stopPropagation()
    void openExternal(a.href)
  }
}

let themeObserver: MutationObserver | null = null

onMounted(() => {
  void nextTick(() => {
    rootRef.value?.addEventListener('click', onRootClick, true)
  })
  void nextTick(() => {
    void runMermaid()
  })
  if (typeof MutationObserver !== 'undefined') {
    themeObserver = new MutationObserver(() => {
      mermaidInitTheme = null
      void runMermaid()
    })
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['theme-mode'] })
  }
})

onUnmounted(() => {
  rootRef.value?.removeEventListener('click', onRootClick, true)
  themeObserver?.disconnect()
  themeObserver = null
})

watch(
  [html, () => props.content],
  async () => {
    await nextTick()
    await runMermaid()
  },
  { flush: 'post' }
)
</script>

<template>
  <div ref="rootRef" class="md-root" v-html="html" />
</template>

<style lang="scss" scoped>
.md-root {
  font-size: 14px;
  line-height: 1.6;
  color: var(--color-text-1);
  word-break: break-word;
  min-width: 0;
}

:deep(p) {
  margin: 0.5em 0;
}

:deep(p.md-empty-p) {
  min-height: 0;
  margin: 0;
}

:deep(ul, ol) {
  margin: 0.4em 0 0.4em 1.25em;
  padding: 0;
}

:deep(ul.contains-task-list) {
  list-style: none;
  margin-left: 0.4em;
}

:deep(.task-list-item) {
  list-style: none;
}

:deep(.task-list-item input[type='checkbox']) {
  margin-right: 0.45em;
  vertical-align: middle;
  accent-color: var(--fox-accent-fg, var(--el-color-primary));
  cursor: pointer;
}

:deep(li) {
  margin: 0.2em 0;
}

:deep(a) {
  color: var(--color-info);
  text-decoration: none;
  border-bottom: 1px solid var(--color-border);
  transition: color 0.15s, border-color 0.15s;
  cursor: pointer;
}

:deep(a:hover) {
  color: var(--color-text-1);
  border-color: var(--color-text-3);
}

:deep(a.md-external-link) {
  border-bottom-style: dashed;
}

:deep(blockquote) {
  margin: 0.6em 0;
  padding: 0.25em 0 0.25em 0.8em;
  border-left: 3px solid var(--fox-accent-border);
  color: var(--color-text-2);
  background: var(--color-hover);
  border-radius: 0 var(--fox-radius-sm) var(--fox-radius-sm) 0;
}

:deep(.katex) {
  font-size: 1.05em;
  color: var(--color-text-1);
}
:deep(.katex-display) {
  margin: 0.75em 0;
  overflow: auto;
  max-width: 100%;
  padding: 0.3em 0;
}

:deep(.md-mermaid-wrap) {
  margin: 0.75em 0;
  border-radius: var(--fox-radius-sm);
  border: 1px solid var(--color-border);
  background: var(--color-background);
  box-shadow: var(--shadow-sm);
  overflow: auto;
  padding: 8px 10px 10px;
}

:deep(.md-mermaid-wrap pre.mermaid) {
  margin: 0;
  background: transparent;
  min-height: 40px;
}

:deep(.md-fence) {
  margin: 0.75em 0;
  border-radius: var(--fox-radius-sm);
  border: 1px solid var(--color-border);
  background: var(--color-background);
  box-shadow: var(--shadow-sm);
  overflow: hidden;
  position: relative;
}

:deep(.md-fence__toolbar) {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 6px 10px;
  background: var(--color-background-mute);
  border-bottom: 1px solid var(--color-border);
  position: relative;
}

:deep(.md-fence__lang) {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--color-text-3);
}

:deep(.md-fence__line-count) {
  font-size: 10px;
  color: var(--color-text-3);
  opacity: 0.7;
}

:deep(.md-fence__body) {
  display: flex;
  overflow: auto;
}

:deep(.md-fence__body--numbered) {
  display: flex;
}

:deep(.md-fence__lines) {
  display: flex;
  flex-direction: column;
  padding: 12px 0;
  min-width: 36px;
  text-align: right;
  user-select: none;
  border-right: 1px solid var(--color-border);
  background: var(--color-background-mute);
  flex-shrink: 0;

  span {
    display: block;
    padding: 0 8px 0 4px;
    font-size: 11px;
    line-height: 1.45;
    color: var(--color-text-3);
    opacity: 0.5;
  }
}

:deep(.md-fence__collapse) {
  border: none;
  margin: 0;
  padding: 0;
}

:deep(.md-fence__collapse-btn) {
  display: block;
  width: 100%;
  padding: 4px 12px;
  font-size: 11px;
  color: var(--color-text-3);
  background: var(--color-background-mute);
  cursor: pointer;
  user-select: none;
  text-align: center;
  border-top: 1px solid var(--color-border);
  &:hover {
    color: var(--color-text-2);
  }
}

:deep(.md-fence__copy) {
  margin-left: auto;
  flex-shrink: 0;
  z-index: 1;
  font-size: 12px;
  padding: 4px 10px;
  border-radius: var(--fox-radius-sm);
  border: 1px solid var(--color-border);
  background: var(--color-background-soft);
  color: var(--color-text-2);
  cursor: pointer;
  transition: background 0.15s, color 0.15s, border-color 0.15s;
}

:deep(.md-fence__copy:hover) {
  background: var(--color-hover);
  color: var(--color-text-1);
  border-color: var(--color-text-3);
}

:deep(.md-fence__copy:active) {
  background: var(--color-active);
}

:deep(.md-fence__pre) {
  flex: 1;
  min-width: 0;
  margin: 0;
  padding: 12px 14px;
  overflow: auto;
  max-height: 400px;
  background: var(--color-background) !important;
  font-size: 13px;
  line-height: 1.45;
}

:deep(.md-fence__pre code) {
  font-family: ui-monospace, 'Cascadia Code', 'Consolas', Menlo, monospace;
}

:deep(p code) {
  padding: 0.1em 0.35em;
  border-radius: 6px;
  background: var(--color-background-mute);
  border: 1px solid var(--color-border);
  font-size: 0.9em;
  font-family: ui-monospace, 'Cascadia Code', 'Consolas', Menlo, monospace;
}

:deep(table) {
  border-collapse: separate;
  border-spacing: 0;
  width: 100%;
  font-size: 13px;
  margin: 0.75em 0;
  border: 1px solid var(--color-border);
  border-radius: var(--fox-radius-sm);
  overflow: hidden;
}

:deep(th, td) {
  border: 1px solid var(--color-border);
  padding: 8px 12px;
  text-align: left;
}

:deep(tr:first-child th) {
  background: var(--color-background-mute);
  color: var(--color-text-1);
  font-weight: 600;
}

:deep(tbody tr:nth-child(odd)) {
  background: var(--color-background);
}

:deep(tbody tr:nth-child(even)) {
  background: var(--color-hover);
}

:deep(tbody tr:hover td) {
  background: var(--color-active);
}

:deep(h1) {
  font-size: 1.35em;
  margin: 0.5em 0 0.35em;
  font-weight: 600;
}
:deep(h2) {
  font-size: 1.2em;
  margin: 0.5em 0 0.35em;
  font-weight: 600;
}
:deep(h3) {
  font-size: 1.08em;
  margin: 0.4em 0 0.3em;
  font-weight: 600;
}
</style>

<style lang="scss">
[theme-mode='dark'] .md-root .hljs {
  background: transparent !important;
  color: #e3e3e3;
}
[theme-mode='light'] .md-root .hljs {
  background: transparent !important;
  color: #24292e;
}
</style>
