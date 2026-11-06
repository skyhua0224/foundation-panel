<template>
  <span
    class="pad-glyph"
    :class="[`size-${size}`, `type-${resolvedType}`, { 'is-pair': glyphs.length > 1 }]"
    role="img"
    :aria-label="words"
    :data-pad-action="action || undefined"
    :data-pad-type="resolvedType"
  >
    <template v-for="glyph in glyphs" :key="`${resolvedType}-${glyph.button}`">
      <!-- 面键：圆盘。Xbox / Nintendo 画字，PlayStation 画 ✕ ○ □ △ 的形状，认不出的手柄画四个点里亮着的那一个 -->
      <svg v-if="glyph.kind === 'face'" class="pg pg-face" :class="`tone-${glyph.tone}`" viewBox="0 0 48 48" aria-hidden="true">
        <circle class="pg-disc" cx="24" cy="24" r="23" />
        <g class="pg-mark">
          <template v-if="glyph.shape === 'cross'"><path d="M16.5 16.5 L31.5 31.5 M31.5 16.5 L16.5 31.5" /></template>
          <template v-else-if="glyph.shape === 'circle'"><circle cx="24" cy="24" r="9.5" /></template>
          <template v-else-if="glyph.shape === 'square'"><rect x="15" y="15" width="18" height="18" rx="2.5" /></template>
          <template v-else-if="glyph.shape === 'triangle'"><path d="M24 14.5 L34.5 32.5 H13.5 Z" /></template>
          <template v-else-if="glyph.shape">
            <circle v-for="dot in DOTS" :key="dot.position" :class="{ 'is-on': dot.position === glyph.shape }" :cx="dot.x" :cy="dot.y" :r="dot.position === glyph.shape ? 5.6 : 3.4" />
          </template>
          <text v-else x="24" y="32" text-anchor="middle">{{ glyph.text }}</text>
        </g>
      </svg>

      <!-- 菜单键：圆盘里三条线（Nintendo 是加号） -->
      <svg v-else-if="glyph.kind === 'menu'" class="pg pg-face pg-menu" viewBox="0 0 48 48" aria-hidden="true">
        <circle class="pg-disc" cx="24" cy="24" r="23" />
        <g class="pg-mark">
          <path v-if="resolvedType === 'nintendo'" d="M24 14 V34 M14 24 H34" />
          <path v-else d="M15 17.5 H33 M15 24 H33 M15 30.5 H33" />
        </g>
      </svg>

      <!-- 肩键、扳机、摇杆键：带字的键帽，形状各不相同（肩键上圆下平，扳机反过来，摇杆键是圆） -->
      <span v-else class="pg pg-key" :class="[`pg-${glyph.kind}`, `tone-${glyph.tone}`]" aria-hidden="true">{{ glyph.text }}</span>
    </template>
  </span>
</template>

<script setup>
import { computed } from 'vue'
import { glyphsFor, glyphOf, wordsFor } from '../composables/padGlyphs.js'
import { padType } from '../composables/padType.js'

const props = defineProps({
  /** 动作 id（confirm、back、pages…，见 padGlyphs 的 PAD_ACTION_BUTTONS） */
  action: { type: String, default: '' },
  /** 或者直接一颗键的标准名（A B X Y LB RB LT RT L3 R3 Start Back） */
  button: { type: String, default: '' },
  /** 'md' 条里的大小，'sm' 随句子的字号，'lg' 对话框与大标题 */
  size: { type: String, default: 'md' },
  /** 不给就跟随当前手柄类型（padType），换手柄时即时变 */
  type: { type: String, default: '' },
})

const resolvedType = computed(() => props.type || padType.value)
const glyphs = computed(() => (props.button ? [glyphOf(props.button, resolvedType.value)] : glyphsFor(props.action, resolvedType.value)))
const words = computed(() => (props.button ? glyphs.value.map((g) => g.text).join('/') : wordsFor(props.action, resolvedType.value)))

// generic 手柄面键的四个位置（viewBox 48）
const DOTS = [
  { position: 'north', x: 24, y: 12 },
  { position: 'east', x: 36, y: 24 },
  { position: 'south', x: 24, y: 36 },
  { position: 'west', x: 12, y: 24 },
]
</script>

<style lang="less" scoped>
// 按键图标：用 md-sys 颜色画的功能色圆盘与键帽，不是任何品牌的图。--pg 是面键的直径：
//   md  条里（沙发十英尺，44px）   sm  随句子（1.5em）   lg  对话框（56px）
.pad-glyph {
  --pg: 44px;
  display: inline-flex;
  align-items: center;
  gap: calc(var(--pg) * 0.12);
  vertical-align: middle;
  flex: none;
  line-height: 1;
  &.size-sm { --pg: 1.6em; }
  &.size-lg { --pg: 56px; }
}

.pg {
  flex: none;
  display: inline-flex;
  animation: pg-in var(--m-spring-fast-spatial-duration, 220ms) var(--m-spring-fast-spatial, ease-out) both;
}

.pg-face {
  width: var(--pg);
  height: var(--pg);
  overflow: visible;

  .pg-disc {
    fill: var(--pg-fill, var(--md-sys-color-surface-container-highest));
    stroke: color-mix(in srgb, var(--pg-ink, var(--md-sys-color-on-surface)) 20%, transparent);
    stroke-width: 1.5;
  }
  .pg-mark {
    fill: none;
    stroke: var(--pg-ink, var(--md-sys-color-on-surface));
    stroke-width: 3.6;
    stroke-linecap: round;
    stroke-linejoin: round;
    text { fill: var(--pg-ink, var(--md-sys-color-on-surface)); stroke: none; font: 800 25px/1 var(--md-ref-typeface-brand, system-ui); }
    // generic：四个位置，亮着的实心，其余只是环
    circle { stroke-width: 2.4; }
    circle.is-on { fill: var(--pg-ink, var(--md-sys-color-on-surface)); }
  }
  &.pg-menu .pg-mark { stroke-width: 3.2; }
}

.pg-key {
  align-items: center;
  justify-content: center;
  min-width: calc(var(--pg) * 1.12);
  height: calc(var(--pg) * 0.72);
  padding: 0 calc(var(--pg) * 0.2);
  background: var(--md-sys-color-surface-container-highest);
  color: var(--md-sys-color-on-surface);
  box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--md-sys-color-on-surface) 20%, transparent);
  font: 800 calc(var(--pg) * 0.36) / 1 var(--md-ref-typeface-brand, system-ui);
  letter-spacing: 0.02em;
  white-space: nowrap;
  // 肩键：上缘圆起；扳机：下缘圆起（扳机在肩键下面往手心弯）
  &.pg-bumper { border-radius: calc(var(--pg) * 0.36) calc(var(--pg) * 0.36) calc(var(--pg) * 0.1) calc(var(--pg) * 0.1); }
  &.pg-trigger { border-radius: calc(var(--pg) * 0.1) calc(var(--pg) * 0.1) calc(var(--pg) * 0.36) calc(var(--pg) * 0.36); height: calc(var(--pg) * 0.8); }
  &.pg-stick { border-radius: 50%; min-width: var(--pg); height: var(--pg); padding: 0; }
  &.pg-view { border-radius: calc(var(--pg) * 0.36); }
}

// 色位（功能色，跟随主题）
.tone-green { --pg-fill: var(--md-sys-color-success-container, var(--md-sys-color-secondary-container)); --pg-ink: var(--md-sys-color-on-success-container, var(--md-sys-color-on-secondary-container)); }
.tone-red { --pg-fill: var(--md-sys-color-error-container); --pg-ink: var(--md-sys-color-on-error-container); }
.tone-blue { --pg-fill: var(--md-sys-color-primary-container); --pg-ink: var(--md-sys-color-on-primary-container); }
.tone-yellow { --pg-fill: var(--md-sys-color-warning-container, var(--md-sys-color-tertiary-container)); --pg-ink: var(--md-sys-color-on-warning-container, var(--md-sys-color-on-tertiary-container)); }
// 粉（□）：用 tertiary 容器与它自己的 on 色，这一对由主题保证对比度（两色混出来的墨色在深色下会和底色糊在一起）
.tone-pink { --pg-fill: var(--md-sys-color-tertiary-container); --pg-ink: var(--md-sys-color-on-tertiary-container); }
.pg-key.tone-neutral, .pg-face.tone-neutral { --pg-fill: var(--md-sys-color-surface-container-highest); --pg-ink: var(--md-sys-color-on-surface); }
.pg-key { background: var(--pg-fill, var(--md-sys-color-surface-container-highest)); color: var(--pg-ink, var(--md-sys-color-on-surface)); }

@keyframes pg-in {
  from { opacity: 0; transform: scale(0.6); }
  to { opacity: 1; transform: scale(1); }
}
// 减少动效：跟随系统时看 prefers-reduced-motion，设置里选「完整」就不降，选「减少」一律降（foundation-ui 的 data-motion）
@media (prefers-reduced-motion: reduce) {
  :global(html:not([data-motion='full'])) .pg { animation: none; }
}
:global(html[data-motion='reduced']) .pg { animation: none; }
</style>
