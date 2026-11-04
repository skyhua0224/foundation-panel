<template>
  <div class="m-app m-app--scroll panel-shell">
    <!-- slim caption bar over the whole window: draggable, the window controls at the end; the pages start below it -->
    <MCaption bem="panel-caption" data-tauri-drag-region @pointerdown="onTouchWindowDragStart">
      <template #brand><MBrand class="panel-caption__brand" :size="20" data-tauri-drag-region /></template>
      <template #announcements><MStatusBell :locale="locale" /><MAnnouncementButton :unread="announcements.unread.value" :locale="locale" @open="router.navigate(ROUTES.ANNOUNCEMENTS)" /></template>
      <template #search><div class="panel-caption__search" data-tauri-drag-region><MUnifiedSearch :search="uni" :texts="searchT" :compact="compactSearch" :height="38" /></div></template>
      <template #connect><button type="button" class="m-caption-connect" :aria-label="connectT.title" :data-tip="connectT.title" @click="connect.show('pair')"><MIcon name="person_add" /><span>{{ connectT.title }}</span></button></template>
      <template #controls><div class="panel-caption__controls">
        <button type="button" class="panel-caption__btn" :aria-label="t.sidebar.minimize" :data-tip="t.sidebar.minimize" @click="minimizeWindow"><MIcon name="remove" /></button>
        <button type="button" class="panel-caption__btn" :aria-label="isMaximized ? t.sidebar.restore : t.sidebar.maximize" :data-tip="isMaximized ? t.sidebar.restore : t.sidebar.maximize" @click="toggleMaximize"><MIcon :name="isMaximized ? 'filter_none' : 'crop_square'" /></button>
        <button type="button" class="panel-caption__btn panel-caption__btn--close" :aria-label="t.sidebar.hideWindow" :data-tip="t.sidebar.hideWindow" @click="closeWindow"><MIcon name="close" /></button>
      </div></template>
    </MCaption>
    <div class="panel-body">
    <MScene :nav="nav" :caption-height="48" :rail-items="rail" :bar-items="bar" :bar-current="barCur" :texts="sceneTexts" :overlay="isWebPage" storage-key="foundation-ui.panel.list" @rail="onRail">
      <template #rail-foot><MIconButton icon="palette" :label="appearanceLabel" :current="router.currentRoute.value === ROUTES.CFG_APPEARANCE" @click="router.navigate(ROUTES.CFG_APPEARANCE)" /><MIconButton icon="info" :label="aboutTexts(locale).title" :current="router.currentRoute.value === ROUTES.ABOUT" @click="router.navigate(ROUTES.ABOUT)" /><MSponsorButton :label="t.m3e.home.sponsor" :title="t.m3e.home.sponsorTitle" :sections="sponsorSections" :close-label="t.m3e.home.done" @open="(it) => actions_openExternal(it.url)" /></template>
      <template #list="{ entry }">
        <MNavList
          :groups="listGroups(entry.rail)"
          :current="currentRow"
          :label="railLabel(entry.rail)"
          @navigate="onListNavigate"
          @context="onRowContext"
        />
      </template>
      <template #detail="{ entry }">
        <MHostOffline v-if="offlineShown" :state="hostConn.state" :locale="locale" can-restart can-user-mode can-logs @retry="hostConn.retry" @restart="hostConn.restart('service')" @user-mode="hostConn.restart('user')" @logs="router.navigate(ROUTES.DIAGNOSTICS)" />
        <PanelHome v-else-if="entry.id === ROUTES.HOME" :runtime="runtime" :update-available="!!updateInfo" @open="openDestination" @webui-navigate="openWebUi" />
        <Welcome v-else-if="entry.id === ROUTES.WELCOME" @close="goHome" @webui-navigate="openWebUi" />
        <WebStreamSettings v-else-if="entry.id === ROUTES.WEB_STREAM" @close="goHome" />
        <VddSettings v-else-if="entry.id === ROUTES.VDD_SETTINGS" @close="back" />
        <AiAssistant v-else-if="entry.id === ROUTES.AI_ASSISTANT" @close="back" />
        <SettingsHubPage v-else-if="entry.id === ROUTES.SETTINGS_HUB" :lines="settingsList.lines.value" @select="onRail" />
        <HdrEnhancedManager v-else-if="entry.id === ROUTES.HDR_ENHANCED" @manage-components="router.navigate(ROUTES.COMPONENTS)" />
        <AppearancePage v-else-if="entry.id === ROUTES.CFG_APPEARANCE" />
        <MicrophonePage v-else-if="entry.id === ROUTES.MICROPHONE" @manage-components="router.navigate(ROUTES.COMPONENTS)" />
        <UsbPage v-else-if="entry.id === ROUTES.USB" @manage-components="router.navigate(ROUTES.COMPONENTS)" />
        <ComponentStatusPage v-else-if="entry.id === ROUTES.COMPONENT_STATUS" />
        <ToolboxPage v-else-if="entry.id === ROUTES.TOOLBOX" :tools="toolsMenuItems" />
        <LiveSessionPage v-else-if="entry.id === ROUTES.LIVE_SESSION" />
        <RtssPage v-else-if="entry.id === ROUTES.RTSS" />
        <DiagnosticsPage
          v-else-if="entry.id === ROUTES.DIAGNOSTICS"
          @go="goDiagnosticsPlace"
          @webui-navigate="openWebUi"
          @open-guide="openWelcome"
        />
        <MAnnouncementPage v-else-if="entry.id === ROUTES.ANNOUNCEMENTS" :items="announcements.list.value" :locale="locale" :acked-at="ackedAt" :open-id="entry.params?.id || ''" :on-refresh="() => announcements.refresh(true)" @dismiss="announcements.dismiss" @read="announcements.read" @open-url="actions_openExternal" @back="back" />
        <AboutPage v-else-if="entry.id === ROUTES.ABOUT" :current-version="currentVersion" :update-info="updateInfo" :locale="locale" :check="handleCheckForUpdates" :on-refresh="() => announcements.refresh(true)" :consent="telemetry.consent.value" @consent="telemetry.answer" :include-prerelease="includePrerelease" @channel-change="setIncludePrerelease" @open-update="showUpdateDialog = true" @open="actions_openExternal" />
        <ControlPanelPage
          v-else-if="entry.id === ROUTES.PANEL_SETTINGS || entry.id === ROUTES.HELP"
          :page="entry.id"
          :tools="toolsMenuItems"
          :theme-mode="themeMode"
          :locale="locale" :locale-choice="localeChoice" :system-locale="systemLocale"
          :include-prerelease="includePrerelease"
          :current-version="currentVersion"
          :clipboard-sync-enabled="clipboardSyncEnabled"
          :is-admin="isAdmin"
          :actions="toolsCtx"
          @theme-change="setTheme"
          @locale-change="setLocaleChoice"
          @channel-change="setIncludePrerelease"
          @webui-navigate="openWebUi"
          @open-guide="openWelcome"
          @open-remote="remoteOpen = true"
          @open-about="router.navigate(ROUTES.ABOUT)"
        />
      </template>
      <template #overlay>
        <!-- the frame stays mounted while the host is away (it only is hidden): it loads again when the host is back -->
        <div v-show="!offlineShown" class="panel-web-keep"><PanelWebHost :sub="frameSub" :page="isWebPage ? router.currentRoute.value : ''"><slot /></PanelWebHost></div>
        <MHostOffline v-if="offlineShown" :state="hostConn.state" :locale="locale" can-restart can-user-mode can-logs @retry="hostConn.retry" @restart="hostConn.restart('service')" @user-mode="hostConn.restart('user')" @logs="router.navigate(ROUTES.DIAGNOSTICS)" />
      </template>
    </MScene>
    </div>
    <MContextMenu ref="contextMenu" @select="onContextSelect" />
    <!-- every uncaught error is drawn in our own surface: the way back, the restart of Sunshine, then copy, download and the report to the developers -->
    <MTelemetryConsent v-model="consentOpen" :locale="locale" @answer="telemetry.answer" />
    <MErrorHost :locale="locale" :can-go-back="nav.canPop.value" :host-online="hostConn.state.status === 'online'" :restart="() => hostConn.restart('service')" :open-url="actions_openExternal" @back="errorBack" />
    <!-- 更多 of the bottom bar: what the bar has no room for -->
    <MSheet v-model="moreOpen" :title="t.m3e.nav.moreTitle" side="end" :close-label="t.m3e.home.done">
      <MGroup>
        <MItem action icon="search" :title="t.m3e.search" @click="moreGo(() => uni.show())" />
        <MItem action icon="settings" :title="t.m3e.nav.settings" :text="settingsList.lines.value.panel" @click="moreGo(() => onRail({ id: 'settings' }))" />
        <MItem action icon="palette" :title="appearanceLabel" @click="moreGo(() => router.navigate(ROUTES.CFG_APPEARANCE))" />
        <MItem action icon="help" :title="t.m3e.nav.help" :text="settingsList.lines.value.help" :tone="updateWaiting ? 'warn' : ''" @click="moreGo(() => onRail({ id: 'help' }))" />
        <MItem action icon="favorite" :title="t.m3e.home.sponsor" @click="moreGo(() => { sponsorOpen = true })" />
      </MGroup>
    </MSheet>
    <MAnnouncementPopup :item="announcements.popup.value" :locale="locale" @shown="announcements.markShown" @acknowledge="announcements.acknowledge" @hide="announcements.hide" @open-url="actions_openExternal" />
    <MCoachMarks :locale="locale" storage-key="foundation.coach.panel" guide-key="foundation.guide.panel" />
    <MConnectDialog v-model="connect.open.value" v-model:tab="connect.tab.value" v-model:os="connect.os.value" :assets-of="connect.assetsOf" :version-of="connect.versionOf" :locale="locale" :qr="connect.matrix" :pair-qr="connect.pairQr" :count="hostClients.length" :latest="hostClients.length ? hostClients[hostClients.length - 1].name : ''" :close-label="t.m3e.home.done"
      @pair-pin="(e) => connect.pairPin(e.pin)" @pair-qr-begin="connect.qrBegin()" @pair-qr-stop="connect.qrStop()" @open-url="actions_openExternal" @open-downloads="() => { connect.open.value = false; openRoute(ROUTES.WEB_DOWNLOADS) }" />
    <MPairPrompt :requests="pairing.requests.value" :locale="locale" @submit="onPairSubmit" @reject="(id) => pairing.reject(id)" @install="connect.show('install')" />
    <PanelPending :locale="locale" @go="onPendingGo" />
    <MSupportSheet v-model="sponsorOpen" :title="t.m3e.home.sponsorTitle" :sections="sponsorSections" :close-label="t.m3e.home.done" @open="(it) => actions_openExternal(it.url)" />
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
import { computed, defineAsyncComponent, getCurrentInstance, ref, toRef, watch, onMounted, onBeforeUnmount } from 'vue'
import { sunshine, tools, openExternalUrl } from '../tauri-adapter.js'
import { SPONSORS } from '../config/sponsors.js'
import QRCode from 'qrcode'
import { MTelemetryConsent, useTelemetryConsent, MErrorHost, installErrorHandlers, setErrorContext, startAutoReport, reportHostStop, useNotify, useSystemNotices, MStatusBell, connectTexts, filterHash, MConnectDialog, MPairPrompt, pinToHome, unpinFromHome, isPinnedOnHome, createPairingPending, MSnackbar, useConnectDialog, MAnnouncementButton, MAnnouncementPage, MAnnouncementPopup, ackedAt, useAnnouncements, MCaption, MCoachMarks, MRemoteWindow, MUnifiedSearch, useUnifiedSearch, searchTexts, actionDocs, settingDocs, clientDocs, appDocs, tileDocs, addTileToHome, MHostOffline, useHostConnection, MSheet, MGroup, MItem, MSupportSheet, MBrand, MScene, MNavList, MIconButton, MIcon, MContextMenu, MThanksLine, aboutTexts, statusTexts, diagnosticsTexts, useAnnouncementBadge, useSettingsList, useAppearance, registerModeHandler, MSponsorButton, useThanks } from 'foundation-ui/vue'
import { labelOf, settingsHash } from 'foundation-ui/settings/index.js'
import PanelHome from './PanelHome.vue'
import PanelWebHost from './PanelWebHost.vue'
import VddSettings from './VddSettings.vue'
import Welcome from './welcome.vue'
import WebStreamSettings from './WebStreamSettings.vue'
import AiAssistant from './AiAssistant.vue'
import { mitaState } from '../composables/mitaState.js'
import SettingsHubPage from './m3e/SettingsHubPage.vue'
import HdrEnhancedManager from './HdrEnhancedManager.vue'
import ControlPanelPage from './ControlPanelPage.vue'
import MicrophonePage from './m3e/MicrophonePage.vue'
import AppearancePage from './m3e/AppearancePage.vue'
import UsbPage from './m3e/UsbPage.vue'
import AboutPage from './m3e/AboutPage.vue'
import ComponentStatusPage from './m3e/ComponentStatusPage.vue'
import { useComponentStatus } from '../composables/useComponentStatus.js'
import ToolboxPage from './m3e/ToolboxPage.vue'
import LiveSessionPage from './m3e/LiveSessionPage.vue'
import RtssPage from './m3e/RtssPage.vue'
import DiagnosticsPage from './m3e/DiagnosticsPage.vue'
import { useDeviceRuntime } from '../composables/useDeviceRuntime.js'
import { useGuideAutoOpen } from '../composables/useGuideAutoOpen.js'
import { useSidebarState } from '../composables/useSidebarState.js'
import { useWindowControls } from '../composables/useWindowControls.js'
import { useTools } from '../composables/useTools.js'
import { useTouchWindowDrag } from '../composables/useTouchWindowDrag.js'
import { createUtilityTools } from '../composables/toolsRegistry.js'
import { ROUTES, PAGES, routeForWebPath, routeOfPlace, webPathOf } from '../composables/useRouter.js'
import { createGuideData } from '../composables/guideData.js'
import PanelPending from './PanelPending.vue'
import { recordUpdateCheck } from '../composables/useUpdateCheckTime.js'
import { useHostReach } from '../composables/useHostReach.js'
import { ROW_OF_ROUTE, railItems, pageItems, barItems, barCurrent } from '../composables/panelNav.js'
import { rowKeywords } from 'foundation-ui/settings/index.js'
import { useI18n } from '../desktop/i18n/index.js'
import { useDualSenseStatus } from '../composables/useDualSenseStatus.js'
import { useDualSenseText } from '../composables/useDualSenseText.js'
const UpdateDialog = defineAsyncComponent(() => import('./UpdateDialog.vue'))
const props = defineProps({ webuiPath: { type: String, default: '/' } })
const emit = defineEmits(['route-change', 'webui-navigate'])
const { t, locale, choice: localeChoice, setChoice: setLocaleChoice, systemLocale } = useI18n()
const {
  isMaximized, isAdmin, showUpdateDialog, updateInfo, currentVersion, router, themeMode, setTheme,
  openVddSettings, openWelcome, openWebStream, openAiAssistant, openControllers, openRtxHdr, skipVersion,
  includePrerelease, setIncludePrerelease,
} = useSidebarState()
const { minimizeWindow, toggleMaximize, closeWindow } = useWindowControls(isMaximized)
// the shared appearance (wallpaper, colour, light/dark); light/dark goes through the panel's own theme switch
registerModeHandler((mode) => setTheme(mode))
onMounted(() => { void useAppearance().init() })
// the overview has its own (lighter) frost over the wallpaper; every other page the stronger one
watch(() => router.currentRoute.value, (route) => useAppearance().setScope(route === ROUTES.HOME ? 'home' : 'other'), { immediate: true })
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
  recordUpdateCheck()
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

// what is wrong with the host or the app right now goes to the status button of the caption bar
useSystemNotices({
  status: componentStatus,
  locale,
  go: { components: () => router.navigate(ROUTES.COMPONENTS), microphone: () => router.navigate(ROUTES.MICROPHONE), vdd: () => router.navigate(ROUTES.VDD_SETTINGS), update: () => { showUpdateDialog.value = true } },
  update: () => ({ state: updateInfo.value && !updateInfo.value.is_latest ? 'available' : '', next: updateInfo.value?.version || '' }),
})
const sponsorSections = computed(() => [
  { title: t.value.m3e.home.sponsorTitle, items: SPONSORS.map((x) => ({ id: x.id, icon: 'favorite', title: x.label, url: x.url })) },
  { items: [{ id: 'star', icon: 'star', title: t.value.m3e.home.star, url: 'https://github.com/AlkaidLab/foundation-sunshine' }, { id: 'qq', icon: 'groups', title: t.value.m3e.home.qq, url: 'https://qm.qq.com/q/AfMQoyKrkc' }] },
])
const actions_openExternal = (url) => openExternalUrl(url)
const nav = router.nav
const isWebPage = computed(() => !!webPathOf(router.currentRoute.value))
// The frame shows the page of the route. Callers that open a Web page tell the frame themselves; a route that is set any other way (a link, a deep link, a caller that only
// switches the route) comes here: when the frame is not on the page of the route it is told, so a route never shows another page's content.
watch(() => router.currentRoute.value, (route) => {
  const path = webPathOf(route)
  if (path && props.webuiPath !== path && routeForWebPath(props.webuiPath) !== route) emit('webui-navigate', path)
}, { flush: 'post', immediate: true })
// The host (Sunshine) does not answer: the content area shows the offline state (Vivi, the actions, the countdown); the rail stays usable and the pages that
// do not need the host (the panel's settings, 米塔 AI, help, about, the diagnostics) stay as they are. When the host is back the frame loads again.
const probeHost = async () => {
  const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
  const response = await fetch(`${base}/`, { cache: 'no-store', signal: AbortSignal.timeout(2500) })
  return response.status !== 502 && response.status !== 503
}
const hostConn = useHostConnection({
  probe: probeHost,
  restartService: () => tools.restartSunshineService(),
  restartUser: () => tools.restartSunshineInUserMode(),
  onRecovered: () => { for (const frame of document.querySelectorAll('iframe')) { const src = frame.src; frame.src = src } },
  // the host answered and stopped without a request from this window: the error surface in its 「Sunshine 意外停止」 variant (telemetry/crash.js has the interface for the crash bundle the host leaves)
  onLost: ({ at }) => reportHostStop({ at }),
})
const NEEDS_NO_HOST = new Set([ROUTES.PANEL_SETTINGS, ROUTES.AI_ASSISTANT, ROUTES.HELP, ROUTES.ABOUT, ROUTES.SETTINGS_HUB, ROUTES.DIAGNOSTICS, ROUTES.WELCOME])
const offlineShown = computed(() => hostConn.state.status !== 'online' && !NEEDS_NO_HOST.has(router.currentRoute.value))
const back = () => nav.pop()
// the way out of the error surface: the previous page, or the overview when there is none
// The choice about usage counts and error information: the host keeps it, the Web UI and this window agree; asked once when nothing else is on screen, changed later in 关于
const telemetryApi = { getJson: async (path) => (await fetch(`${String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')}${path}`)).json(), postJson: async (path, body) => (await fetch(`${String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')}${path}`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })).json() }
const telemetry = useTelemetryConsent({ api: telemetryApi })
const consentOpen = ref(false)
const errorBack = () => { if (nav.canPop.value) nav.pop(); else router.navigate(ROUTES.HOME) }
const osName = () => (typeof navigator === 'undefined' ? '' : String(navigator.userAgentData?.platform || navigator.platform || ''))
const errorContext = () => ({ product: 'panel', version: currentVersion.value || '', os: osName(), page: String(router.currentRoute.value || ''), locale: String(locale.value), logs: '', components: `panel=${currentVersion.value || '?'}` })
setErrorContext(errorContext)
let removeErrorHandlers = null
const shellApp = getCurrentInstance()?.appContext.app
// the remote access tutorials (help page, and the fix of a reason of the reachability check)
const remoteOpen = ref(false)
const remoteData = createGuideData({ proxyUrl: () => sunshine.getProxyUrl(), openUrl: (url) => actions_openExternal(url), copy: (text) => navigator.clipboard.writeText(text), reach: useHostReach(), vddStatus: async () => null, openApps: () => {}, navigate: () => {} })
const announceVersion = ref('')
onMounted(async () => { try { announceVersion.value = (await sunshine.getVersion()) || '' } catch { /* the version only narrows version-bound notices */ } })
const announcements = useAnnouncements({ locale, version: announceVersion })
const appearanceLabel = computed(() => labelOf(locale.value, 'shell.appearance'))
const announcementsUnread = announcements.unread
// a dot on the help rail while an update is waiting (the about page checks and opens the update sheet)
const updateWaiting = computed(() => !!updateInfo.value && !updateInfo.value.is_latest)
const rail = computed(() => railItems(t.value).map((item) => (item.id === 'overview' && announcementsUnread.value ? { ...item, badge: announcementsUnread.value } : item.id === 'help' && updateWaiting.value ? { ...item, badge: 'dot' } : item.id === 'assistant' && mitaState.unread ? { ...item, badge: 'dot' } : item)))
// the one-time thank-you line after the guided first run
const thanks = useThanks({ guideKey: 'foundation.guide.panel' })
const thanksStar = () => { actions_openExternal('https://github.com/AlkaidLab/foundation-sunshine'); thanks.dismiss() }
const thanksSponsor = () => { thanks.dismiss(); router.navigate(ROUTES.ABOUT) }
const railLabel = (id) => t.value.m3e.nav[id]
const runtimeView = computed(() => ({ ...runtime, lines: settingsList.lines.value, statusTitle: statusTexts(locale.value).title, diagTitle: diagnosticsTexts(locale.value).title }))
const listItems = (id) => pageItems(t.value, id, runtimeView.value)
// The stream settings are the shared model (categories in groups with a live line, search of every setting, recently used)
const hostConfig = ref(null)
const dualsenseKeys = ref([])
const hostClients = ref([])
const hostApps = ref([])
const settingsList = useSettingsList({ locale, config: hostConfig, dualsenseKeys, sources: { clients: hostClients, apps: hostApps, components: computed(() => componentStatus.rows.value), ds: computed(() => runtime.ds), mic: computed(() => runtime.mic), usb: computed(() => runtime.usb) } })
const loadHostData = async () => {
  try {
    const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
    const [config, tuning] = await Promise.allSettled([fetch(`${base}/api/config`).then((r) => (r.ok ? r.json() : null)), fetch(`${base}/api/dualsense/tuning`).then((r) => (r.ok ? r.json() : null))])
    if (config.status === 'fulfilled' && config.value) hostConfig.value = config.value
    if (tuning.status === 'fulfilled' && tuning.value?.keys) dualsenseKeys.value = tuning.value.keys
    const [clients, apps] = await Promise.allSettled([fetch(`${base}/api/clients/list`).then((r) => (r.ok ? r.json() : null)), fetch(`${base}/api/apps`).then((r) => (r.ok ? r.json() : null))])
    if (clients.status === 'fulfilled' && clients.value?.named_certs) hostClients.value = clients.value.named_certs
    if (apps.status === 'fulfilled' && Array.isArray(apps.value?.apps)) hostApps.value = apps.value.apps
  } catch { /* the list shows no live lines and the search the pages only */ }
}
const listGroups = (rail) => (rail === 'config' ? settingsList.groups.value : [{ id: rail, items: listItems(rail) }])
const currentRow = computed(() => {
  const route = nav.current.value
  if (route === ROUTES.WEB_CONFIG) return `config:${nav.detail.value?.params?.tab || 'general'}`
  if (route === ROUTES.WEB_PASSWORD) return 'password'
  return route
})
// the settings of the display and audio rails are not under 串流设置: their path starts with the rail's own name (the category of the entry)
const settingsPlace = (e) => (e.params?.tab === 'display' || e.params?.tab === 'audio' ? '' : railLabel('config'))
const searchItems = computed(() => [
  ...Object.keys(PAGES)
    .filter((id) => PAGES[id].list !== false)
    .flatMap((id) => {
      const item = pageItems(t.value, PAGES[id].rail, runtimeView.value).find((p) => p.id === id)
      return item ? [{ id, title: item.label, path: railLabel(PAGES[id].rail), icon: item.icon, route: id, keywords: rowKeywords(ROW_OF_ROUTE[id]) }] : []
    }),
  // 米塔 AI is a destination of the rail (no list pane): a result of its own, and one for the settings behind the gear of its chat
  { id: ROUTES.AI_ASSISTANT, title: t.value.m3e.nav.assistant, path: t.value.m3e.nav.main, icon: 'smart_toy', route: ROUTES.AI_ASSISTANT, keywords: rowKeywords('assistant') },
  ...settingsList.searchItems.value,
])
// ---- the unified search: one entry in the caption bar (an icon and a full-screen view on a narrow window) over everything a person can look for ----
const compactSearch = ref(typeof window !== 'undefined' && window.innerWidth < 600)
const onSearchResize = () => { compactSearch.value = window.innerWidth < 600 }
onMounted(() => window.addEventListener('resize', onSearchResize))
onBeforeUnmount(() => window.removeEventListener('resize', onSearchResize))
const searchT = computed(() => searchTexts(locale.value))
// 连接设备: the dialog of the caption button and of the search; the host API through the proxy, like the overview's pairing card
const connectT = computed(() => connectTexts(locale.value))
const connectNotice = ref('')
// short results go to the one snackbar of the window
watch(connectNotice, (message) => { if (message) { useNotify().snack(message); connectNotice.value = '' } })
const hostApiOf = async (path, init) => { const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, ''); return (await fetch(`${base}${path}`, init)).json() }
const connect = useConnectDialog({
  hostApi: { getJson: (path) => hostApiOf(path), postJson: (path, body) => hostApiOf(path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) }) },
  render: (url) => QRCode.toDataURL(url, { width: 280, margin: 2, color: { dark: '#000000', light: '#ffffff' } }),
  matrix: (text) => QRCode.create(text, { errorCorrectionLevel: 'M' }).modules,
  say: (kind) => { connectNotice.value = kind === 'sent' ? connectT.value.pinSent : connectT.value.pinFailed },
  defaultName: locale.value.startsWith('zh') ? '新客户端' : 'New client',
})
// a device waits for its PIN: the prompt comes forward on any page (the host's long poll through the proxy, docs/api/pairing-pending.md); the native notification and the tray are the host tray's own
const failing = (r) => Object.assign(new Error(`HTTP ${r.status}`), { status: r.status })
const pairingApi = {
  getJson: async (path) => { const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, ''); const r = await fetch(`${base}${path}`); if (!r.ok) throw failing(r); return r.json() },
  postJson: async (path, body) => { const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, ''); const r = await fetch(`${base}${path}`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) }); if (!r.ok) throw failing(r); return r.json() },
  delete: async (path) => { const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, ''); const r = await fetch(`${base}${path}`, { method: 'DELETE' }); if (!r.ok) throw failing(r); return r.json() },
}
const pairing = createPairingPending({ hostApi: pairingApi })
const onPairSubmit = async (e) => { const ok = await pairing.submit(e.id, e.pin, e.name); connectNotice.value = ok ? connectT.value.pinSent : connectT.value.pinFailed }
// the tray's entries 配对新设备 and 下载客户端 open the dialog on a tab (the native side emits `open-connect` once the panel is ready)
// a Web page inside the panel pins to the panel's overview through the shell (it has no home of its own)
const onFramePin = (event) => {
  const d = event.data
  if (!d || d.type !== 'm3e-pin' || !d.spec || !([...document.querySelectorAll('iframe')].some((f) => f.contentWindow === event.source))) return
  if (d.op === 'pin') pinToHome('panel-home', d.spec); else if (d.op === 'unpin') unpinFromHome('panel-home', d.spec)
}
let unlistenConnect = null
onMounted(async () => {
  pairing.start()
  window.addEventListener('message', onFramePin)
  try { const { listen } = await import('@tauri-apps/api/event'); unlistenConnect = await listen('open-connect', (event) => connect.show(event.payload?.tab === 'install' ? 'install' : 'pair')) } catch { /* not in the app window */ }
})
onBeforeUnmount(() => { pairing.stop(); window.removeEventListener('message', onFramePin); if (unlistenConnect) unlistenConnect(); removeErrorHandlers?.() })
onMounted(async () => { removeErrorHandlers = installErrorHandlers({ app: shellApp }); startAutoReport({ context: errorContext }); await telemetry.refresh(); telemetry.askWhenQuiet(consentOpen) })
const actionHandlers = {
  restart: () => tools.restartSunshineService(),
  userMode: () => tools.restartSunshineInUserMode(),
  restartDriver: () => tools.restartGraphicsDriver(),
  connect: () => connect.show('pair'),
  download: () => connect.show('install'),
  exportDiag: () => router.navigate(ROUTES.DIAGNOSTICS),
  checkUpdate: () => handleCheckForUpdates(),
  logs: () => router.navigate(ROUTES.DIAGNOSTICS),
  addApp: () => openRoute(ROUTES.WEB_APPS),
}
const searchDocs = computed(() => [
  ...settingDocs(searchItems.value, { prefix: settingsPlace, locale: locale.value }),
  ...actionDocs(locale.value, actionHandlers, searchT.value.types.action),
  ...clientDocs(hostClients.value, { place: `${railLabel('stream')}` }),
  ...appDocs(hostApps.value, { place: `${railLabel('stream')}` }),
  ...tileDocs(locale.value),
])
const recentSettingDocs = computed(() => settingDocs(settingsList.recentView.value.items.map((r) => r.entry), { prefix: settingsPlace, locale: locale.value }))
const onUnifiedChoose = (doc) => {
  if (doc.type === 'setting' || doc.type === 'page') onSearch(doc.entry)
  else if (doc.type === 'client') { emit('webui-navigate', `/pin${filterHash(doc.client?.name || doc.title)}`); router.navigate(ROUTES.WEB_CLIENTS) }
  else if (doc.type === 'app') { emit('webui-navigate', `/apps${filterHash(doc.app?.name || doc.title)}`); router.navigate(ROUTES.WEB_APPS) }
  else if (doc.type === 'tile') { addTileToHome('panel-home', doc.tile); router.navigate(ROUTES.HOME) }
}
const uni = useUnifiedSearch({ docs: searchDocs, onPin: (doc) => pinToHome('panel-home', doc.pin), isPinned: (doc) => isPinnedOnHome('panel-home', doc.pin), recentSettings: recentSettingDocs, onChoose: onUnifiedChoose })
// Where an error of the diagnostics page leads
const goDiagnosticsPlace = (place) => {
  if (place === 'drivers') router.navigate(ROUTES.COMPONENTS)
  else if (place === 'encoder') openConfig({ tab: 'encoders' })
  else if (place === 'network') openConfig({ tab: 'network' })
  else if (place === 'audio') openConfig({ tab: 'audio', sub: 'sound' })
  else if (place === 'display') openConfig({ tab: 'display', sub: 'output' })
}
// A place of the stream settings: the list pane opens the Web page in the frame at that tab, section and row
const openConfig = (params = { tab: 'general' }) => {
  // the display and audio places are destinations of their own rails (the virtual display and the microphone are pages of the panel)
  const place = routeOfPlace(params.tab, params.sub)
  if (place) {
    if (webPathOf(place)) emit('webui-navigate', `/config${settingsHash(params.sub ? params : { ...params, sub: undefined })}`)
    router.navigate(place, params)
    return
  }
  emit('webui-navigate', `/config${settingsHash(params)}`)
  router.navigate(ROUTES.WEB_CONFIG, params)
}
const openSettingsRoute = (item) => {
  if (item.route === 'appearance' || (item.route === 'config' && item.params?.tab === 'appearance')) router.navigate(ROUTES.CFG_APPEARANCE)
  else if (item.route === 'config') openConfig(item.params)
  else if (item.route === 'password') openRoute(ROUTES.WEB_PASSWORD)
  else openRoute(item.route || item.id)
}
// a page of the unsaved changes (the bar's list): the settings tab, the gamepad, the application list or the virtual display
const onPendingGo = ({ page, entry, route }) => {
  if (route) openRoute(route)
  else if (page.startsWith('config:')) openConfig({ tab: entry?.tab || page.slice(7), sub: entry?.sub, focus: entry?.focus })
  else openRoute(page === 'apps' ? ROUTES.WEB_APPS : ROUTES.GAMEPAD)
}
const onListNavigate = (item) => (item.route ? openSettingsRoute(item) : openRoute(item.id))

const sceneTexts = computed(() => ({ resize: t.value.m3e.resize, main: t.value.m3e.nav.main, expand: t.value.m3e.nav.expand, collapse: t.value.m3e.nav.collapse, shortcuts: t.value.m3e.shortcuts, back: t.value.m3e.back, search: t.value.m3e.search, help: t.value.m3e.shortcutHelp, section: t.value.m3e.section, close: t.value.m3e.clear }))
// the bottom bar of a narrow window: five destinations, the settings categories behind one, the rest behind 更多
const bar = computed(() => barItems(t.value).map((item) => (item.id === 'overview' && announcementsUnread.value ? { ...item, badge: announcementsUnread.value } : item.id === 'more' && updateWaiting.value ? { ...item, badge: 'dot' } : item)))
const barCur = computed(() => (router.currentRoute.value === ROUTES.SETTINGS_HUB ? 'settings-hub' : barCurrent(nav.rail.value)))
const moreOpen = ref(false)
const sponsorOpen = ref(false)
const moreGo = (action) => { moreOpen.value = false; setTimeout(action, 120) }
const onRail = (item) => {
  if (item.id === 'settings-hub') { router.navigate(ROUTES.SETTINGS_HUB); return }
  if (item.id === 'more') { moreOpen.value = true; return }
  if (item.id === 'config') { loadHostData(); openConfig({ tab: 'general' }); router.navigate(ROUTES.CAT_CONFIG); return }
  // the stream rail opens on its first page: the frame has to show it
  if (item.id === 'stream') { emit('webui-navigate', webPathOf(ROUTES.WEB_CLIENTS)); router.navigate(ROUTES.CAT_STREAM); return }
  router.navigate({ overview: ROUTES.HOME, settings: ROUTES.PANEL_SETTINGS, help: ROUTES.HELP, stream: ROUTES.CAT_STREAM, input: ROUTES.CAT_INPUT, audio: ROUTES.CAT_AUDIO, display: ROUTES.CAT_DISPLAY, system: ROUTES.CAT_SYSTEM, assistant: ROUTES.AI_ASSISTANT }[item.id])
  const first = nav.list.value && nav.defaultDetailOf(nav.list.value)
  const path = first && webPathOf(first.id)
  if (path) emit('webui-navigate', path)
}
const onSearch = (item) => {
  settingsList.remember(item.keyName)
  openSettingsRoute(item)
}
// A setting changed in the frame joins the recently used list
const frameSub = ref(false)
const ds5 = useDualSenseStatus()
const { tx } = useDualSenseText()
const TRANSPORT_STATES = ['usbip_missing', 'usbip_version', 'usbip_unsigned', 'usbip_not_ready']
const pushDs5Status = () => {
  const s = ds5.status.value
  if (!s) return
  const message = { type: 'foundation-ds5-status', state: s.state, label: tx(`status.states.${s.state}`), next: tx(`status.next.${s.state}`, { pinned: s.transport?.pinned_version || '', version: s.transport?.version || '' }), needsTransport: TRANSPORT_STATES.includes(s.state) }
  for (const frame of document.querySelectorAll('iframe')) frame.contentWindow?.postMessage(message, '*')
}
watch(() => ds5.status.value?.state, pushDs5Status)
const onFrameMessage = (event) => {
  const data = event.data
  if (![...document.querySelectorAll('iframe')].some((frame) => frame.contentWindow === event.source)) return
  if (data?.type === 'foundation-subpage') frameSub.value = data.open === true
  else if (data?.type === 'foundation-search-open') uni.show()
  else if (data?.type === 'foundation-settings-recent' && typeof data.key === 'string') settingsList.remember(data.key)
  // what the gamepad page of the frame cannot do by itself: the controller test tool, the component page, and the state of the
  // DualSense transport that only the panel reads
  else if (data?.type === 'foundation-open' && data.page === 'controller-test') openGamepadTest()
  else if (data?.type === 'foundation-open' && data.page === 'components') router.navigate(ROUTES.COMPONENTS)
  else if (data?.type === 'foundation-open' && data.page === 'hdr-enhanced') router.navigate(ROUTES.HDR_ENHANCED)
  else if (data?.type === 'foundation-ds5-status-request') pushDs5Status()}
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
  if (id === 'check-update') { handleCheckForUpdates(); return }
  if (id.startsWith('announcement:')) { router.navigate(ROUTES.ANNOUNCEMENTS, { id: id.slice(13) }); return }
  if (id.startsWith('setting:')) { const entry = searchItems.value.find((e) => e.keyName === id.slice(8)); if (entry) onSearch(entry); return }
  if (id === 'welcome') { openWelcome(); return }
  // the pairing sheet and the new application editor open at once: the Web pages read #pairing and #guide-add
  if (id === 'pair') { emit('webui-navigate', '/pin#pairing'); router.navigate(ROUTES.WEB_CLIENTS); return }
  if (id === 'pair:qr') { emit('webui-navigate', '/pin#pairing-qr'); router.navigate(ROUTES.WEB_CLIENTS); return }
  if (id === 'apps:add') { emit('webui-navigate', '/apps#guide-add'); router.navigate(ROUTES.WEB_APPS); return }
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
.panel-shell { position: relative; height: 100vh; height: 100dvh; min-height: 0; overflow: clip; }
.panel-caption { position: absolute; top: 0; left: var(--m-rail-w, 0px); right: 0; z-index: 40; transition: left var(--m-spring-default-spatial-duration, .4s) var(--m-spring-default-spatial, ease); display: grid; grid-template-columns: minmax(max-content, 1fr) minmax(0, 560px) minmax(max-content, 1fr); align-items: stretch; height: 48px; background: transparent; user-select: none; }
.panel-caption__brand { align-self: center; padding-inline-start: 16px; pointer-events: none; }
.panel-caption__controls { display: flex; justify-self: end; }
.panel-caption__search { display: flex; align-items: center; justify-content: center; min-width: 0; padding: 0 12px; }
@media (max-width: 599.98px) { .panel-caption { grid-template-columns: minmax(max-content, 1fr) auto auto; } .panel-caption__search { padding: 0 4px; } }
.panel-caption__btn { display: grid; place-items: center; width: 46px; height: 48px; padding: 0; border: 0; background: transparent; color: var(--md-sys-color-on-surface); font-size: 18px; cursor: default; transition: background-color 120ms linear, color 120ms linear; }
.panel-caption__btn:hover { background: color-mix(in srgb, var(--md-sys-color-on-surface) 9%, transparent); }
.panel-caption__btn:active { background: color-mix(in srgb, var(--md-sys-color-on-surface) 14%, transparent); }
.panel-caption__btn--close:hover { background: #c42b1c; color: #fff; } .panel-caption__btn--close:active { background: #b02417; color: #fff; }
.panel-caption__btn:focus-visible { outline: 2px solid var(--md-sys-color-primary); outline-offset: -2px; }
.panel-body { position: absolute; inset: 0; min-height: 0; overflow: clip; }
.panel-web-keep { height: 100%; min-height: 0; }
.panel-body > .m-scene { position: absolute; inset: 0; height: auto; }
</style>
