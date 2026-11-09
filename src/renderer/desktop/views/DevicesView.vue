<template>
  <DeskPage class="devices-view" :title="t.devices.pageTitle" wide>
    <!-- pairing: a code to scan, or a PIN to type; the same sections and rows as every settings page -->
    <MSection :title="t.devices.qrCodePairing">
      <MNotice v-if="qrError" tone="error">{{ qrError }}</MNotice>
      <MGroup>
        <MItem
          v-if="!qrActive"
          action
          icon="qr_code_2"
          :title="qrLoading ? t.devices.qrGenerating : t.devices.generateQrCode"
          :text="t.devices.qrDesc"
          :disabled="qrLoading"
          data-focus-key="devices-qr"
          @click="generateQrCode"
        />
        <template v-else>
          <MItem icon="timer" title="PIN" :text="`${qrRemaining}s`">
            <template #trail><span class="dsk-pin__code">{{ qrPin }}</span></template>
          </MItem>
          <MItem plain stack :title="t.devices.qrCodePairing">
            <template #control><div class="dsk-qr"><img :src="qrDataUrl" alt="QR Code" /></div></template>
          </MItem>
          <MItem action icon="refresh" :title="t.devices.qrRefresh" data-focus-key="devices-qr" @click="generateQrCode" />
          <MItem action icon="close" :title="t.devices.qrCancel" @click="cancelQrCode" />
        </template>
      </MGroup>
    </MSection>

    <MSection :title="t.devices.pinPairing">
      <MGroup>
        <MItem plain stack :title="t.devices.pinInput">
          <template #control><div class="dsk-pin"><MPinPad v-model="pinInput" :length="4" :label="t.devices.pinInput" :disabled="pinSubmitting" data-focus-key="devices-pin" /></div></template>
        </MItem>
        <MItem plain stack :title="t.devices.deviceName">
          <template #control>
            <MField
              v-model="deviceNameInput"
              :label="t.devices.deviceName"
              :placeholder="t.devices.deviceNamePlaceholder"
              data-focusable
              data-focus-key="devices-name"
            />
          </template>
        </MItem>
        <MItem
          action
          icon="link"
          :title="pinSubmitting ? t.devices.pairing : t.devices.pairBtn"
          :disabled="pinSubmitting || pinInput.length < 4 || !deviceNameInput"
          data-focus-key="devices-pair"
          @click="submitPin"
        />
      </MGroup>
      <MNotice v-if="pinStatus" :tone="pinStatus.type === 'success' ? 'info' : 'error'" :icon="pinStatus.type === 'success' ? 'check_circle' : 'error'">{{ pinStatus.message }}</MNotice>
    </MSection>

    <!-- paired devices -->
    <MSection :title="`${t.devices.pairedDevicesTitle} · ${devices.length}`">
      <MGroup v-if="devices.length">
        <MItem v-for="device in devices" :key="device.uuid" icon="mobile" :title="device.name || t.devices.unnamedDevice" :text="device.uuid" :stack="editingUuid === device.uuid">
          <template v-if="editingUuid === device.uuid" #control>
            <MField
              v-model="editName"
              class="name-input"
              :label="t.devices.renameBtn"
              data-focusable
              @keyup.enter="saveRename(device)"
              @keyup.escape="cancelRename"
            />
          </template>
          <template #trail>
            <template v-if="editingUuid !== device.uuid">
              <MIconButton icon="edit" :label="t.devices.renameBtn" @click="startRename(device)" />
              <MIconButton icon="link_off" class="is-danger" :label="t.devices.unpairBtn" @click="unpairDevice(device)" />
            </template>
            <template v-else>
              <MIconButton icon="check" :label="t.devices.saveBtn" @click="saveRename(device)" />
              <MIconButton icon="close" :label="t.devices.cancelBtn" @click="cancelRename" />
            </template>
          </template>
        </MItem>
      </MGroup>

      <div v-else-if="loading" class="dsk-wait"><MLoading size="32px" :label="t.devices.loading" /><span>{{ t.devices.loading }}</span></div>
      <MEmpty v-else icon="mobile" :title="t.devices.emptyState" />
    </MSection>
  </DeskPage>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { MEmpty, MField, MGroup, MIconButton, MItem, MLoading, MNotice, MPinPad, MSection } from 'foundation-ui/vue'
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
.dsk-pin { display: flex; justify-content: center; padding: 8px 0; }
.dsk-pin__code { font: var(--m-title-large); letter-spacing: .14em; color: var(--md-sys-color-on-surface); font-variant-numeric: tabular-nums; }
.dsk-qr { display: flex; justify-content: center; padding: 8px 0; }
.dsk-qr img { display: block; width: 200px; height: 200px; padding: 12px; border-radius: var(--md-sys-shape-corner-large); background: #fff; }
.dsk-wait { display: flex; align-items: center; justify-content: center; gap: 12px; padding: 24px; font: var(--m-body-large); color: var(--md-sys-color-on-surface-variant); }
.is-danger { --m-icon-btn-fg: var(--md-sys-color-error); color: var(--md-sys-color-error); }
</style>
