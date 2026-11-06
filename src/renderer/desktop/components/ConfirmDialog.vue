<template>
  <MDialog
    ref="dialogRef"
    class="confirm-dialog"
    :model-value="state.open"
    :title="state.title || t.common.confirm"
    :icon="state.danger ? 'warning' : 'help'"
    :tone="state.danger ? 'error' : 'info'"
    @update:model-value="onModel"
  >
    <p class="confirm-message">{{ state.message }}</p>
    <template #actions>
      <MButton variant="text" size="lg" data-focusable data-focus-key="confirm-cancel" @click="reject">
        {{ state.cancelLabel || t.common.cancel }}<PadGlyph v-if="gamepadActive" action="back" size="sm" class="dialog-pad" />
      </MButton>
      <MButton :variant="state.danger ? 'danger' : 'filled'" size="lg" data-focusable data-focus-key="confirm-accept" @click="accept">
        {{ state.confirmLabel || t.common.confirm }}
      </MButton>
    </template>
  </MDialog>
</template>

<script setup>
import { computed, ref } from 'vue'
import { MButton, MDialog } from 'foundation-ui/vue'
import { acceptConfirm, confirmState, rejectConfirm } from '../composables/useConfirm.js'
import { useModalFocusScope } from '../composables/useFocusNav.js'
import { useI18n } from '../i18n/index.js'
import { gamepadActive } from '../composables/useGamepad.js'
import PadGlyph from './PadGlyph.vue'

const { t } = useI18n()

const state = confirmState
const dialogRef = ref(null)
// MDialog 的根就是 <dialog>：手柄焦点栈以它为作用域，B 键走 rejectConfirm
const dialogEl = computed(() => dialogRef.value?.$el || null)
useModalFocusScope(dialogEl, () => state.value.open)

function accept() {
  acceptConfirm()
}

function reject() {
  rejectConfirm()
}

// Esc / 点遮罩 → MDialog 把 v-model 置 false：等同于取消
function onModel(open) {
  if (!open && state.value.open) reject()
}
</script>

<style lang="less" scoped>
.confirm-message {
  margin: 0;
  font: 400 18px/28px var(--md-ref-typeface-brand, system-ui);
  color: var(--md-sys-color-on-surface-variant);
  white-space: pre-line;
}
</style>
