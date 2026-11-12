<template>
  <DeskPage class="stream-view" :title="t.stream.pageTitle" wide>
    <div v-if="loading" class="dsk-stream__wait">
      <MLoading size="40px" :label="t.stream.loading" />
      <span>{{ t.stream.loading }}</span>
    </div>

    <template v-else>
      <SettingsNotice :notice="saveNotice" />

      <MSection v-if="currentSession" :title="t.stream.hostAudio">
        <MGroup>
          <MItem icon="volume_up" :title="t.stream.hostAudio" :text="t.stream.hostAudioDesc">
            <template #trail><MSwitch :model-value="currentSession.host_audio === true" :disabled="audioBusy" :label="t.stream.hostAudio" @update:model-value="setHostAudio" /></template>
          </MItem>
        </MGroup>
      </MSection>

      <!-- the same rows as the Web UI and the phone: the bitrate is one row with the shared control (value, steps, slider, chips) -->
      <MSection>
        <MGroup>
          <MItem class="dsk-bitrate" icon="monitoring" :title="t.stream.bitrateLimit" :text="applies('max_bitrate')" stack>
            <template #trail>
              <MIconButton icon="remove" :label="`${t.stream.bitrateLimit} −`" @click="nudgeBitrate(-NUDGE_MBPS)" />
              <span class="m-slider-value dsk-bitrate__value">{{ bitrateDisplay }}</span>
              <MIconButton icon="add" :label="`${t.stream.bitrateLimit} +`" @click="nudgeBitrate(NUDGE_MBPS)" />
            </template>
            <template #control>
              <MSlider :model-value="bitrateSliderProgress(bitrateKbps)" :min="0" :max="SLIDER_MAX_PROGRESS" :step="1" unit="" :marks="bitrateChipMarks(BITRATE_PRESET_MBPS)" :off-text="t.stream.bitrateAuto" size="m" icon-start="signal_cellular_alt_2_bar" icon-end="signal_cellular_alt" :label="t.stream.bitrateLimit" @update:model-value="(v) => setCfg('max_bitrate', String(bitrateFromProgress(v)))" />
              <MSeg :model-value="bitrateKbps" block :label="t.stream.bitrateLimit" :options="BITRATE_PRESETS" @update:model-value="(v) => setCfg('max_bitrate', String(v))" />
            </template>
          </MItem>
        </MGroup>
      </MSection>

      <MSection :title="t.stream.codec.format">
        <MGroup>
          <SettingsToggleRow
            v-for="codec in codecs"
            :key="codec.key"
            icon="movie"
            :name="codec.name"
            :description="`${codec.desc} · ${applies(codec.key)}`"
            :model-value="configData[codec.key] > 0"
            stack
            @update:model-value="toggleCodec(codec.key)"
          >
            <MSeg v-if="configData[codec.key] > 0" :model-value="configData[codec.key]" block :label="codec.name" :options="codecModes" @update:model-value="(v) => setCfg(codec.key, v)" />
          </SettingsToggleRow>
        </MGroup>
      </MSection>

      <MSection>
        <MGroup>
          <!-- HDR -->
          <SettingsToggleRow
            icon="hdr_on"
            :name="t.stream.hdrAutoSwitch"
            :description="`${t.stream.hdrAutoSwitchDesc} · ${applies('hdr_prep')}`"
            :model-value="configData.hdr_prep === 'automatic'"
            @update:model-value="(on) => setCfg('hdr_prep', on ? 'automatic' : 'no_operation')"
          />
          <!-- launch mode -->
          <SettingsToggleRow
            icon="send"
            :name="t.stream.autoLaunchDesktopUI"
            :description="t.stream.autoLaunchDesktopUIDesc"
            :model-value="autoLaunchDesktop"
            @update:model-value="setAutoLaunch"
          />
        </MGroup>
      </MSection>

      <!-- what a sofa does not need often: display and resolution, the virtual display, the virtual mouse driver -->
      <PanelLinkSection :links="['displayModes', 'vdd', 'vmouse']" />
    </template>
  </DeskPage>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { MGroup, MIconButton, MItem, MLoading, MSection, MSeg, MSlider, MSwitch, createSerialCalls, saveSettingKey } from 'foundation-ui/vue'
import { appliesText, bitrateChips, bitrateFromProgress, bitrateKbps as parseBitrateKbps, bitrateNudge, bitrateSliderProgress, bitrateText, bitrateChipMarks, BITRATE_SLIDER_MAX_PROGRESS } from 'foundation-ui/settings/index.js'
import DeskPage from '../components/DeskPage.vue'
import PanelLinkSection from '../components/PanelLinkSection.vue'
import SettingsNotice from '../components/settings/SettingsNotice.vue'
import SettingsToggleRow from '../components/settings/SettingsToggleRow.vue'
import { useI18n } from '../i18n/index.js'

const { t, locale } = useI18n()

// The settings of this page are the ones a sofa changes: codec, bitrate, HDR. Each is saved at once through the panel's own one-key path
// (saveSettingKey: read the host's configuration, change the key, write it back), one at a time like the panel's cards. Display, resolution,
// the virtual display and the virtual mouse driver are pages of the panel (panelLinks.js).
const invoke = ref(null)
const proxyUrl = ref('http://localhost:48081')
const loading = ref(true)
const saveMsg = ref(null)
const autoLaunchDesktop = ref(false)
const appsData = ref(null) // apps.json as read
const hostPlatform = ref('')
const liveSessions = ref([])
const audioBusy = ref(false)
const currentSession = computed(() => liveSessions.value.find((s) => s?.state === 'RUNNING') || null)

// max_bitrate: the host's own default is 0 (自动, the client decides) and a config file that never wrote the key does not send it, so 0 is what shows then
const configData = ref({ hevc_mode: 2, av1_mode: 0, max_bitrate: '0', hdr_prep: 'automatic' })
const INTEGER_KEYS = ['hevc_mode', 'av1_mode']

const bitrateKbps = computed(() => parseBitrateKbps(configData.value.max_bitrate))
const SLIDER_MAX_PROGRESS = BITRATE_SLIDER_MAX_PROGRESS
const NUDGE_MBPS = 5
const bitrateDisplay = computed(() => bitrateText(bitrateKbps.value, t.value.stream.bitrateAuto))

const codecs = computed(() => [
  { key: 'hevc_mode', name: t.value.stream.codec.hevc, desc: t.value.stream.codec.hevcDesc },
  { key: 'av1_mode', name: t.value.stream.codec.av1, desc: t.value.stream.codec.av1Desc },
])
const codecModes = computed(() => [
  { value: 1, label: t.value.stream.codec.allow },
  { value: 2, label: t.value.stream.codec.always },
  { value: 3, label: t.value.stream.codec.alwaysHdr },
])
// the chips: the presets (a value that is none of them gets a chip of its own, so the stored value is always one of the choices)
// the chips and the marks of the slider are the same list: a mark stands under the value its chip writes
const BITRATE_PRESET_MBPS = [20, 50, 100, 0]
const BITRATE_PRESETS = computed(() => bitrateChips(bitrateKbps.value, t.value.stream.bitrateAuto, BITRATE_PRESET_MBPS))

const applies = (key) => appliesText(key, hostPlatform.value, locale.value)
const saveNotice = computed(() => (saveMsg.value ? { type: saveMsg.value.type, message: saveMsg.value.text } : null))
let noticeTimer = null
function say(type, text) {
  saveMsg.value = { type, text }
  clearTimeout(noticeTimer)
  noticeTimer = setTimeout(() => { saveMsg.value = null }, 5000)
}

// the step buttons beside the value (the slider itself moves by the pad's left and right, useFocusNav): 5 Mbps inside the slider's range; automatic steps up to 5 and 5 steps back to automatic
const nudgeBitrate = (delta) => setCfg('max_bitrate', String(bitrateNudge(bitrateKbps.value, delta, 800)))

async function initTauri() {
  try {
    const tauri = await import('@tauri-apps/api/core')
    invoke.value = tauri.invoke
    const url = await invoke.value('get_proxy_url_command')
    if (url) proxyUrl.value = url
  } catch (e) {
    // not in Tauri
  }
}

async function apiFetch(path, options = {}) {
  const response = await fetch(`${proxyUrl.value}${path}`, options)
  return await response.json()
}
async function refreshRuntimeSessions() {
  try {
    const data = await apiFetch('/api/runtime/sessions')
    liveSessions.value = Array.isArray(data?.sessions) ? data.sessions : []
  } catch { liveSessions.value = [] }
}
async function setHostAudio(enabled) {
  const session = currentSession.value
  if (!session || audioBusy.value) return
  audioBusy.value = true
  try {
    const data = await apiFetch('/api/runtime/audio', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ clientname: session.client_name, enabled }) })
    if (!data?.success) throw new Error(data?.status_message || 'host audio')
    await refreshRuntimeSessions()
  } catch (e) { say('error', e?.message || t.value.stream.msg.connectionError) } finally { audioBusy.value = false }
}
const hostApi = {
  getJson: (path) => apiFetch(path),
  postJson: (path, body) => apiFetch(path, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) }),
}

async function loadSettings() {
  loading.value = true
  try {
    const data = await apiFetch('/api/config')
    hostPlatform.value = data.platform || ''
    if (data.status?.toString() === 'true') {
      for (const key of Object.keys(configData.value)) {
        if (data[key] == null) continue
        configData.value[key] = INTEGER_KEYS.includes(key) ? parseInt(data[key]) || 0 : data[key]
      }
    }
    // does the Desktop app of the host start the big screen with the stream
    try {
      const appsResp = await apiFetch('/api/apps')
      const appsList = appsResp.apps || appsResp || []
      appsData.value = { apps: appsList, env: appsResp.env || {} }
      const desktopApp = appsList.find(isDesktopApp)
      if (desktopApp) {
        const detached = desktopApp.detached || []
        autoLaunchDesktop.value = detached.some((cmd) => cmd.includes('sunshine-gui') && (cmd.includes('--desktop') || cmd.includes('-d')))
      }
    } catch (e) {
      // apps load failed
    }
  } catch (e) {
    console.error('Failed to load settings:', e)
    say('error', t.value.stream.msg.connectionError)
  } finally {
    loading.value = false
  }
}

// One key at a time, in the order asked; a change still waiting takes the newest value of its key.
const saveKey = createSerialCalls(async ({ key, value, previous }) => {
  const result = await saveSettingKey(hostApi, key, value)
  if (result.ok) {
    say('success', `${t.value.stream.msg.saveSuccess} · ${applies(key)}`)
    return
  }
  configData.value[key] = previous
  say('error', result.error === 'read' ? t.value.stream.msg.readFailed : (typeof result.error === 'string' && result.error !== 'write' ? result.error : t.value.stream.msg.saveFailed))
}, ({ key }) => key)

function setCfg(key, value) {
  const previous = configData.value[key]
  if (previous === value) return
  configData.value[key] = value
  void saveKey({ key, value, previous })
}

function toggleCodec(key) {
  setCfg(key, configData.value[key] > 0 ? 0 : 2)
}

const GUI_DESKTOP_CMD = '.\\assets\\gui\\sunshine-gui.exe --desktop'
const DESKTOP_APP_NAMES = ['Desktop', '桌面']
function isDesktopApp(app) { return DESKTOP_APP_NAMES.includes(app.name) }

async function setAutoLaunch(on) {
  const previous = autoLaunchDesktop.value
  autoLaunchDesktop.value = on
  if (!(await saveDesktopLaunchMode())) autoLaunchDesktop.value = previous
}

// the Desktop app of the host starts the big screen with the stream: its `detached` list carries the command
async function saveDesktopLaunchMode() {
  if (!appsData.value) return false
  const appsList = appsData.value.apps || []
  const desktopIdx = appsList.findIndex(isDesktopApp)
  if (desktopIdx === -1) return false

  const desktopApp = { ...appsList[desktopIdx] }
  let detached = [...(desktopApp.detached || [])]
  detached = detached.filter((cmd) => !(cmd.includes('sunshine-gui') && (cmd.includes('--desktop') || cmd.includes('-d'))))
  if (autoLaunchDesktop.value) detached.push(GUI_DESKTOP_CMD)
  desktopApp.detached = detached

  try {
    const editApp = { ...desktopApp, index: desktopIdx }
    const appsResult = await hostApi.postJson('/api/apps', { apps: appsList, editApp })
    if (appsResult.status?.toString() !== 'true') {
      say('error', t.value.stream.msg.launchModeSaveFailed + ': ' + (appsResult.error || ''))
      return false
    }
    appsData.value = { ...appsData.value, apps: appsList.map((app, i) => (i === desktopIdx ? desktopApp : app)) }
    say('success', t.value.stream.msg.saveSuccess)
    return true
  } catch (e) {
    say('error', t.value.stream.msg.launchModeSaveFailed)
    return false
  }
}

onMounted(async () => {
  await initTauri()
  await loadSettings()
  await refreshRuntimeSessions()
  runtimeTimer = setInterval(refreshRuntimeSessions, 3000)
})
let runtimeTimer = null
onBeforeUnmount(() => clearInterval(runtimeTimer))
</script>

<style scoped>
.dsk-stream__wait { display: flex; align-items: center; justify-content: center; gap: 16px; padding: 64px; font: var(--m-title-medium); color: var(--md-sys-color-on-surface-variant); }
.dsk-bitrate :deep(.m-item__control) { display: flex; flex-direction: column; gap: 16px; }
.dsk-bitrate__value { min-width: 5.5em; }
</style>
