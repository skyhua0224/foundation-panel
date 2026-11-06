import { onBeforeUnmount, onMounted, ref } from 'vue'

// The big-screen pages are read from a sofa: every page of this slice is drawn 1.25 times the Material size.
// The theme editor's 字号 (--fd-font-size, 12 to 18 px around the 16 px of the Material theme) scales it further.
export const DESK_BASE_SCALE = 1.25
const DEFAULT_FONT_PX = 16

/** The zoom for a --fd-font-size value; anything that is not a sensible pixel size leaves the base scale. */
export function deskScale(fontSize) {
  const px = Number.parseFloat(fontSize)
  if (!Number.isFinite(px) || px < 8 || px > 40) return DESK_BASE_SCALE
  return Math.round(DESK_BASE_SCALE * (px / DEFAULT_FONT_PX) * 1000) / 1000
}

export function useDeskScale() {
  const scale = ref(DESK_BASE_SCALE)
  let observer = null
  const read = () => {
    scale.value = deskScale(getComputedStyle(document.documentElement).getPropertyValue('--fd-font-size'))
  }
  onMounted(() => {
    read()
    if (typeof MutationObserver === 'function') {
      observer = new MutationObserver(read)
      observer.observe(document.documentElement, { attributes: true, attributeFilter: ['style'] })
    }
  })
  onBeforeUnmount(() => observer?.disconnect())
  return scale
}
