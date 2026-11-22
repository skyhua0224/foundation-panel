<template>
  <MPage :title="heading.title">
    <template v-if="page === ROUTES.PANEL_SETTINGS">
      <MSection>
        <MGroup>
          <MChoiceItem icon="language" :title="t.sidebar.language" :options="localeOptions" :model-value="localeChoice" data-panel-option="locale" :search-label="zhUi ? '搜索语言' : 'Search languages'" :empty-text="zhUi ? '没有匹配的语言' : 'No matching language'" :close-label="zhUi ? '关闭' : 'Close'" :cancel-label="zhUi ? '取消' : 'Cancel'" @update:model-value="emit('locale-change', $event)" />
        </MGroup>
      </MSection>
      <MSection :title="t.sidebar.softwareUpdate">
        <MGroup>
          <MItem icon="system_update_alt" :title="t.sidebar.updateChannel" stack>
            <template #control><MSeg block :label="t.sidebar.updateChannel" :model-value="includePrerelease" :options="channelOptions" @update:model-value="emit('channel-change', $event)" /></template>
          </MItem>
          <!-- the four sources as radio rows (the same rows as the first run): Mirror酱 carries 国内高速 and 付费 and says what it is -->
          <MItem plain icon="cloud_download" :title="src.source" />
          <MSourceList :texts="src" :source="updateSource.source.value" @pick="updateSource.setSource" />
          <template v-if="updateSource.source.value === 'mirror'">
            <MFieldItem v-model="cdkDraft" icon="key" type="password" :label="src.cdk" :supporting="updateSource.available.value ? src.cdkHint : src.unavailable" :disabled="!updateSource.available.value || updateSource.busy.value" autocomplete="off" />
            <MItem action icon="link" :title="src.getCdk" @click="actions.openUrl(MIRRORCHYAN_CDK_URL)"><template #trail><MIcon name="open_in_new" /></template></MItem>
            <MItem icon="verified_user" wrap :title="cdkResult.label" :text="cdkResult.lines.join(' ')">
              <template #trail>
                <MButton size="sm" icon="save" :disabled="!cdkDraft.trim() || !updateSource.available.value || updateSource.busy.value" @click="saveCdk">{{ src.save }}</MButton>
                <MButton variant="text" size="sm" icon="fact_check" :disabled="!updateSource.hasCdk.value || updateSource.busy.value" @click="updateSource.checkCdk()">{{ src.check }}</MButton>
                <MButton variant="text" size="sm" icon="delete" :disabled="!updateSource.hasCdk.value || updateSource.busy.value" @click="updateSource.clearCdk()">{{ src.clear }}</MButton>
              </template>
            </MItem>
          </template>
          <MItem icon="info" :title="t.sidebar.currentVersion" :text="currentVersion">
            <template #trail><MButton variant="tonal" icon="refresh" :loading="checkingUpdate" @click="checkUpdate">{{ t.sidebar.checkUpdate }}</MButton></template>
          </MItem>
          <MItem icon="schedule" :title="t.sidebar.lastCheck" :text="lastCheck.text() || t.sidebar.notChecked" />
        </MGroup>
      </MSection>
      <PanelOptions :locale="locale" />
      <MSection :title="t.sidebar.syncFeatures">
        <MGroup>
          <MItem icon="content_paste" :title="t.sidebar.clipboardSync">
            <template #trail>
              <MChip :tone="clipboardSyncEnabled ? 'ok' : clipboardSyncState === 'degraded' ? 'warn' : 'neutral'" :icon="clipboardSyncEnabled ? 'check_circle' : clipboardSyncState === 'degraded' ? 'warning' : ''">{{ clipboardSyncEnabled ? t.sidebar.connected : clipboardSyncState === 'degraded' ? t.sidebar.degraded : t.sidebar.notConnected }}</MChip>
              <MButton variant="outlined" size="sm" @click="actions.showClipboardSyncStatus">{{ t.sidebar.viewStatus }}</MButton>
            </template>
          </MItem>
        </MGroup>
      </MSection>
      <MSection :title="t.sidebar.accountSettings">
        <MGroup>
          <MItem action icon="key" :title="t.sidebar.accountCredentials" @click="emit('webui-navigate', '/password')" />
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
          <MItem action icon="tv" :title="t.sidebar.openBigScreen" :text="t.sidebar.openBigScreenHelp" data-help-row="bigScreen" @click="openDesktop" />
          <MItem action icon="lightbulb" :title="coach.rerun" @click="startCoach" />
          <MItem action icon="public" :title="remote.openText" @click="emit('open-remote')" />
          <MItem action icon="checklist" :title="startText.guideTitle" :text="startText.guideSub" @click="startCard.reopen()" />
        </MGroup>
      </MSection>
      <MSection>
        <MGroup>
          <MItem action icon="download" :title="t.sidebar.clientDownloads" @click="emit('webui-navigate', '/downloads')" />
          <MItem action icon="keyboard" :title="t.toolbar.shortcutGuide" @click="shortcutsOpen = true" />
          <MItem action icon="health_and_safety" :title="doctorTexts(locale).start" @click="emit('open-doctor')" />
          <MFeedbackRow :locale="locale" />
          <MItem action icon="bug_report" :title="t.m3e.system.troubleshooting" @click="emit('webui-navigate', '/troubleshooting')" />
          <MItem action icon="public" :title="t.sidebar.officialWebsite" @click="actions.openUrl('https://www.alkaidlab.com/')"><template #trail><MIcon name="open_in_new" /></template></MItem>
          <MItem action icon="favorite" :title="t.m3e.home.sponsor" @click="sponsorOpen = true" />
        </MGroup>
      </MSection>
      <ShortcutsSheet v-model="shortcutsOpen" />
      <MSupportSheet v-model="sponsorOpen" :title="t.m3e.home.sponsorTitle" :sections="[{ title: t.m3e.home.sponsorTitle, items: SPONSORS.map((s) => ({ id: s.id, icon: 'favorite', title: s.label, url: s.url })) }]" :close-label="t.m3e.home.done" @open="(it) => actions.openUrl(it.url)" />
    </template>
  </MPage>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { MPage, MSection, MGroup, MItem, MSourceList, MChoiceItem, MSeg, MChip, MButton, MIcon, MFieldItem, MBanner, MSupportSheet, useNotify, aboutTexts, coachTexts, guideTexts, homeTexts, remoteTexts, useGuideCard, doctorTexts, MFeedbackRow } from 'foundation-ui/vue'
import { SPONSORS } from '../config/sponsors.js'
import ShortcutsSheet from './m3e/ShortcutsSheet.vue'
import { usePanelPart } from '../composables/panelParts.js'
import { useI18n } from '../desktop/i18n/index.js'
import { DESKTOP_LOCALES, LOCALE_NAMES } from '../desktop/i18n/merge.js'
import { openBigScreenWithNotice } from '../composables/openBigScreen.js'
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
  /** what the person chose: 'system' (跟随系统, the default) or a language */
  localeChoice: { type: String, default: 'system' },
  /** the language the system resolves to ('zh' or 'en'), for the line 跟随系统 · 简体中文 */
  systemLocale: { type: String, default: 'en' },
  includePrerelease: Boolean,
  currentVersion: { type: String, default: '' },
  clipboardSyncEnabled: Boolean,
  /** the word of the sync state (clipboardStatusView.js): connected | degraded | ... */
  clipboardSyncState: { type: String, default: 'unknown' },
  isAdmin: Boolean,
  actions: { type: Object, required: true },
})
const emit = defineEmits(['theme-change', 'locale-change', 'channel-change', 'webui-navigate', 'open-guide', 'open-remote', 'open-about', 'open-doctor'])
const { t } = useI18n()
const notify = useNotify()
const openDesktop = () => openBigScreenWithNotice((m) => notify.snack(m), t.value.sidebar.openBigScreenFailed)
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
const zhUi = computed(() => String(props.locale).startsWith('zh'))
const localeOptions = computed(() => {
  const systemName = LOCALE_NAMES[props.systemLocale] || props.systemLocale
  return [{ value: 'system', label: guide.value.intro.followSystem + ' · ' + systemName }, ...DESKTOP_LOCALES.map((code) => ({ value: code, label: LOCALE_NAMES[code] }))]
})
const channelOptions = computed(() => [{ value: false, label: t.value.sidebar.stableChannel }, { value: true, label: t.value.sidebar.betaChannel }])
const lastCheck = useUpdateCheckTime()
// the update source and the Mirror酱 CDK (the CDK stays in the native side; this window only sets it and asks about it)
const updateSource = useUpdateSource()
const src = computed(() => updateSourceTexts(props.locale))
const cdkDraft = ref('')
// the state of the CDK and its sentences are the same words as in the first run (foundation-ui/update cdkView)
const cdkResult = computed(() => cdkView(updateSource.status.value, src.value, { lastValid: updateSource.lastValid.value }))
const saveCdk = async () => { if (await updateSource.saveCdk(cdkDraft.value)) cdkDraft.value = '' }
onMounted(() => { void updateSource.refresh() })
const sponsorOpen = ref(false)
// 快捷键手册 (帮助与关于, and the toolbar menu's item of the same name)
const shortcutsOpen = ref(false)
usePanelPart('shortcuts', () => (props.page === ROUTES.HELP ? (shortcutsOpen.value = true) : false))
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

