<template>
  <section class="dsk-about">
    <div class="dsk-about__logo"><img :src="sunshineIcon" alt="" /></div>
    <div class="dsk-about__info">
      <div class="dsk-about__name">Foundation Desktop</div>
      <div class="dsk-about__version">{{ t.settings.version }} {{ appVersion }}</div>
      <div class="dsk-about__links">
        <MButton variant="text" size="sm" icon="code" @click="emit('open-link', 'github')">GitHub</MButton>
        <MButton variant="text" size="sm" icon="menu_book" @click="emit('open-link', 'docs')">{{ t.settings.docs }}</MButton>
        <MButton variant="text" size="sm" icon="forum" @click="emit('open-link', 'discord')">Discord</MButton>
      </div>
    </div>
    <MButton variant="tonal" icon="update" :loading="checking" @click="emit('check-update')">
      {{ checking ? t.settings.checking : t.settings.checkUpdate }}
    </MButton>
  </section>
</template>

<script setup>
import { MButton } from 'foundation-ui/vue'
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
.dsk-about { display: flex; align-items: center; gap: 20px; padding: 20px 24px; border-radius: var(--md-sys-shape-corner-extra-large); background: var(--md-sys-color-primary-container); color: var(--md-sys-color-on-primary-container); }
.dsk-about__logo { flex: none; width: 64px; height: 64px; display: grid; place-items: center; }
.dsk-about__logo img { width: 100%; height: 100%; object-fit: contain; }
.dsk-about__info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
.dsk-about__name { font: var(--m-title-large); }
.dsk-about__version { font: var(--m-body-large); opacity: .8; }
.dsk-about__links { display: flex; flex-wrap: wrap; gap: 4px; margin: 4px 0 0 -12px; }
.dsk-about__links .m-btn--text { --m-btn-fg: var(--md-sys-color-on-primary-container); }
</style>
