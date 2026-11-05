<script setup>
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { MLoading } from 'foundation-ui/vue'
import { useTools } from '../composables/useTools.js'

// The full-window layer while the gamepad test tool downloads or writes its files: the modal dialog puts the rest of the document out of reach, pointer and
// keyboard both (the document is blocked by the modal dialog, which makes everything outside it inert — HTML Standard 4.11.4 showModal, 6.3.1 modal dialogs
// and inert subtrees), it carries the live words of the work in a polite live region (role status — WAI-ARIA 1.2) and it leaves on every path together with
// the busy text the work owns. It is the panel's own surface drawn with the tokens of foundation-ui, not the mask of the earlier windows.
// Sources: docs/authority/source-index.tsv panel-whatwg-dialog-20261106, panel-whatwg-inert-20261106, panel-w3c-aria12-20261106.
const { controllerBusyText } = useTools()
const layer = ref(null)
watch(controllerBusyText, async (text) => {
  await nextTick()
  const el = layer.value
  if (!el) return
  if (text && !el.open) el.showModal()
  else if (!text && el.open) el.close()
}, { immediate: true, flush: 'post' })
onBeforeUnmount(() => { if (layer.value?.open) layer.value.close() })
</script>

<template>
  <Teleport to="body">
    <dialog ref="layer" class="panel-busy" aria-labelledby="panel-busy-text" :aria-busy="controllerBusyText ? 'true' : 'false'" @cancel.prevent>
      <div class="panel-busy__card" role="status" aria-live="polite">
        <MLoading size="28px" />
        <span id="panel-busy-text" class="panel-busy__text">{{ controllerBusyText }}</span>
      </div>
    </dialog>
  </Teleport>
</template>

<style scoped>
.panel-busy {
  border: 0;
  padding: 0;
  background: transparent;
  overflow: visible;
}

.panel-busy::backdrop {
  background: var(--m-modal-scrim);
  backdrop-filter: var(--m-modal-blur);
  animation: m-fade var(--m-spring-default-effects-duration) var(--m-spring-default-effects) both;
}

.panel-busy__card {
  display: flex;
  align-items: center;
  gap: 16px;
  min-width: 280px;
  max-width: min(560px, calc(100vw - 32px));
  padding: 14px 16px 14px 20px;
  border-radius: 16px;
  background: var(--md-sys-color-surface-container-highest);
  color: var(--md-sys-color-on-surface);
  font: var(--m-body-medium);
  box-shadow: 0 8px 24px color-mix(in srgb, var(--md-sys-color-shadow) 24%, transparent), 0 0 0 1px color-mix(in srgb, var(--md-sys-color-outline-variant) 70%, transparent);
  animation: m-dialog-in var(--m-spring-default-spatial-duration) var(--m-spring-default-spatial) both;
}

.panel-busy__text {
  min-width: 0;
  overflow-wrap: anywhere;
}
</style>
