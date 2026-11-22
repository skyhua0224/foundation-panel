<template>
  <div class="m-app m-app--scroll panel-shell">
    <!-- slim caption bar over the whole window: draggable, the window controls at the end; the pages start below it -->
    <MCaption bem="panel-caption" more bar-more :more-label="t.m3e.nav.moreTitle" data-tauri-drag-region="deep" @pointerdown="onTouchWindowDragStart" @more="moreOpen = true">
      <template v-if="backButton" #back><MCaptionBack :page="backPage" :locale="locale" @back="jumps.back()" /></template>
      <template #version><MCaptionVersion :version="currentVersion" :build="buildInfo" :os="osName()" :locale="locale" :update="notice.version" @copied="onVersionCopied" @open-update="openUpdates" /></template>
      <template #brand><MBrand class="panel-caption__brand" :size="20" data-tauri-drag-region /></template>
      <template #announcements><UpdateDownloadChip @open="showUpdateDialog = true" /><MStatusBell :locale="locale" /><MFeedbackButton :locale="locale" /><MAnnouncementButton :unread="announcements.unread.value" :locale="locale" @open="router.navigate(ROUTES.ANNOUNCEMENTS)" /></template>
      <template #search><div class="panel-caption__search" data-tauri-drag-region><MUnifiedSearch :search="uni" :texts="searchT" :compact="compactSearch || caption.tight.value" :height="38" /></div></template>
      <template #connect><button type="button" class="m-caption-connect" :aria-label="connectT.title" :data-tip="connectT.title" :disabled="hostUnusable" @click="connect.show('pair')"><MIcon name="person_add" /><span>{{ connectT.title }}</span></button></template>
      <template #controls><div class="panel-caption__controls">
        <button type="button" class="panel-caption__btn" :aria-label="t.sidebar.minimize" :data-tip="t.sidebar.minimize" @click="minimizeWindow"><MIcon name="remove" /></button>
        <button type="button" class="panel-caption__btn" :aria-label="isMaximized ? t.sidebar.restore : t.sidebar.maximize" :data-tip="isMaximized ? t.sidebar.restore : t.sidebar.maximize" @click="toggleMaximize"><MIcon :name="isMaximized ? 'filter_none' : 'crop_square'" /></button>
        <button type="button" class="panel-caption__btn panel-caption__btn--close" :aria-label="t.sidebar.hideWindow" :data-tip="t.sidebar.hideWindow" data-panel-hide @click="hidePanel"><MIcon name="close" /></button>
      </div></template>
    </MCaption>
    <div class="panel-body">
    <MScene :nav="nav" :caption-height="48" :jumps="jumps" :chip="backChip" :rail-items="rail" :bar-items="bar" :bar-current="barCur" :texts="sceneTexts" :overlay="isWebPage || offlineShown" :status="offlineShown" storage-key="foundation-ui.panel.list" @rail="onRail">
      <template #back-chip><MBackChip :page="backPage" :locale="locale" @back="jumps.back()" /></template>
      <template #rail-foot><MIconButton icon="palette" :label="appearanceLabel" :current="nav.rail.value === 'appearance'" @click="router.navigate(ROUTES.CFG_APPEARANCE)" /><MIconButton icon="info" :label="aboutTexts(locale).title" :current="router.currentRoute.value === ROUTES.ABOUT" @click="router.navigate(ROUTES.ABOUT, undefined, { root: true })" /><MSponsorButton :label="t.m3e.home.sponsor" :title="t.m3e.home.sponsorTitle" :sections="sponsorSections" :close-label="t.m3e.home.done" @open="(it) => actions_openExternal(it.url)" /></template>
      <template #list="{ entry, scene }">
        <MNavList
          :groups="listGroups(entry.rail)"
          :current="currentRow"
          :label="railLabel(entry.rail)"
          :title="railLabel(entry.rail)"
          :titled="scene.kind === 'single'"
          @navigate="onListNavigate"
          @context="onRowContext"
        />
      </template>
      <template #detail="{ entry }">
        <!-- while the host cannot be used the status page (the overlay) is the whole content area; a page that needs the host is not drawn under it -->
        <template v-if="offlineShown && !isWebPage" />
        <PanelHome v-else-if="entry.id === ROUTES.HOME" :runtime="runtime" :update-available="updateAvailable(updateInfo)" @open="openDestination" @webui-navigate="openWebUi" />
        <Welcome v-else-if="entry.id === ROUTES.WELCOME" :key="guideRun" @close="closeWelcome" @open-privacy="closeWelcome(); router.navigate(ROUTES.ABOUT)" @webui-navigate="openWebUi" @open-doctor="openDoctor" />
        <WebStreamSettings v-else-if="entry.id === ROUTES.WEB_STREAM" @close="goHome" />
        <VddSettings v-else-if="entry.id === ROUTES.VDD_SETTINGS" @close="back" @webui-navigate="openWebUi" />
        <AiAssistant v-else-if="entry.id === ROUTES.AI_ASSISTANT" @close="back" />
        <StreamHubPage v-else-if="entry.id === ROUTES.STREAM_HUB" :runtime="runtimeView" @open="onListNavigate" />
        <SettingsHubPage v-else-if="entry.id === ROUTES.SETTINGS_HUB" :groups="settingsList.groups.value" :runtime="runtimeView" :focus="entry.params?.section || ''" @open="onListNavigate" />
        <HdrEnhancedManager v-else-if="entry.id === ROUTES.HDR_ENHANCED" @manage-components="jumpTo(ROUTES.COMPONENTS)" />
        <AppearancePage v-else-if="APPEARANCE_ROUTES.includes(entry.id)" :page="entry.id" />
        <MicrophonePage v-else-if="entry.id === ROUTES.MICROPHONE" @manage-components="jumpTo(ROUTES.COMPONENTS)" />
        <UsbPage v-else-if="entry.id === ROUTES.USB" @manage-components="jumpTo(ROUTES.COMPONENTS)" />
        <SunshineServicePage v-else-if="entry.id === ROUTES.SUNSHINE_SERVICE" :locale="locale" />
        <ComponentStatusPage v-else-if="entry.id === ROUTES.COMPONENT_STATUS" />
        <ToolboxPage v-else-if="entry.id === ROUTES.TOOLBOX" :tools="toolsMenuItems" />
        <LatencyClockPage v-else-if="entry.id === ROUTES.LATENCY_CLOCK" />
        <StreamTimerPage v-else-if="entry.id === ROUTES.STREAM_TIMER" />
        <TestToolsPage v-else-if="entry.id === ROUTES.TEST_TOOLS" />
        <LiveSessionPage v-else-if="entry.id === ROUTES.LIVE_SESSION" @open-quality="jumpTo(ROUTES.HDR_ENHANCED)" @webui-navigate="openWebUi" />
        <RtssPage v-else-if="entry.id === ROUTES.RTSS" />
        <NetworkOverviewPage v-else-if="entry.id === ROUTES.NETWORK_OVERVIEW" @open-network="jumpConfig({ tab: 'network' })" />
        <DiagnosticsPage
          v-else-if="entry.id === ROUTES.DIAGNOSTICS || entry.id === ROUTES.WEB_TROUBLESHOOTING"
          @go="goDiagnosticsPlace"
          @webui-navigate="openWebUi"
          @open-guide="openWelcome"
          @open-remote="remoteOpen = true"
          @open-reset="jumpTo(ROUTES.RESET)"
        />
        <ResetSettingsPage v-else-if="entry.id === ROUTES.RESET" @back="back" />
        <DoctorPage v-else-if="entry.id === ROUTES.DOCTOR" :autostart="entry.params?.autostart || false" @go="goDoctorTarget" @restart="hostConn.restart('service')" @ai="askMita" />
        <MAnnouncementPage v-else-if="entry.id === ROUTES.ANNOUNCEMENTS" :items="announcements.list.value" :locale="locale" :acked-at="ackedAt" :open-id="entry.params?.id || ''" :on-refresh="() => announcements.refresh(true)" @dismiss="announcements.dismiss" @read="announcements.read" @open-url="actions_openExternal" @back="back" />
        <DevHub v-else-if="entry.id === ROUTES.DEV_HUB" :locale="locale" :pages="devHubPages" @open="(p) => openRoute(p.route)" />
        <DeveloperPages v-else-if="DEV_PAGES.includes(entry.id)" :key="entry.id" :page="entry.id" :locale="locale" :version="currentVersion" :consent="telemetry.consent.value" :consent-ready="telemetry.loaded.value" :error-context="errorContext" :simulate-host="simulateHost" @disabled="leaveDeveloper" @notify="(text) => useNotify().snack(text)" />
        <AboutPage v-else-if="entry.id === ROUTES.ABOUT" :preview-loader="() => panelTelemetry.preview()" :current-version="currentVersion" :update-info="updateInfo" :locale="locale" :check="handleCheckForUpdates" :on-refresh="() => announcements.refresh(true)" :consent="telemetry.consent.value" @consent="telemetry.answer" :errors="telemetry.errors.value" @errors="telemetry.setErrors" support-link @open-update-source="router.navigate(ROUTES.PANEL_SETTINGS)" :include-prerelease="includePrerelease" @channel-change="setIncludePrerelease" @open-update="showUpdateDialog = true" @open="actions_openExternal" @open-developer="router.navigate(ROUTES.DEVELOPER)" />
        <ControlPanelPage
          v-else-if="entry.id === ROUTES.PANEL_SETTINGS || entry.id === ROUTES.HELP"
          :page="entry.id"
          :tools="toolsMenuItems"
          :theme-mode="themeMode"
          :locale="locale" :locale-choice="localeChoice" :system-locale="systemLocale"
          :include-prerelease="includePrerelease"
          :current-version="currentVersion"
          :clipboard-sync-enabled="clipboardSyncEnabled"
          :clipboard-sync-state="clipboardSyncStateWord"
          :is-admin="isAdmin"
          :actions="toolsCtx"
          @theme-change="setTheme"
          @locale-change="setLocaleChoice"
          @channel-change="setIncludePrerelease"
          @webui-navigate="openWebUi"
          @open-guide="openWelcome"
          @open-remote="remoteOpen = true"
          @open-about="router.navigate(ROUTES.ABOUT)"
          @open-doctor="openDoctor"
        />
      </template>
      <template #overlay>
        <!-- the frame stays mounted while the host is away (it only is hidden): it loads again when the host is back -->
        <div v-show="!offlineShown" class="panel-web-keep" :inert="offlineShown ? '' : undefined"><PanelWebHost :sub="frameSub" :page="isWebPage ? router.currentRoute.value : ''"><slot /></PanelWebHost></div>
        <MHostOffline v-if="offlineShown" :cause="offlineCause" :state="offlineState" :locale="locale" :port="hostPort" :can-restart="offlineCause !== 'proxy'" :can-user-mode="offlineCause !== 'proxy'" can-logs can-doctor @retry="retryOffline" @restart="confirmedRestart('service')" @user-mode="confirmedRestart('user')" @logs="jumpTo(ROUTES.DIAGNOSTICS)" @doctor="openDoctor" />
        <div v-if="offlineShown" class="panel-sim-end"><DevSimChip :locale="locale" /></div>
      </template>
    </MScene>
    </div>
    <MContextMenu ref="contextMenu" @select="onContextSelect" />
    <MContextProvider :locale="locale" :callbacks="ctxCallbacks" :capabilities="{ product: 'panel', developer: developerMode.enabled.value }" />
    <DevReportHost :locale="locale" />
    <!-- every uncaught error is drawn in our own surface: the way back, the restart of Sunshine, then copy, download and the report to the developers -->
    <MErrorHost :locale="locale" report-sheet :can-go-back="nav.canPop.value" :host-online="hostConn.state.status === 'online'" :restart="() => hostConn.restart('service')" :doctor="openDoctor" :open-url="actions_openExternal" @back="errorBack" />
    <!-- 更多 of the bottom bar: what the bar has no room for -->
    <MSheet v-model="moreOpen" :title="t.m3e.nav.moreTitle" side="end" :close-label="t.m3e.home.done">
      <div class="panel-more-version"><MBrand :size="20" /><MCaptionVersion :version="currentVersion" :build="buildInfo" :os="osName()" :locale="locale" :update="notice.version" @copied="onVersionCopied" @open-update="moreGo(openUpdates)" /></div>
      <MGroup>
        <!-- what the caption bar has no room for (its tier, MCaption): in the order it has there -->
        <MItem v-if="caption.tier.value >= 5" action icon="person_add" :title="connectT.title" :disabled="hostUnusable" @click="moreGo(() => connect.show('pair'))" />
        <MItem v-if="caption.tier.value >= 4" action icon="notifications" :title="announcementTexts(locale).button" @click="moreGo(() => router.navigate(ROUTES.ANNOUNCEMENTS))" />
        <MItem v-if="caption.tier.value >= 4" action icon="feedback" :title="feedbackTexts(locale).button" @click="moreGo(() => openFeedbackHere({}))" />
        <MItem action icon="search" :title="t.m3e.search" @click="moreGo(() => uni.show())" />
        <MItem action icon="settings" :title="t.m3e.nav.settings" :text="settingsList.lines.value.panel" :tone="notice.dot ? 'warn' : ''" @click="moreGo(() => onRail({ id: 'settings' }))" />
        <MItem v-if="developerMode.enabled.value" action icon="terminal" :title="devmodeTexts(locale).rail" @click="moreGo(() => router.navigate(ROUTES.DEV_HUB))" />
        <MItem action icon="palette" :title="appearanceLabel" :text="settingsList.lines.value['config:appearance']" @click="moreGo(() => router.navigate(ROUTES.SETTINGS_HUB, { section: 'appearance' }))" />
        <MItem action icon="info" :title="aboutTexts(locale).title" :text="currentVersion" @click="moreGo(() => router.navigate(ROUTES.ABOUT, undefined, { root: true }))" />
        <MItem action icon="help" :title="t.m3e.nav.help" :text="settingsList.lines.value.help" @click="moreGo(() => onRail({ id: 'help' }))" />
        <MItem action icon="favorite" :title="t.m3e.home.sponsor" @click="moreGo(() => { sponsorOpen = true })" />
      </MGroup>
    </MSheet>
    <MAnnouncementPopup :item="announcements.popup.value" :locale="locale" @shown="announcements.markShown" @acknowledge="announcements.acknowledge" @hide="announcements.hide" @open-url="actions_openExternal" />
    <MCoachMarks :locale="locale" storage-key="foundation.coach.panel" guide-key="foundation.guide.panel" />
    <MRemoteWindow v-model="remoteOpen" :locale="locale" :state="remoteData.state" :callbacks="remoteData.callbacks" />
    <MConnectDialog v-model="connect.open.value" v-model:tab="connect.tab.value" v-model:os="connect.os.value" :assets-of="connect.assetsOf" :version-of="connect.versionOf" :locale="locale" :qr="connect.matrix" :pair-qr="connect.pairQr" :count="hostClients.length" :latest="hostClients.length ? hostClients[hostClients.length - 1].name : ''" :close-label="t.m3e.home.done"
      @pair-pin="(e) => connect.pairPin(e.pin, e.name)" @pair-qr-begin="connect.qrBegin()" @pair-qr-stop="connect.qrStop()" @open-url="actions_openExternal" @open-downloads="() => { connect.open.value = false; jumps.jump(() => openRoute(ROUTES.WEB_DOWNLOADS)) }" />
    <MPairPrompt :requests="pairing.requests.value" :locale="locale" @submit="onPairSubmit" @reject="(id) => pairing.reject(id)" @install="connect.show('install')" />
    <PanelPending :locale="locale" @go="onPendingGo" />
    <PanelBusyLayer />
    <MSupportSheet v-model="sponsorOpen" :title="t.m3e.home.sponsorTitle" :sections="sponsorSections" :close-label="t.m3e.home.done" @open="(it) => actions_openExternal(it.url)" />
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
import { computed, defineAsyncComponent, getCurrentInstance, nextTick, provide, ref, toRef, watch, onMounted, onBeforeUnmount } from 'vue'
import { sunshine, sunshineService, tools, openExternalUrl } from '../tauri-adapter.js'
import { SPONSORS, sponsorUrl } from '../config/sponsors.js'
import QRCode from 'qrcode'
import { invoke } from '@tauri-apps/api/core'
import { qrSvgDataUrl } from '../composables/qrSvg.js'
import { audioDevicesChannel } from '../composables/deviceStatusStore.js'
import { openBigScreenWithNotice } from '../composables/openBigScreen.js'
import { installMenuPopup } from '../composables/menuPopup.js'
import { installSearchShortcut } from '../composables/searchShortcut.js'
import { installTrayTheme } from '../composables/trayTheme.js'
import { announcementTexts, feedbackTexts, MFeedbackButton, openFeedbackHere, askDevReport, devReport, DevSimChip, MContextProvider, contextTexts, pinSpecForPage, pinTexts, useTelemetryConsent, MErrorHost, installErrorHandlers, setErrorContext, startAutoReport, reportHostStop, reportHostHung, dismissHostHung, useNotify, useSystemNotices, MStatusBell, connectTexts, guideTexts, filterHash, MConnectDialog, MPairPrompt, pinToHome, unpinFromHome, isPinnedOnHome, createPairingPending, MSnackbar, useConnectDialog, MAnnouncementButton, MAnnouncementPage, MAnnouncementPopup, ackedAt, useAnnouncements, MCaption, MCoachMarks, MRemoteWindow, MUnifiedSearch, useUnifiedSearch, searchTexts, actionDocs, settingDocs, clientDocs, appDocs, tileDocs, addTileToHome, MHostOffline, useHostConnection, MSheet, MGroup, MItem, MSupportSheet, MBrand, MScene, MNavList, resetTexts, MCaptionVersion, MCaptionBack, MBackChip, useCaptionRoom, createJumpHistory, useBackVariant, showsButton, showsChip, captionTexts, MIconButton, MIcon, MContextMenu, aboutTexts, statusTexts, diagnosticsTexts, doctorTexts, useAnnouncementBadge, useSettingsList, useAppearance, registerModeHandler, MSponsorButton, scopeRecent, hostRecentKey, useErrors, useDeveloperMode, useDialogs, DevReportHost, DevHub, devmodeTexts, useDevActivity } from 'foundation-ui/vue'
import { developerOverlays, developerTexts, DEFAULT_FLAGS } from 'foundation-ui/developer'
import { SOURCE_KEY, afterCheck, afterOpened, noticeView, readNotice, writeNotice } from 'foundation-ui/update'
import { createPanelTelemetry } from '../composables/panelTelemetry.js'
import { panelVersion } from '../composables/panelVersion.js'
import { SEARCH_INDEX } from '@settings-data'
import { labelOf, settingsHash } from 'foundation-ui/settings/index.js'
import PanelHome from './PanelHome.vue'
import PanelBusyLayer from './PanelBusyLayer.vue'
import UpdateDownloadChip from './UpdateDownloadChip.vue'
import PanelWebHost from './PanelWebHost.vue'
import VddSettings from './VddSettings.vue'
import Welcome from './welcome.vue'
import { armStartGuide } from '../composables/startGuide.js'
import WebStreamSettings from './WebStreamSettings.vue'
import AiAssistant from './AiAssistant.vue'
import { mitaState } from '../composables/mitaState.js'
import SettingsHubPage from './m3e/SettingsHubPage.vue'
import StreamHubPage from './m3e/StreamHubPage.vue'
import HdrEnhancedManager from './HdrEnhancedManager.vue'
import ControlPanelPage from './ControlPanelPage.vue'
import MicrophonePage from './m3e/MicrophonePage.vue'
import AppearancePage from './m3e/AppearancePage.vue'
import UsbPage from './m3e/UsbPage.vue'
import AboutPage from './m3e/AboutPage.vue'
import DeveloperPages from './m3e/DeveloperPages.vue'
import { registerPanelReport } from '../composables/developerReport.js'
import ComponentStatusPage from './m3e/ComponentStatusPage.vue'
import SunshineServicePage from './m3e/SunshineServicePage.vue'
import { serviceTexts } from 'foundation-ui/service'
import { createHostRestart } from '../composables/hostRestart.js'
import { useComponentStatus } from '../composables/useComponentStatus.js'
import ToolboxPage from './m3e/ToolboxPage.vue'
import LatencyClockPage from './m3e/LatencyClockPage.vue'
import StreamTimerPage from './m3e/StreamTimerPage.vue'
import TestToolsPage from './m3e/TestToolsPage.vue'
import LiveSessionPage from './m3e/LiveSessionPage.vue'
import RtssPage from './m3e/RtssPage.vue'
import NetworkOverviewPage from './m3e/NetworkOverviewPage.vue'
import DiagnosticsPage from './m3e/DiagnosticsPage.vue'
import ResetSettingsPage from './m3e/ResetSettingsPage.vue'
import DoctorPage from './m3e/DoctorPage.vue'
import { MITA_PENDING_PROMPT } from '../composables/useAiAssistant.js'
import { useDeviceRuntime } from '../composables/useDeviceRuntime.js'
import { useGuideAutoOpen } from '../composables/useGuideAutoOpen.js'
import { useSidebarState } from '../composables/useSidebarState.js'
import { useWindowControls } from '../composables/useWindowControls.js'
import { useTools } from '../composables/useTools.js'
import { useTouchWindowDrag } from '../composables/useTouchWindowDrag.js'
import { createUtilityTools } from '../composables/toolsRegistry.js'
import { ROUTES, PAGES, APPEARANCE_ROUTES, routeForWebPath, routeOfPlace, webPathOf, isPhone } from '../composables/useRouter.js'
import { framedMove } from '../composables/frameFollow.js'
import { PANEL_PAGE_PATHS, requestPanelPage } from '../composables/panelParts.js'
import { proxyBlock } from '../composables/proxyBlock.js'
import { hostData } from '../composables/hostData.js'
import { createGuideData } from '../composables/guideData.js'
import PanelPending from './PanelPending.vue'
import { recordUpdateCheck } from '../composables/useUpdateCheckTime.js'
import { useHostReach } from '../composables/useHostReach.js'
import { ROW_OF_ROUTE, railItems, pageItems, barItems, barCurrent } from '../composables/panelNav.js'
import { rtssInstalled, readRtssInstall } from '../composables/rtssInstall.js'
import { rowKeywords, parseSettingsHash } from 'foundation-ui/settings/index.js'
import { hasUpdate } from '../composables/updateCheckError.js'
import { useI18n } from '../desktop/i18n/index.js'
import { useDualSenseStatus } from '../composables/useDualSenseStatus.js'
import { useDualSenseText } from '../composables/useDualSenseText.js'
let rtssRefresh = 0
const refreshRtssInstall = () => import('@tauri-apps/api/core')
  .then(({ invoke }) => readRtssInstall(invoke))
  .catch(() => { if (rtssInstalled.value === null) rtssInstalled.value = false })
const UpdateDialog = defineAsyncComponent(() => import('./UpdateDialog.vue'))
const props = defineProps({ webuiPath: { type: String, default: '/' } })
const emit = defineEmits(['route-change', 'webui-navigate'])
const { t, locale, choice: localeChoice, setChoice: setLocaleChoice, systemLocale } = useI18n()
const {
  isMaximized, isAdmin, showUpdateDialog, updateInfo, currentVersion, skippedVersion, router, themeMode, setTheme,
  openVddSettings, openWelcome, openWebStream, openAiAssistant, openControllers, openRtxHdr, skipVersion,
  includePrerelease, setIncludePrerelease,
} = useSidebarState()
const { minimizeWindow, toggleMaximize, closeWindow } = useWindowControls(isMaximized)
// the shared appearance (wallpaper, colour, light/dark); light/dark goes through the panel's own theme switch
registerModeHandler((mode) => setTheme(mode))
onMounted(() => { void useAppearance().init() })
// the overview has its own (lighter) frost over the wallpaper; every other page the stronger one
watch(() => router.currentRoute.value, (route) => useAppearance().setScope(route === ROUTES.HOME ? 'home' : 'other'), { immediate: true })
const { onTouchWindowDragStart } = useTouchWindowDrag(isMaximized, { onDoubleTap: toggleMaximize })
const {
  openUrl, cleanupCovers, restartAsAdmin, checkForUpdates, openGamepadTest, openStylusInputProbe,
  showClipboardSyncStatus, initClipboardSyncStatus, clipboardSyncEnabled, clipboardSyncStateWord,
} = useTools()
const { runtime, refresh: refreshRuntime } = useDeviceRuntime()
// First run: the guide opens by itself when the host has no setup and no paired client
useGuideAutoOpen(openWelcome)
// `--start-guide`: a new key remounts the guide so it reads the cleared progress at the first step
const guideRun = ref(0)
const openStartGuide = () => {
  armStartGuide()
  guideRun.value += 1
  openWelcome()
}
onMounted(() => {
  initClipboardSyncStatus()
  refreshRuntime()
  // RivaTuner Statistics Server is optional: read whether it is installed, the RTSS page exists only then. A login-time read can
  // happen before RTSS's autostart registers it, so keep the answer fresh while the window is open.
  refreshRtssInstall()
  rtssRefresh = setInterval(() => { if (!document.hidden) refreshRtssInstall() }, 60000)
})

const openWebUi = (path) => {
  const route = routeForWebPath(path)
  if (webPathOf(route)) emit('webui-navigate', path)
  router.navigate(route)
}
const goHome = () => openWebUi('/')
// 新手引导 opens over the page it was opened from (帮助与关于): closing it, with its button or Esc, goes back there; started alone (first run) it opens the overview
const closeWelcome = () => (router.nav.canPop.value ? router.goBack() : goHome())
// the result of a check also stands for "up to date" (is_latest, shown as the sheet of the newest version): only a result that offers a newer version is an update to show on the overview
const updateAvailable = hasUpdate
const handleCheckForUpdates = async (channel = null) => {
  const result = await checkForUpdates(channel)
  recordUpdateCheck()
  if (!result) return false
  updateInfo.value = result
  showUpdateDialog.value = true
  // the person asked and is looking at the answer
  remember(afterOpened(afterCheck(noticeState.value, { version: result.version, is_latest: result.is_latest }, { installed: installedKnown.value ? currentVersion.value : '' })))
  return true
}
const toolsCtx = {
  t, router, webuiPath: toRef(props, 'webuiPath'), openWebUi, openControllers, openUrl, openGamepadTest,
  openStylusInputProbe, cleanupCovers, restartAsAdmin, showClipboardSyncStatus, handleCheckForUpdates,
}
const toolsMenuItems = computed(() => createUtilityTools(toolsCtx))
// the component status: one controller for the page and for the home cards the catalogue offers
const componentStatus = useComponentStatus({
  locale,
  go: { vdd: () => jumpTo(ROUTES.VDD_SETTINGS), components: () => jumpTo(ROUTES.COMPONENTS), microphone: () => jumpTo(ROUTES.MICROPHONE), page: () => jumpTo(ROUTES.COMPONENT_STATUS) },
  update: () => ({ state: updateInfo.value ? (updateInfo.value.is_latest ? 'latest' : 'available') : '', next: updateInfo.value?.version || '' }),
  check: () => handleCheckForUpdates(),
})

// what is wrong with the host or the app right now goes to the status button of the caption bar
useSystemNotices({
  status: componentStatus,
  locale,
  go: { components: () => jumpTo(ROUTES.COMPONENTS), microphone: () => jumpTo(ROUTES.MICROPHONE), vdd: () => jumpTo(ROUTES.VDD_SETTINGS), update: () => { showUpdateDialog.value = true } },
  update: () => ({ state: updateInfo.value && !updateInfo.value.is_latest ? 'available' : '', next: updateInfo.value?.version || '' }),
})
const sponsorSections = computed(() => [
  { title: t.value.m3e.home.sponsorTitle, items: SPONSORS.map((x) => ({ id: x.id, icon: 'favorite', title: x.label, url: sponsorUrl(x, locale.value) })) },
  { items: [{ id: 'star', icon: 'star', title: t.value.m3e.home.star, url: 'https://github.com/AlkaidLab/foundation-sunshine' }, { id: 'qq', icon: 'groups', title: t.value.m3e.home.qq, url: 'https://qm.qq.com/q/AfMQoyKrkc' }] },
])
const actions_openExternal = (url) => openExternalUrl(url)
const nav = router.nav
// ---- the version in the caption bar and the notice of a waiting update (kept from the checks the updater already runs; nothing here asks the network)
const buildInfo = ref(null)
onMounted(async () => { try { buildInfo.value = await sunshine.getBuildInfo() } catch { buildInfo.value = null } })
const noticeState = ref(readNotice(localStorage))
const installedKnown = computed(() => /^[vV]?\d/.test(String(currentVersion.value || '')) && currentVersion.value !== '0.0.0')
const notice = computed(() => (installedKnown.value ? noticeView(noticeState.value, { installed: currentVersion.value, skipped: skippedVersion.value }) : { waiting: false, version: '', dot: false }))
const remember = (next) => { noticeState.value = next; writeNotice(localStorage, next) }
watch(updateInfo, (info) => { if (info) remember(afterCheck(noticeState.value, { version: info.version, is_latest: info.is_latest }, { installed: installedKnown.value ? currentVersion.value : '' })) }, { immediate: true })
// 软件更新 is on 面板设置 (关于 shows it too, and so does the sheet a person's own 检查更新 opens): opening one of them puts the dot out for the version that waits; a newer one brings it back.
// The sheet the updater opens by itself when it finds an update is not the person opening it: the dot stays.
const updatesOpen = computed(() => router.currentRoute.value === ROUTES.PANEL_SETTINGS || router.currentRoute.value === ROUTES.ABOUT)
watch([updatesOpen, () => notice.value.version], ([open]) => { if (open && noticeState.value.available) remember(afterOpened(noticeState.value)) }, { immediate: true })
const openUpdates = () => { jumpTo(ROUTES.PANEL_SETTINGS) }
const onVersionCopied = (text) => useNotify().snack(text ? captionTexts(locale.value).copiedNote : captionTexts(locale.value).copyFailed)
// ---- the global back: the places a person left by a jump (体检 → 网络, a home tile → its settings); the person's own moves clear it
const jumps = createJumpHistory({
  locate: () => ({ id: nav.current.value, params: nav.detail.value?.params, rail: nav.rail.value }),
  restore: (place) => restorePlace(place),
})
jumps.attach(nav.entries)
const jumpTo = (route, params) => jumps.jump(() => router.navigate(route, params))
const jumpConfig = (params) => jumps.jump(() => openConfig(params))
const { variant } = useBackVariant()
const backButton = computed(() => showsButton(variant.value))
const backChip = computed(() => showsChip(variant.value) && jumps.canBack.value)
// the name of the place Back goes to: the setting's tab, the page's own name, or its section
const backPage = computed(() => {
  const place = jumps.top.value
  if (!place) return ''
  if (place.params?.tab) return labelOf(locale.value, `tabs.${place.params.tab}`) || railLabel(place.rail)
  const row = ['stream', 'input', 'audio', 'display', 'system', 'appearance', 'settings', 'help'].flatMap((r) => listItems(r)).find((i) => i.id === place.id)
  return row?.label || (place.id === ROUTES.HOME ? t.value.m3e.nav.overview : '') || railLabel(place.rail)
})
const restorePlace = (place) => {
  if (place.params?.tab || place.id === ROUTES.WEB_CONFIG) { openConfig(place.params || { tab: 'general' }); return }
  const path = webPathOf(place.id)
  if (path) emit('webui-navigate', path)
  router.navigate(place.id, place.params)
}
// The frame moved by itself (a link inside a Web page, an address, a place set by the page's hash): the route follows what is displayed, so the rail, the list row and the title are those of the page that is
// shown (composables/frameFollow.js decides; it is a jump, so Back returns to the page the link was on). Only while the frame IS what is displayed: behind a page of the panel it may load whatever it likes.
const followFrame = (path) => {
  if (!isWebPage.value) return
  const move = framedMove(path, { route: router.currentRoute.value, params: nav.detail.value?.params }, { routeForWebPath, routeOfPlace, parseHash: (hash) => parseSettingsHash(hash, SEARCH_INDEX), configRoute: ROUTES.WEB_CONFIG })
  if (move) jumps.jump(() => router.navigate(move.route, move.params))
}
// the back stack mirrors into the window's history (as in the Web UI): the back arrow of a pushed page, Esc, the mouse's back button and the browser's agree; without it the arrow only went back in the history and the page stayed
onMounted(() => { nav.widen(); nav.attach() })
onBeforeUnmount(() => nav.detach())
const isWebPage = computed(() => !!webPathOf(router.currentRoute.value))
// The frame shows the page of the route. Callers that open a Web page tell the frame themselves; a route that is set any other way (a link, a deep link, a caller that only
// switches the route) comes here: when the frame is not on the page of the route it is told, so a route never shows another page's content.
watch(() => router.currentRoute.value, (route) => {
  const path = webPathOf(route)
  if (path && props.webuiPath !== path && routeForWebPath(props.webuiPath) !== route) emit('webui-navigate', path)
}, { flush: 'post', immediate: true })
// The host (Sunshine) does not answer: the content area shows the offline state (Vivi, the actions, the countdown); the rail stays usable and the pages that
// do not need the host (the panel's settings, 米塔 AI, help, about, the diagnostics) stay as they are. When the host is back the frame loads again.
// The probe is made by the panel's own process (sunshine.probeHost), not by a fetch of this window: at a start the window's connections to the proxy are taken by the pages' first requests,
// and a probe queued behind them made a host that answers in milliseconds look silent.
// While the window is still connecting (it opened with the computer, the host may be seconds from its first answer: a cold host answered in 2.5 to 4 s on a freshly started VM) a probe waits longer.
const probeHost = async () => {
  const status = await hostData.probe(hostConn.state.status === 'connecting' ? 6000 : 2500)
  // The native probe reaches the configured host directly. Any valid HTTP
  // status proves that the listener answered; 503 is the host's temporary
  // overload/startup response, not a dead service. The shared connection
  // state requires consecutive probe failures before showing offline.
  // [panel-rfc9110 §15.6.4; panel-reqwest-client-builder-send Client::send]
  return status >= 100 && status < 600
}
// 重启 and 用户模式 of the offline page and the search actions confirm first, as 系统 › Sunshine 服务 does, and pass the acknowledgement on
const hostRestart = createHostRestart({ locale, ask: (options) => useDialogs().ask(options), status: () => sunshineService.status(), restart: (ack) => tools.restartSunshineService(ack), userMode: (ack) => tools.restartSunshineInUserMode(ack) })
let restartAck = false
const hostConn = useHostConnection({
  // the panel opens with the computer: the host may still be starting. Neutral `connecting` until the first answer or a certain failure (m3e status/hostConnection.js), never the error page for a slow start.
  starting: true,
  probe: probeHost,
  restartService: () => tools.restartSunshineService(restartAck),
  restartUser: () => tools.restartSunshineInUserMode(restartAck),
  onRecovered: () => { for (const frame of document.querySelectorAll('iframe')) { const src = frame.src; frame.src = src } },
  // A silence is not a stop. The connection reads what the operating system says about the host (the process and its start, the service, whether the panel asked for a stop) and the
  // error surface in its 「Sunshine 意外停止」 variant opens only when the process is gone or is a new one and nobody asked (m3e status/hostLoss.js; telemetry/crash.js has the interface
  // for the crash bundle the host leaves). A host that is silent while its process runs is 「主机无响应」 in the offline state, without the stop surface.
  facts: () => tools.getHostFacts(),
  onLost: ({ at }) => reportHostStop({ at }),
  // the process runs and has not answered for 30 s: the same dialog in its hung variant 「Sunshine 没有响应」 (继续等待 or 重启); it closes by itself when the host answers again
  onHung: ({ at, seconds }) => reportHostHung({ at, seconds }),
  onHungEnd: () => dismissHostHung(),
})
// The developer page shows the surfaces of a host that is away without the host going anywhere: 'offline' is the offline page, 'unresponsive' the page of a host that runs and does not answer, 'stopped' the 「意外停止」 error surface.
// It ends by itself after a minute (m3e's useHostSimulation counts the same minute and shows it; this timer is the one that puts the panel back if that page never runs again).
const hostSimulated = ref('')
let hostSimulatedTimer = 0
let hostStopSimulated = false
// tools/realshot and the QA harness show the failure states with events of the page itself (a real fault is not repeatable on a guarded instance, where the panel would show its window): the verdict and the numbers the card shows; the next real probe of a host that answers puts it back
const forceHostState = (event) => { const d = event.detail || {}; Object.assign(hostConn.state, { verdict: d.verdict || 'down', status: 'offline', fails: d.fails ?? 3, round: 1, nextIn: d.nextIn ?? 12, probing: false }) }
const forceProxyState = (event) => { proxyBlock.attempts = Number(event.detail?.attempts) || 4; proxyBlock.blocked = event.detail?.on !== false }
window.addEventListener('panel-force-host-state', forceHostState)
window.addEventListener('panel-force-proxy-state', forceProxyState)
onBeforeUnmount(() => { window.removeEventListener('panel-force-host-state', forceHostState); window.removeEventListener('panel-force-proxy-state', forceProxyState) })
// the pages of the panel itself, which work without the host: 面板设置, 个性化, 帮助与关于, 公告 (from the announcement feed), the diagnostics, 米塔 AI, the guide
const NEEDS_NO_HOST = new Set([ROUTES.DOCTOR, ROUTES.RESET, ROUTES.SUNSHINE_SERVICE, ROUTES.DEVELOPER, ROUTES.PANEL_SETTINGS, ...APPEARANCE_ROUTES, ROUTES.AI_ASSISTANT, ROUTES.HELP, ROUTES.ABOUT, ROUTES.ANNOUNCEMENTS, ROUTES.SETTINGS_HUB, ROUTES.DIAGNOSTICS, ROUTES.WELCOME, ROUTES.DEV_ISSUES, ROUTES.DEV_LOGS, ROUTES.DEV_API, ROUTES.DEV_FLAGS, ROUTES.DEV_BUILD, ROUTES.DEV_TESTS, ROUTES.DEV_HUB])
// The host cannot be used for one of three causes, and the content area is then ONE status page (MScene status): the host is down, it runs and does not answer, or the panel's own proxy does not answer the window.
// The rail and the caption bar stay: the pages of the panel itself (NEEDS_NO_HOST) still work and open as always. What needs the host does not look live: the pages give way to the status page and are inert, the
// frame stays mounted but hidden, so an unsaved edit is there when the connection is back, and 连接设备 (it pairs through the host) is disabled.
const hostUnusable = computed(() => hostSimulated.value === 'offline' || hostSimulated.value === 'unresponsive' || proxyBlock.blocked || hostConn.state.status !== 'online')
const offlineCause = computed(() => (hostSimulated.value === 'offline' ? 'down' : hostSimulated.value === 'unresponsive' ? 'unresponsive' : proxyBlock.blocked ? 'proxy' : hostConn.state.verdict === 'unresponsive' ? 'unresponsive' : 'down'))
const offlineState = computed(() => (hostSimulated.value === 'offline' ? { ...hostConn.state, status: 'offline', verdict: 'down', fails: 3, round: 1, nextIn: 5, probing: false, error: '' } : hostSimulated.value === 'unresponsive' ? { ...hostConn.state, status: 'offline', verdict: 'unresponsive', fails: 3, round: 1, nextIn: 5, probing: false, error: '' } : proxyBlock.blocked ? { status: 'offline', verdict: 'proxy', fails: proxyBlock.attempts, round: 1, nextIn: 0, probing: proxyBlock.checking, mode: '', error: '' } : hostConn.state))
const retryOffline = () => (proxyBlock.blocked ? proxyBlock.retryNow() : hostConn.retry())
const offlineShown = computed(() => hostUnusable.value && !NEEDS_NO_HOST.has(router.currentRoute.value))
watch(hostUnusable, (on) => { if (on) connect.open.value = false })
function simulateHost(kind) {
  clearTimeout(hostSimulatedTimer)
  const wasStopped = hostStopSimulated
  hostStopSimulated = kind === 'stopped'
  hostSimulated.value = kind === 'offline' || kind === 'unresponsive' ? kind : ''
  if (kind === 'stopped') reportHostStop({ at: Date.now() })
  else if (wasStopped || !kind) useErrors().dismissError() // the error surface of a simulated stop goes when the simulation ends or turns into another one
  if (kind) hostSimulatedTimer = setTimeout(() => simulateHost(''), 60000)
}
const back = () => nav.pop()
// the way out of the error surface: the previous page, or the overview when there is none
// The choice about usage counts and error information: the host keeps it, the Web UI and this window agree; both are on by default, nobody is asked (the first run tells once), changed in 关于 › 隐私
const telemetryApi = { getJson: async (path) => (await fetch(`${String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')}${path}`)).json(), postJson: async (path, body) => (await fetch(`${String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')}${path}`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })).json() }
const telemetry = useTelemetryConsent({ api: telemetryApi })
// Usage counts, experience numbers and errors (telemetry v1): handed to the host, which owns the two choices, the identity and the queue; one page time per route change
const panelTelemetry = createPanelTelemetry({
  getBase: async () => sunshine.getProxyUrl(),
  getVersion: panelVersion,
  // the source the last download really came from, else the setting (the native side never answers empty); outside the app: the remembered choice
  getUpdateSource: async () => { try { return String(await invoke('get_update_source_used')) } catch { try { return localStorage.getItem(SOURCE_KEY) || 'auto' } catch { return 'auto' } } },
  // why the chosen source (Mirror酱) was not the one used in the last download: version_lag, cdk_invalid, quota, network, no_cdk; empty when it was used or not chosen
  getUpdateSourceMiss: async () => { try { return String(await invoke('get_update_source_miss')) } catch { return '' } },
})
watch(() => nav.current.value, (page) => { if (page) panelTelemetry.page(String(page)) }, { immediate: true })
// the first-run guide counts its steps through the same sender (guide_funnel; usage statistics choice)
provide('guideFunnel', (props) => panelTelemetry.sender.track.guideFunnel(props))
const errorBack = () => { if (nav.canPop.value) nav.pop(); else router.navigate(ROUTES.HOME) }
const osName = () => (typeof navigator === 'undefined' ? '' : String(navigator.userAgentData?.platform || navigator.platform || ''))
// the last host log lines go along with a report (scrubbed by buildReport): read once at start, as the Web UI does
const recentLogText = ref('')
const refreshLogs = async () => { try { const r = await fetch(`${String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')}/api/logs`); if (r.ok) recentLogText.value = (await r.text()).split('\n').slice(-60).join('\n') } catch { /* a report without log lines */ } }
const errorContext = () => ({ product: 'panel', version: currentVersion.value || '', os: osName(), page: String(router.currentRoute.value || ''), locale: String(locale.value), logs: String(recentLogText.value || '').split('\n').slice(-40).join('\n'), components: `panel=${currentVersion.value || '?'}` })
setErrorContext(errorContext)
// the report and sign-in sheets of foundation-ui get what only this window has (the capture, the log, the environment, saving a file, the browser, the anonymous send)
registerPanelReport({ locale: () => locale.value, version: () => currentVersion.value || '', errorContext, hostNames: () => [hostConfig.value?.sunshine_name, ...hostClients.value.map((c) => c.name)].filter(Boolean) })
const DEV_PAGES = [ROUTES.DEV_ISSUES, ROUTES.DEV_LOGS, ROUTES.DEV_API, ROUTES.DEV_FLAGS, ROUTES.DEV_BUILD, ROUTES.DEV_TESTS, ROUTES.DEVELOPER]
const devHubPages = computed(() => [{ id: 'issues', route: ROUTES.DEV_ISSUES }, { id: 'logs', route: ROUTES.DEV_LOGS }, { id: 'api', route: ROUTES.DEV_API }, { id: 'flags', route: ROUTES.DEV_FLAGS }, { id: 'build', route: ROUTES.DEV_BUILD }, { id: 'tests', route: ROUTES.DEV_TESTS }, { id: 'options', route: ROUTES.DEVELOPER }])
const leaveDeveloper = () => router.navigate(ROUTES.HOME)
let removeErrorHandlers = null
const shellApp = getCurrentInstance()?.appContext.app
// the remote access tutorials (help page, and the fix of a reason of the reachability check)
const remoteOpen = ref(false)
const remoteData = createGuideData({ proxyUrl: () => sunshine.getProxyUrl(), openUrl: (url) => actions_openExternal(url), copy: (text) => navigator.clipboard.writeText(text), reach: useHostReach(), vddStatus: async () => null, openApps: () => {}, navigate: () => {} })
const announceVersion = ref('')
// the offline tips name the port of the Web UI this panel talks to (the host's real one, not the default 47990)
const hostPort = ref(47990)
onMounted(async () => { try { const port = Number(new URL(String(await sunshine.getUrl())).port); if (port) hostPort.value = port } catch { /* the default stays */ } })
onMounted(async () => { try { announceVersion.value = (await sunshine.getVersion()) || '' } catch { /* the version only narrows version-bound notices */ } })
const announcements = useAnnouncements({ locale, version: announceVersion })
const appearanceLabel = computed(() => labelOf(locale.value, 'shell.appearance'))
const announcementsUnread = announcements.unread
// a dot on the help rail while an update is waiting (the about page checks and opens the update sheet)
const rail = computed(() => railItems(t.value, { developer: devmodeTexts(locale.value).rail }).filter((item) => item.id !== 'developer' || developerMode.enabled.value).map((item) => (item.id === 'overview' && announcementsUnread.value ? { ...item, badge: announcementsUnread.value } : item.id === 'settings' && notice.value.dot ? { ...item, badge: 'dot' } : item.id === 'assistant' && mitaState.unread ? { ...item, badge: 'dot' } : item.id === 'developer' && devActivity.unseen.value ? { ...item, badge: 'dot' } : item)))
// the one-time thank-you line after the guided first run
const railLabel = (id) => (id === 'developer' ? devmodeTexts(locale.value).rail : id === 'appearance' ? appearanceLabel.value : t.value.m3e.nav[id])
// Developer mode (seven taps on the version in 关于): the row of 开发者选项 in 系统, and the overlays the page chose, applied at start
const developerMode = useDeveloperMode()
const devActivity = useDevActivity() // the dot of the 开发者 rail: something new in the host's repository since the last look at Issue 与 PR
watch(developerMode.enabled, (on) => { if (on) { developerOverlays(); devActivity.start() } else if (!on) { devActivity.stop(); developerOverlays().set({ ...DEFAULT_FLAGS }); if (nav.rail.value === 'developer') leaveDeveloper() } }, { immediate: true })
const runtimeView = computed(() => ({ ...runtime, developer: developerMode.enabled.value, dev: devmodeTexts(locale.value), developerTitle: developerTexts(locale.value).title, rtssInstalled: rtssInstalled.value === true, lines: settingsList.lines.value, statusTitle: statusTexts(locale.value).title, serviceTitle: serviceTexts(locale.value).title, diagTitle: diagnosticsTexts(locale.value).title, doctorTitle: doctorTexts(locale.value).title, resetTitle: resetTexts(locale.value).title }))
const listItems = (id) => pageItems(t.value, id, runtimeView.value)
// The stream settings are the shared model (categories in groups with a live line, search of every setting, recently used)
const hostConfig = ref(null)
const dualsenseKeys = ref([])
const hostClients = ref([])
const hostApps = ref([])
const settingsList = useSettingsList({ locale, config: hostConfig, dualsenseKeys, sources: { clients: hostClients, apps: hostApps, components: computed(() => componentStatus.rows.value), rtss: computed(() => ({ installed: rtssInstalled.value === true })), ds: computed(() => runtime.ds), mic: computed(() => runtime.mic), usb: computed(() => runtime.usb), audio: computed(() => audioDevicesChannel.data.value?.render || null) } })
// the playback devices of the host: the audio row names the chosen device, the setting holds an id
onMounted(() => { audioDevicesChannel.fetch().catch(() => null) })
// the settings used last belong to the host this window manages: the proxy target is this machine's host while it is a loopback address, and the origin of the host it points at
// otherwise (the panel accepts a --url that turns the whole window to another host); the first scoping keeps the list this window already has
onMounted(async () => {
  const target = await sunshine.refreshTarget()
  try {
    const u = new URL(String(target))
    const local = u.hostname === 'localhost' || u.hostname === '127.0.0.1' || u.hostname === '[::1]' || u.hostname === '::1'
    scopeRecent(hostRecentKey(local ? 'local' : u.origin))
  } catch { scopeRecent(hostRecentKey('local')) }
})
const keptHostConfig = hostData.peek('config')
if (keptHostConfig) hostConfig.value = keptHostConfig // the list draws its live lines from the last answer of the host at once; the fresh one replaces it
const loadHostData = async () => {
  try {
    const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
    const [config, tuning] = await Promise.allSettled([hostData.refreshConfig(), fetch(`${base}/api/dualsense/tuning`).then((r) => (r.ok ? r.json() : null))])
    if (config.status === 'fulfilled' && config.value) hostConfig.value = config.value
    if (tuning.status === 'fulfilled' && tuning.value?.keys) dualsenseKeys.value = tuning.value.keys
    const [clients, apps] = await Promise.allSettled([fetch(`${base}/api/clients/list`).then((r) => (r.ok ? r.json() : null)), fetch(`${base}/api/apps`).then((r) => (r.ok ? r.json() : null))])
    if (clients.status === 'fulfilled' && clients.value?.named_certs) hostClients.value = clients.value.named_certs
    if (apps.status === 'fulfilled' && Array.isArray(apps.value?.apps)) hostApps.value = apps.value.apps
  } catch { /* the list shows no live lines and the search the pages only */ }
}
// a row marked `separated` (the 高级 of 显示与视频) is a group of its own at the end of the list
const listGroups = (rail) => {
  if (rail === 'config') return settingsList.groups.value
  const items = listItems(rail)
  const apart = items.filter((i) => i.separated)
  return apart.length ? [{ id: rail, items: items.filter((i) => !i.separated) }, { id: `${rail}-apart`, items: apart }] : [{ id: rail, items }]
}
const currentRow = computed(() => {
  const route = nav.current.value
  if (route === ROUTES.WEB_CONFIG) return `config:${nav.detail.value?.params?.tab || 'general'}`
  if (route === ROUTES.WEB_PASSWORD) return 'password'
  if (route === ROUTES.WEB_TROUBLESHOOTING) return ROUTES.DIAGNOSTICS
  return route
})
// the settings of the display and audio rails are not under 主机设置: their path starts with the rail's own name (the category of the entry)
const settingsPlace = (e) => (e.params?.tab === 'display' || e.params?.tab === 'audio' || e.params?.tab === 'input' ? '' : railLabel('config'))
const searchItems = computed(() => [
  ...Object.keys(PAGES)
    // 恢复默认设置 is found as an action (resetSettings): the page is not a second result of the same name
    .filter((id) => PAGES[id].list !== false && id !== ROUTES.RESET)
    .flatMap((id) => {
      const item = pageItems(t.value, PAGES[id].rail, runtimeView.value).find((p) => p.id === id)
      return item ? [{ id, title: item.label, path: railLabel(PAGES[id].rail), icon: item.icon, route: id, keywords: rowKeywords(ROW_OF_ROUTE[id]) }] : []
    }),
  // 米塔 AI is a destination of the rail (no list pane): a result of its own, and one for the settings behind the gear of its chat
  { id: ROUTES.AI_ASSISTANT, title: t.value.m3e.nav.assistant, path: t.value.m3e.nav.main, icon: 'smart_toy', route: ROUTES.AI_ASSISTANT, keywords: rowKeywords('assistant') },
  // 新手引导 is the page of the first run: found by its name and by what people type for it (it is a row of 帮助与关于 too)
  { id: ROUTES.WELCOME, title: guideTexts(locale.value).guide, path: railLabel('help'), icon: 'rocket_launch', route: ROUTES.WELCOME, keywords: 'guide welcome wizard first run setup 向导 引导 首次 新手' },
  // the rail of the window itself (音频, 显示与视频 ...): found by its name; the ones that are pages of their own (米塔 AI, 设置, 帮助) are listed above
  ...rail.value.filter((r) => ['overview', 'stream', 'config', 'input', 'audio', 'display', 'system'].includes(r.id)).map((r) => ({ id: `rail-${r.id}`, title: r.label, path: '', icon: r.icon, route: `rail-${r.id}`, railId: r.id, keywords: '' })),
  ...settingsList.searchItems.value,
])
// ---- the unified search: one entry in the caption bar (an icon and a full-screen view on a narrow window) over everything a person can look for ----
const compactSearch = ref(typeof window !== 'undefined' && window.innerWidth < 600)
// the version sits beside the name: when what is left of the bar cannot show the search field with its hint, the search is its icon (as on a phone) instead of being cut
const caption = useCaptionRoom()
let wasPhone = isPhone()
const onSearchResize = () => { compactSearch.value = window.innerWidth < 600; const now = isPhone(); if (now !== wasPhone) { wasPhone = now; if (now) nav.narrow(); else nav.widen() } }
onMounted(() => window.addEventListener('resize', onSearchResize))
onBeforeUnmount(() => window.removeEventListener('resize', onSearchResize))
const searchT = computed(() => searchTexts(locale.value))
// 连接设备: the dialog of the caption button and of the search; the host API through the proxy, like the overview's pairing card
const connectT = computed(() => connectTexts(locale.value))
const connectNotice = ref('')
// short results go to the one snackbar of the window
watch(connectNotice, (message) => { if (message) { useNotify().snack(message); connectNotice.value = '' } })
const hostApiOf = async (path, init) => {
  const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
  const response = await fetch(`${base}${path}`, init)
  let data = null
  try { data = await response.json() } catch { /* the proxy may have no JSON body */ }
  if (!response.ok) throw Object.assign(new Error(data?.error || `HTTP ${response.status}`), { status: response.status })
  return data
}
const connect = useConnectDialog({
  hostApi: { getJson: (path) => hostApiOf(path), postJson: (path, body) => hostApiOf(path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) }) },
  render: (url) => qrSvgDataUrl(QRCode.create(url, { errorCorrectionLevel: 'M' }).modules),
  matrix: (text) => QRCode.create(text, { errorCorrectionLevel: 'M' }).modules,
  say: (kind, reason) => {
    if (kind === 'sent') connectNotice.value = connectT.value.pinSent
    else connectNotice.value = ({
      incorrect: connectT.value.pinIncorrect || connectT.value.pinFailed,
      timeout: connectT.value.pinTimeout || connectT.value.pinFailed,
      network: connectT.value.pinNetwork || connectT.value.pinFailed,
      'not-ready': connectT.value.pinNotReady || connectT.value.pinFailed,
    }[reason] || connectT.value.pinFailed)
  },
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
let unlistenFeedback = null
let unlistenShowMessage = null
let stopCrashWatch = null
onMounted(async () => {
  pairing.start()
  window.addEventListener('message', onFramePin)
  try { const { listen } = await import('@tauri-apps/api/event'); unlistenConnect = await listen('open-connect', (event) => connect.show(event.payload?.tab === 'install' ? 'install' : 'pair')) } catch { /* not in the app window */ }
  // the tray menu's 反馈问题… opens the one feedback panel; a crash that was recovered at the start asks once whether to tell the developers (a short decision in a centred dialog, then the drawer); the answer of `crash_recovered_ask` is true once, when the previous run left a crash report behind
  try {
    const { listen } = await import('@tauri-apps/api/event')
    unlistenFeedback = await listen('open-feedback', () => { void openFeedbackHere({}) })
    unlistenShowMessage = await listen('show-message', (event) => {
      const msg = String(event?.payload?.message || '').trim()
      if (msg) useNotify().snack(msg)
    })
    if (await invoke('crash_recovered_ask').catch(() => false)) askDevReport('crash')
    // the answer (feedback sent, or let go) is what stops the question: a program that ends before it asks again at the next start
    stopCrashWatch = watch(() => devReport.decision, (now, was) => { if (was === 'crash' && !now) invoke('crash_recovered_answered').catch(() => {}) })
  } catch { /* not in the app window */ }
})
onBeforeUnmount(() => { pairing.stop(); window.removeEventListener('message', onFramePin); if (unlistenConnect) unlistenConnect(); if (unlistenFeedback) unlistenFeedback(); if (unlistenShowMessage) unlistenShowMessage(); stopCrashWatch?.(); removeErrorHandlers?.() })
onMounted(async () => { removeErrorHandlers = installErrorHandlers({ app: shellApp }); startAutoReport({ context: errorContext }); void refreshLogs(); void panelTelemetry.start({ startMs: typeof performance !== 'undefined' ? performance.now() : undefined }); await telemetry.refresh() })
// what the buttons of 日志与诊断 do, from the search: the same host calls, the answer in the snackbar
const hostAction = async (path, doneText, method = 'POST') => {
  try {
    const base = String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
    const r = await fetch(`${base}${path}`, { method })
    if (!r.ok) throw new Error(`HTTP ${r.status}`)
    useNotify().snack(doneText)
  } catch { useNotify().snack(diagnosticsTexts(locale.value).failed) }
}
async function confirmedRestart(mode) {
  const answer = await hostRestart.confirm(mode === 'user' ? 'user-mode' : 'restart')
  if (!answer) return
  restartAck = answer.acknowledgeLiveStream
  await hostConn.restart(mode)
}
const actionHandlers = {
  restart: () => hostRestart.run('restart'),
  userMode: () => hostRestart.run('user-mode'),
  restartDriver: () => tools.restartGraphicsDriver(),
  connect: () => connect.show('pair'),
  download: () => connect.show('install'),
  exportDiag: () => jumpTo(ROUTES.DIAGNOSTICS),
  checkUpdate: () => handleCheckForUpdates(),
  logs: () => jumpTo(ROUTES.DIAGNOSTICS),
  addApp: () => jumps.jump(() => { emit('webui-navigate', '/apps#guide-add'); router.navigate(ROUTES.WEB_APPS) }),
  scanGames: () => jumps.jump(() => { emit('webui-navigate', '/apps#guide-scan'); router.navigate(ROUTES.WEB_APPS) }),
  stopApp: () => hostAction('/api/apps/close', diagnosticsTexts(locale.value).forceClose),
  resetDisplay: () => hostAction('/api/reset-display-device-persistence', diagnosticsTexts(locale.value).resetDisplay),
  setupGuide: () => openWelcome(),
  quitHost: () => hostAction('/api/boom', diagnosticsTexts(locale.value).boom, 'GET'),
  openDesktop: () => openBigScreenWithNotice((m) => useNotify().snack(m), t.value.sidebar.openBigScreenFailed),
  doctor: () => openDoctor(),
  resetSettings: () => jumpTo(ROUTES.RESET),
  // 反馈: the form of MErrorHost (mounted below, so it is always there); the help page has the same row
  feedback: () => { useErrors().reportFeedback() },
}
const searchDocs = computed(() => [
  ...settingDocs(searchItems.value, { prefix: settingsPlace, locale: locale.value }),
  ...actionDocs(locale.value, actionHandlers, searchT.value.types.action),
  ...clientDocs(hostClients.value, { place: `${railLabel('stream')}` }),
  ...appDocs(hostApps.value, { place: `${railLabel('stream')}` }),
  ...tileDocs(locale.value),
])
const recentSettingDocs = computed(() => settingDocs(settingsList.recentView.value.items.map((r) => r.entry), { prefix: settingsPlace, locale: locale.value }))
const onUnifiedChoose = (doc) => jumps.jump(() => {
  if (doc.type === 'page' && doc.entry?.railId) onRail({ id: doc.entry.railId })
  else if (doc.type === 'setting' || doc.type === 'page') onSearch(doc.entry)
  else if (doc.type === 'client') { emit('webui-navigate', `/pin${filterHash(doc.client?.name || doc.title)}`); router.navigate(ROUTES.WEB_CLIENTS) }
  else if (doc.type === 'app') { emit('webui-navigate', `/apps${filterHash(doc.app?.name || doc.title)}`); router.navigate(ROUTES.WEB_APPS) }
  else if (doc.type === 'tile') { addTileToHome('panel-home', doc.tile); router.navigate(ROUTES.HOME) }
})
const uni = useUnifiedSearch({ docs: searchDocs, onPin: (doc) => pinToHome('panel-home', doc.pin), isPinned: (doc) => isPinnedOnHome('panel-home', doc.pin), recentSettings: recentSettingDocs, onChoose: onUnifiedChoose, shortcuts: false })
// the shared search judges a field by the event target, which is the host element of a Material field; this guard looks through the whole path
let removeSearchShortcut = () => {}
onMounted(() => { removeSearchShortcut = installSearchShortcut(uni) })
onBeforeUnmount(() => removeSearchShortcut())
let searchBeforeHide = null

// Capture on Window before the search component handles the outside press
// on Document (panel-memory-dom-dispatch, Dispatching events).
function captureHideIntent(event) {
  searchBeforeHide = event.button === 0 && event.target?.closest?.('[data-panel-hide]')
    ? uni.query.value : null
}
function clearHideIntent() { searchBeforeHide = null }

async function hidePanel(event) {
  if (event.detail > 0 && searchBeforeHide !== null) uni.query.value = searchBeforeHide
  clearHideIntent()
  await nextTick()
  await window.__panelViewState?.flush()
  await closeWindow()
}

onMounted(() => {
  window.addEventListener('pointerdown', captureHideIntent, true)
  window.addEventListener('pointercancel', clearHideIntent, true)
})
onBeforeUnmount(() => {
  window.removeEventListener('pointerdown', captureHideIntent, true)
  window.removeEventListener('pointercancel', clearHideIntent, true)
})
// Where an error of the diagnostics page leads
// 体检: the entries of help and search open it running; a fix leads to a setting row or a page of the panel; 米塔 AI gets the result
// each press of 「开始体检」 carries its own token: the history entry keeps it, so Back from a fix finds the result instead of running every check again (MDoctor)
const openDoctor = () => jumps.jump(() => router.navigate(ROUTES.DOCTOR, { autostart: Date.now() }))
const DOCTOR_PAGES = {
  network: () => openConfig({ tab: 'network' }),
  audio: () => openConfig({ tab: 'audio', sub: 'sound' }),
  display: () => openConfig({ tab: 'display', sub: 'output' }),
  vdd: () => router.navigate(ROUTES.VDD_SETTINGS),
  components: () => router.navigate(ROUTES.COMPONENTS),
  logs: () => router.navigate(ROUTES.DIAGNOSTICS),
  config: () => openConfig({ tab: 'files' }),
  update: () => router.navigate(ROUTES.ABOUT),
}
const goDoctorTarget = (target) => jumps.jump(() => {
  const [kind, id] = String(target).split(':')
  if (kind === 'setting') { const entry = settingsList.searchItems.value.find((e) => e.keyName === id); if (entry) openSettingsRoute(entry) }
  else DOCTOR_PAGES[id]?.()
})
const askMita = (prompt) => {
  try { sessionStorage.setItem(MITA_PENDING_PROMPT, prompt) } catch { /* the chat opens empty */ }
  router.navigate(ROUTES.AI_ASSISTANT)
}
const goDiagnosticsPlace = (place) => jumps.jump(() => {
  if (place === 'drivers') router.navigate(ROUTES.COMPONENTS)
  else if (place === 'encoder') openConfig({ tab: 'display', sub: 'encoders' })
  else if (place === 'network') openConfig({ tab: 'network' })
  else if (place === 'audio') openConfig({ tab: 'audio', sub: 'sound' })
  else if (place === 'display') openConfig({ tab: 'display', sub: 'output' })
})
// A place of the stream settings: the list pane opens the Web page in the frame at that tab, section and row
const openConfig = (params = { tab: 'general' }) => {
  // The Web row stays in 捕获与视频, while the panel's native manager has its own 画质增强 place.
  if (params.focus === 'hdr_enhanced') { router.navigate(ROUTES.HDR_ENHANCED); return }
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
const onPendingGo = ({ page, entry, route }) => jumps.jump(() => {
  if (route) openRoute(route)
  else if (page.startsWith('config:')) openConfig({ tab: entry?.tab || page.slice(7), sub: entry?.sub, focus: entry?.focus })
  else openRoute(page === 'apps' ? ROUTES.WEB_APPS : ROUTES.GAMEPAD)
})
const onListNavigate = (item) => (item.route ? openSettingsRoute(item) : openRoute(item.id))

const sceneTexts = computed(() => ({ resize: t.value.m3e.resize, main: t.value.m3e.nav.main, expand: t.value.m3e.nav.expand, collapse: t.value.m3e.nav.collapse, shortcuts: t.value.m3e.shortcuts, back: t.value.m3e.back, search: t.value.m3e.search, help: t.value.m3e.shortcutHelp, section: t.value.m3e.section, close: t.value.m3e.clear }))
// the bottom bar of a narrow window: five destinations, the settings categories behind one, the rest behind 更多
const bar = computed(() => barItems(t.value).map((item) => (item.id === 'overview' && announcementsUnread.value ? { ...item, badge: announcementsUnread.value } : item.id === 'more' && notice.value.dot ? { ...item, badge: 'dot' } : item)))
const barCur = computed(() => (router.currentRoute.value === ROUTES.SETTINGS_HUB ? 'settings-hub' : barCurrent(nav.rail.value)))
const moreOpen = ref(false)
const sponsorOpen = ref(false)
const moreGo = (action) => { moreOpen.value = false; setTimeout(action, 120) }
const onRail = (item) => {
  if (item.id === 'settings-hub') { router.navigate(ROUTES.SETTINGS_HUB); return }
  if (item.id === 'more') { moreOpen.value = true; return }
  if (item.id === 'config') { loadHostData(); openConfig({ tab: 'general' }); router.navigate(ROUTES.CAT_CONFIG); return }
  // the stream rail opens on its first page: the frame has to show it
  if (item.id === 'stream') { router.navigate(ROUTES.CAT_STREAM); if (!isPhone()) emit('webui-navigate', webPathOf(ROUTES.WEB_CLIENTS)); return } // a phone has the stream hub: no page opens by itself
  router.navigate({ overview: ROUTES.HOME, settings: ROUTES.PANEL_SETTINGS, help: ROUTES.HELP, stream: ROUTES.CAT_STREAM, input: ROUTES.CAT_INPUT, audio: ROUTES.CAT_AUDIO, display: ROUTES.CAT_DISPLAY, system: ROUTES.CAT_SYSTEM, assistant: ROUTES.AI_ASSISTANT, developer: ROUTES.CAT_DEVELOPER }[item.id])
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
// The context menus of this window and of the Web pages in its frame are shown in the menu window (src-tauri/src/ctx_popup.rs), which may extend beyond this window; without it they stay in the page
let popupMenu = null
let stopTrayTheme = () => {}
const startPopupMenu = async () => {
  try {
    const [{ invoke }, { listen }] = await Promise.all([import('@tauri-apps/api/core'), import('@tauri-apps/api/event')])
    if (!window.__TAURI_INTERNALS__ || popupMenu) return
    popupMenu = installMenuPopup({ invoke, listen, backLabel: () => contextTexts(locale.value).backToMenu })
    stopTrayTheme = installTrayTheme({ invoke }) // the tray icon's menus use this window's colours
    for (const frame of document.querySelectorAll('iframe')) frame.contentWindow?.postMessage({ type: 'foundation-menu-host' }, '*')
  } catch { /* not in the app window: the menus stay in the page */ }
}
const onFrameMessage = (event) => {
  const data = event.data
  const frame = [...document.querySelectorAll('iframe')].find((f) => f.contentWindow === event.source)
  if (!frame) return
  if (data?.type === 'foundation-menu-open' || data?.type === 'foundation-menu-hello') { popupMenu?.frameMessage(event, frame); return }
  if (data?.type === 'foundation-subpage') frameSub.value = data.open === true
  else if (data?.type === 'foundation-search-open') uni.show()
  else if (data?.type === 'foundation-settings-recent' && typeof data.key === 'string') settingsList.remember(data.key)
  // what the gamepad page of the frame cannot do by itself: the controller test tool, the component page, and the state of the
  // DualSense transport that only the panel reads
  else if (data?.type === 'foundation-open' && data.page === 'controller-test') openGamepadTest()
  else if (data?.type === 'foundation-open' && data.page === 'components') router.navigate(ROUTES.COMPONENTS)
  else if (data?.type === 'foundation-open' && data.page === 'reset') router.navigate(ROUTES.RESET)
  else if (data?.type === 'foundation-open' && data.page === 'hdr-enhanced') router.navigate(ROUTES.HDR_ENHANCED)
  else if (data?.type === 'foundation-ds5-status-request') pushDs5Status()}
onMounted(() => { window.addEventListener('message', onFrameMessage); loadHostData(); componentStatus.start(60000); startPopupMenu() })
onBeforeUnmount(() => { window.removeEventListener('message', onFrameMessage); componentStatus.stop(); clearInterval(rtssRefresh); popupMenu?.stop(); popupMenu = null; stopTrayTheme() })
const contextMenu = ref(null)
// the window's own context menu (SPEC-context-menu): what this product can do for it; an action that is not here is not an item
// 运行体检 is 全部检查 of the components page; 打开 Web UI opens the host's address in the system browser (the dashboard card of the old panel did the same); the pet's entry is the 桌宠 sheet of 面板设置, the page
// the floating toolbar's menu opens (openPanelPage)
const ctxCallbacks = {
  search: (text) => { uni.query.value = text; uni.show() }, connect: () => { connect.open.value = true }, notice: (m) => useNotify().snack(m), openExternal: (u) => openExternalUrl(u), restartHost: () => hostConn.restart('service'),
  doctor: () => { componentStatus.openPage(); void componentStatus.checkAll() },
  openWebUi: async () => { const base = String((await sunshine.getUrl()) || ''); if (base) await openExternalUrl(base) },
  openDevtools: () => { invoke('dev_open_devtools').catch(() => useNotify().snack(devmodeTexts(locale.value).ctxDevtoolsFail)) },
  // 反馈此处的问题: the one feedback panel, with the page and what was under the pointer (the text shows in the panel and is not sent)
  reportHere: ({ place, text }) => { void openFeedbackHere({ place, text }) },
  get pet() { return [{ id: 'pet-settings', label: contextTexts(locale.value).petSettings, icon: 'pets', run: () => openPanelPage('pet') }] },
}
// the rows that have a tile on the overview that opens them: 固定到概览 pins that tile (the same functions as the pin of the search results)
const ROW_TILES = { [ROUTES.WEB_CLIENTS]: 'clients', [ROUTES.WEB_APPS]: 'apps', [ROUTES.WEB_DOWNLOADS]: 'downloads', [ROUTES.DIAGNOSTICS]: 'diagnostics' }
const rowPinSpec = (item) => (ROW_TILES[item.id] ? pinSpecForPage(ROW_TILES[item.id], { title: item.label, icon: item.icon }) : null)
const onRowContext = (event, item) => {
  const items = [{ id: 'open', label: t.value.m3e.ctx.open, icon: 'open_in_new' }]
  if (webPathOf(item.id)) items.push({ id: 'browser', label: t.value.m3e.ctx.browser, icon: 'public' })
  const spec = rowPinSpec(item)
  if (spec) items.push({ id: 'pin', label: isPinnedOnHome('panel-home', spec) ? pinTexts(locale.value).unpin : pinTexts(locale.value).pin, icon: 'keep' })
  contextMenu.value?.show(event, items, item)
}
const onContextSelect = (entry, item) => {
  if (entry.id === 'open') router.navigate(item.id)
  else if (entry.id === 'browser') actions_openUrl(item)
  else if (entry.id === 'pin') {
    const spec = rowPinSpec(item)
    if (isPinnedOnHome('panel-home', spec)) unpinFromHome('panel-home', spec)
    else if (pinToHome('panel-home', spec).ok) useNotify().snack(pinTexts(locale.value).pinned)
  }
}
const actions_openUrl = async (item) => {
  const base = String((await sunshine.getUrl()) || '').replace(/\/$/, '')
  if (base) toolsCtx.openUrl(base + webPathOf(item.id))
}
// Old call sites speak in short ids; each maps to a destination.
const openDestination = (id) => jumps.jump(() => {
  if (id === 'check-update') { handleCheckForUpdates(); return }
  if (id.startsWith('announcement:')) { router.navigate(ROUTES.ANNOUNCEMENTS, { id: id.slice(13) }); return }
  if (id.startsWith('setting:')) { const entry = searchItems.value.find((e) => e.keyName === id.slice(8)); if (entry) onSearch(entry); return }
  if (id === 'welcome') { openWelcome(); return }
  // the pairing sheet and the new application editor open at once: the Web pages read #pairing and #guide-add
  if (id === 'pair') { emit('webui-navigate', '/pin#pairing'); router.navigate(ROUTES.WEB_CLIENTS); return }
  if (id === 'pair:qr') { emit('webui-navigate', '/pin#pairing-qr'); router.navigate(ROUTES.WEB_CLIENTS); return }
  if (id === 'apps:add') { emit('webui-navigate', '/apps#guide-add'); router.navigate(ROUTES.WEB_APPS); return }
  if (id === 'apps:scan') { emit('webui-navigate', '/apps#guide-scan'); router.navigate(ROUTES.WEB_APPS); return }
  if (id.startsWith('downloads:')) { emit('webui-navigate', `/downloads?platform=${encodeURIComponent(id.slice(10))}`); router.navigate(ROUTES.WEB_DOWNLOADS); return }
  const web = { clients: ROUTES.WEB_CLIENTS, apps: ROUTES.WEB_APPS, settings: ROUTES.WEB_CONFIG, downloads: ROUTES.WEB_DOWNLOADS }
  if (id === 'settings') { openConfig({ tab: 'general' }); return }
  const route = web[id] || { gamepad: ROUTES.GAMEPAD, microphone: ROUTES.MICROPHONE, usb: ROUTES.USB, vdd: ROUTES.VDD_SETTINGS, quality: ROUTES.HDR_ENHANCED, components: ROUTES.COMPONENTS, diagnostics: ROUTES.DIAGNOSTICS, assistant: ROUTES.AI_ASSISTANT }[id]
  if (route) openRoute(route)
})
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
// the RTSS page of a machine without RivaTuner (a link, or a refresh that finds it gone) leaves for the first page of 系统
watch([rtssInstalled, () => router.currentRoute.value], ([installed, route]) => { if (installed === false && route === ROUTES.RTSS) router.navigate(ROUTES.SUNSHINE_SERVICE) })
// a page another window opens here (the toolbar menu: toolbar.rs `open-panel-page`, SunshineFrame passes it on)
const openPanelPage = (page) => {
  const route = requestPanelPage(page)
  if (!route) return
  if (PANEL_PAGE_PATHS[page]) emit('webui-navigate', PANEL_PAGE_PATHS[page])
  router.navigate(route)
}
defineExpose({
  openVddSettings, openWelcome, openWebStream, openAiAssistant, openControllers, openRtxHdr, openPanelPage, openStartGuide,
  goHome, checkForUpdates: handleCheckForUpdates, router, followFrame,
})
</script>

<style lang="less">
.panel-shell { position: relative; height: 100vh; height: 100dvh; min-height: 0; overflow: clip; }
.panel-caption { position: absolute; top: 0; left: var(--m-rail-w, 0px); right: 0; z-index: 40; transition: left var(--m-spring-default-spatial-duration, .4s) var(--m-spring-default-spatial, ease); display: grid; grid-template-columns: minmax(max-content, 1fr) minmax(0, 560px) minmax(max-content, 1fr); align-items: stretch; height: 48px; background: transparent; user-select: none; touch-action: none; }
.panel-caption__brand { align-self: center; padding-inline-start: 16px; pointer-events: none; }
.panel-more-version { display: flex; align-items: center; flex-wrap: wrap; gap: 4px 8px; padding: 4px 16px 12px; }
.panel-more-version .m-capver { margin-inline-start: 0; }
.panel-caption__controls { display: flex; justify-self: end; }
.panel-caption__search { display: flex; align-items: center; justify-content: center; min-width: 0; padding: 0 12px; }
/* the caption bar of a phone (its grid, the brand giving way, the sizes and the gap of the buttons) is foundation-ui's (css/layout.css, `.m-caption[data-phone]`): the Web UI has the same bar. The three window buttons below are the control panel's alone. */
.panel-caption__btn { display: grid; place-items: center; width: 46px; height: 48px; padding: 0; border: 0; background: transparent; color: var(--md-sys-color-on-surface); font-size: 18px; cursor: default; transition: background-color 120ms linear, color 120ms linear; }
.panel-caption__btn:hover { background: color-mix(in srgb, var(--md-sys-color-on-surface) 9%, transparent); }
.panel-caption__btn:active { background: color-mix(in srgb, var(--md-sys-color-on-surface) 14%, transparent); }
.panel-caption__btn--close:hover { background: #c42b1c; color: #fff; } .panel-caption__btn--close:active { background: #b02417; color: #fff; }
.panel-caption__btn:focus-visible { outline: 2px solid var(--md-sys-color-primary); outline-offset: -2px; }
/* touch: the three window buttons are a finger wide (48 px, see foundation-ui css/touch.css) */
@media (pointer: coarse) { .panel-caption__btn { width: 48px; } }
@media (any-pointer: coarse) { .panel-caption__btn[data-pressed] { background: color-mix(in srgb, var(--md-sys-color-on-surface) 14%, transparent); } .panel-caption__btn--close[data-pressed] { background: #b02417; color: #fff; } }
.panel-body { position: absolute; inset: 0; min-height: 0; overflow: clip; }
.panel-web-keep { height: 100%; min-height: 0; }
// a simulation of the host that is away shows the offline page, which has no header of its own: its chip and the way to end it sit in the corner of that page
.panel-sim-end { position: fixed; top: 60px; right: 20px; z-index: 5; }
.panel-body > .m-scene { position: absolute; inset: 0; height: auto; }
/* a framed page has no scrollbar (inject-styles.css); a page of the panel has none either, or a page that scrolls would sit its title 5 px left of the same title in the frame */
.panel-body .m-scene__pane--detail { scrollbar-width: none; }
.panel-body .m-scene__pane--detail::-webkit-scrollbar { display: none; }
</style>
