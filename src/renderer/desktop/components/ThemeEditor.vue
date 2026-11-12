<template>
  <Teleport to="body">
    <Transition name="dsk-te-fade">
      <div v-if="open" class="dsk-te__scrim" @click="$emit('close')"></div>
    </Transition>

    <Transition name="dsk-te-slide">
      <aside
        v-if="open"
        ref="editorRef"
        class="dsk-te"
        role="dialog"
        aria-modal="true"
        :aria-label="t.themeEditor.title"
        :style="{ zoom: scale }"
        @click.stop
      >
        <header class="dsk-te__head">
          <MIcon name="palette" fill />
          <h2 class="dsk-te__title">{{ t.themeEditor.title }}</h2>
          <MIconButton icon="close" :label="T.close" @click="$emit('close')" />
        </header>

        <div class="dsk-te__body">
          <!-- wallpaper -->
          <MSection :title="t.themeEditor.wallpaper">
            <div
              class="dsk-te__drop"
              :class="{ 'is-dragging': isDragging, 'has-wallpaper': wallpaper }"
              @dragover.prevent="isDragging = true"
              @dragleave="isDragging = false"
              @drop.prevent="handleDrop"
            >
              <img v-if="wallpaper" :src="wallpaper" class="dsk-te__preview" alt="" />
              <div v-else class="dsk-te__empty">
                <MIcon name="image" fill />
                <span>{{ t.themeEditor.dropImage }}</span>
              </div>
              <div class="dsk-te__drop-actions">
                <template v-if="wallpaper">
                  <MButton variant="tonal" icon="sync" @click="triggerFileInput">{{ t.themeEditor.replace }}</MButton>
                  <MButton variant="text" icon="delete" @click="$emit('removeWallpaper')">{{ t.themeEditor.remove }}</MButton>
                </template>
                <MButton v-else icon="upload_file" @click="triggerFileInput">{{ T.pick }}</MButton>
              </div>
            </div>
            <input
              ref="fileInputRef"
              type="file"
              accept="image/*"
              hidden
              @change="handleFileSelect"
            />
            <!-- colours taken from the picture: choose one to make it the accent (Material You) -->
            <template v-if="wallpaperColors.length">
              <MSwatches v-model="selectedSwatch" :colors="swatches" :label="t.themeEditor.pickSeed" />
              <p class="dsk-te__hint">{{ t.themeEditor.pickSeed }}</p>
            </template>
          </MSection>

          <!-- light or dark -->
          <MSection :title="T.mode">
            <MSeg block :model-value="ap.state.mode" :label="T.mode" :options="modeOptions" @update:model-value="(v) => ap.set({ mode: v })" />
            <MSeg v-if="ap.state.mode !== 'light'" block :model-value="ap.state.dark" :label="T.darkStyle" :options="darkOptions" @update:model-value="(v) => ap.set({ dark: v })" />
          </MSection>

          <!-- presets -->
          <MSection :title="t.themeEditor.presets">
            <MGroup>
              <MItem
                v-for="(preset, key) in presets"
                :key="key"
                action
                :selected="activePreset === key"
                :title="t.themeEditor.presetNames[key] || preset.label"
                @click="$emit('applyPreset', key)"
              >
                <template #lead><span class="dsk-te__disc" :style="{ background: `linear-gradient(135deg, ${preset.vars['--fd-accent'] || 'var(--md-sys-color-primary)'} 50%, ${preset.vars['--fd-accent-secondary'] || 'var(--md-sys-color-tertiary)'} 50%)` }"></span></template>
                <template v-if="activePreset === key" #trail><MIcon name="check" /></template>
              </MItem>
            </MGroup>
          </MSection>

          <!-- colours -->
          <MSection :title="t.themeEditor.colors">
            <MGroup>
              <MItem v-for="row in colorRows" :key="row.key" :title="row.label" :text="shown[row.key]">
                <template #lead><span class="dsk-te__dot" :style="{ background: shown[row.key] }"></span></template>
                <template #trail>
                  <MIconButton icon="colorize" tonal :label="row.label" @click="pickColor(row.key)" />
                  <input
                    :ref="(el) => setColorInput(row.key, el)"
                    type="color"
                    class="dsk-te__native"
                    tabindex="-1"
                    aria-hidden="true"
                    :value="shown[row.key]"
                    @input="throttledSetVar(row.key, $event.target.value)"
                    @change="$emit('setVar', row.key, $event.target.value)"
                  />
                </template>
              </MItem>
            </MGroup>
          </MSection>

          <!-- appearance -->
          <MSection :title="t.themeEditor.appearance">
            <MGroup>
              <MItem v-for="row in rangeRows" :key="row.key" stack :title="row.label" :value="vars[row.key]">
                <template #control>
                  <div class="dsk-te__range">
                    <MIconButton icon="remove" tonal :label="`${row.label} −`" :disabled="rangeValue(row) <= row.min" @click="nudge(row, -1)" />
                    <MSlider :model-value="rangeValue(row)" :min="row.min" :max="row.max" :step="1" size="m" :label="row.label" @update:model-value="(v) => setRange(row, v)" />
                    <MIconButton icon="add" tonal :label="`${row.label} +`" :disabled="rangeValue(row) >= row.max" @click="nudge(row, 1)" />
                  </div>
                </template>
              </MItem>
            </MGroup>
          </MSection>

          <!-- effects -->
          <MSection :title="t.themeEditor.effects">
            <MGroup>
              <MSwitchItem :model-value="vars['--fd-grid-visible'] === '1'" :title="t.themeEditor.backgroundGrid" @update:model-value="(on) => $emit('setVar', '--fd-grid-visible', on ? '1' : '0')" />
              <MSwitchItem :model-value="vars['--fd-scanline-visible'] === '1'" :title="t.themeEditor.scanlines" @update:model-value="(on) => $emit('setVar', '--fd-scanline-visible', on ? '1' : '0')" />
            </MGroup>
          </MSection>

          <!-- export and import -->
          <MSection :title="t.themeEditor.data">
            <MGroup>
              <MItem action icon="download" :title="t.themeEditor.exportTheme" @click="handleExport" />
              <MItem action icon="upload_file" :title="t.themeEditor.importTheme" @click="handleImport" />
            </MGroup>
          </MSection>
        </div>
      </aside>
    </Transition>
  </Teleport>
</template>

<script setup>
import { computed, nextTick, ref, watch } from 'vue'
import { MButton, MGroup, MIcon, MIconButton, MItem, MSection, MSeg, MSlider, MSwatches, MSwitchItem, appearanceTexts, useAppearance } from 'foundation-ui/vue'
import { useModalFocusScope } from '../composables/useFocusNav.js'
import { useDeskScale } from '../composables/useDeskScale.js'
import { useI18n } from '../i18n/index.js'

const props = defineProps({
  open: { type: Boolean, required: true },
  vars: { type: Object, required: true },
  activePreset: { type: String, required: true },
  presets: { type: Object, required: true },
  wallpaper: { type: String, default: null },
  wallpaperColors: { type: Array, default: () => [] },
  wallpaperSeeds: { type: Array, default: () => [] },
  activeWallpaperSeed: { type: Number, default: null },
})

const emit = defineEmits(['close', 'setVar', 'applyPreset', 'export', 'import', 'setWallpaper', 'applySeed', 'removeWallpaper'])

const editorRef = ref(null)
useModalFocusScope(editorRef, () => props.open)

const { t, locale } = useI18n()
const scale = useDeskScale()
const isDragging = ref(false)
const fileInputRef = ref(null)

// light or dark follows the appearance of foundation-ui (system, light, dark; tinted or true black)
const ap = useAppearance()
const T = computed(() => ({ ...appearanceTexts(locale.value), close: t.value.fileSharing?.close || 'Close' }))
// The accent of the editor (a colour of the picture, a preset, the picker, an import) is also the colour the window's Material surfaces are drawn
// from. The 'material' preset has no colours of its own: it leaves the appearance as the person set it.
let mirrored = false
watch([() => props.activePreset, () => props.vars['--fd-accent']], ([preset, hex]) => {
  if (preset === 'material') {
    if (mirrored) { mirrored = false; void ap.set({ color: 'wallpaper' }) }
    return
  }
  if (/^#[0-9a-f]{6}$/i.test(hex || '')) { mirrored = true; void ap.set({ color: 'custom', custom: hex.toLowerCase() }) }
})
const modeOptions = computed(() => [
  { value: 'system', label: T.value.system, icon: 'contrast' },
  { value: 'light', label: T.value.light, icon: 'light_mode' },
  { value: 'dark', label: T.value.dark, icon: 'dark_mode' },
])
const darkOptions = computed(() => [
  { value: 'tinted', label: T.value.darkTinted, icon: 'invert_colors' },
  { value: 'black', label: T.value.darkBlack, icon: 'brightness_1' },
])

// the colours of the picture
const toHex = (color) => `#${color.map((channel) => Math.round(channel).toString(16).padStart(2, '0')).join('')}`
const swatches = computed(() => props.wallpaperColors.map((color) => ({ value: toHex(color), label: `rgb(${color.map(Math.round).join(', ')})` })))
const selectedSwatch = computed({
  get: () => swatches.value[props.wallpaperSeeds.indexOf(props.activeWallpaperSeed)]?.value ?? '',
  set: (value) => {
    const index = swatches.value.findIndex((swatch) => swatch.value === value)
    if (index >= 0) emit('applySeed', index)
  },
})

// the four colours: the chip opens the system colour picker
const colorRows = computed(() => [
  { key: '--fd-accent', label: t.value.themeEditor.accentColor },
  { key: '--fd-accent-secondary', label: t.value.themeEditor.secondaryColor },
  { key: '--fd-bg-primary', label: t.value.themeEditor.backgroundPrimary },
  { key: '--fd-bg-secondary', label: t.value.themeEditor.backgroundSecondary },
])
// The 'material' preset keeps no colours in vars: they are the Material colours the bridge writes on <html>. The rows show what is in force.
const shown = ref({})
function readShown() {
  const style = typeof document === 'undefined' ? null : getComputedStyle(document.documentElement)
  const out = {}
  for (const row of colorRows.value) out[row.key] = props.vars[row.key] || style?.getPropertyValue(row.key).trim() || ''
  shown.value = out
}
// the bridge follows light/dark and the appearance a frame later: read again once it has written
watch([() => props.open, () => props.activePreset, () => props.vars, () => ap.state.mode, () => ap.state.dark], () => {
  void nextTick(readShown)
  setTimeout(readShown, 160)
}, { immediate: true, deep: true })
const colorInputs = {}
function setColorInput(key, el) {
  if (el) colorInputs[key] = el
  else delete colorInputs[key]
}
function pickColor(key) {
  const input = colorInputs[key]
  if (!input) return
  if (typeof input.showPicker === 'function') input.showPicker()
  else input.click()
}

// 颜色选择器节流 —— 拖拽时最多每 50ms 更新一次
let colorThrottleTimer = null
function throttledSetVar(key, value) {
  if (colorThrottleTimer) return
  emit('setVar', key, value)
  colorThrottleTimer = setTimeout(() => { colorThrottleTimer = null }, 50)
}

// card corner and text size: a slider with a step button on each side (a pad cannot drag)
const rangeRows = computed(() => [
  { key: '--fd-card-radius', label: t.value.themeEditor.cardRadius, min: 0, max: 32 },
  { key: '--fd-font-size', label: t.value.themeEditor.fontSize, min: 12, max: 18 },
])
const rangeValue = (row) => parseInt(props.vars[row.key])
const setRange = (row, value) => emit('setVar', row.key, `${Math.round(value)}px`)
const nudge = (row, delta) => setRange(row, Math.min(row.max, Math.max(row.min, rangeValue(row) + delta)))

function handleExport() {
  emit('export')
}

function handleImport() {
  emit('import')
}

function triggerFileInput() {
  fileInputRef.value?.click()
}

function handleFileSelect(e) {
  const file = e.target.files?.[0]
  if (file && file.type.startsWith('image/')) {
    emit('setWallpaper', file)
  }
  e.target.value = ''
}

function handleDrop(e) {
  isDragging.value = false
  const file = e.dataTransfer?.files?.[0]
  if (file && file.type.startsWith('image/')) {
    emit('setWallpaper', file)
  }
}
</script>

<style scoped>
.dsk-te__scrim { position: fixed; inset: 0; z-index: 9998; background: color-mix(in srgb, var(--md-sys-color-scrim) 40%, transparent); }
.dsk-te { position: fixed; top: 0; right: 0; bottom: 0; z-index: 9999; width: 460px; max-width: 100vw; box-sizing: border-box; display: flex; flex-direction: column; background: var(--md-sys-color-surface-container-low); color: var(--md-sys-color-on-surface); border-radius: var(--md-sys-shape-corner-extra-large) 0 0 var(--md-sys-shape-corner-extra-large); box-shadow: 0 8px 32px color-mix(in srgb, var(--md-sys-color-shadow) 36%, transparent); }
.dsk-te__head { display: flex; align-items: center; gap: 12px; padding: 20px 20px 12px 28px; font-size: 28px; color: var(--md-sys-color-primary-text); }
.dsk-te__title { margin: 0; flex: 1; min-width: 0; font: var(--m-headline-small); color: var(--md-sys-color-on-surface); letter-spacing: 0; }
.dsk-te__body { flex: 1; min-height: 0; overflow-y: auto; padding: 0 20px 28px; display: flex; flex-direction: column; gap: 4px; }

.dsk-te__drop { position: relative; display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 16px; border-radius: var(--md-sys-shape-corner-large); background: var(--md-sys-color-surface-container); box-shadow: inset 0 0 0 2px transparent; transition: box-shadow var(--m-spring-fast-effects-duration) var(--m-spring-fast-effects); }
.dsk-te__drop.is-dragging { box-shadow: inset 0 0 0 2px var(--md-sys-color-primary); background: var(--md-sys-color-primary-container); }
.dsk-te__preview { width: 100%; aspect-ratio: 16 / 9; object-fit: cover; border-radius: var(--md-sys-shape-corner-medium); }
.dsk-te__empty { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 20px 0 4px; font: var(--m-body-large); color: var(--md-sys-color-on-surface-variant); }
.dsk-te__empty .m-icon { font-size: 48px; color: var(--md-sys-color-primary); }
.dsk-te__drop-actions { display: flex; flex-wrap: wrap; justify-content: center; gap: 8px; }
.dsk-te__hint { margin: 0; font: var(--m-body-small); color: var(--md-sys-color-on-surface-variant); }

.dsk-te__disc { display: block; width: 32px; height: 32px; border-radius: 50%; box-shadow: 0 0 0 2px var(--md-sys-color-outline-variant); }
.dsk-te__dot { display: block; width: 32px; height: 32px; border-radius: 12px; box-shadow: 0 0 0 2px var(--md-sys-color-outline-variant); }
.dsk-te__native { position: absolute; width: 1px; height: 1px; opacity: 0; pointer-events: none; }
.dsk-te__range { display: flex; align-items: center; gap: 12px; }
.dsk-te__range .m-slider { flex: 1; min-width: 0; }

.dsk-te-fade-enter-active, .dsk-te-fade-leave-active { transition: opacity var(--m-spring-default-effects-duration) var(--m-spring-default-effects); }
.dsk-te-fade-enter-from, .dsk-te-fade-leave-to { opacity: 0; }
.dsk-te-slide-enter-active, .dsk-te-slide-leave-active { transition: transform var(--m-spring-default-spatial-duration) var(--m-spring-default-spatial), opacity var(--m-spring-default-effects-duration) var(--m-spring-default-effects); }
.dsk-te-slide-enter-from, .dsk-te-slide-leave-to { transform: translateX(100%); opacity: 0; }
</style>
