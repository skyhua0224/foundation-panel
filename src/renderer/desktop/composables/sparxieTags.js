// The chips of a Sparxie application on a big-screen tile (the same three the Web UI's cards carry): Sparxie, the region, the launch mode. A tile of an ordinary application has none.
// The block is the `sparxie` object of the application in the host's list (see the Web UI's utils/sparxie/model.js).
const WORDS = {
  zh: { regions: { cn: '国服', bili: 'B 服', global: '国际服' }, touch: '触控', pc: '键鼠与手柄', more: '更多' },
  en: { regions: { cn: 'CN', bili: 'Bilibili', global: 'Global' }, touch: 'Touch', pc: 'Keyboard & gamepad', more: 'More' },
}
export const sparxieLang = (locale) => (String(locale || 'zh').toLowerCase().startsWith('zh') ? 'zh' : 'en')
export const sparxieMore = (locale) => WORDS[sparxieLang(locale)].more

/** `present`: the host reports the component installed; without it nothing of Sparxie is drawn */
export function sparxieTags(app, locale, present = true) {
  const b = present && app && typeof app.sparxie === 'object' && app.sparxie ? app.sparxie : null
  if (!b) return []
  const W = WORDS[sparxieLang(locale)]
  // `mark`: the chip carries the Sparxie picture (the component maps it to the URL of its own asset)
  const tags = [{ key: 'spx', icon: 'extension', mark: true, tone: 'tertiary', label: 'Sparxie' }]
  if (b.region && W.regions[b.region]) tags.push({ key: 'region', icon: 'public', tone: 'neutral', label: W.regions[b.region] })
  tags.push(b.mode === 'pc' ? { key: 'mode', icon: 'keyboard', tone: 'neutral', label: W.pc } : { key: 'mode', icon: 'touch_app', tone: 'info', label: W.touch })
  return tags
}
