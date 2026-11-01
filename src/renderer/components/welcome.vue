<template>
  <MWelcome :title="t('greeting')" icon="shield" :rows="[{ icon: 'warning', tone: 'tertiary', text: t('create_creds_alert') }]">
    <template #top>
      <label class="welcome-lang">
        <MIcon name="language" />
        <select v-model="currentLocale" :aria-label="supportedLocales.find((l) => l.code === currentLocale)?.name" @change="handleLocaleChange">
          <option v-for="loc in supportedLocales" :key="loc.code" :value="loc.code">{{ loc.name }}</option>
        </select>
      </label>
    </template>
    <template #intro><p class="welcome-sub">{{ t('create_creds') }}</p></template>

    <form id="panel-welcome-form" class="welcome-form" @submit.prevent="handleSubmit">
      <MGroup>
        <MFieldItem v-model="formData.username" :label="t('username')" autocomplete="username" />
        <MFieldItem v-model="formData.password" type="password" :label="t('password')" autocomplete="new-password" required />
        <MFieldItem v-model="formData.confirmPassword" type="password" :label="t('confirm_password')" autocomplete="new-password" required :error="showPasswordError ? t('password_mismatch') : ''" :supporting="showPasswordSuccess ? t('password_match') : ''" />
      </MGroup>
    </form>
    <MBanner v-if="errorMessage" tone="error" icon="error">{{ errorMessage }}</MBanner>
    <MBanner v-if="successMessage" tone="info" icon="check_circle">{{ successMessage }}</MBanner>

    <template #actions>
      <MButton type="submit" form="panel-welcome-form" size="lg" icon="login" :loading="loading" :disabled="loading || !isFormValid">{{ t('login') }}</MButton>
    </template>
  </MWelcome>
</template>

<script setup>
import { ref, computed, onMounted, watch } from 'vue'
import { supportedLocales, setLocale as setI18nLocale, getDefaultLocale, useWelcomeI18n } from '../../i18n/index.js'
import { sunshine } from '../tauri-adapter.js'
import { MWelcome, MGroup, MFieldItem, MBanner, MButton, MIcon } from 'foundation-ui/vue'

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
.welcome-lang { display: inline-flex; align-items: center; gap: 8px; height: 40px; padding: 0 12px 0 14px; border-radius: 20px; background: var(--md-sys-color-surface-container-highest); color: var(--md-sys-color-on-surface-variant); }
.welcome-lang select { border: 0; outline: 0; background: transparent; color: var(--md-sys-color-on-surface); font: var(--m-label-large); cursor: pointer; }
.welcome-sub { margin: 0; text-align: center; color: var(--md-sys-color-on-surface-variant); font: var(--m-body-medium); }
</style>
