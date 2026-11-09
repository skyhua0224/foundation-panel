<template>
  <DeskPage class="stream-view" :title="t.stream.pageTitle" icon="cast_connected" :text="heroText" wide>
    <div v-if="loading" class="dsk-stream__wait">
      <MLoading size="40px" :label="t.stream.loading" />
      <span>{{ t.stream.loading }}</span>
    </div>

    <template v-else>
      <SettingsNotice :notice="saveNotice" />

      <div class="dsk-stream">
        <!-- bitrate: the number is the tile -->
        <section class="dsk-bit">
          <header class="dsk-bit__head">
            <span class="dsk-tile__icon"><MIcon name="monitoring" fill /></span>
            <h2 class="dsk-bit__title">{{ t.stream.bitrateLimit }}</h2>
          </header>
          <div class="dsk-bit__number">
            <span class="dsk-bit__value">{{ bitrateNumber }}</span>
            <span class="dsk-bit__unit">{{ bitrateUnit }}</span>
          </div>
          <div class="dsk-stream__range">
            <MIconButton icon="remove" tonal :label="`${t.stream.bitrateLimit} −`" @click="nudgeBitrate(-NUDGE_MBPS)" />
            <MSlider :model-value="bitrateSliderMbps(bitrateKbps)" :min="0" :max="SLIDER_MAX_MBPS" :step="1" unit=" Mbps" :off-text="t.stream.bitrateAuto" size="m" :label="t.stream.bitrateLimit" @update:model-value="(v) => setCfg('max_bitrate', String(bitrateFromMbps(v)))" />
            <MIconButton icon="add" tonal :label="`${t.stream.bitrateLimit} +`" @click="nudgeBitrate(NUDGE_MBPS)" />
          </div>
          <MSeg :model-value="bitrateKbps" block :label="t.stream.bitrateLimit" :options="BITRATE_PRESETS" @update:model-value="(v) => setCfg('max_bitrate', String(v))" />
          <p class="dsk-applies">{{ applies('max_bitrate') }}</p>
        </section>

        <div class="dsk-stream__col">
          <!-- codecs -->
          <section v-for="codec in codecs" :key="codec.key" class="dsk-codec" :class="{ 'is-on': configData[codec.key] > 0 }">
            <header class="dsk-codec__head">
              <span class="dsk-tile__icon"><MIcon name="movie" fill /></span>
              <div class="dsk-codec__name">
                <h2 class="dsk-codec__title">{{ codec.name }}</h2>
                <span class="dsk-codec__desc">{{ codec.desc }}</span>
              </div>
              <MSwitch :model-value="configData[codec.key] > 0" :label="codec.name" @update:model-value="toggleCodec(codec.key)" />
            </header>
            <MSeg v-if="configData[codec.key] > 0" :model-value="configData[codec.key]" block :label="codec.name" :options="codecModes" @update:model-value="(v) => setCfg(codec.key, v)" />
            <p class="dsk-applies">{{ applies(codec.key) }}</p>
          </section>

          <!-- HDR -->
          <section class="dsk-flag" :class="{ 'is-on': configData.hdr_prep === 'automatic' }">
            <span class="dsk-tile__icon"><MIcon name="hdr_on" fill /></span>
            <div class="dsk-codec__name">
              <h2 class="dsk-codec__title">{{ t.stream.hdrAutoSwitch }}</h2>
              <span class="dsk-codec__desc">{{ t.stream.hdrAutoSwitchDesc }}</span>
              <p class="dsk-applies">{{ applies('hdr_prep') }}</p>
            </div>
            <MSwitch
              :model-value="configData.hdr_prep === 'automatic'"
              :label="t.stream.hdrAutoSwitch"
              @update:model-value="(on) => setCfg('hdr_prep', on ? 'automatic' : 'no_operation')"
            />
          </section>

          <!-- launch mode -->
          <section class="dsk-flag" :class="{ 'is-on': autoLaunchDesktop }">
            <span class="dsk-tile__icon"><MIcon name="send" fill /></span>
            <div class="dsk-codec__name">
              <h2 class="dsk-codec__title">{{ t.stream.autoLaunchDesktopUI }}</h2>
              <span class="dsk-codec__desc">{{ t.stream.autoLaunchDesktopUIDesc }}</span>
            </div>
            <MSwitch :model-value="autoLaunchDesktop" :label="t.stream.autoLaunchDesktopUI" @update:model-value="setAutoLaunch" />
          </section>
        </div>
      </div>

      <!-- what a sofa does not need often: display and resolution, the virtual display, the virtual mouse driver -->
      <PanelLinkSection :links="['displayModes', 'vdd', 'vmouse']" />
    </template>
  </DeskPage>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { MIcon, MIconButton, MLoading, MSeg, MSlider, MSwitch, createSerialCalls, saveSettingKey } from 'foundation-ui/vue'
import { appliesText, bitrateChips, bitrateFromMbps, bitrateKbps as parseBitrateKbps, bitrateNudge, bitrateSliderMbps, bitrateText } from 'foundation-ui/settings/index.js'
import DeskPage from '../components/DeskPage.vue'
import PanelLinkSection from '../components/PanelLinkSection.vue'
import SettingsNotice from '../components/settings/SettingsNotice.vue'
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

// max_bitrate: the host's own default is 0 (自动, the client decides) and a config file that never wrote the key does not send it, so 0 is what shows then
const configData = ref({ hevc_mode: 2, av1_mode: 0, max_bitrate: '0', hdr_prep: 'automatic' })
const INTEGER_KEYS = ['hevc_mode', 'av1_mode']

const bitrateKbps = computed(() => parseBitrateKbps(configData.value.max_bitrate))
const SLIDER_MAX_MBPS = 200
const NUDGE_MBPS = 5
const bitrateNumber = computed(() => (bitrateKbps.value <= 0 ? t.value.stream.bitrateAuto : String(Math.round(bitrateKbps.value / 100) / 10)))
const bitrateUnit = computed(() => (bitrateKbps.value <= 0 ? '' : 'Mbps'))
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
const BITRATE_PRESETS = computed(() => bitrateChips(bitrateKbps.value, t.value.stream.bitrateAuto, [20, 50, 100, 0]))

// the hero's fact: what is in force now
const activeCodecs = computed(() => codecs.value.filter((codec) => configData.value[codec.key] > 0).map((codec) => codec.name))
const heroText = computed(() => [...activeCodecs.value, bitrateDisplay.value, configData.value.hdr_prep === 'automatic' ? 'HDR' : ''].filter(Boolean).join(' · '))
const applies = (key) => appliesText(key, hostPlatform.value, locale.value)
const saveNotice = computed(() => (saveMsg.value ? { type: saveMsg.value.type, message: saveMsg.value.text } : null))
let noticeTimer = null
function say(type, text) {
  saveMsg.value = { type, text }
  clearTimeout(noticeTimer)
  noticeTimer = setTimeout(() => { saveMsg.value = null }, 5000)
}

// the step buttons beside the slider (a pad cannot drag): 5 Mbps inside the slider's range; automatic steps up to 5 and 5 steps back to automatic
const nudgeBitrate = (delta) => setCfg('max_bitrate', String(bitrateNudge(bitrateKbps.value, delta, SLIDER_MAX_MBPS)))

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
})
</script>

<style scoped>
/* Bold, not a form: the bitrate is a number you can read from the sofa; codecs and HDR are tiles that light up when they are on. */
.dsk-stream { display: grid; grid-template-columns: minmax(0, 1fr); gap: 20px; }
.dsk-stream__col { display: flex; flex-direction: column; gap: 16px; min-width: 0; }
@media (min-width: 1500px) {
  .dsk-stream { grid-template-columns: minmax(0, 1.05fr) minmax(0, 1fr); align-items: stretch; }
  .dsk-bit { justify-content: space-between; }
}
.dsk-stream__wait { display: flex; align-items: center; justify-content: center; gap: 16px; padding: 64px; font: var(--m-title-medium); color: var(--md-sys-color-on-surface-variant); }

.dsk-tile__icon { flex: none; width: 56px; height: 56px; display: grid; place-items: center; font-size: 28px; background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary); clip-path: var(--m-shape-squircle); transition: clip-path var(--m-spring-default-spatial-duration, 300ms) var(--m-spring-default-spatial, ease); }
.dsk-bit { position: relative; overflow: hidden; isolation: isolate; display: flex; flex-direction: column; gap: 18px; padding: 28px 30px 24px; border-radius: var(--md-sys-shape-corner-extra-extra-large); background: var(--md-sys-color-primary-container); color: var(--md-sys-color-on-primary-container); }
.dsk-bit::after { content: ''; position: absolute; z-index: -1; right: -70px; top: -80px; width: 300px; height: 300px; background: var(--md-sys-color-primary); opacity: .1; clip-path: var(--m-shape-cookie); animation: m-turn 50s linear infinite; }
.dsk-bit__head { display: flex; align-items: center; gap: 14px; }
.dsk-bit__title { margin: 0; font: var(--m-headline-small-emphasized); letter-spacing: 0; }
.dsk-bit__number { display: flex; align-items: baseline; gap: 14px; }
.dsk-bit__value { font: var(--m-display-large); font-weight: 700; letter-spacing: -0.02em; line-height: 1; font-size: 128px; }
.dsk-bit__unit { font: var(--m-headline-medium-emphasized); opacity: .85; }
.dsk-stream__range { display: flex; align-items: center; gap: 12px; }
.dsk-stream__range .m-slider { flex: 1; min-width: 0; }

.dsk-codec, .dsk-flag { position: relative; display: flex; flex-direction: column; gap: 14px; padding: 20px 22px; border-radius: var(--md-sys-shape-corner-extra-large); background: var(--md-sys-color-surface-container-high); color: var(--md-sys-color-on-surface); transition: background-color var(--m-spring-default-effects-duration, 200ms) var(--m-spring-default-effects, ease), border-radius var(--m-spring-default-spatial-duration, 300ms) var(--m-spring-default-spatial, ease); }
.dsk-codec .dsk-tile__icon, .dsk-flag .dsk-tile__icon { background: var(--md-sys-color-surface-container-highest); color: var(--md-sys-color-on-surface-variant); clip-path: var(--m-shape-circle); }
.dsk-codec.is-on, .dsk-flag.is-on { background: var(--dsk-tert-bg); color: var(--dsk-tert-fg); border-radius: var(--md-sys-shape-corner-extra-extra-large); }
.dsk-codec.is-on .dsk-tile__icon, .dsk-flag.is-on .dsk-tile__icon { background: var(--md-sys-color-tertiary); color: var(--md-sys-color-on-tertiary); clip-path: var(--m-shape-cookie); }
.dsk-codec__head { display: flex; align-items: center; gap: 16px; }
.dsk-flag { flex-direction: row; align-items: center; gap: 16px; }
.dsk-codec__name { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
.dsk-codec__title { margin: 0; font: var(--m-title-large); letter-spacing: 0; }
.dsk-codec__desc { font: var(--m-body-medium); opacity: .82; }
.dsk-applies { margin: 0; font: var(--m-label-large); opacity: .72; }
</style>
