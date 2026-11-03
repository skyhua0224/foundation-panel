<template>
  <MPage :title="heading.title">
    <template v-if="page === ROUTES.PANEL_SETTINGS">
      <MSection>
        <MGroup>
          <MItem action icon="palette" :title="locale && String(locale).startsWith('zh') ? '个性化' : 'Personalise'" @click="emit('open-appearance')" />
        </MGroup>
      </MSection>
      <MSection :title="t.sidebar.language">
        <MGroup>
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
          <MItem icon="cloud_download" :title="src.source" :text="src.sourceNote" stack>
            <template #control><MSeg block :label="src.source" :model-value="updateSource.source.value" :options="sourceOptions" @update:model-value="updateSource.setSource" /></template>
          </MItem>
          <template v-if="updateSource.source.value === 'mirror'">
            <MFieldItem v-model="cdkDraft" icon="key" type="password" :label="src.cdk" :supporting="updateSource.available.value ? src.cdkHint : src.unavailable" :disabled="!updateSource.available.value || updateSource.busy.value" autocomplete="off" />
            <MItem action icon="link" :title="src.getCdk" @click="actions.openUrl(MIRRORCHYAN_CDK_URL)"><template #trail><MIcon name="open_in_new" /></template></MItem>
            <MItem icon="verified_user" :title="cdkResult.label" :text="cdkResult.lines.join(' ')">
              <template #trail>
                <MButton variant="tonal" size="sm" icon="save" :disabled="!cdkDraft.trim() || !updateSource.available.value || updateSource.busy.value" @click="saveCdk">{{ src.save }}</MButton>
                <MButton variant="text" size="sm" icon="fact_check" :disabled="!updateSource.hasCdk.value || updateSource.busy.value" @click="updateSource.checkCdk()">{{ src.check }}</MButton>
                <MButton variant="text" size="sm" icon="delete" :disabled="!updateSource.hasCdk.value || updateSource.busy.value" @click="updateSource.clearCdk()">{{ src.clear }}</MButton>
              </template>
            </MItem>
          </template>
          <MFooter>{{ sourceHint }} {{ src.verified }}</MFooter>
          <MItem icon="verified" :title="t.sidebar.currentChannel" :text="includePrerelease ? t.sidebar.betaChannelHint : t.sidebar.stableChannelHint" />
          <MItem icon="info" :title="t.sidebar.currentVersion" :text="currentVersion">
            <template #trail><MButton variant="tonal" icon="refresh" :loading="checkingUpdate" @click="checkUpdate">{{ t.sidebar.checkUpdate }}</MButton></template>
          </MItem>
          <MItem icon="schedule" :title="t.sidebar.lastCheck" :text="lastCheck.text() || t.sidebar.notChecked" />
        </MGroup>
      </MSection>
      <PanelOptions :locale="locale" />
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
      <MSection>
        <MGroup>
          <MItem action icon="info" :title="aboutText.title" :text="'Foundation Sunshine ' + currentVersion" @click="emit('open-about')" />
          <MItem action icon="rocket_launch" :title="guide.guide" :text="guide.helpRow" @click="emit('open-guide')" />
          <MItem action icon="lightbulb" :title="coach.rerun" :text="coach.rerunText" @click="startCoach" />
          <MItem action icon="public" :title="remote.openText" :text="remote.lead" @click="emit('open-remote')" />
          <MItem action icon="checklist" :title="startText.guideTitle" :text="startText.guideSub" @click="startCard.reopen()" />
        </MGroup>
      </MSection>
      <MSection>
        <MGroup>
          <MItem action icon="download" :title="t.sidebar.clientDownloads" @click="emit('webui-navigate', '/downloads')" />
          <MItem action icon="bug_report" :title="t.sidebar.diagnostics" @click="emit('webui-navigate', '/troubleshooting')" />
          <MItem action icon="public" :title="t.sidebar.officialWebsite" @click="actions.openUrl('https://www.alkaidlab.com/')"><template #trail><MIcon name="open_in_new" /></template></MItem>
          <MItem action icon="favorite" :title="t.m3e.home.sponsor" @click="sponsorOpen = true" />
        </MGroup>
      </MSection>
      <MSupportSheet v-model="sponsorOpen" :title="t.m3e.home.sponsorTitle" :sections="[{ title: t.m3e.home.sponsorTitle, items: SPONSORS.map((s) => ({ id: s.id, icon: 'favorite', title: s.label, url: s.url })) }]" :close-label="t.m3e.home.done" @open="(it) => actions.openUrl(it.url)" />
    </template>
  </MPage>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { MPage, MSection, MGroup, MItem, MSeg, MChip, MButton, MIcon, MFooter, MFieldItem, MBanner, MSupportSheet, aboutTexts, coachTexts, guideTexts, homeTexts, remoteTexts, useGuideCard } from 'foundation-ui/vue'
import { SPONSORS } from '../config/sponsors.js'
import { useI18n } from '../desktop/i18n/index.js'
import PanelOptions from './m3e/PanelOptions.vue'
import { ROUTES } from '../composables/useRouter.js'
import { useUpdateCheckTime } from '../composables/useUpdateCheckTime.js'
import { useUpdateSource } from '../composables/useUpdateSource.js'
import { MIRRORCHYAN_CDK_URL, UPDATE_SOURCES, cdkView, updateSourceTexts } from 'foundation-ui/update'
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
const emit = defineEmits(['theme-change', 'locale-change', 'channel-change', 'webui-navigate', 'open-guide', 'open-remote', 'open-about', 'open-appearance'])
const { t } = useI18n()
// The guided first run can be opened again from here; its texts come with the package (zh and en)
const guide = computed(() => guideTexts(props.locale))
const remote = computed(() => remoteTexts(props.locale))
const coach = computed(() => coachTexts(props.locale))
const startCoach = () => window.dispatchEvent(new CustomEvent('foundation-coach-start'))
const aboutText = computed(() => aboutTexts(props.locale))
// the get started card of the overview comes back from here
const startText = computed(() => homeTexts(props.locale))
const startCard = useGuideCard('panel-home', () => [])
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
const lastCheck = useUpdateCheckTime()
// the update source and the Mirror酱 CDK (the CDK stays in the native side; this window only sets it and asks about it)
const updateSource = useUpdateSource()
const src = computed(() => updateSourceTexts(props.locale))
const sourceOptions = computed(() => UPDATE_SOURCES.map((value) => ({ value, label: src.value.sources[value] })))
const cdkDraft = ref('')
// the state of the CDK and its sentences are the same words as in the first run (foundation-ui/update cdkView)
const cdkResult = computed(() => cdkView(updateSource.status.value, src.value, { lastValid: updateSource.lastValid.value }))
const sourceHint = computed(() => (updateSource.source.value === 'auto' ? src.value.autoHint + '.' : updateSource.source.value === 'mirror' ? src.value.mirrorTip : ''))
const saveCdk = async () => { if (await updateSource.saveCdk(cdkDraft.value)) cdkDraft.value = '' }
onMounted(() => { void updateSource.refresh() })
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

