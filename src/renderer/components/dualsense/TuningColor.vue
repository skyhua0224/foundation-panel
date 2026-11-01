<!--
  Lightbar colour row. The profile keeps red, green and blue as three keys; this
  row edits them together through swatches, the system colour picker or a hex code.
-->
<template>
  <div class="m-item m-item--plain ds5t-color-row" :class="{ 'is-quiet': mode !== 'fixed' }">
    <span :id="labelId" class="ds5t-row-label">{{ tr('color.label') }}</span>

    <div v-if="mode === 'fixed'" class="ds5t-row-control">
      <div class="ds5t-color-tools" role="group" :aria-labelledby="labelId">
        <button
          v-for="swatch in swatches"
          :key="swatch.name"
          type="button"
          class="ds5t-dot"
          :class="{ selected: hex === swatch.hex }"
          :style="{ '--dot': swatch.hex }"
          :title="swatchLabel(swatch)"
          :aria-label="swatchLabel(swatch)"
          :aria-pressed="hex === swatch.hex"
          @click="applyHex(swatch.hex)"
        ></button>
        <label class="ds5t-dot ds5t-dot-custom" :class="{ selected: !isSwatch }" :style="{ '--dot': hex }">
          <input type="color" :value="hex" :aria-label="tr('color.picker')" @input="onColorInput" />
        </label>
        <input
          class="ds5t-hex"
          type="text"
          :value="hex"
          maxlength="7"
          spellcheck="false"
          :aria-label="tr('color.hex')"
          @change="onHexInput"
        />
      </div>
    </div>

    <div v-else class="ds5t-row-control">
      <span class="ds5t-color-note">
        <span class="ds5t-game-chip" :class="{ off: mode === 'off' }" aria-hidden="true"></span>
        {{ mode === 'off' ? tr('color.is_off') : tr('color.by_game') }}
      </span>
    </div>

    <span class="ds5t-row-reset" aria-hidden="true"></span>
    <MLightbar class="ds5t-lightbar" :mode="mode === 'off' ? 'off' : mode === 'fixed' ? 'fixed' : 'game'" :color="hex" :label="mode === 'off' ? tr('color.is_off') : mode === 'fixed' ? hex : tr('color.by_game')" />
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { MLightbar } from 'foundation-ui/vue'
import { useDualSenseText } from '../../composables/useDualSenseText.js'

const props = defineProps({
  r: { type: Number, default: 0 },
  g: { type: Number, default: 0 },
  b: { type: Number, default: 0 },
  mode: { type: String, default: 'followGame' },
})

const emit = defineEmits(['update'])
const { tr } = useDualSenseText()

const labelId = 'ds5t-lightbar-color-label'
const swatches = Object.freeze([
  { name: 'blue', hex: '#4a9eff' },
  { name: 'red', hex: '#ff5252' },
  { name: 'green', hex: '#34c759' },
  { name: 'orange', hex: '#ff9f0a' },
  { name: 'violet', hex: '#af52de' },
  { name: 'white', hex: '#ffffff' },
])

const clamp = (value) => Math.min(255, Math.max(0, Number(value) || 0))
const hex = computed(() =>
  '#' + [props.r, props.g, props.b].map((value) => clamp(value).toString(16).padStart(2, '0')).join(''))
const isSwatch = computed(() => swatches.some((swatch) => swatch.hex === hex.value))
const swatchLabel = (swatch) => tr('color.swatch', { name: tr('color.' + swatch.name) })

const applyHex = (value) => {
  emit('update', {
    r: parseInt(value.slice(1, 3), 16),
    g: parseInt(value.slice(3, 5), 16),
    b: parseInt(value.slice(5, 7), 16),
  })
}

const onColorInput = (event) => applyHex(event.target.value)

const onHexInput = (event) => {
  const value = event.target.value.trim().toLowerCase()
  const normalised = value.startsWith('#') ? value : '#' + value
  if (!/^#[0-9a-f]{6}$/.test(normalised)) {
    event.target.value = hex.value
    return
  }
  applyHex(normalised)
}
</script>

<style scoped>
.ds5t-row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) 1.5rem;
  align-items: center;
  column-gap: var(--ds5t-row-gap);
  min-height: var(--ds5t-row-height);
  padding: var(--ds5t-row-pad) 0;
}

.ds5t-row-label {
  color: var(--ds5t-text);
  font-size: 0.875rem;
  font-weight: 500;
}

.is-quiet .ds5t-row-label {
  color: var(--ds5t-muted);
}

.ds5t-row-control {
  display: flex;
  justify-content: flex-end;
  min-width: 0;
}

.ds5t-row-reset {
  width: 1.5rem;
  height: 1.5rem;
}

.ds5t-color-tools {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.ds5t-dot {
  position: relative;
  flex: none;
  width: 1.375rem;
  height: 1.375rem;
  padding: 0;
  border: 0;
  border-radius: 50%;
  background: var(--dot);
  box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.16);
  cursor: pointer;
}

.ds5t-dot.selected {
  box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.16), 0 0 0 2px var(--ds5t-card-bg), 0 0 0 4px var(--ds5t-accent);
}

.ds5t-dot:focus-visible,
.ds5t-dot-custom:focus-within {
  outline: 2px solid var(--ds5t-accent);
  outline-offset: 3px;
}

.ds5t-dot-custom {
  background:
    radial-gradient(circle at center, var(--dot) 0 38%, transparent 40%),
    conic-gradient(#ff5252, #ffd60a, #34c759, #4a9eff, #af52de, #ff5252);
}

.ds5t-dot-custom input {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  opacity: 0;
  cursor: pointer;
}

.ds5t-hex {
  width: 5.25rem;
  height: 2rem;
  padding: 0 0.6rem;
  border: 1px solid var(--ds5t-border);
  border-radius: var(--ds5t-control-radius);
  background: var(--ds5t-input-bg);
  color: var(--ds5t-text);
  font-family: Consolas, 'Courier New', monospace;
  font-size: 0.8125rem;
  text-transform: lowercase;
}

.ds5t-hex:focus-visible {
  outline: 2px solid var(--ds5t-accent);
  outline-offset: 1px;
}

.ds5t-color-note {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  color: var(--ds5t-muted);
  font-size: 0.8125rem;
}

.ds5t-game-chip {
  width: 1.375rem;
  height: 1.375rem;
  border-radius: 50%;
  background: conic-gradient(#ff5252, #ffd60a, #34c759, #4a9eff, #af52de, #ff5252);
  opacity: 0.8;
}

.ds5t-game-chip.off {
  background: transparent;
  box-shadow: inset 0 0 0 1.5px var(--ds5t-border-strong);
  opacity: 1;
}

@container ds5t-card (max-width: 26rem) {
  .ds5t-row {
    grid-template-areas:
      'label reset'
      'control control';
    grid-template-columns: minmax(0, 1fr) 1.5rem;
    row-gap: 0.4rem;
  }

  .ds5t-row-label { grid-area: label; }
  .ds5t-row-control { grid-area: control; justify-content: flex-start; }
  .ds5t-row-reset { grid-area: reset; }
}
</style>

<style>
.ds5t-color-row { flex-wrap: wrap; } .ds5t-lightbar { flex: 1 0 100%; margin-top: 8px; }
</style>
