<template>
  <SettingsCard :title="t.settings.appearance">
    <SettingsRow icon="palette" :name="t.settings.themeEditorName" :description="t.settings.appearanceDesc">
      <MButton variant="tonal" icon="palette" @click="emit('open-theme-editor')">{{ t.settings.themeEditor }}</MButton>
    </SettingsRow>
  </SettingsCard>
</template>

<script setup>
import { MButton } from 'foundation-ui/vue'
import { useI18n } from '../../i18n/index.js'
import SettingsCard from './SettingsCard.vue'
import SettingsRow from './SettingsRow.vue'

const emit = defineEmits(['open-theme-editor'])
const { t } = useI18n()
</script>
