<template>
  <DeskPage class="devices-view" :title="t.devices.pageTitle" wide>
    <!-- pairing: a code to scan, or a PIN to type -->
    <div class="dsk-pair">
      <section class="dsk-panel">
        <header class="dsk-panel__head">
          <span class="dsk-panel__icon"><MIcon name="qr_code_2" fill /></span>
          <h2 class="dsk-panel__title">{{ t.devices.qrCodePairing }}</h2>
        </header>

        <div v-if="qrActive" class="dsk-qr">
          <div class="dsk-qr__code"><img :src="qrDataUrl" alt="QR Code" /></div>
          <div class="dsk-qr__meta">
            <div class="dsk-qr__pin">
              <span class="dsk-qr__pin-label">PIN</span>
              <span class="dsk-qr__pin-value">{{ qrPin }}</span>
            </div>
            <MChip :tone="qrRemaining <= 30 ? 'warn' : 'info'" icon="timer">{{ qrRemaining }}s</MChip>
            <div class="dsk-qr__actions">
              <MButton variant="tonal" icon="refresh" @click="generateQrCode">{{ t.devices.qrRefresh }}</MButton>
              <MButton variant="text" @click="cancelQrCode">{{ t.devices.qrCancel }}</MButton>
            </div>
          </div>
        </div>

        <MNotice v-else-if="qrError" tone="error">{{ qrError }}</MNotice>

        <div v-else class="dsk-idle">
          <p>{{ t.devices.qrDesc }}</p>
          <MButton icon="qr_code_2" :loading="qrLoading" @click="generateQrCode">
            {{ qrLoading ? t.devices.qrGenerating : t.devices.generateQrCode }}
          </MButton>
        </div>
      </section>

      <section class="dsk-panel">
        <header class="dsk-panel__head">
          <span class="dsk-panel__icon"><MIcon name="pin" fill /></span>
          <h2 class="dsk-panel__title">{{ t.devices.pinPairing }}</h2>
        </header>

        <div class="dsk-pin">
          <MField
            v-model="pinInput"
            :label="t.devices.pinInput"
            :placeholder="t.devices.pinPlaceholder"
            maxlength="4"
            inputmode="numeric"
            data-focusable
            data-focus-key="devices-pin"
          />
          <MField
            v-model="deviceNameInput"
            :label="t.devices.deviceName"
            :placeholder="t.devices.deviceNamePlaceholder"
            data-focusable
            data-focus-key="devices-name"
          />
          <MButton
            size="lg"
            icon="link"
            :loading="pinSubmitting"
            :disabled="!pinInput || !deviceNameInput"
            @click="submitPin"
          >{{ pinSubmitting ? t.devices.pairing : t.devices.pairBtn }}</MButton>
          <MNotice v-if="pinStatus" :tone="pinStatus.type === 'success' ? 'info' : 'error'" :icon="pinStatus.type === 'success' ? 'check_circle' : 'error'">{{ pinStatus.message }}</MNotice>
        </div>
      </section>
    </div>

    <!-- paired devices -->
    <MSection class="dsk-devices">
      <div class="dsk-devices__head">
        <h2 class="dsk-devices__title">{{ t.devices.pairedDevicesTitle }}</h2>
        <MChip>{{ devices.length }}</MChip>
      </div>

      <MGroup v-if="devices.length" spaced>
        <MItem v-for="device in devices" :key="device.uuid" icon="mobile" :stack="editingUuid === device.uuid">
          <template #title>{{ device.name || t.devices.unnamedDevice }}</template>
          <template #default><span class="dsk-devices__uuid">{{ device.uuid }}</span></template>
          <template v-if="editingUuid !== device.uuid" #trail>
            <MIconButton icon="edit" tonal :label="t.devices.renameBtn" @click="startRename(device)" />
            <MIconButton icon="link_off" :label="t.devices.unpairBtn" @click="unpairDevice(device)" />
          </template>
          <template v-else #control>
            <div class="dsk-devices__rename">
              <MField
                v-model="editName"
                class="name-input"
                :label="t.devices.renameBtn"
                data-focusable
                @keyup.enter="saveRename(device)"
                @keyup.escape="cancelRename"
              />
              <MIconButton icon="check" tonal :label="t.devices.saveBtn" @click="saveRename(device)" />
              <MIconButton icon="close" :label="t.devices.cancelBtn" @click="cancelRename" />
            </div>
          </template>
        </MItem>
      </MGroup>

      <div v-else-if="loading" class="dsk-devices__wait"><MLoading size="32px" :label="t.devices.loading" /><span>{{ t.devices.loading }}</span></div>
      <MEmpty v-else icon="mobile" :title="t.devices.emptyState" />
    </MSection>
  </DeskPage>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { MButton, MChip, MEmpty, MField, MGroup, MIcon, MIconButton, MItem, MLoading, MNotice, MSection } from 'foundation-ui/vue'
import QRCode from 'qrcode'
import { requestConfirm } from '../composables/useConfirm.js'
import { useI18n } from '../i18n/index.js'
import DeskPage from '../components/DeskPage.vue'

const { t } = useI18n()

// === Tauri invoke ===
const invoke = ref(null)
const proxyUrl = ref('http://localhost:48081')

async function initTauri() {
  try {
    const tauri = await import('@tauri-apps/api/core')
    invoke.value = tauri.invoke
    const url = await invoke.value('get_proxy_url_command')
    if (url) proxyUrl.value = url
  } catch (e) {
    console.log('Tauri invoke not available:', e)
  }
}

async function apiFetch(path, options = {}) {
  const response = await fetch(`${proxyUrl.value}${path}`, options)
  return await response.json()
}

// === 设备列表 ===
const devices = ref([])
const loading = ref(false)

async function loadDevices() {
  loading.value = true
  try {
    const data = await apiFetch('/api/clients/list')
    if (data.status?.toString() === 'true' && data.named_certs) {
      devices.value = data.named_certs
    }
  } catch (e) {
    console.error('Failed to load devices:', e)
  } finally {
    loading.value = false
  }
}

// === 重命名 ===
const editingUuid = ref(null)
const editName = ref('')
const nameInputRef = ref(null)

function startRename(device) {
  editingUuid.value = device.uuid
  editName.value = device.name || ''
  nextTick(() => {
    const input = document.querySelector('.name-input')
    if (input) input.focus()
  })
}

function cancelRename() {
  editingUuid.value = null
  editName.value = ''
}

async function saveRename(device) {
  const newName = editName.value.trim()
  if (!newName) return
  try {
    const data = await apiFetch('/api/clients/rename', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ uuid: device.uuid, name: newName }),
    })
    if (data.status?.toString() === 'true') {
      device.name = newName
    }
  } catch (e) {
    console.error('Failed to rename:', e)
  }
  cancelRename()
}

// === 取消配对 ===
async function unpairDevice(device) {
  const name = device.name || device.uuid
  // The window's own confirm dialog: a native one cannot be answered with a pad.
  const confirmed = await requestConfirm({
    title: t.value.devices.unpairBtn,
    message: t.value.devices.unpairConfirm.replace('{name}', name),
    confirmLabel: t.value.devices.unpairBtn,
    danger: true,
  })
  if (!confirmed) return
  try {
    const data = await apiFetch('/api/clients/unpair', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ uuid: device.uuid }),
    })
    if (data.status?.toString() === 'true') {
      devices.value = devices.value.filter(d => d.uuid !== device.uuid)
    }
  } catch (e) {
    console.error('Failed to unpair:', e)
  }
}

// === QR 配对 ===
const qrDataUrl = ref('')
const qrPin = ref('')
const qrExpiresAt = ref(0)
const qrRemaining = ref(0)
const qrLoading = ref(false)
const qrError = ref('')
let countdownTimer = null

const qrActive = computed(() => qrRemaining.value > 0 && qrDataUrl.value !== '')

function stopCountdown() {
  if (countdownTimer) {
    clearInterval(countdownTimer)
    countdownTimer = null
  }
}

function startCountdown() {
  stopCountdown()
  countdownTimer = setInterval(() => {
    const remaining = Math.max(0, Math.floor((qrExpiresAt.value - Date.now()) / 1000))
    qrRemaining.value = remaining
    if (remaining <= 0) {
      stopCountdown()
      qrDataUrl.value = ''
      qrPin.value = ''
    }
  }, 1000)
}

async function generateQrCode() {
  qrLoading.value = true
  qrError.value = ''
  try {
    console.log('QR pair: using proxy URL:', proxyUrl.value)
    const data = await apiFetch('/api/qr-pair', { method: 'POST' })

    if (data.status?.toString() !== 'true') {
      qrError.value = data.error || t.value.devices.qrGenerateFailed
      return
    }

    qrPin.value = data.pin
    qrExpiresAt.value = Date.now() + data.expires_in * 1000
    qrRemaining.value = data.expires_in

    qrDataUrl.value = await QRCode.toDataURL(data.url, {
      width: 280,
      margin: 2,
      color: { dark: '#000000', light: '#ffffff' },
    })

    startCountdown()
  } catch (e) {
    console.error('Failed to generate QR:', e)
    qrError.value = `${t.value.devices.qrNetworkError}: ${e.message}`
  } finally {
    qrLoading.value = false
  }
}

function cancelQrCode() {
  stopCountdown()
  qrDataUrl.value = ''
  qrPin.value = ''
  qrRemaining.value = 0
}

// === 手动 PIN 配对 ===
const pinInput = ref('')
const deviceNameInput = ref('')
const pinSubmitting = ref(false)
const pinStatus = ref(null)

async function submitPin() {
  pinSubmitting.value = true
  pinStatus.value = null
  try {
    const data = await apiFetch('/api/pin', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ pin: pinInput.value, name: deviceNameInput.value }),
    })
    if (data.status?.toString() === 'true') {
      pinStatus.value = { type: 'success', message: t.value.devices.pinStatus.success }
      pinInput.value = ''
      deviceNameInput.value = ''
      loadDevices()
    } else {
      pinStatus.value = { type: 'error', message: t.value.devices.pinStatus.error }
    }
  } catch (e) {
    pinStatus.value = { type: 'error', message: t.value.devices.pinStatus.networkError }
  } finally {
    pinSubmitting.value = false
  }
}

// === 生命周期 ===
onMounted(async () => {
  await initTauri()
  loadDevices()
})

onUnmounted(() => {
  stopCountdown()
})
</script>

<style scoped>
.dsk-pair { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 20px; align-items: stretch; margin-bottom: 28px; }
.dsk-panel { display: flex; flex-direction: column; gap: 16px; min-width: 0; padding: 20px; border-radius: var(--md-sys-shape-corner-extra-large); background: var(--m-card); color: var(--md-sys-color-on-surface); }
.dsk-panel__head { display: flex; align-items: center; gap: 12px; }
.dsk-panel__icon { flex: none; width: 40px; height: 40px; border-radius: 14px; display: grid; place-items: center; font-size: 24px; background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.dsk-panel__title { margin: 0; font: var(--m-title-large); color: var(--md-sys-color-on-surface); letter-spacing: 0; }

.dsk-idle { display: flex; flex-direction: column; align-items: flex-start; gap: 16px; }
.dsk-idle p { margin: 0; font: var(--m-body-large); color: var(--md-sys-color-on-surface-variant); }
.dsk-qr { display: flex; align-items: center; gap: 20px; }
.dsk-qr__code { flex: none; padding: 12px; border-radius: var(--md-sys-shape-corner-large); background: #fff; }
.dsk-qr__code img { display: block; width: 180px; height: 180px; }
.dsk-qr__meta { display: flex; flex-direction: column; align-items: flex-start; gap: 12px; min-width: 0; }
.dsk-qr__pin { display: flex; flex-direction: column; }
.dsk-qr__pin-label { font: var(--m-label-large); color: var(--md-sys-color-on-surface-variant); }
.dsk-qr__pin-value { font: var(--m-display-small); letter-spacing: .12em; }
.dsk-qr__actions { display: flex; flex-wrap: wrap; gap: 8px; }
.dsk-pin { display: flex; flex-direction: column; gap: 16px; }

.dsk-devices { margin: 0; }
.dsk-devices__head { display: flex; align-items: center; gap: 12px; padding: 4px 24px 4px 4px; align-self: flex-start; }
.dsk-devices__title { margin: 0; padding: 4px 20px; border-radius: 9999px; background: var(--md-sys-color-surface-container); font: var(--m-title-large); color: var(--md-sys-color-on-surface); letter-spacing: 0; }
.dsk-devices__uuid { font-family: var(--m-font-mono, ui-monospace, 'Consolas', monospace); }
.dsk-devices__rename { display: flex; align-items: center; gap: 8px; width: 100%; }
.dsk-devices__wait { display: flex; align-items: center; justify-content: center; gap: 12px; padding: 24px; font: var(--m-body-large); color: var(--md-sys-color-on-surface-variant); }

@media (max-width: 1300px) {
  .dsk-pair { grid-template-columns: minmax(0, 1fr); }
}
</style>
