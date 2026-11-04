<script setup>
import { computed, onMounted, ref, unref, watch } from 'vue'
import { useNotify, MButton, MDialog, MExpertItem, MFooter, MGroup, MIcon, MIconButton, MItem, MPathItem, MSection, MSeg, MSheet, MSnackbar, MSwitchItem, MSwitch, useAppearance } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'
import { defaultDesktopSettings, desktopSettings, loadDesktopSettings, saveDesktopSettings } from '../../desktop/composables/useDesktopSettings.js'
import { bigScreenSettings, defaultBigScreenSettings, setBigScreenSetting } from '../../desktop/composables/useBigScreenSettings.js'
import { useLaunchHelpers } from '../../desktop/composables/useLaunchHelpers.js'
import { useDesktopPet } from '../../composables/useDesktopPet.js'
import { isTauriRuntime } from '../../desktop/composables/useTauri.js'
import { fileMapping, sunshine } from '../../tauri-adapter.js'

// Every option of the earlier desktop window, in the settings of the control panel: start up, notifications, the big screen
// mode, the launch helpers (paths), shared folders, the desktop pet and, behind 高级, the developer and log options. Nothing
// here needs the old window any more. A change is saved at once (the old window had a save button).
const props = defineProps({ locale: { type: String, default: 'zh' } })
const { t } = useI18n()
const zh = computed(() => String(props.locale).startsWith('zh'))
const st = computed(() => t.value.settings)
const settings = desktopSettings
const hasTauri = ref(false)
const notice = ref('')
// short results go to the one snackbar of the window
watch(notice, (message) => { if (message) { useNotify().snack(message); notice.value = '' } })
const sheet = ref('')
const open = (id) => { sheet.value = id }
// how many switches of a group are on, for the row that opens it (`{n} / {total}`)
const onCount = (rows, source) => { const o = unref(source) || {}; return `${rows.filter(([key]) => Boolean(o[key])).length} / ${rows.length}` }
const bigOn = computed(() => bigScreen.value.filter(([key]) => Boolean(unref(bigScreenSettings)?.[key])).length + (autoLaunchDesktop.value ? 1 : 0))
const closeSheet = (value) => { if (!value) sheet.value = '' }

onMounted(async () => {
  hasTauri.value = await isTauriRuntime()
  loadDesktopLaunch()
  await loadDesktopSettings()
  if (sheet.value === 'sharing') loadMappings()
})

async function set(key, value) {
  try { await saveDesktopSettings({ ...settings.value, [key]: value }) } catch (e) { notice.value = `${st.value.updateError || ''} ${e?.message || ''}`.trim() }
}
const startup = computed(() => [['autoStart', st.value.autoStart, st.value.autoStartDesc], ['startMinimized', st.value.startMinimized, st.value.startMinimizedDesc], ['autoStartSunshine', st.value.autoStartSunshine, st.value.autoStartSunshineDesc]])
const notifications = computed(() => [['notifications', st.value.desktopNotifications, st.value.desktopNotificationsDesc], ['connectionNotify', st.value.connectionNotify, st.value.connectionNotifyDesc], ['updateNotify', st.value.updateNotify, st.value.updateNotifyDesc]])
const bigScreen = computed(() => [['autoYieldOnLaunch', st.value.autoYield, st.value.autoYieldDesc], ['gamepadCursorEnabled', st.value.gamepadCursor, st.value.gamepadCursorDesc], ['oskAutoOpen', st.value.oskAutoOpen, st.value.oskAutoOpenDesc], ['navSounds', st.value.navSounds, st.value.navSoundsDesc]])

// ---- open the Desktop UI when a client starts the Desktop application (an entry of its `detached` commands in apps.json) ----
const GUI_DESKTOP_CMD = '.\\assets\\gui\\sunshine-gui.exe --desktop'
const isDesktopApp = (app) => ['Desktop', '桌面'].includes(app.name)
const isGuiCmd = (cmd) => cmd.includes('sunshine-gui') && (cmd.includes('--desktop') || cmd.includes('-d'))
const autoLaunchDesktop = ref(false)
const hostJson = async (path, init) => { const base = await sunshine.getProxyUrl(); return (await fetch(`${base}${path}`, init)).json() }
async function loadDesktopLaunch() {
  try {
    const resp = await hostJson('/api/apps')
    const app = (resp.apps || resp || []).find(isDesktopApp)
    autoLaunchDesktop.value = !!app && (app.detached || []).some(isGuiCmd)
  } catch { /* the host is not reachable: the switch stays off */ }
}
async function setDesktopLaunch(on) {
  const before = autoLaunchDesktop.value
  autoLaunchDesktop.value = on
  try {
    const resp = await hostJson('/api/apps')
    const apps = resp.apps || resp || []
    const index = apps.findIndex(isDesktopApp)
    if (index < 0) throw new Error('Desktop')
    const detached = (apps[index].detached || []).filter((cmd) => !isGuiCmd(cmd))
    if (on) detached.push(GUI_DESKTOP_CMD)
    const result = await hostJson('/api/apps', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ apps, editApp: { ...apps[index], detached, index } }) })
    if (String(result.status) !== 'true') throw new Error(result.error || '')
  } catch (e) { autoLaunchDesktop.value = before; notice.value = `${t.value.stream?.msg?.launchModeSaveFailed || 'Could not save'} ${e?.message || ''}`.trim() }
}

// ---- launch helpers: the path of each tool ----
const { templates, getGlobalPath, setGlobalPath } = useLaunchHelpers(t)
const helpers = computed(() => templates.value.filter((x) => x.id !== 'custom' && x.params?.some((p) => p.key === 'path')))
const browseTool = (id) => async () => {
  try {
    const { open: pick } = await import('@tauri-apps/plugin-dialog')
    const path = await pick({ filters: [{ name: t.value.launchHelper?.executableFiles || 'Executables', extensions: ['exe', 'bat', 'cmd', 'lnk', 'com', 'scr'] }, { name: t.value.launchHelper?.allFiles || 'All files', extensions: ['*'] }] })
    return typeof path === 'string' ? path : ''
  } catch { return '' }
}

// ---- shared folders ----
const fs = computed(() => t.value.fileSharing)
const mappings = ref([])
const sharing = ref({ loading: false, busy: false, error: '' })
const friendly = (e) => String(e?.message || e || fs.value.operationFailed)
async function loadMappings() {
  if (!hasTauri.value) { mappings.value = []; return }
  sharing.value.loading = true; sharing.value.error = ''
  try { mappings.value = await fileMapping.list() } catch (e) { mappings.value = []; sharing.value.error = friendly(e) } finally { sharing.value.loading = false }
}
async function addFolder() {
  sharing.value.busy = true; sharing.value.error = ''
  try {
    const { open: pick } = await import('@tauri-apps/plugin-dialog')
    const path = await pick({ directory: true, multiple: false })
    if (!path) return
    const m = await fileMapping.quickShareFolder(path)
    await loadMappings()
    notice.value = fs.value.sharedSuccess.replace('{name}', m?.name || String(path).split(/[\\/]/).pop())
  } catch (e) { sharing.value.error = friendly(e) } finally { sharing.value.busy = false }
}
const revoking = ref(null)
async function revoke() {
  const m = revoking.value; revoking.value = null
  if (!m?.id) return
  sharing.value.busy = true
  try { await fileMapping.remove(m.id); mappings.value = mappings.value.filter((x) => x.id !== m.id); notice.value = fs.value.revokedSuccess.replace('{name}', m.name || m.id) } catch (e) { sharing.value.error = friendly(e) } finally { sharing.value.busy = false }
}
async function menu(on) {
  sharing.value.busy = true
  try { on ? await fileMapping.installMenu() : await fileMapping.uninstallMenu(); notice.value = on ? fs.value.menuInstalled : fs.value.menuRemoved } catch (e) { sharing.value.error = friendly(e) } finally { sharing.value.busy = false }
}
const openSharing = () => { open('sharing'); loadMappings() }

// ---- desktop pet ----
const { petEnabled, isObserving, observeInterval, pokeFailed, startObserving, stopObserving, setIntervalSeconds, poke } = useDesktopPet()
const consent = ref(false)
const petSeconds = computed({ get: () => Math.round(observeInterval.value / 1000), set: (v) => setIntervalSeconds(Number(v)) })
const intervals = computed(() => [[15, 's15'], [30, 's30'], [60, 's60'], [120, 'm2'], [300, 'm5']].map(([value, key]) => ({ value, label: st.value.intervals[key] })))
const togglePet = (on) => { if (!on) stopObserving(); else consent.value = true }
const acceptPet = () => { consent.value = false; startObserving() }

// ---- appearance: export and import (what the old theme editor offered as theme export and import) ----
const ap = useAppearance()
const picker = ref(null)
const KEYS = ['background', 'source', 'url', 'color', 'preset', 'custom', 'mode', 'dim', 'blur']
function exportAppearance() {
  const data = Object.fromEntries(KEYS.map((k) => [k, ap.state[k]]).filter(([, v]) => v !== undefined))
  const a = document.createElement('a')
  a.href = URL.createObjectURL(new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' }))
  a.download = 'foundation-appearance.json'
  a.click()
  setTimeout(() => URL.revokeObjectURL(a.href), 1000)
}
async function importAppearance(event) {
  const file = event.target.files?.[0]
  event.target.value = ''
  if (!file) return
  try {
    const data = JSON.parse(await file.text())
    await ap.set(Object.fromEntries(KEYS.filter((k) => k in data).map((k) => [k, data[k]])))
  } catch (e) { notice.value = `${t.value.themeEditor?.importFailed || 'Import failed'} ${e?.message || ''}`.trim() }
}

// ---- 高级 (expert) ----
const X = computed(() => (zh.value
  ? { changed: '已修改', desc: '作用', effect: '影响', default: '默认值', reset: '恢复默认', on: '开', off: '关',
      devMode: '打开开发者功能（例如调试输出）；日常使用保持关闭。', textContext: '允许把选中的文字作为上下文交给助手。', logLevel: '面板写入日志的最低级别；级别越低日志越大。' }
  : { changed: 'Changed', desc: 'What it does', effect: 'Effect', default: 'Default', reset: 'Reset to default', on: 'On', off: 'Off',
      devMode: 'Turns on developer features such as debug output; leave off for daily use.', textContext: 'Lets selected text be handed to the assistant as context.', logLevel: 'The least severe message the panel writes to its log; lower levels make the log larger.' }))
const logOptions = computed(() => ['error', 'warn', 'info', 'debug'].map((value) => ({ value, label: value.toUpperCase() })))
const reset = (key) => set(key, defaultDesktopSettings[key])
const resetAll = async () => {
  try { await saveDesktopSettings({ ...defaultDesktopSettings }) } catch { /* the notice below */ }
  for (const [k, v] of Object.entries(defaultBigScreenSettings)) setBigScreenSetting(k, v)
  notice.value = st.value.resetSuccess
}
</script>

<template>
  <MSection :title="st.appearance">
    <MGroup>
      <MItem action icon="download" :title="t.themeEditor.exportTheme" data-panel-option="themeExport" @click="exportAppearance" />
      <MItem action icon="upload_file" :title="t.themeEditor.importTheme" data-panel-option="themeImport" @click="picker.click()" />
    </MGroup>
    <input ref="picker" type="file" accept="application/json,.json" hidden @change="importAppearance" />
  </MSection>
  <MSection>
    <MGroup>
      <MItem action icon="power_settings_new" :title="st.startup" data-panel-option="startup" @click="open('startup')"><template #trail><span class="m-item__count">{{ onCount(startup, settings) }}</span></template></MItem>
      <MItem action icon="notifications" :title="st.notifications" data-panel-option="notifications" @click="open('notifications')"><template #trail><span class="m-item__count">{{ onCount(notifications, settings) }}</span></template></MItem>
      <MItem action icon="tv_gen" :title="st.bigScreen" data-panel-option="bigScreen" @click="open('bigscreen')"><template #trail><span class="m-item__count">{{ bigOn }} / {{ bigScreen.length + 1 }}</span></template></MItem>
    </MGroup>
  </MSection>
  <MSection>
    <MGroup>
      <MItem action icon="bolt" :title="st.launchAssistant" data-panel-option="launchAssistant" @click="open('helpers')" />
      <MItem action icon="folder_shared" :title="fs.title" data-panel-option="fileSharing" @click="openSharing" />
      <MItem action icon="pets" :title="st.pet" data-panel-option="pet" @click="open('pet')" />
      <MItem action icon="tune" :title="st.advanced" data-panel-option="advanced" @click="open('expert')" />
    </MGroup>
  </MSection>

  <MSheet :model-value="sheet === 'startup'" :title="st.startup" side="end" :close-label="fs.close" @update:model-value="closeSheet">
    <MGroup>
      <MSwitchItem v-for="[key, name, info] in startup" :key="key" :model-value="Boolean(settings[key])" :title="name" :info="info" :data-panel-option="key" @update:model-value="set(key, $event)" />
    </MGroup>
  </MSheet>
  <MSheet :model-value="sheet === 'notifications'" :title="st.notifications" side="end" :close-label="fs.close" @update:model-value="closeSheet">
    <MGroup>
      <MSwitchItem v-for="[key, name, info] in notifications" :key="key" :model-value="Boolean(settings[key])" :title="name" :info="info" :disabled="key === 'connectionNotify' && !settings.notifications" :data-panel-option="key" @update:model-value="set(key, $event)" />
    </MGroup>
  </MSheet>
  <MSheet :model-value="sheet === 'bigscreen'" :title="st.bigScreen" side="end" :close-label="fs.close" @update:model-value="closeSheet">
    <MGroup>
      <MSwitchItem :model-value="autoLaunchDesktop" :title="t.stream.autoLaunchDesktopUI" :info="t.stream.autoLaunchDesktopUIDesc" data-panel-option="autoLaunchDesktop" @update:model-value="setDesktopLaunch" />
      <MSwitchItem v-for="[key, name, info] in bigScreen" :key="key" :model-value="Boolean(bigScreenSettings[key])" :title="name" :info="info" :data-panel-option="key" @update:model-value="setBigScreenSetting(key, $event)" />
    </MGroup>
  </MSheet>

  <MSheet :model-value="sheet === 'helpers'" :title="st.launchAssistant" side="end" :close-label="fs.close" @update:model-value="closeSheet">
    <p class="m-panel-opt__lead">{{ st.launchAssistantDesc }}</p>
    <MGroup>
      <MPathItem v-for="h in helpers" :key="h.id" :label="h.name" :info="h.description" :model-value="getGlobalPath(h.id, 'path') || ''" :placeholder="h.params.find((p) => p.key === 'path')?.placeholder || ''" :browse="hasTauri ? browseTool(h.id) : null" :browse-label="fs.chooseFolder" :data-panel-option="`tool:${h.id}`" @update:model-value="setGlobalPath(h.id, 'path', $event)" />
    </MGroup>
  </MSheet>

  <MSheet :model-value="sheet === 'sharing'" :title="fs.title" side="end" :close-label="fs.close" @update:model-value="closeSheet">
    <MGroup>
      <MItem icon="info" :title="fs.status" :text="!hasTauri ? fs.desktopOnly : sharing.loading ? fs.syncing : mappings.length ? fs.shareCount.replace('{count}', mappings.length) : fs.notShared">
        <template #trail><MIconButton icon="refresh" :label="fs.refresh" :disabled="sharing.loading || !hasTauri" @click="loadMappings" /></template>
      </MItem>
      <MItem v-for="m in mappings" :key="m.id" icon="folder" :title="m.name || m.id" :text="m.path">
        <template #trail><span class="m-item__count">{{ m.mode === 'readwrite' ? fs.readWrite : fs.readOnly }}</span><MIconButton icon="delete" :label="fs.revoke" :disabled="sharing.busy" @click="revoking = m" /></template>
      </MItem>
      <MItem action icon="add" :title="fs.sharedFolders" :text="fs.sharedFoldersDesc" :disabled="sharing.busy || !hasTauri" data-panel-option="sharedFolders" @click="addFolder" />
      <MItem icon="link" :title="fs.explorerMenu" :text="fs.explorerMenuDesc" data-panel-option="explorerMenu">
        <template #trail><MButton variant="tonal" size="sm" :disabled="sharing.busy || !hasTauri" @click="menu(true)">{{ fs.enable }}</MButton><MButton variant="text" size="sm" :disabled="sharing.busy || !hasTauri" @click="menu(false)">{{ fs.disable }}</MButton></template>
      </MItem>
    </MGroup>
    <p v-if="sharing.error" class="m-panel-opt__error" role="alert">{{ sharing.error }}</p>
    <MFooter>{{ fs.readOnly }} · {{ fs.pairedOnly }} · {{ fs.blockLinks }}</MFooter>
  </MSheet>

  <MSheet :model-value="sheet === 'pet'" :title="st.pet" side="end" :close-label="fs.close" @update:model-value="closeSheet">
    <MGroup>
      <MSwitchItem :model-value="petEnabled" :title="st.deskObserve" :info="st.deskObserveDesc" data-panel-option="deskObserve" @update:model-value="togglePet" />
      <MItem v-if="petEnabled" icon="timer" :title="st.observeInterval" :text="st.observeIntervalDesc" stack data-panel-option="observeInterval">
        <template #control><MSeg v-model="petSeconds" block :label="st.observeInterval" :options="intervals" /></template>
      </MItem>
      <MItem v-if="petEnabled" action icon="touch_app" :title="st.pokeMita" :text="pokeFailed ? t.petTool.pokeFailed : st.pokeMitaDesc" :disabled="isObserving" data-panel-option="pokeMita" @click="poke">
        <template #trail><span class="m-item__count">{{ isObserving ? st.pokeBtnObserving : st.pokeBtn }}</span></template>
      </MItem>
    </MGroup>
  </MSheet>
  <MDialog v-model="consent" :title="t.petTool.visionPrivacyConfirm.title" icon="visibility" tone="info">
    <p>{{ t.petTool.visionPrivacyConfirm.message }}</p>
    <template #actions><MButton variant="text" @click="consent = false">{{ t.petTool.visionPrivacyConfirm.cancel }}</MButton><MButton @click="acceptPet">{{ t.petTool.visionPrivacyConfirm.accept }}</MButton></template>
  </MDialog>

  <MSheet :model-value="sheet === 'expert'" :title="st.advanced" side="end" :close-label="fs.close" @update:model-value="closeSheet">
    <MGroup>
      <MExpertItem :title="st.textContextEnabled" :value="settings.textContextEnabled ? X.on : X.off" :changed="Boolean(settings.textContextEnabled) !== defaultDesktopSettings.textContextEnabled" :desc="st.textContextEnabledDesc" :effect="X.textContext" :def-text="defaultDesktopSettings.textContextEnabled ? X.on : X.off" :texts="X" data-panel-option="textContextEnabled" @reset="reset('textContextEnabled')">
        <MSwitch :model-value="Boolean(settings.textContextEnabled)" :label="st.textContextEnabled" @update:model-value="set('textContextEnabled', $event)" />
      </MExpertItem>
      <MExpertItem stack :title="st.logLevel" :value="st.logLevels[settings.logLevel]" :changed="settings.logLevel !== defaultDesktopSettings.logLevel" :desc="st.logLevelDesc" :effect="X.logLevel" :def-text="st.logLevels[defaultDesktopSettings.logLevel]" :texts="X" data-panel-option="logLevel" @reset="reset('logLevel')">
        <MSeg :model-value="settings.logLevel" block :label="st.logLevel" :options="logOptions" @update:model-value="set('logLevel', $event)" />
      </MExpertItem>
    </MGroup>
    <MButton variant="text" icon="restart_alt" @click="resetAll">{{ st.resetDefaults }}</MButton>
  </MSheet>

  <MDialog :model-value="!!revoking" :title="fs.revoke" icon="delete" tone="warning" @update:model-value="(v) => { if (!v) revoking = null }">
    <p>{{ revoking && fs.revokeConfirm.replace('{name}', revoking.name || revoking.id) }}</p>
    <template #actions><MButton variant="text" @click="revoking = null">{{ fs.close }}</MButton><MButton variant="danger" @click="revoke">{{ fs.revoke }}</MButton></template>
  </MDialog>
</template>

<style>
.m-panel-opt__lead { margin: 0 0 12px; font: var(--m-body-medium); color: var(--md-sys-color-on-surface-variant); }
.m-panel-opt__error { margin: 8px 16px 0; font: var(--m-body-small); color: var(--md-sys-color-error); }
</style>
