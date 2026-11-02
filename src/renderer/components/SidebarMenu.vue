<template>
  <div class="m-app m-app--scroll panel-shell">
    <!-- slim caption bar over the whole window: draggable, the window controls at the end; the pages start below it -->
    <header class="panel-caption" data-tauri-drag-region @pointerdown="onTouchWindowDragStart">
      <span class="panel-caption__title" data-tauri-drag-region>Foundation Sunshine</span>
      <div class="panel-caption__controls">
        <button type="button" class="panel-caption__btn" :aria-label="t.sidebar.minimize" :data-tip="t.sidebar.minimize" @click="minimizeWindow"><MIcon name="remove" /></button>
        <button type="button" class="panel-caption__btn" :aria-label="isMaximized ? t.sidebar.restore : t.sidebar.maximize" :data-tip="isMaximized ? t.sidebar.restore : t.sidebar.maximize" @click="toggleMaximize"><MIcon :name="isMaximized ? 'filter_none' : 'crop_square'" /></button>
        <button type="button" class="panel-caption__btn panel-caption__btn--close" :aria-label="t.sidebar.hideWindow" :data-tip="t.sidebar.hideWindow" @click="closeWindow"><MIcon name="close" /></button>
      </div>
    </header>
    <div class="panel-body">
    <MScene :nav="nav" :rail-items="rail" :texts="sceneTexts" :overlay="isWebPage" storage-key="foundation-ui.panel.list" @rail="onRail">
      <template #rail-foot><MIconButton icon="info" :label="aboutTexts(locale).title" @click="router.navigate(ROUTES.ABOUT)" /><MSponsorButton :label="t.m3e.home.sponsor" :title="t.m3e.home.sponsorTitle" :sections="sponsorSections" :close-label="t.m3e.home.done" @open="(it) => actions_openExternal(it.url)" /></template>
      <template #list="{ entry }">
        <MNavList
          :groups="listGroups(entry.rail)"
          :current="currentRow"
          :recent="entry.rail === 'config' ? settingsList.recentView.value : null"
          :search-items="searchItems"
          :search-placeholder="t.m3e.search"
          :search-empty="t.m3e.noResults"
          :clear-label="t.m3e.clear"
          :label="railLabel(entry.rail)"
          @navigate="onListNavigate"
          @search="onSearch"
          @recent="(row) => onSearch(row.entry)"
          @clear-recent="settingsList.clearRecent"
          @context="onRowContext"
        />
      </template>
      <template #detail="{ entry }">
        <PanelHome v-if="entry.id === ROUTES.HOME" :runtime="runtime" :update-available="!!updateInfo" @open="openDestination" @webui-navigate="openWebUi" />
        <Welcome v-else-if="entry.id === ROUTES.WELCOME" @close="goHome" @webui-navigate="openWebUi" />
        <WebStreamSettings v-else-if="entry.id === ROUTES.WEB_STREAM" @close="goHome" />
        <VddSettings v-else-if="entry.id === ROUTES.VDD_SETTINGS" @close="back" />
        <AiAssistant v-else-if="entry.id === ROUTES.AI_ASSISTANT" @close="back" />
        <HdrEnhancedManager v-else-if="entry.id === ROUTES.HDR_ENHANCED" />
        <GamepadPage v-else-if="entry.id === ROUTES.GAMEPAD || entry.id === ROUTES.CFG_DUALSENSE" :params="entry.params" @open-controller-meta="openGamepadTest" @manage-components="router.navigate(ROUTES.COMPONENTS)" />
        <AppearancePage v-else-if="entry.id === ROUTES.CFG_APPEARANCE" />
        <MicrophonePage v-else-if="entry.id === ROUTES.MICROPHONE" @manage-components="router.navigate(ROUTES.COMPONENTS)" />
        <UsbPage v-else-if="entry.id === ROUTES.USB" @manage-components="router.navigate(ROUTES.COMPONENTS)" />
        <ComponentStatusPage v-else-if="entry.id === ROUTES.COMPONENT_STATUS" />
        <ComponentsPage v-else-if="entry.id === ROUTES.COMPONENTS" />
        <DiagnosticsPage
          v-else-if="entry.id === ROUTES.DIAGNOSTICS"
          :tools="toolsMenuItems"
          @open-controller-meta="openGamepadTest"
          @open-stylus-input-probe="openStylusInputProbe"
        />
        <AboutPage v-else-if="entry.id === ROUTES.ABOUT" :current-version="currentVersion" :update-info="updateInfo" :locale="locale" :check="handleCheckForUpdates" @open-update="showUpdateDialog = true" @open="actions_openExternal" />
        <ControlPanelPage
          v-else-if="entry.id === ROUTES.PANEL_SETTINGS || entry.id === ROUTES.HELP"
          :page="entry.id"
          :tools="toolsMenuItems"
          :theme-mode="themeMode"
          :locale="locale"
          :include-prerelease="includePrerelease"
          :current-version="currentVersion"
          :clipboard-sync-enabled="clipboardSyncEnabled"
          :is-admin="isAdmin"
          :actions="toolsCtx"
          @theme-change="setTheme"
          @locale-change="locale = $event"
          @channel-change="setIncludePrerelease"
          @webui-navigate="openWebUi"
          @open-guide="openWelcome"
          @open-about="router.navigate(ROUTES.ABOUT)"
        />
      </template>
      <template #overlay>
        <PanelWebHost :sub="frameSub"><slot /></PanelWebHost>
      </template>
    </MScene>
    </div>
    <MContextMenu ref="contextMenu" @select="onContextSelect" />
    <MThanksLine v-if="thanks.visible.value" :locale="locale" @star="thanksStar" @sponsor="thanksSponsor" @dismiss="thanks.dismiss" />
    <UpdateDialog
      v-if="showUpdateDialog"
      v-model="showUpdateDialog"
      :update-info="updateInfo"
      :current-version="currentVersion"
      @close="showUpdateDialog = false"
      @skip-version="skipVersion"
    />
  </div>
</template>

<script setup>
import { computed, defineAsyncComponent, ref, toRef, watch, onMounted, onBeforeUnmount } from 'vue'
import { sunshine, openExternalUrl } from '../tauri-adapter.js'
import { SPONSORS } from '../config/sponsors.js'
import { MScene, MNavList, MIconButton, MIcon, MContextMenu, MThanksLine, aboutTexts, statusTexts, useAnnouncementBadge, useSettingsList, useAppearance, registerModeHandler, MSponsorButton, useThanks } from 'foundation-ui/vue'
import { settingsHash } from 'foundation-ui/settings/index.js'
import PanelHome from './PanelHome.vue'
import PanelWebHost from './PanelWebHost.vue'
import VddSettings from './VddSettings.vue'
import Welcome from './welcome.vue'
import WebStreamSettings from './WebStreamSettings.vue'
import AiAssistant from './AiAssistant.vue'
import HdrEnhancedManager from './HdrEnhancedManager.vue'
import ControlPanelPage from './ControlPanelPage.vue'
import GamepadPage from './m3e/GamepadPage.vue'
import MicrophonePage from './m3e/MicrophonePage.vue'
import AppearancePage from './m3e/AppearancePage.vue'
import UsbPage from './m3e/UsbPage.vue'
import AboutPage from './m3e/AboutPage.vue'
import ComponentStatusPage from './m3e/ComponentStatusPage.vue'
import { useComponentStatus } from '../composables/useComponentStatus.js'
import ComponentsPage from './m3e/ComponentsPage.vue'
import DiagnosticsPage from './m3e/DiagnosticsPage.vue'
import { microphoneOverviewState } from '../composables/deviceHubStatus.js'
import { useDeviceRuntime } from '../composables/useDeviceRuntime.js'
import { useGuideAutoOpen } from '../composables/useGuideAutoOpen.js'
import { useSidebarState } from '../composables/useSidebarState.js'
import { useWindowControls } from '../composables/useWindowControls.js'
import { useTools } from '../composables/useTools.js'
import { useTouchWindowDrag } from '../composables/useTouchWindowDrag.js'
import { createUtilityTools } from '../composables/toolsRegistry.js'
import { ROUTES, PAGES, routeForWebPath, webPathOf } from '../composables/useRouter.js'
import { railItems, pageItems } from '../composables/panelNav.js'
import { useI18n } from '../desktop/i18n/index.js'
const UpdateDialog = defineAsyncComponent(() => import('./UpdateDialog.vue'))
const props = defineProps({ webuiPath: { type: String, default: '/' } })
const emit = defineEmits(['route-change', 'webui-navigate'])
const { t, locale } = useI18n()
const {
  isMaximized, isAdmin, showUpdateDialog, updateInfo, currentVersion, router, themeMode, setTheme,
  openVddSettings, openWelcome, openWebStream, openAiAssistant, openControllers, openRtxHdr, skipVersion,
  includePrerelease, setIncludePrerelease,
} = useSidebarState()
const { minimizeWindow, toggleMaximize, closeWindow } = useWindowControls(isMaximized)
// the shared appearance (wallpaper, colour, light/dark); light/dark goes through the panel's own theme switch
registerModeHandler((mode) => setTheme(mode))
onMounted(() => { void useAppearance().init() })
const { onTouchWindowDragStart } = useTouchWindowDrag(isMaximized)
const {
  openTimer, openUrl, cleanupCovers, restartAsAdmin, checkForUpdates, openGamepadTest, openStylusInputProbe,
  showClipboardSyncStatus, initClipboardSyncStatus, clipboardSyncEnabled,
} = useTools()
const { runtime, refresh: refreshRuntime } = useDeviceRuntime()
// First run: the guide opens by itself when the host has no setup and no paired client
useGuideAutoOpen(openWelcome)
onMounted(() => {
  initClipboardSyncStatus()
  refreshRuntime()
})

const openWebUi = (path) => {
  const route = routeForWebPath(path)
  if (webPathOf(route)) emit('webui-navigate', path)
  router.navigate(route)
}
const goHome = () => openWebUi('/')
const handleCheckForUpdates = async (channel = null) => {
  const result = await checkForUpdates(channel)
  if (!result) return false
  updateInfo.value = result
  showUpdateDialog.value = true
  return true
}
const toolsCtx = {
  t, router, webuiPath: toRef(props, 'webuiPath'), openWebUi, openControllers, openTimer, openUrl, openGamepadTest,
  openStylusInputProbe, cleanupCovers, restartAsAdmin, showClipboardSyncStatus, handleCheckForUpdates,
}
const toolsMenuItems = computed(() => createUtilityTools(toolsCtx))
// the component status: one controller for the page and for the home cards the catalogue offers
const componentStatus = useComponentStatus({
  locale,
  go: { vdd: () => router.navigate(ROUTES.VDD_SETTINGS), components: () => router.navigate(ROUTES.COMPONENTS), microphone: () => router.navigate(ROUTES.MICROPHONE), page: () => router.navigate(ROUTES.COMPONENT_STATUS) },
  update: () => ({ state: updateInfo.value ? (updateInfo.value.is_latest ? 'latest' : 'available') : '', next: updateInfo.value?.version || '' }),
  check: () => handleCheckForUpdates(),
})

const sponsorSections = computed(() => [
  { title: t.value.m3e.home.sponsorTitle, items: SPONSORS.map((x) => ({ id: x.id, icon: 'favorite', title: x.label, url: x.url })) },
  { items: [{ id: 'star', icon: 'star', title: t.value.m3e.home.star, url: 'https://github.com/AlkaidLab/foundation-sunshine' }, { id: 'qq', icon: 'groups', title: t.value.m3e.home.qq, url: 'https://qm.qq.com/q/AfMQoyKrkc' }] },
])
const actions_openExternal = (url) => openExternalUrl(url)
const nav = router.nav
const isWebPage = computed(() => !!webPathOf(router.currentRoute.value))
const back = () => nav.pop()
const announceVersion = ref('')
onMounted(async () => { try { announceVersion.value = (await sunshine.getVersion()) || '' } catch { /* the version only narrows version-bound notices */ } })
const { unread: announcementsUnread } = useAnnouncementBadge({ locale, version: announceVersion })
// a dot on the help rail while an update is waiting (the about page checks and opens the update sheet)
const updateWaiting = computed(() => !!updateInfo.value && !updateInfo.value.is_latest)
const rail = computed(() => railItems(t.value).map((item) => (item.id === 'overview' && announcementsUnread.value ? { ...item, badge: announcementsUnread.value } : item.id === 'help' && updateWaiting.value ? { ...item, badge: 'dot' } : item)))
// the one-time thank-you line after the guided first run
const thanks = useThanks({ guideKey: 'foundation.guide.panel' })
const thanksStar = () => { actions_openExternal('https://github.com/AlkaidLab/foundation-sunshine'); thanks.dismiss() }
const thanksSponsor = () => { thanks.dismiss(); router.navigate(ROUTES.ABOUT) }
const railLabel = (id) => t.value.m3e.nav[id]
const runtimeView = computed(() => ({ ...runtime, micLabel: micLabel.value, statusTitle: statusTexts(locale.value).title }))
const micLabel = computed(() => t.value.deviceHub.microphone.statusLabels[microphoneOverviewState(runtime.mic || {})] || '')
const listItems = (id) => pageItems(t.value, id, runtimeView.value)
// The stream settings are the shared model (categories in groups with a live line, search of every setting, recently used)
const hostConfig = ref(null)
const dualsenseKeys = ref([])
const settingsList = useSettingsList({ locale, config: hostConfig, dualsenseKeys })
const loadHostData = async () => {
  try {
    const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
    const [config, tuning] = await Promise.allSettled([fetch(`${base}/api/config`).then((r) => (r.ok ? r.json() : null)), fetch(`${base}/api/dualsense/tuning`).then((r) => (r.ok ? r.json() : null))])
    if (config.status === 'fulfilled' && config.value) hostConfig.value = config.value
    if (tuning.status === 'fulfilled' && tuning.value?.keys) dualsenseKeys.value = tuning.value.keys
  } catch { /* the list shows no live lines and the search the pages only */ }
}
const listGroups = (rail) => (rail === 'config' ? settingsList.groups.value : [{ id: rail, items: listItems(rail) }])
const currentRow = computed(() => {
  const route = nav.current.value
  if (route === ROUTES.WEB_CONFIG) return `config:${nav.detail.value?.params?.tab || 'general'}`
  if (route === ROUTES.CFG_DUALSENSE) return 'config:input' // the DualSense page sits under the gamepad of 输入
  if (route === ROUTES.CFG_APPEARANCE) return 'config:appearance'
  if (route === ROUTES.WEB_PASSWORD) return 'password'
  return route
})
const searchItems = computed(() => [
  ...Object.keys(PAGES)
    .filter((id) => PAGES[id].list !== false)
    .flatMap((id) => {
      const item = pageItems(t.value, PAGES[id].rail, runtimeView.value).find((p) => p.id === id)
      return item ? [{ id, title: item.label, path: railLabel(PAGES[id].rail), icon: item.icon, route: id }] : []
    }),
  ...settingsList.searchItems.value,
])
// A place of the stream settings: the list pane opens the Web page in the frame at that tab, section and row
const openConfig = (params = { tab: 'general' }) => {
  emit('webui-navigate', `/config${settingsHash(params)}`)
  router.navigate(ROUTES.WEB_CONFIG, params)
}
const openSettingsRoute = (item) => {
  if (item.route === 'config' && item.params?.tab === 'appearance') router.navigate(ROUTES.CFG_APPEARANCE)
  else if (item.route === 'config') openConfig(item.params)
  else if (item.route === 'dualsense') router.navigate(ROUTES.CFG_DUALSENSE, item.params)
  else if (item.route === 'password') openRoute(ROUTES.WEB_PASSWORD)
  else openRoute(item.route || item.id)
}
const onListNavigate = (item) => (item.route ? openSettingsRoute(item) : openRoute(item.id))

const sceneTexts = computed(() => ({ resize: t.value.m3e.resize, main: t.value.m3e.nav.main, shortcuts: t.value.m3e.shortcuts, back: t.value.m3e.back, search: t.value.m3e.search, help: t.value.m3e.shortcutHelp, section: t.value.m3e.section, close: t.value.m3e.clear }))
const onRail = (item) => {
  if (item.id === 'config') { loadHostData(); openConfig({ tab: 'general' }); router.navigate(ROUTES.CAT_CONFIG); return }
  // the stream rail opens on its first page: the frame has to show it
  if (item.id === 'stream') { emit('webui-navigate', webPathOf(ROUTES.WEB_CLIENTS)); router.navigate(ROUTES.CAT_STREAM); return }
  router.navigate({ overview: ROUTES.HOME, settings: ROUTES.PANEL_SETTINGS, help: ROUTES.HELP, stream: ROUTES.CAT_STREAM, input: ROUTES.CAT_INPUT, display: ROUTES.CAT_DISPLAY, system: ROUTES.CAT_SYSTEM }[item.id])
}
const onSearch = (item) => {
  settingsList.remember(item.keyName)
  openSettingsRoute(item)
}
// A setting changed in the frame joins the recently used list
const frameSub = ref(false)
const onFrameMessage = (event) => {
  const data = event.data
  if (![...document.querySelectorAll('iframe')].some((frame) => frame.contentWindow === event.source)) return
  if (data?.type === 'foundation-subpage') frameSub.value = data.open === true
  else if (data?.type === 'foundation-settings-recent' && typeof data.key === 'string') settingsList.remember(data.key)
  // a page of the frame that the panel shows natively (the DualSense tuning, opened from the gamepad rows of 输入)
  else if (data?.type === 'foundation-open' && data.page === 'dualsense') router.navigate(ROUTES.CFG_DUALSENSE)
}
onMounted(() => { window.addEventListener('message', onFrameMessage); loadHostData(); componentStatus.start(60000) })
onBeforeUnmount(() => { window.removeEventListener('message', onFrameMessage); componentStatus.stop() })
const contextMenu = ref(null)
const onRowContext = (event, item) => {
  const items = [{ id: 'open', label: t.value.m3e.ctx.open, icon: 'open_in_new' }]
  if (webPathOf(item.id)) items.push({ id: 'browser', label: t.value.m3e.ctx.browser, icon: 'public' })
  contextMenu.value?.show(event, items, item)
}
const onContextSelect = (entry, item) => {
  if (entry.id === 'open') router.navigate(item.id)
  else if (entry.id === 'browser') actions_openUrl(item)
}
const actions_openUrl = async (item) => {
  const base = String((await sunshine.getUrl()) || '').replace(/\/$/, '')
  if (base) toolsCtx.openUrl(base + webPathOf(item.id))
}
// Old call sites speak in short ids; each maps to a destination.
const openDestination = (id) => {
  if (id === 'welcome') { openWelcome(); return }
  if (id.startsWith('downloads:')) { emit('webui-navigate', `/downloads?platform=${encodeURIComponent(id.slice(10))}`); router.navigate(ROUTES.WEB_DOWNLOADS); return }
  const web = { clients: ROUTES.WEB_CLIENTS, apps: ROUTES.WEB_APPS, settings: ROUTES.WEB_CONFIG, downloads: ROUTES.WEB_DOWNLOADS }
  if (id === 'settings') { openConfig({ tab: 'general' }); return }
  const route = web[id] || { gamepad: ROUTES.GAMEPAD, microphone: ROUTES.MICROPHONE, usb: ROUTES.USB, vdd: ROUTES.VDD_SETTINGS, quality: ROUTES.HDR_ENHANCED, components: ROUTES.COMPONENTS, diagnostics: ROUTES.DIAGNOSTICS, assistant: ROUTES.AI_ASSISTANT }[id]
  if (route) openRoute(route)
}
const openRoute = (route) => {
  const path = webPathOf(route)
  if (path) emit('webui-navigate', path)
  router.navigate(route)
}
watch(
  () => router.currentRoute.value,
  (to, from) => {
    if (to !== from) emit('route-change', { from, to })
  },
)
defineExpose({
  openVddSettings, openWelcome, openWebStream, openAiAssistant, openControllers, openRtxHdr,
  goHome, checkForUpdates: handleCheckForUpdates, router,
})
</script>

<style lang="less">
.panel-shell { position: relative; display: flex; flex-direction: column; height: 100vh; height: 100dvh; min-height: 0; overflow: hidden; }
.panel-caption { flex: none; position: relative; z-index: 40; display: flex; align-items: stretch; justify-content: space-between; height: 40px; background: var(--m-page); user-select: none; }
.panel-caption__title { align-self: center; padding-inline-start: 16px; font: 600 12px/1 var(--md-ref-typeface-brand); letter-spacing: .02em; color: var(--md-sys-color-on-surface-variant); }
.panel-caption__controls { display: flex; }
.panel-caption__btn { display: grid; place-items: center; width: 46px; height: 40px; padding: 0; border: 0; background: transparent; color: var(--md-sys-color-on-surface); font-size: 18px; cursor: default; transition: background-color 120ms linear, color 120ms linear; }
.panel-caption__btn:hover { background: color-mix(in srgb, var(--md-sys-color-on-surface) 9%, transparent); }
.panel-caption__btn:active { background: color-mix(in srgb, var(--md-sys-color-on-surface) 14%, transparent); }
.panel-caption__btn--close:hover { background: #c42b1c; color: #fff; } .panel-caption__btn--close:active { background: #b02417; color: #fff; }
.panel-caption__btn:focus-visible { outline: 2px solid var(--md-sys-color-primary); outline-offset: -2px; }
.panel-body { position: relative; flex: 1; min-height: 0; }
.panel-body > .m-scene { position: absolute; inset: 0; height: auto; }
</style>
