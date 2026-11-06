<template>
  <MDialog
    ref="dialogRef"
    class="osk-dialog"
    :model-value="state.open"
    :title="state.title || t.osk.title"
    icon="keyboard"
    tone="info"
    @update:model-value="onModel"
    @keydown="onKeydown"
  >
    <div class="osk-panel">
      <div class="osk-preview" :class="{ empty: !draft }">
        <span class="osk-preview-text">{{ previewText }}</span>
        <span class="osk-caret" aria-hidden="true"></span>
        <span v-if="state.maxLength" class="osk-counter">{{ draft.length }} / {{ state.maxLength }}</span>
      </div>

      <div class="osk-rows">
        <div v-for="(row, rowIndex) in rows" :key="'row-' + rowIndex" class="osk-row">
          <MButton
            v-for="key in row"
            :key="key"
            class="osk-key"
            variant="tonal"
            data-focusable
            :data-focus-key="'osk-' + key"
            @click="append(key)"
          >{{ shifted ? key.toUpperCase() : key }}</MButton>
        </div>
      </div>

      <div class="osk-row osk-actions">
        <MButton
          v-if="state.mode === 'text'"
          class="osk-key wide"
          :variant="shifted ? 'filled' : 'tonal'"
          :aria-pressed="shifted"
          icon="keyboard_arrow_up"
          data-focusable
          data-focus-key="osk-shift"
          @click="shifted = !shifted"
        >{{ t.osk.shift }}</MButton>
        <MButton
          v-if="state.mode === 'text'"
          class="osk-key wide"
          :variant="symbols ? 'filled' : 'tonal'"
          :aria-pressed="symbols"
          data-focusable
          data-focus-key="osk-symbols"
          @click="symbols = !symbols"
        >{{ symbols ? t.osk.letters : t.osk.symbols }}</MButton>
        <MButton
          v-if="state.mode === 'text'"
          class="osk-key grow"
          variant="tonal"
          data-focusable
          data-focus-key="osk-space"
          @click="append(' ')"
        >{{ t.osk.space }}</MButton>
        <MButton class="osk-key wide" variant="tonal" icon="backspace" data-focusable data-focus-key="osk-backspace" :aria-label="t.osk.clear" @click="backspace" />
        <MButton class="osk-key wide" variant="tonal" data-focusable data-focus-key="osk-clear" @click="draft = ''">{{ t.osk.clear }}</MButton>
      </div>

      <p class="osk-hint"><PadHint :text="t.osk.hint" /></p>
    </div>

    <template #actions>
      <MButton variant="text" size="lg" data-focusable data-focus-key="osk-cancel" @click="cancel">{{ t.osk.cancel }}<PadGlyph v-if="gamepadActive" action="back" size="sm" class="dialog-pad" /></MButton>
      <MButton size="lg" icon="check" data-focusable data-focus-key="osk-confirm" @click="commit">{{ t.osk.confirm }}</MButton>
    </template>
  </MDialog>
</template>

<script setup>
import { nextTick, ref, computed, watch, onUnmounted } from 'vue'
import { MButton, MDialog } from 'foundation-ui/vue'
import { cancelOsk, commitOsk, oskState } from '../composables/useOsk.js'
import { popFocusScope, pushFocusScope } from '../composables/useFocusNav.js'
import { useI18n } from '../i18n/index.js'
import { gamepadActive } from '../composables/useGamepad.js'
import PadGlyph from './PadGlyph.vue'
import PadHint from './PadHint.vue'

const { t } = useI18n()

const state = oskState
const dialogRef = ref(null)
// MDialog 的根就是 <dialog>：键盘成为顶层模态（盖在其它弹层之上），手柄焦点栈以它为作用域
const overlayRef = computed(() => dialogRef.value?.$el || null)
const draft = ref('')
const shifted = ref(false)
const symbols = ref(false)

const LETTER_ROWS = [
  ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'],
  ['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p'],
  ['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l'],
  ['z', 'x', 'c', 'v', 'b', 'n', 'm', '-', '_'],
]

const SYMBOL_ROWS = [
  ['!', '@', '#', '$', '%', '^', '&', '*', '(', ')'],
  ['-', '_', '=', '+', '[', ']', '{', '}', ';', ':'],
  ["'", '"', ',', '.', '<', '>', '/', '?', '\\', '|', '~', '`'],
]

const NUMBER_ROWS = [
  ['1', '2', '3'],
  ['4', '5', '6'],
  ['7', '8', '9'],
  ['0'],
]

/** 预览框只显示尾部，长文本从左侧截断，光标始终可见。 */
const PREVIEW_TAIL = 60
const previewText = computed(() => {
  if (!draft.value) return state.value.placeholder || t.value.osk.placeholder
  if (draft.value.length <= PREVIEW_TAIL) return draft.value
  return `…${draft.value.slice(-PREVIEW_TAIL)}`
})

const rows = computed(() => {
  if (state.value.mode === 'number') return NUMBER_ROWS
  return symbols.value ? SYMBOL_ROWS : LETTER_ROWS
})

let disposeScope = null

watch(
  () => state.value.open,
  async (open) => {
    if (open) {
      draft.value = state.value.value
      shifted.value = false
      symbols.value = false
      await nextTick()
      if (overlayRef.value) disposeScope = pushFocusScope(overlayRef.value)
    } else {
      disposeScope?.()
      disposeScope = null
    }
  },
  { immediate: true }
)

onUnmounted(() => {
  disposeScope?.()
})

function atLimit() {
  return state.value.maxLength > 0 && draft.value.length >= state.value.maxLength
}

function append(character) {
  if (atLimit()) return
  const value = shifted.value ? character.toUpperCase() : character
  draft.value += value
  // Shift 只作用于一个字符，和手机键盘一致
  if (shifted.value) shifted.value = false
}

function backspace() {
  draft.value = draft.value.slice(0, -1)
}

function commit() {
  commitOsk(draft.value)
}

function cancel() {
  cancelOsk()
}

// 点遮罩 / 原生 Esc → 取消
function onModel(open) {
  if (!open && state.value.open) cancel()
}

/** 物理键盘依然可用：串流用手柄，本机调试用键盘。 */
function onKeydown(event) {
  if (event.key === 'Escape') {
    event.preventDefault()
    cancel()
    return
  }
  if (event.key === 'Enter') {
    event.preventDefault()
    commit()
    return
  }
  if (event.key === 'Backspace') {
    event.preventDefault()
    backspace()
    return
  }
  if (event.key.length === 1 && !event.ctrlKey && !event.altKey && !event.metaKey) {
    if (state.value.mode === 'number' && !/[0-9]/.test(event.key)) return
    event.preventDefault()
    if (!atLimit()) draft.value += event.key
  }
}
</script>
<style lang="less" scoped>
.osk-dialog {
  max-width: min(1040px, calc(100vw - 64px));
}

.osk-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

// 预览框：outlined 文本框的外观，光标闪烁
.osk-preview {
  position: relative;
  display: flex;
  align-items: center;
  min-height: 72px;
  padding: 12px 24px;
  border: 2px solid var(--md-sys-color-primary);
  border-radius: 20px;
  background: var(--md-sys-color-surface-container-highest);
  font: 500 28px/36px var(--md-ref-typeface-brand, system-ui);
  color: var(--md-sys-color-on-surface);
  overflow: hidden;

  &.empty .osk-preview-text {
    color: var(--md-sys-color-on-surface-variant);
    opacity: 0.7;
  }

  .osk-preview-text {
    white-space: pre;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .osk-caret {
    flex: none;
    width: 3px;
    height: 34px;
    margin-left: 3px;
    border-radius: 2px;
    background: var(--md-sys-color-primary);
    animation: osk-blink 1.1s steps(1) infinite;
  }

  .osk-counter {
    margin-left: auto;
    padding-left: 16px;
    font: 500 15px/20px var(--md-ref-typeface-brand, system-ui);
    color: var(--md-sys-color-on-surface-variant);
  }
}

@keyframes osk-blink {
  50% {
    opacity: 0;
  }
}

.osk-rows {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.osk-row {
  display: flex;
  justify-content: center;
  gap: 10px;
}

// 键：56 高、等宽；特殊键更宽。覆盖 MButton 的横向 padding 才能排满一行
.osk-key {
  flex: 0 0 76px;
  min-width: 0;
  height: 64px;
  padding: 0;
  justify-content: center;
  border-radius: 20px;
  font: 600 24px/32px var(--md-ref-typeface-brand, system-ui);

  &.wide {
    flex-basis: 140px;
    font-size: 18px;
  }

  &.grow {
    flex: 1 1 auto;
    font-size: 18px;
  }
}

.osk-hint {
  margin: 4px 0 0;
  text-align: center;
  // the keys of the pad in the sentence are drawn at the size of the text: 10 feet away it is read at 17px, not 14
  font: 500 17px/26px var(--md-ref-typeface-brand, system-ui);
  color: var(--md-sys-color-on-surface-variant);
}
</style>
