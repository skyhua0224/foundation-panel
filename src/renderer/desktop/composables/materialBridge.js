/**
 * Material 3 → 旧 --fd-* 变量的桥。
 *
 * 大屏壳的颜色现在只有一个来源：foundation-ui 的 --md-sys-color-*（随外观里的
 * 跟随系统 / 浅色 / 深色、壁纸取色变化）。设置、主题编辑器等仍读 --fd-*，
 * 所以默认的 'material' 预设把这些变量从 token 实时推出来，旧代码不用改就跟着变。
 */

const COLOUR_MAP = [
  ['--fd-accent', 'primary'],
  ['--fd-accent-secondary', 'tertiary'],
  ['--fd-bg-primary', 'surface'],
  ['--fd-bg-secondary', 'surface-container'],
  ['--fd-bg-tertiary', 'surface-container-high'],
  ['--fd-text-primary', 'on-surface'],
  ['--fd-status-success', 'success'],
  ['--fd-status-warning', 'warning'],
  ['--fd-status-danger', 'error'],
]

export const BRIDGED_KEYS = COLOUR_MAP.flatMap(([key]) => [key, `${key}-rgb`]).concat([
  '--fd-text-secondary',
  '--fd-text-muted',
])

/** 'rgb(1, 2, 3)' / 'rgba(1, 2, 3, .5)' / '#abc' / '#aabbcc' → [r, g, b]；认不出来返回 null。 */
export function parseColour(text) {
  const value = String(text || '').trim()
  let match = value.match(/^rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)/i)
  if (match) return [match[1], match[2], match[3]].map((part) => Math.round(Number(part)))
  match = value.match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/i)
  if (!match) return null
  const hex = match[1].length === 3 ? match[1].replace(/./g, '$&$&') : match[1]
  return [0, 2, 4].map((at) => parseInt(hex.slice(at, at + 2), 16))
}

export function hexOf([r, g, b]) {
  return `#${[r, g, b].map((part) => part.toString(16).padStart(2, '0')).join('')}`
}

/** 用一个探针元素让浏览器把 var(--md-sys-color-x) 解析成 rgb()，hex、color-mix 等写法都能吃。 */
function resolveTokens(root) {
  const probe = document.createElement('span')
  probe.style.cssText = 'position:absolute;visibility:hidden;pointer-events:none;width:0;height:0'
  root.appendChild(probe)
  const out = {}
  for (const [, token] of COLOUR_MAP) {
    probe.style.color = `var(--md-sys-color-${token})`
    out[token] = parseColour(getComputedStyle(probe).color)
  }
  probe.remove()
  return out
}

let lastSignature = ''

/** 把当前 token 写进 --fd-*。值没变就不写（观察器靠这个收敛）。 */
export function applyMaterialBridge(root = document.documentElement) {
  const resolved = resolveTokens(root)
  const signature = JSON.stringify(resolved)
  if (signature === lastSignature) return false
  lastSignature = signature
  for (const [key, token] of COLOUR_MAP) {
    const rgb = resolved[token]
    if (!rgb) continue
    root.style.setProperty(key, hexOf(rgb))
    root.style.setProperty(`${key}-rgb`, rgb.join(', '))
  }
  const text = resolved['on-surface']
  if (text) {
    root.style.setProperty('--fd-text-secondary', `rgba(${text.join(',')},0.74)`)
    root.style.setProperty('--fd-text-muted', `rgba(${text.join(',')},0.5)`)
  }
  return true
}

/**
 * 桥当前写在根上的 --fd-* 颜色，原样取出。material 预设只存圆角和尺寸，颜色在桥里；
 * 用户一改变量就离开 material 预设，桥被清掉，没有这份快照就只剩被改的那一个键。
 */
export function snapshotMaterialBridge(root = document.documentElement) {
  const out = {}
  for (const key of BRIDGED_KEYS) {
    const value = root.style.getPropertyValue(key)
    if (value) out[key] = value
  }
  return out
}

/** vars 缺哪个桥接颜色，就用快照补上（vars 里已有的不动）。 */
export function withBridgedColours(vars, snapshot) {
  const out = { ...vars }
  for (const [key, value] of Object.entries(snapshot)) if (!(key in out)) out[key] = value
  return out
}

export function clearMaterialBridge(root = document.documentElement) {
  lastSignature = ''
  for (const key of BRIDGED_KEYS) root.style.removeProperty(key)
}

let observer = null

/** 外观变了（浅/深、取色、壁纸）就重算。返回停止函数。 */
export function watchMaterialBridge(root = document.documentElement) {
  applyMaterialBridge(root)
  if (typeof MutationObserver === 'undefined') return () => {}
  observer?.disconnect()
  let queued = false
  observer = new MutationObserver(() => {
    if (queued) return
    queued = true
    requestAnimationFrame(() => {
      queued = false
      applyMaterialBridge(root)
    })
  })
  observer.observe(root, { attributes: true, attributeFilter: ['class', 'style', 'data-theme', 'data-bs-theme', 'data-dark', 'data-wallpaper'] })
  return () => {
    observer?.disconnect()
    observer = null
  }
}
