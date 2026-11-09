<template>
  <MSection>
    <MGroup role="list">
      <MItem :title="'Foundation Desktop'" :text="`${t.settings.version} ${appVersion}`">
        <template #lead><img class="dsk-about__logo" :src="sunshineIcon" alt="" /></template>
        <template #trail>
          <MButton variant="tonal" icon="update" :loading="checking" @click="emit('check-update')">
            {{ checking ? t.settings.checking : t.settings.checkUpdate }}
          </MButton>
        </template>
      </MItem>
      <MItem action icon="code" title="GitHub" @click="emit('open-link', 'github')" />
      <MItem action icon="menu_book" :title="t.settings.docs" @click="emit('open-link', 'docs')" />
      <MItem action icon="forum" title="Discord" @click="emit('open-link', 'discord')" />
    </MGroup>
  </MSection>
</template>

<script setup>
import { MButton, MGroup, MItem, MSection } from 'foundation-ui/vue'
import sunshineIcon from '../../../../assets/sunshine.ico'
import { useI18n } from '../../i18n/index.js'

defineProps({
  appVersion: { type: String, required: true },
  checking: { type: Boolean, required: true },
})

const emit = defineEmits(['open-link', 'check-update'])
const { t } = useI18n()
</script>

<style scoped>
.dsk-about__logo { width: 40px; height: 40px; object-fit: contain; }
</style>
