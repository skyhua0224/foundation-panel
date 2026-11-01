<template>
  <div class="m-app m-app--scroll panel-shell">
    <div class="panel-drag" data-tauri-drag-region @pointerdown="onTouchWindowDragStart"></div>
    <div class="panel-window">
      <MIconButton icon="remove" :label="t.sidebar.minimize" @click="minimizeWindow" />
      <MIconButton :icon="isMaximized ? 'filter_none' : 'crop_square'" :label="isMaximized ? t.sidebar.restore : t.sidebar.maximize" @click="toggleMaximize" />
      <MIconButton icon="close" :label="t.sidebar.hideWindow" @click="closeWindow" />
    </div>
    <MScene :nav="nav" :rail-items="rail" :texts="sceneTexts" :overlay="isWebPage" storage-key="foundation-ui.panel.list" @rail="onRail">
      <template #list="{ entry }">
        <MNavList
          :groups="[{ id: entry.rail, items: listItems(entry.rail) }]"
          :current="nav.current.value"
          :search-items="searchItems"
          :search-placeholder="t.m3e.search"
          :search-empty="t.m3e.noResults"
          :clear-label="t.m3e.clear"
          :label="railLabel(entry.rail)"
          @navigate="(item) => router.navigate(item.id)"
          @search="onSearch"
          @context="onRowContext"
        />
      </template>
      <template #detail="{ entry }">
        <PanelHome v-if="entry.id === ROUTES.HOME" :runtime="runtime" :update-available="!!updateInfo" @open="openDestination" @webui-navigate="openWebUi" />
        <Welcome v-else-if="entry.id === ROUTES.WELCOME" @close="goHome" />
        <WebStreamSettings v-else-if="entry.id === ROUTES.WEB_STREAM" @close="goHome" />
        <VddSettings v-else-if="entry.id === ROUTES.VDD_SETTINGS" @close="back" />
        <AiAssistant v-else-if="entry.id === ROUTES.AI_ASSISTANT" @close="back" />
        <HdrEnhancedManager v-else-if="entry.id === ROUTES.HDR_ENHANCED" />
        <GamepadPage v-else-if="entry.id === ROUTES.GAMEPAD" @open-controller-meta="openGamepadTest" @manage-components="router.navigate(ROUTES.COMPONENTS)" />
        <MicrophonePage v-else-if="entry.id === ROUTES.MICROPHONE" @manage-components="router.navigate(ROUTES.COMPONENTS)" />
        <UsbPage v-else-if="entry.id === ROUTES.USB" @manage-components="router.navigate(ROUTES.COMPONENTS)" />
        <ComponentsPage v-else-if="entry.id === ROUTES.COMPONENTS" />
        <DiagnosticsPage
          v-else-if="entry.id === ROUTES.DIAGNOSTICS"
          :tools="toolsMenuItems"
          @open-controller-meta="openGamepadTest"
          @open-stylus-input-probe="openStylusInputProbe"
        />
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
        />
      </template>
      <template #supporting="{ entry }">
        <GamepadState v-if="entry.id === 'gamepad-state'" :runtime="runtime" />
      </template>
      <template #overlay>
        <PanelWebHost><slot /></PanelWebHost>
      </template>
    </MScene>
    <MContextMenu ref="contextMenu" @select="onContextSelect" />
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
import { computed, defineAsyncComponent, ref, toRef, watch, onMounted } from 'vue'
import { sunshine } from '../tauri-adapter.js'
import { MScene, MNavList, MIconButton, MContextMenu, useAnnouncementBadge } from 'foundation-ui/vue'
import PanelHome from './PanelHome.vue'
import GamepadState from './m3e/GamepadState.vue'
import PanelWebHost from './PanelWebHost.vue'
import VddSettings from './VddSettings.vue'
import Welcome from './welcome.vue'
import WebStreamSettings from './WebStreamSettings.vue'
import AiAssistant from './AiAssistant.vue'
import HdrEnhancedManager from './HdrEnhancedManager.vue'
import ControlPanelPage from './ControlPanelPage.vue'
import GamepadPage from './m3e/GamepadPage.vue'
import MicrophonePage from './m3e/MicrophonePage.vue'
import UsbPage from './m3e/UsbPage.vue'
import ComponentsPage from './m3e/ComponentsPage.vue'
import DiagnosticsPage from './m3e/DiagnosticsPage.vue'
import { microphoneOverviewState } from '../composables/deviceHubStatus.js'
import { useDeviceRuntime } from '../composables/useDeviceRuntime.js'
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
const { onTouchWindowDragStart } = useTouchWindowDrag(isMaximized)
const {
  openTimer, openUrl, cleanupCovers, restartAsAdmin, checkForUpdates, openGamepadTest, openStylusInputProbe,
  showClipboardSyncStatus, initClipboardSyncStatus, clipboardSyncEnabled,
} = useTools()
const { runtime, refresh: refreshRuntime } = useDeviceRuntime()
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

const nav = router.nav
const isWebPage = computed(() => !!webPathOf(router.currentRoute.value))
const back = () => nav.pop()
const announceVersion = ref('')
onMounted(async () => { try { announceVersion.value = (await sunshine.getVersion()) || '' } catch { /* the version only narrows version-bound notices */ } })
const { unread: announcementsUnread } = useAnnouncementBadge({ locale, version: announceVersion })
const rail = computed(() => railItems(t.value).map((item) => (item.id === 'overview' && announcementsUnread.value ? { ...item, badge: announcementsUnread.value } : item)))
const railLabel = (id) => t.value.m3e.nav[id]
const runtimeView = computed(() => ({ ...runtime, micLabel: micLabel.value }))
const micLabel = computed(() => t.value.deviceHub.microphone.statusLabels[microphoneOverviewState(runtime.mic || {})] || '')
const listItems = (id) => pageItems(t.value, id, runtimeView.value)
const searchItems = computed(() => Object.keys(PAGES)
  .filter((id) => PAGES[id].list !== false)
  .flatMap((id) => {
    const item = pageItems(t.value, PAGES[id].rail, runtimeView.value).find((p) => p.id === id)
    return item ? [{ id, title: item.label, path: railLabel(PAGES[id].rail), icon: item.icon, route: id }] : []
  }))
const sceneTexts = computed(() => ({ resize: t.value.m3e.resize, main: t.value.m3e.nav.main, shortcuts: t.value.m3e.shortcuts, back: t.value.m3e.back, search: t.value.m3e.search, help: t.value.m3e.shortcutHelp, section: t.value.m3e.section, close: t.value.m3e.clear }))
const onRail = (item) => router.navigate({ overview: ROUTES.HOME, settings: ROUTES.PANEL_SETTINGS, help: ROUTES.HELP, stream: ROUTES.CAT_STREAM, input: ROUTES.CAT_INPUT, display: ROUTES.CAT_DISPLAY, system: ROUTES.CAT_SYSTEM }[item.id])
const onSearch = (item) => router.navigate(item.route || item.id)
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
.panel-shell { position: relative; }
.panel-drag { position: fixed; top: 0; left: 0; right: 132px; height: 32px; z-index: 6; }
.panel-window { position: fixed; top: 4px; right: 8px; z-index: 40; display: flex; gap: 0; }
.panel-window .m-icon-btn { width: 40px; height: 32px; border-radius: 9999px; font-size: 18px; }
.panel-shell .m-scene--list-detail .m-scene__pane--detail .m-page, .panel-shell .m-scene--single .m-page, .panel-shell .m-scene__pane--supporting .m-page { --m-bar-end: 132px; }
</style>
