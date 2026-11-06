<template>
  <div class="m-savebar dsk-actions">
    <MButton variant="text" icon="restart_alt" @click="emit('reset')">{{ t.settings.resetDefaults }}</MButton>
    <MButton icon="check" @click="emit('save')">{{ t.settings.saveSettings }}</MButton>
  </div>
</template>

<script setup>
import { MButton } from 'foundation-ui/vue'
import { useI18n } from '../../i18n/index.js'

const emit = defineEmits(['reset', 'save'])
const { t } = useI18n()
</script>

<style scoped>
.dsk-actions { padding-left: 8px; justify-self: center; }
</style>
