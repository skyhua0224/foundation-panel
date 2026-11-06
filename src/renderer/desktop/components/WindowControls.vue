<template>
  <div class="window-controls">
    <MIconButton
      class="control-btn minimize-btn"
      icon="remove"
      :label="t.sidebar.minimize"
      :disabled="disabled"
      @click="handleMinimize"
    />
    <MIconButton
      class="control-btn maximize-btn"
      :icon="isMaximized ? 'filter_none' : 'crop_square'"
      :label="isMaximized ? t.sidebar.restore : t.sidebar.maximize"
      :disabled="disabled"
      @click="handleToggleMaximize"
    />
    <MIconButton
      class="control-btn close-btn"
      icon="close"
      :label="t.windowActions.close"
      :disabled="disabled"
      @click="handleClose"
    />
  </div>
</template>

<script setup>
import { MIconButton } from 'foundation-ui/vue'
import { useWindowControls } from '../composables/useWindowControls'
import { useI18n } from '../i18n/index.js'

const { t } = useI18n()

const props = defineProps({
  disabled: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['minimize', 'maximize', 'close'])

const { isMaximized, minimize, maximize, close, toggleMaximize } = useWindowControls()

async function handleMinimize() {
  if (props.disabled) return
  await minimize()
  emit('minimize')
}

async function handleToggleMaximize() {
  if (props.disabled) return
  await toggleMaximize()
  emit('maximize')
}

async function handleClose() {
  if (props.disabled) return
  await close()
  emit('close')
}
</script>

<style lang="less" scoped>
.window-controls {
  display: flex;
  align-items: center;
  gap: 4px;
  -webkit-app-region: no-drag;
}

// 关闭按钮悬停时给出危险语义（error 容器），不再用固定的红
.close-btn:hover:not(:disabled) {
  background: var(--md-sys-color-error-container);
  color: var(--md-sys-color-on-error-container);
}
</style>
