import fs from 'node:fs'
import path from 'node:path'

const FA = process.env.FA_SVGS || '/var/tmp/ds5-redesign-b/node_modules/@fortawesome/fontawesome-free/svgs'
const cache = new Map()
export function icon(name, style = 'solid') {
  const key = `${style}/${name}`
  if (!cache.has(key)) {
    const file = path.join(FA, style, `${name}.svg`)
    if (!fs.existsSync(file)) throw new Error(`missing icon ${key}`)
    const svg = fs.readFileSync(file, 'utf8')
    const viewBox = svg.match(/viewBox="([^"]+)"/)[1]
    const body = svg.replace(/<\?xml[^>]*>/, '').replace(/<!--[\s\S]*?-->/g, '').replace(/<svg[^>]*>/, '').replace('</svg>', '').trim()
    cache.set(key, `<svg class="ic" viewBox="${viewBox}" aria-hidden="true">${body}</svg>`)
  }
  return cache.get(key)
}

export const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;')

export const chip = (kind, text) => `<span class="chip${kind ? ' chip--' + kind : ''}"><i class="dot"></i>${esc(text)}</span>`
export const btn = (text, kind = '', extra = '') => `<button type="button" class="btn${kind ? ' btn--' + kind : ''} ${extra}">${text}</button>`
export const seg = (options, on) => `<div class="seg" role="radiogroup">${options.map((o) => `<button type="button" class="seg__opt${o === on ? ' is-on' : ''}" role="radio" aria-checked="${o === on}">${esc(o)}</button>`).join('')}</div>`
export const sw = (on) => `<span class="switch${on ? ' is-on' : ''}" role="switch" aria-checked="${!!on}"></span>`
export const slider = (value, min = 0, max = 2) => {
  const pct = ((value - min) / (max - min)) * 100
  return `<div class="slider"><div class="slider__track"><div class="slider__fill" style="width:${pct}%"></div><div class="slider__thumb" style="left:${pct}%"></div></div><span class="slider__value">${value.toFixed(2)}</span></div>`
}
export const row = (label, desc, control) => `<div class="row"><div class="row__text"><div class="row__label">${label}</div>${desc ? `<div class="row__desc">${desc}</div>` : ''}</div><div class="row__ctl">${control}</div></div>`
export const winctl = () => `<div class="winctl" aria-hidden="true"><span>${icon('minus')}</span><span>${icon('square', 'regular')}</span><span>${icon('xmark')}</span></div>`
export const notice = (kind, html) => `<div class="notice notice--${kind}">${icon(kind === 'warn' ? 'triangle-exclamation' : 'circle-info')}<div>${html}</div></div>`

export const statusDot = (kind) => (kind ? `<i class="navdot navdot--${kind}"></i>` : '')

export function page(variant, theme, title, body) {
  return `<!doctype html>
<html lang="zh-CN" data-theme="${theme}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${esc(title)}</title>
<link rel="stylesheet" href="../tokens.css">
<link rel="stylesheet" href="./${variant}.css">
</head>
<body class="v-${variant}">
${body}
</body>
</html>
`
}
