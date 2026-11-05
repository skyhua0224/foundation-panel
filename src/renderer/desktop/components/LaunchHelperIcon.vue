<template>
  <span class="launch-helper-icon" :style="{ '--helper-icon-size': `${size}px` }">
    <MIcon :name="iconName" />
  </span>
</template>

<script setup>
import { computed } from 'vue'
import { MIcon } from 'foundation-ui/vue'

const props = defineProps({
  templateId: { type: String, default: '' },
  size: { type: Number, default: 18 },
})

const helperIconMap = {
  'controller-mapper': 'key',
  'locale-emulator': 'language',
  translator: 'description',
  'game-trainer': 'build',
  'rtss-fps-limiter': 'speed',
  custom: 'settings',
}

const iconName = computed(() => helperIconMap[props.templateId] || 'settings')
</script>

<style lang="less" scoped>
.launch-helper-icon {
  width: var(--helper-icon-size);
  height: var(--helper-icon-size);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: currentColor;

  :deep(svg) {
    width: 100%;
    height: 100%;
  }
}
</style>
