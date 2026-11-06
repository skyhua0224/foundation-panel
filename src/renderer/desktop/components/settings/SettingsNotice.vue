<template>
  <Transition name="dsk-notice">
    <MNotice v-if="notice" class="dsk-notice" :tone="tone" :icon="icon">{{ notice.message }}</MNotice>
  </Transition>
</template>

<script setup>
import { computed } from 'vue'
import { MNotice } from 'foundation-ui/vue'

// The result of the last save, reset or update check, in the tone it has. It goes by itself (the state clears it).
const TONES = {
  success: { tone: 'info', icon: 'check_circle' },
  info: { tone: 'info', icon: 'info' },
  warning: { tone: 'warn', icon: 'warning' },
  error: { tone: 'error', icon: 'error' },
}

const props = defineProps({
  notice: { type: Object, default: null },
})

const tone = computed(() => (TONES[props.notice?.type] || TONES.info).tone)
const icon = computed(() => (TONES[props.notice?.type] || TONES.info).icon)
</script>

<style scoped>
.dsk-notice-enter-active, .dsk-notice-leave-active { transition: opacity var(--m-spring-default-effects-duration) var(--m-spring-default-effects), transform var(--m-spring-default-spatial-duration) var(--m-spring-default-spatial); }
.dsk-notice-enter-from, .dsk-notice-leave-to { opacity: 0; transform: translateY(8px); }
</style>
