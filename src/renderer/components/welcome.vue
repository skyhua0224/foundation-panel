<template>
  <MPage :title="t('greeting')" class="m-welcome">
    <img src="../public/gura-pix.png" class="m-welcome__art" alt="Sunshine">
    <form class="m-welcome__form" @submit.prevent="handleSubmit">
      <MBanner tone="warn" icon="warning">{{ t('create_creds_alert') }}</MBanner>
      <MGroup>
        <MFieldItem v-model="formData.username" :label="t('username')" autocomplete="username" />
        <MFieldItem v-model="formData.password" type="password" :label="t('password')" autocomplete="new-password" required />
        <MFieldItem v-model="formData.confirmPassword" type="password" :label="t('confirm_password')" autocomplete="new-password" required :error="showPasswordError ? t('password_mismatch') : ''" :supporting="showPasswordSuccess ? t('password_match') : ''" />
        <MChoiceItem v-model="currentLocale" icon="translate" title="Language / 语言" :options="supportedLocales.map((l) => ({ value: l.code, label: l.name }))" cancel-label="×" @update:model-value="handleLocaleChange" />
      </MGroup>
      <MNotice v-if="errorMessage" tone="error">{{ errorMessage }}</MNotice>
      <MNotice v-if="successMessage" icon="check_circle">{{ successMessage }}</MNotice>
      <MButton class="m-welcome__submit" type="submit" size="lg" :loading="loading" :disabled="loading || !isFormValid">{{ t('login') }}</MButton>
    </form>
  </MPage>
</template>

<script setup>
import { ref, computed, onMounted, watch } from 'vue'
import { supportedLocales, setLocale as setI18nLocale, getDefaultLocale, useWelcomeI18n } from '../../i18n/index.js'
import { sunshine } from '../tauri-adapter.js'
import { MPage, MGroup, MFieldItem, MChoiceItem, MBanner, MNotice, MButton } from 'foundation-ui/vue'

const REDIRECT_DELAY = 2000

const { t, locale } = useWelcomeI18n()

const currentLocale = ref(locale.value)
const formData = ref({ username: 'sunshine', password: '', confirmPassword: '' })
const loading = ref(false)
const errorMessage = ref('')
const successMessage = ref('')

const passwordsMatch = computed(() => {
  const { password, confirmPassword } = formData.value
  return !password || !confirmPassword || password === confirmPassword
})

const showPasswordError = computed(() => !passwordsMatch.value && formData.value.confirmPassword)

const showPasswordSuccess = computed(() => 
  passwordsMatch.value && formData.value.confirmPassword && formData.value.password
)

const isFormValid = computed(() => {
  const { username, password, confirmPassword } = formData.value
  return username && password && confirmPassword && passwordsMatch.value
})

watch(locale, (val) => { currentLocale.value = val })

watch(currentLocale, (val) => {
  if (locale.value !== val) {
    setI18nLocale(val)
    locale.value = val
  }
})

const handleLocaleChange = () => setI18nLocale(currentLocale.value)

const handleSubmit = async () => {
  errorMessage.value = ''
  successMessage.value = ''

  if (!passwordsMatch.value) {
    errorMessage.value = t('password_mismatch')
    return
  }

  const { username, password, confirmPassword } = formData.value
  if (!username || !password || !confirmPassword) {
    errorMessage.value = `${t('error')} ${t('create_creds')}`
    return
  }

  loading.value = true

  try {
    const proxyUrl = await sunshine.getProxyUrl()
    const response = await fetch(`${proxyUrl}/api/password`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ 
        newUsername: username, 
        newPassword: password, 
        confirmNewPassword: confirmPassword 
      }),
    })

    const result = await response.json()

    if (result.status?.toString() === 'true') {
      successMessage.value = `${t('success')} ${t('welcome_success')}`
      setTimeout(() => {
        window.location.href = `${proxyUrl}/`
      }, REDIRECT_DELAY)
    } else {
      errorMessage.value = `${t('error')} ${result.error || t('server_error')}`
    }
  } catch (err) {
    console.error('Failed to save password:', err)
    errorMessage.value = `${t('error')} ${err.message || t('network_error')}`
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  const defaultLocale = getDefaultLocale()
  currentLocale.value = defaultLocale
  setI18nLocale(defaultLocale)
})
</script>

<style>
.m-welcome__art { align-self: center; width: min(55%, 220px); image-rendering: pixelated; margin: 8px 0 4px; }
.m-welcome__form { display: flex; flex-direction: column; gap: 12px; }
.m-welcome__submit { width: 100%; height: 56px; }
</style>
