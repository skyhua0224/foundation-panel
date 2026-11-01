<template>
  <MPage :title="heading.title">
    <template v-if="page === ROUTES.PANEL_SETTINGS">
      <MSection :title="t.sidebar.appearanceLanguage">
        <MGroup>
          <MItem icon="palette" :title="t.sidebar.theme" stack>
            <template #control><MSeg block :label="t.sidebar.theme" :model-value="themeMode" :options="themeOptions" @update:model-value="emit('theme-change', $event)" /></template>
          </MItem>
          <MItem icon="language" :title="t.sidebar.language" stack>
            <template #control><MSeg block :label="t.sidebar.language" :model-value="locale" :options="localeOptions" @update:model-value="emit('locale-change', $event)" /></template>
          </MItem>
        </MGroup>
      </MSection>
      <MSection :title="t.sidebar.softwareUpdate">
        <MGroup>
          <MItem icon="system_update_alt" :title="t.sidebar.updateChannel" stack>
            <template #control><MSeg block :label="t.sidebar.updateChannel" :model-value="includePrerelease" :options="channelOptions" @update:model-value="emit('channel-change', $event)" /></template>
          </MItem>
          <MItem icon="info" :title="t.sidebar.currentVersion" :text="currentVersion">
            <template #trail><MButton variant="tonal" icon="refresh" :loading="checkingUpdate" @click="checkUpdate">{{ t.sidebar.checkUpdate }}</MButton></template>
          </MItem>
        </MGroup>
      </MSection>
      <MSection :title="t.sidebar.syncFeatures">
        <MGroup>
          <MItem icon="content_paste" :title="t.sidebar.clipboardSync" :text="t.sidebar.clipboardDescription">
            <template #trail>
              <MChip :tone="clipboardSyncEnabled ? 'ok' : 'neutral'" :icon="clipboardSyncEnabled ? 'check_circle' : ''">{{ clipboardSyncEnabled ? t.sidebar.connected : t.sidebar.notConnected }}</MChip>
              <MButton variant="outlined" size="sm" @click="actions.showClipboardSyncStatus">{{ t.sidebar.viewStatus }}</MButton>
            </template>
          </MItem>
        </MGroup>
      </MSection>
      <MSection :title="t.sidebar.accountSettings">
        <MGroup>
          <MItem action icon="key" :title="t.sidebar.accountCredentials" :text="t.sidebar.accountDescription" @click="emit('webui-navigate', '/password')" />
          <MItem action icon="logout" :title="t.sidebar.logout" :text="t.sidebar.logoutDescription" @click="emit('webui-navigate', '/password#logout')" />
        </MGroup>
      </MSection>
      <MSection :title="t.sidebar.maintenance">
        <MGroup>
          <MItem icon="cleaning_services" :title="t.sidebar.coverCache" :text="t.sidebar.coverCacheDescription">
            <template #trail><MButton variant="tonal" size="sm" @click="actions.cleanupCovers">{{ t.sidebar.clean }}</MButton></template>
          </MItem>
          <MItem v-if="!isAdmin" icon="shield" :title="t.sidebar.restartAsAdmin">
            <template #trail><MButton variant="outlined" size="sm" @click="actions.restartAsAdmin">{{ t.sidebar.restartAsAdmin }}</MButton></template>
          </MItem>
        </MGroup>
      </MSection>
    </template>
    <template v-else>
      <MPromo :title="t.sidebar.clientDownloads" text="Moonlight / VoidLink" icon="download" @click="emit('webui-navigate', '/downloads')" />
      <MPromo :title="t.m3e.home.sponsor" :text="t.m3e.home.sponsorTitle" icon="favorite" tone="primary" @click="sponsorOpen = true" />
      <MSection>
        <MGroup>
          <MItem action icon="public" :title="t.sidebar.officialWebsite" text="AlkaidLab" @click="actions.openUrl('https://www.alkaidlab.com/')"><template #trail><MIcon name="open_in_new" /></template></MItem>
          <MItem action icon="bug_report" :title="t.sidebar.diagnostics" :text="t.sidebar.diagnosticsDescription" @click="emit('webui-navigate', '/troubleshooting')" />
        </MGroup>
        <MFooter>Foundation Sunshine · {{ currentVersion }}</MFooter>
      </MSection>
      <MSheet v-model="sponsorOpen" :title="t.m3e.home.sponsorTitle" side="end" :close-label="t.m3e.home.done">
        <MGroup>
          <MItem v-for="s in SPONSORS" :key="s.id" action icon="favorite" :title="s.label" @click="actions.openUrl(s.url)"><template #trail><MIcon name="open_in_new" /></template></MItem>
        </MGroup>
      </MSheet>
    </template>
  </MPage>
</template>

<script setup>
import { computed, ref } from 'vue'
import { MPage, MSection, MGroup, MItem, MSeg, MChip, MButton, MIcon, MPromo, MFooter, MSheet } from 'foundation-ui/vue'
import { SPONSORS } from '../config/sponsors.js'
import { useI18n } from '../desktop/i18n/index.js'
import { ROUTES } from '../composables/useRouter.js'
const props = defineProps({
  page: { type: String, required: true },
  tools: { type: Array, default: () => [] },
  themeMode: { type: String, default: 'system' },
  locale: { type: String, default: 'en' },
  includePrerelease: Boolean,
  currentVersion: { type: String, default: '' },
  clipboardSyncEnabled: Boolean,
  isAdmin: Boolean,
  actions: { type: Object, required: true },
})
const emit = defineEmits(['theme-change', 'locale-change', 'channel-change', 'webui-navigate'])
const { t } = useI18n()
const heading = computed(() => {
  const s = t.value.sidebar
  if (props.page === ROUTES.TOOLBOX) return { title: s.toolbox, description: s.toolboxDescription }
  if (props.page === ROUTES.PANEL_SETTINGS) return { title: s.panelSettings, description: s.panelSettingsDescription }
  return { title: s.helpAbout, description: 'Foundation Sunshine' }
})
const themeOptions = computed(() => [
  { value: 'system', label: t.value.sidebar.systemTheme, icon: 'tonality' }, { value: 'light', label: t.value.sidebar.lightMode, icon: 'light_mode' }, { value: 'dark', label: t.value.sidebar.darkMode, icon: 'dark_mode' },
])
const localeOptions = [{ value: 'zh', label: '简体中文' }, { value: 'en', label: 'English' }]
const channelOptions = computed(() => [{ value: false, label: t.value.sidebar.stableChannel }, { value: true, label: t.value.sidebar.betaChannel }])
const sponsorOpen = ref(false)
const checkingUpdate = ref(false)
const checkUpdate = async () => {
  if (checkingUpdate.value) return
  checkingUpdate.value = true
  try {
    await props.actions.handleCheckForUpdates()
  } finally {
    checkingUpdate.value = false
  }
}
</script>

