<template>
  <MOnboardWindow :mode="onboardMode" :label="accountTexts?.title || ''" @close="closeGuide">
  <div class="panel-guide">
    <MLoading v-if="state.account.exists === null || !entry" class="panel-guide__loading" />
    <MGuideFlow
      v-else
      :steps="steps"
      :state="flowState"
      :callbacks="callbacks"
      :locale="locale"
      :version="version"
      :mode="entry.mode"
      :restart="!!entry.restart"
      :host-progress="hostProgress"
      :texts="accountTexts"
      :storage-key="GUIDE_STORAGE_KEY"
      :qr="qrMatrix"
      :required="state.account.exists ? [] : ['account']"
      @close="closeGuide"
      @finish="closeGuide"
    >
    </MGuideFlow>
  </div>
  </MOnboardWindow>
</template>

<script setup>
import { computed, onMounted, ref, watch } from 'vue'
import QRCode from 'qrcode'
import { invoke } from '@tauri-apps/api/core'
import { MGuideFlow, MLoading, MOnboardWindow, buildGuideSteps, clearProgress, guideDeviceId, guideEntry, readProgress, upgradeSkips, useTelemetryConsent } from 'foundation-ui/vue'
import { supportedLocales, setLocaleChoice as setWelcomeChoice, localeChoice, systemLocale, useWelcomeI18n } from '../../i18n/index.js'
import { useI18n as useDesktopI18n } from '../desktop/i18n/index.js'
import { telemetryApi } from '../composables/telemetryApi.js'
import { openExternalUrl, sunshine } from '../tauri-adapter.js'
import { GUIDE_DISMISSED_KEY, GUIDE_STORAGE_KEY, createGuideData } from '../composables/guideData.js'
import { consumeStartGuide, guideOnLaunch } from '../composables/startGuide.js'
import { panelVersion } from '../composables/panelVersion.js'
import { useHostReach } from '../composables/useHostReach.js'
import { useUpdateSource } from '../composables/useUpdateSource.js'

// The guided first run of the panel. It opens by itself on a first run, when the host serves its account page (no account yet), and from
// the help page. The account step is part of the flow only while the host has no account.
const emit = defineEmits(['close', 'webui-navigate', 'open-doctor', 'open-privacy'])
const { t, locale } = useWelcomeI18n()

// The language is chosen on the first screen of the flow (跟随系统 by default) and applies at once, to the flow and to the shell; the usage-count question is the step after it, only where it was not answered before
const desktop = useDesktopI18n()
const telemetry = useTelemetryConsent({ api: telemetryApi })

const data = createGuideData({
  proxyUrl: () => sunshine.getProxyUrl(),
  vddStatus: () => invoke('get_vdd_status'),
  activeSessions: () => sunshine.getActiveSessions(),
  openUrl: (url) => openExternalUrl(url),
  copy: (text) => navigator.clipboard.writeText(text),
  reach: useHostReach(),
  // The apps page of the Web UI takes the request from the address (#guide-scan, #guide-add, #guide-restore)
  openApps: (mode) => emit('webui-navigate', `/apps#guide-${mode}`),
  openTools: () => emit('webui-navigate', '/#test-tools'),
  openDiagnostics: () => emit('open-doctor'),
  locale: () => locale.value,
})
const { state, probeAccount, loadGuideHost } = data
const hostProgress = ref(null) // what the host remembers of the guide, read once before the flow opens
// the update method (the last step): the same source and CDK the 更新 page keeps; the CDK itself stays in the native side
const updateSource = useUpdateSource()
const flowState = computed(() => ({
  ...state,
  locale: { choice: localeChoice.value, resolved: systemLocale(), languages: supportedLocales },
  update: { source: updateSource.source.value, available: updateSource.available.value, cdk: true, hasCdk: updateSource.hasCdk.value, status: updateSource.status.value, lastValid: updateSource.lastValid.value, busy: updateSource.busy.value },
}))
const callbacks = {
  ...data.callbacks,
  createAccount: data.createAccount(t),
  setLocale: (choice) => { setWelcomeChoice(choice); desktop.setChoice(choice) },
  // the first screen's one-time notice about error reports links to the switch (关于 › 隐私)
  openPrivacy: () => emit('open-privacy'),
  loadUpdate: () => updateSource.refresh(),
  setUpdateSource: (value) => updateSource.setSource(value),
  saveCdk: async (cdk) => ({ ok: await updateSource.saveCdk(cdk) }),
  clearCdk: () => updateSource.clearCdk(),
}
// First install (the whole flow) or an upgrade (what is new first, what is done left out), from the host's real state: the version stored for this panel, the setup flag, the paired clients and the account.
// Opened by hand on an install that has seen this version, the whole flow is shown.
const version = ref('')
const entry = ref(null)
const steps = computed(() => buildGuideSteps({ mode: entry.value?.mode || 'fresh', skip: [...(entry.value?.skip || []), ...(state.account.exists ? ['account'] : [])] }))
const qrMatrix = (text) => QRCode.create(text, { errorCorrectionLevel: 'M' }).modules

// The credential form keeps the texts of the welcome page, in every language it has. The title and the lead of the step say it is the Web UI account (the shared
// texts do that in Chinese and English); the other languages keep the welcome page's own words
const accountTexts = computed(() => {
  void locale.value
  const shared = /^(zh|en)/i.test(String(locale.value))
  return {
    ...(shared ? {} : { titles: { account: t('greeting') }, leads: { account: t('create_creds') } }),
    account: {
      username: t('username'), password: t('password'), confirm: t('confirm_password'),
      mismatch: t('password_mismatch'), match: t('password_match'),
      ...(shared ? {} : { create: t('login'), note: t('create_creds_alert'), local: t('creds_local_only') }),
    },
  }
})

// the presentation of the first-run window: the floating dialog (default) or the bottom sheet; ?onboard=sheet previews the other
const onboardMode = 'dialog'
const closeGuide = () => {
  try { window.sessionStorage.setItem(GUIDE_DISMISSED_KEY, '1') } catch { /* not remembered */ }
  emit('close')
}

onMounted(async () => {
  // `--start-guide` (the installer's done page, or a second launch forwarded here): the first-run flow from its first step, whatever was saved.
  const fromStart = consumeStartGuide()
  if (fromStart) clearProgress(null, GUIDE_STORAGE_KEY)
  await Promise.all([probeAccount(), telemetry.refresh(), panelVersion().then((v) => { version.value = v })])
  if (state.account.exists) await Promise.all([callbacks.loadSetup?.(), callbacks.refreshClients?.(), loadGuideHost().then((h) => { hostProgress.value = h })])
  if (fromStart) {
    entry.value = { ...guideOnLaunch({ fromStart: true }), restart: true }
    return
  }
  const pairedCount = Array.isArray(state.clients) ? state.clients.length : null
  const found = state.account.exists ? guideEntry({ progress: readProgress(null, GUIDE_STORAGE_KEY), version: version.value, setupCompleted: !!state.setup.completed, pairedCount, configExists: true, host: hostProgress.value, device: guideDeviceId() }) : null
  // the guide the host would not open by itself was asked for by hand (the help page): it starts at the first step, not where an old run was left
  entry.value = found?.mode === 'upgrade' ? { mode: 'upgrade', skip: upgradeSkips({ accountExists: true, setupCompleted: !!state.setup.completed, pairedCount }) } : { mode: 'fresh', skip: [], restart: !found }
})
</script>

<style>
.panel-guide { min-height: 100dvh; }
.panel-guide .m-guide { min-height: 100dvh; }
.panel-guide__loading { margin: 120px auto; }
</style>
