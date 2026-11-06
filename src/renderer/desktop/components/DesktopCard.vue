<template>
  <div 
    class="desktop-card" 
    :class="{
      'hoverable': hoverable,
      'clickable': clickable,
      [`card-${variant}`]: variant
    }"
    @click="handleClick"
  >
    <div v-if="showHeader" class="card-header">
      <div class="card-title">
        <MIcon v-if="typeof icon === 'string'" :name="icon" class="title-icon" />
        <component v-else-if="icon" :is="icon" class="title-icon" />
        <span v-if="title">{{ title }}</span>
        <slot name="title"></slot>
      </div>
      <div class="card-actions">
        <slot name="actions"></slot>
      </div>
    </div>
    
    <div class="card-content" :class="{ 'no-padding': noPadding }">
      <slot></slot>
    </div>
    
    <div v-if="$slots.footer" class="card-footer">
      <slot name="footer"></slot>
    </div>
  </div>
</template>

<script setup>
import { MIcon } from 'foundation-ui/vue'

const props = defineProps({
  title: {
    type: String,
    default: null
  },
  icon: {
    type: [Object, String],
    default: null
  },
  variant: {
    type: String,
    default: 'default', // default, primary, secondary, success, warning, danger
    validator: (value) => ['default', 'primary', 'secondary', 'success', 'warning', 'danger'].includes(value)
  },
  hoverable: {
    type: Boolean,
    default: false
  },
  clickable: {
    type: Boolean,
    default: false
  },
  showHeader: {
    type: Boolean,
    default: true
  },
  noPadding: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['click'])

function handleClick() {
  if (props.clickable) {
    emit('click')
  }
}
</script>

<style lang="less" scoped>
// M3 卡片：surface-container；variant 用对应的 container 色，不再靠发光边框区分
.desktop-card {
  background: var(--md-sys-color-surface-container);
  color: var(--md-sys-color-on-surface);
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--d-radius);
  padding: 24px;
  transition: box-shadow var(--m-spring-default-effects-duration, 200ms) var(--m-spring-default-effects, ease),
    transform var(--m-spring-fast-spatial-duration, 200ms) var(--m-spring-fast-spatial, ease);

  &.hoverable:hover,
  &.clickable:hover {
    box-shadow: var(--d-shadow-3);
    transform: translateY(-2px);
  }

  &.clickable {
    cursor: pointer;

    &:active {
      transform: scale(0.99);
    }
  }

  &.card-primary { background: var(--md-sys-color-primary-container); color: var(--md-sys-color-on-primary-container); border-color: transparent; }
  &.card-secondary { background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); border-color: transparent; }
  &.card-success { background: var(--md-sys-color-success-container, var(--md-sys-color-secondary-container)); color: var(--md-sys-color-on-success-container, var(--md-sys-color-on-secondary-container)); border-color: transparent; }
  &.card-warning { background: var(--md-sys-color-warning-container, var(--md-sys-color-tertiary-container)); color: var(--md-sys-color-on-warning-container, var(--md-sys-color-on-tertiary-container)); border-color: transparent; }
  &.card-danger { background: var(--md-sys-color-error-container); color: var(--md-sys-color-on-error-container); border-color: transparent; }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 20px;
  }

  .card-title {
    display: flex;
    align-items: center;
    gap: 12px;
    font: 600 22px/30px var(--md-ref-typeface-brand, system-ui);

    .title-icon {
      width: 26px;
      height: 26px;
      color: var(--md-sys-color-primary);
    }
  }

  .card-actions {
    display: flex;
    gap: 12px;
  }

  .card-content {
    font: 400 16px/24px var(--md-ref-typeface-brand, system-ui);
    color: var(--md-sys-color-on-surface-variant);

    &.no-padding {
      padding: 0;
      margin: 0;
    }
  }

  .card-footer {
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--md-sys-color-outline-variant);
    display: flex;
    align-items: center;
    gap: 12px;
  }
}
</style>
