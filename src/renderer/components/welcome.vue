<template>
  <MOnboardWindow :mode="onboardMode" :label="accountTexts?.title || ''" @close="closeGuide">
  <div class="panel-guide">
    <MLoading v-if="state.account.exists === null" class="panel-guide__loading" />
    <MGuideFlow
      v-else
      :steps="steps"
      :state="flowState"
      :callbacks="callbacks"
      :locale="locale"
      :texts="accountTexts"
      :storage-key="GUIDE_STORAGE_KEY"
      :qr="qrMatrix"
      :required="state.account.exists ? [] : ['account']"
      @close="closeGuide"
      @finish="closeGuide"
    >
      <template #top>
        <label class="welcome-lang">
          <MIcon name="language" />
          <select v-model="currentLocale" :aria-label="supportedLocales.find((l) => l.code === currentLocale)?.name" @change="handleLocaleChange">
            <option v-for="loc in supportedLocales" :key="loc.code" :value="loc.code">{{ loc.name }}</option>
          </select>
        </label>
      </template>
    </MGuideFlow>
  </div>
  </MOnboardWindow>
</template>

<script setup>
import { computed, onMounted, ref, watch } from 'vue'
import QRCode from 'qrcode'
import { invoke } from '@tauri-apps/api/core'
import { MGuideFlow, MIcon, MLoading, MOnboardWindow, buildGuideSteps } from 'foundation-ui/vue'
import { supportedLocales, setLocale as setI18nLocale, getDefaultLocale, useWelcomeI18n } from '../../i18n/index.js'
import { openExternalUrl, sunshine } from '../tauri-adapter.js'
import { GUIDE_DISMISSED_KEY, GUIDE_STORAGE_KEY, createGuideData } from '../composables/guideData.js'
import { useHostReach } from '../composables/useHostReach.js'
import { useUpdateSource } from '../composables/useUpdateSource.js'

// The guided first run of the panel. It opens by itself on a first run, when the host serves its account page (no account yet), and from
// the help page. The account step is part of the flow only while the host has no account.
const emit = defineEmits(['close', 'webui-navigate'])
const { t, locale } = useWelcomeI18n()

const currentLocale = ref(locale.value)
watch(locale, (val) => { currentLocale.value = val })
watch(currentLocale, (val) => {
  if (locale.value !== val) {
    setI18nLocale(val)
    locale.value = val
  }
})
const handleLocaleChange = () => setI18nLocale(currentLocale.value)

const data = createGuideData({
  proxyUrl: () => sunshine.getProxyUrl(),
  vddStatus: () => invoke('get_vdd_status'),
  activeSessions: () => sunshine.getActiveSessions(),
  openUrl: (url) => openExternalUrl(url),
  copy: (text) => navigator.clipboard.writeText(text),
  reach: useHostReach(),
  // The apps page of the Web UI takes the request from the address (#guide-scan, #guide-add, #guide-restore)
  openApps: (mode) => emit('webui-navigate', `/apps#guide-${mode}`),
})
const { state, probeAccount } = data
// the update method (the last step): the same source and CDK the 更新 page keeps; the CDK itself stays in the native side
const updateSource = useUpdateSource()
const flowState = computed(() => ({
  ...state,
  update: { source: updateSource.source.value, available: updateSource.available.value, cdk: true, hasCdk: updateSource.hasCdk.value, status: updateSource.status.value, lastValid: updateSource.lastValid.value, busy: updateSource.busy.value },
}))
const callbacks = {
  ...data.callbacks,
  createAccount: data.createAccount(t),
  loadUpdate: () => updateSource.refresh(),
  setUpdateSource: (value) => updateSource.setSource(value),
  saveCdk: async (cdk) => ({ ok: await updateSource.saveCdk(cdk) }),
  clearCdk: () => updateSource.clearCdk(),
}
const steps = computed(() => buildGuideSteps({ skip: state.account.exists ? ['account'] : [] }))
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
  const defaultLocale = getDefaultLocale()
  currentLocale.value = defaultLocale
  setI18nLocale(defaultLocale)
  await probeAccount()
})
</script>

<style>
.welcome-lang { display: inline-flex; align-items: center; gap: 8px; height: 40px; padding: 0 12px 0 14px; border-radius: 20px; background: var(--md-sys-color-surface-container-highest); color: var(--md-sys-color-on-surface-variant); }
.welcome-lang select { border: 0; outline: 0; background: transparent; color: var(--md-sys-color-on-surface); font: var(--m-label-large); cursor: pointer; }
.panel-guide { min-height: 100dvh; }
.panel-guide .m-guide { min-height: 100dvh; }
.panel-guide__loading { margin: 120px auto; }
</style>
