/**
 * Markdown for quiz text (prompts, options, answers, explanations).
 *
 * Raw HTML is disabled, so content can't inject markup; links open in a new tab. Single line
 * breaks are kept (`breaks: true`), so older plain-text questions render exactly as before.
 * Fenced code blocks are syntax-highlighted with highlight.js (common languages only).
 */
import hljs from 'highlight.js/lib/common'
import markdownIt, { type MarkdownIt, type RendererRule } from 'markdown-it'

const escapeHtml = (s: string): string =>
  s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')

const md: MarkdownIt = markdownIt({
  html: false,
  linkify: true,
  breaks: true,
  typographer: false,
  highlight(code: string, lang: string): string {
    const language = lang && hljs.getLanguage(lang) ? lang : null
    const body = language
      ? hljs.highlight(code, { language, ignoreIllegals: true }).value
      : escapeHtml(code)
    return `<pre class="hljs"><code${language ? ` data-lang="${escapeHtml(language)}"` : ''}>${body}</code></pre>`
  },
})

// External links: new tab, no opener.
const defaultLink: RendererRule =
  md.renderer.rules.link_open ??
  ((tokens, idx, opts, _env, self) => self.renderToken(tokens, idx, opts))
const linkOpen: RendererRule = (tokens, idx, opts, env, self) => {
  tokens[idx]!.attrSet('target', '_blank')
  tokens[idx]!.attrSet('rel', 'noopener noreferrer')
  return defaultLink(tokens, idx, opts, env, self)
}
md.renderer.rules.link_open = linkOpen

const cache = new Map<string, string>()

/** Render Markdown to safe HTML (memoized: the same question re-renders often). */
export function renderMarkdown(src: string | null | undefined): string {
  if (!src) return ''
  const hit = cache.get(src)
  if (hit !== undefined) return hit
  const html = md.render(src)
  if (cache.size > 500) cache.clear()
  cache.set(src, html)
  return html
}
