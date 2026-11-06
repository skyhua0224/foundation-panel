<template>
  <DeskPage class="devices-view" :title="t.devices.pageTitle" icon="devices" :text="heroText" wide>
    <template v-if="qrActive" #chips>
      <MChip :tone="qrRemaining <= 30 ? 'warn' : 'info'" icon="timer">{{ qrRemaining }}s</MChip>
    </template>

    <!-- pairing: a code to scan, or a PIN to type -->
    <div class="dsk-pair">
      <section class="dsk-tile dsk-tile--qr" :class="{ 'is-live': qrActive }">
        <header class="dsk-tile__head">
          <span class="dsk-tile__icon"><MIcon name="qr_code_2" fill /></span>
          <h2 class="dsk-tile__title">{{ t.devices.qrCodePairing }}</h2>
        </header>

        <div v-if="qrActive" class="dsk-qr">
          <div class="dsk-qr__frame"><div class="dsk-qr__code"><img :src="qrDataUrl" alt="QR Code" /></div></div>
          <div class="dsk-qr__meta">
            <div class="dsk-qr__pin">
              <span class="dsk-qr__pin-label">PIN</span>
              <span class="dsk-qr__pin-value">{{ qrPin }}</span>
            </div>
            <div class="dsk-qr__actions">
              <MButton variant="tonal" icon="refresh" @click="generateQrCode">{{ t.devices.qrRefresh }}</MButton>
              <MButton variant="text" @click="cancelQrCode">{{ t.devices.qrCancel }}</MButton>
            </div>
          </div>
        </div>

        <MNotice v-else-if="qrError" tone="error">{{ qrError }}</MNotice>

        <div v-else class="dsk-idle">
          <MScallop :size="132" :busy="qrLoading"><MIcon name="qr_code_2" fill class="dsk-idle__glyph" /></MScallop>
          <p>{{ t.devices.qrDesc }}</p>
          <MButton size="lg" icon="qr_code_2" :loading="qrLoading" @click="generateQrCode">
            {{ qrLoading ? t.devices.qrGenerating : t.devices.generateQrCode }}
          </MButton>
        </div>
      </section>

      <section class="dsk-tile dsk-tile--pin">
        <header class="dsk-tile__head">
          <span class="dsk-tile__icon"><MIcon name="pin" fill /></span>
          <h2 class="dsk-tile__title">{{ t.devices.pinPairing }}</h2>
        </header>

        <div class="dsk-pin">
          <MPinPad v-model="pinInput" :length="4" :label="t.devices.pinInput" :disabled="pinSubmitting" data-focus-key="devices-pin" />
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
            :disabled="pinInput.length < 4 || !deviceNameInput"
            @click="submitPin"
          >{{ pinSubmitting ? t.devices.pairing : t.devices.pairBtn }}</MButton>
          <MNotice v-if="pinStatus" :tone="pinStatus.type === 'success' ? 'info' : 'error'" :icon="pinStatus.type === 'success' ? 'check_circle' : 'error'">{{ pinStatus.message }}</MNotice>
        </div>
      </section>
    </div>

    <!-- paired devices -->
    <section class="dsk-devices">
      <div class="dsk-devices__head">
        <h2 class="dsk-devices__title">{{ t.devices.pairedDevicesTitle }}</h2>
        <MChip tone="info">{{ devices.length }}</MChip>
      </div>

      <div v-if="devices.length" class="dsk-devices__grid">
        <article v-for="(device, index) in devices" :key="device.uuid" class="dsk-device" :class="`is-${SHAPES[index % SHAPES.length]}`">
          <span class="dsk-device__avatar"><span class="dsk-device__shape" :class="`m-shape--${SHAPES[index % SHAPES.length]}`" /><MIcon name="mobile" fill /></span>
          <div class="dsk-device__body">
            <template v-if="editingUuid !== device.uuid">
              <h3 class="dsk-device__name">{{ device.name || t.devices.unnamedDevice }}</h3>
              <span class="dsk-devices__uuid">{{ device.uuid }}</span>
            </template>
            <div v-else class="dsk-devices__rename">
              <MField
                v-model="editName"
                class="name-input"
                :label="t.devices.renameBtn"
                data-focusable
                @keyup.enter="saveRename(device)"
                @keyup.escape="cancelRename"
              />
            </div>
          </div>
          <div class="dsk-device__actions">
            <template v-if="editingUuid !== device.uuid">
              <MIconButton icon="edit" tonal :label="t.devices.renameBtn" @click="startRename(device)" />
              <MIconButton icon="link_off" class="is-danger" :label="t.devices.unpairBtn" @click="unpairDevice(device)" />
            </template>
            <template v-else>
              <MIconButton icon="check" tonal :label="t.devices.saveBtn" @click="saveRename(device)" />
              <MIconButton icon="close" :label="t.devices.cancelBtn" @click="cancelRename" />
            </template>
          </div>
        </article>
      </div>

      <div v-else-if="loading" class="dsk-devices__wait"><MLoading size="32px" :label="t.devices.loading" /><span>{{ t.devices.loading }}</span></div>
      <MEmpty v-else icon="mobile" :title="t.devices.emptyState" />
    </section>
  </DeskPage>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { MButton, MChip, MEmpty, MField, MIcon, MIconButton, MLoading, MNotice, MPinPad, MScallop } from 'foundation-ui/vue'
import QRCode from 'qrcode'
import { requestConfirm } from '../composables/useConfirm.js'
import { useI18n } from '../i18n/index.js'
import DeskPage from '../components/DeskPage.vue'

const { t } = useI18n()

// the shapes the device avatars turn in (foundation-ui shape vocabulary), in turn
const SHAPES = ['cookie', 'clover', 'sunny']

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
const heroText = computed(() => t.value.devices.pairedCount.replace('{n}', devices.value.length))

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
/* The pairing tiles: a tonal block each, a rotating Material shape behind the icon, big forms (QR, PIN dots, circular keys). */
.dsk-pair { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 20px; align-items: stretch; margin-bottom: 32px; }
.dsk-tile { position: relative; overflow: hidden; isolation: isolate; display: flex; flex-direction: column; gap: 18px; min-width: 0; padding: 26px 28px 28px; border-radius: var(--md-sys-shape-corner-extra-extra-large); color: var(--md-sys-color-on-surface); }
.dsk-tile::after { content: ''; position: absolute; z-index: -1; right: -60px; top: -70px; width: 240px; height: 240px; background: currentColor; opacity: .07; clip-path: var(--m-shape-cookie); animation: m-turn 48s linear infinite; }
.dsk-tile--qr { background: var(--dsk-tert-bg); color: var(--dsk-tert-fg); }
.dsk-tile--pin { background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.dsk-tile__head { display: flex; align-items: center; gap: 14px; }
.dsk-tile__icon { flex: none; width: 52px; height: 52px; display: grid; place-items: center; font-size: 28px; background: currentColor; clip-path: var(--m-shape-squircle); position: relative; }
.dsk-tile__icon :deep(.m-icon) { color: var(--md-sys-color-surface); }
.dsk-tile--qr .dsk-tile__icon { background: var(--md-sys-color-tertiary); }
.dsk-tile--pin .dsk-tile__icon { background: var(--md-sys-color-secondary); }
.dsk-tile--qr .dsk-tile__icon :deep(.m-icon) { color: var(--md-sys-color-on-tertiary); }
.dsk-tile--pin .dsk-tile__icon :deep(.m-icon) { color: var(--md-sys-color-on-secondary); }
.dsk-tile__title { margin: 0; font: var(--m-headline-small-emphasized); letter-spacing: 0; }

.dsk-idle { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 18px; text-align: center; padding: 8px 0; }
.dsk-idle p { margin: 0; max-width: 28ch; font: var(--m-title-medium); opacity: .9; }
.dsk-idle__glyph { font-size: 56px; }
.dsk-qr { flex: 1; display: flex; align-items: center; gap: 28px; }
.dsk-qr__frame { flex: none; padding: 14px; border-radius: var(--md-sys-shape-corner-extra-large); background: color-mix(in srgb, var(--md-sys-color-surface) 55%, transparent); }
.dsk-qr__code { padding: 12px; border-radius: var(--md-sys-shape-corner-large); background: #fff; }
.dsk-qr__code img { display: block; width: 200px; height: 200px; }
.dsk-qr__meta { display: flex; flex-direction: column; align-items: flex-start; gap: 16px; min-width: 0; }
.dsk-qr__pin { display: flex; flex-direction: column; }
.dsk-qr__pin-label { font: var(--m-label-large); opacity: .8; }
.dsk-qr__pin-value { font: var(--m-display-medium); letter-spacing: .14em; }
.dsk-qr__actions { display: flex; flex-wrap: wrap; gap: 8px; }
.dsk-pin { display: flex; flex-direction: column; align-items: stretch; gap: 16px; }
.dsk-pin > :first-child { align-self: center; }

.dsk-devices__head { display: flex; align-items: center; gap: 14px; margin: 0 0 16px 4px; }
.dsk-devices__title { margin: 0; font: var(--m-headline-small-emphasized); color: var(--md-sys-color-on-surface); letter-spacing: 0; }
.dsk-devices__grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
.dsk-device { display: flex; align-items: center; gap: 18px; min-width: 0; padding: 18px 20px; border-radius: var(--md-sys-shape-corner-extra-large); background: var(--md-sys-color-surface-container-high); transition: border-radius var(--m-spring-default-spatial-duration, 300ms) var(--m-spring-default-spatial, ease), background-color var(--m-spring-default-effects-duration, 200ms) var(--m-spring-default-effects, ease); }
.dsk-device:hover, .dsk-device:focus-within { border-radius: var(--md-sys-shape-corner-extra-extra-large); background: var(--md-sys-color-surface-container-highest); }
.dsk-device__avatar { position: relative; flex: none; width: 76px; height: 76px; display: grid; place-items: center; font-size: 36px; color: var(--md-sys-color-on-primary-container); }
.dsk-device__shape { position: absolute; inset: 0; background: var(--md-sys-color-primary-container); animation: m-turn 60s linear infinite; }
.dsk-device__avatar :deep(.m-icon) { position: relative; }
.dsk-device.is-clover .dsk-device__shape { background: var(--dsk-tert-bg); }
.dsk-device.is-clover .dsk-device__avatar { color: var(--dsk-tert-fg); }
.dsk-device.is-sunny .dsk-device__shape { background: var(--md-sys-color-secondary-container); }
.dsk-device.is-sunny .dsk-device__avatar { color: var(--md-sys-color-on-secondary-container); }
.dsk-device__body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
.dsk-device__name { margin: 0; font: var(--m-title-large); color: var(--md-sys-color-on-surface); overflow-wrap: anywhere; }
.dsk-devices__uuid { font: var(--m-label-large); color: var(--md-sys-color-on-surface-variant); font-family: var(--m-font-mono, ui-monospace, 'Consolas', monospace); overflow-wrap: anywhere; }
.dsk-device__actions { flex: none; display: flex; gap: 6px; }
.dsk-device__actions .is-danger { --m-icon-btn-fg: var(--md-sys-color-error); color: var(--md-sys-color-error); }
.dsk-devices__rename { display: flex; align-items: center; width: 100%; }
.dsk-devices__wait { display: flex; align-items: center; justify-content: center; gap: 12px; padding: 24px; font: var(--m-body-large); color: var(--md-sys-color-on-surface-variant); }

@media (max-width: 1300px) {
  .dsk-pair, .dsk-devices__grid { grid-template-columns: minmax(0, 1fr); }
}
</style>
