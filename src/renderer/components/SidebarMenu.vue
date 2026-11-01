<template>
  <div class="m-app m-app--shell m-app--scroll panel-shell">
    <MNav
      :items="navItems"
      :footer="footerItems"
      :current="currentNav"
      :label="t.m3e.nav.main"
      brand-name="Foundation"
      brand-sub="Sunshine"
      @navigate="onNavigate"
    />
    <div class="m-app__main panel-shell__main">
      <div class="panel-drag" data-tauri-drag-region @pointerdown="onTouchWindowDragStart"></div>
      <div class="panel-window">
        <MIconButton icon="remove" :label="t.sidebar.minimize" @click="minimizeWindow" />
        <MIconButton :icon="isMaximized ? 'filter_none' : 'crop_square'" :label="isMaximized ? t.sidebar.restore : t.sidebar.maximize" @click="toggleMaximize" />
        <MIconButton icon="close" :label="t.sidebar.hideWindow" @click="closeWindow" />
      </div>
      <Transition name="m-route" mode="out-in">
        <div :key="routeKey" class="panel-route">
          <VddSettings v-if="router.isRoute(ROUTES.VDD_SETTINGS)" @close="back" />
          <Welcome v-else-if="router.isRoute(ROUTES.WELCOME)" @close="goHome" />
          <WebStreamSettings v-else-if="router.isRoute(ROUTES.WEB_STREAM)" @close="goHome" />
          <AiAssistant v-else-if="router.isRoute(ROUTES.AI_ASSISTANT)" @close="back" />
          <HdrEnhancedManager v-else-if="router.isRoute(ROUTES.HDR_ENHANCED)" />
          <CategoryPage v-else-if="category" :category="category" :runtime="runtime" @open="openDestination" />
          <GamepadPage v-else-if="router.isRoute(ROUTES.GAMEPAD)" @back="back" @open-controller-meta="openGamepadTest" @manage-components="router.navigate(ROUTES.COMPONENTS)" />
          <MicrophonePage v-else-if="router.isRoute(ROUTES.MICROPHONE)" @back="back" @manage-components="router.navigate(ROUTES.COMPONENTS)" />
          <UsbPage v-else-if="router.isRoute(ROUTES.USB)" @back="back" @manage-components="router.navigate(ROUTES.COMPONENTS)" />
          <ComponentsPage v-else-if="router.isRoute(ROUTES.COMPONENTS)" @back="back" />
          <DiagnosticsPage
            v-else-if="router.isRoute(ROUTES.DIAGNOSTICS)"
            :tools="toolsMenuItems"
            @back="back"
            @open-controller-meta="openGamepadTest"
            @open-stylus-input-probe="openStylusInputProbe"
          />
          <ControlPanelPage
            v-else-if="[ROUTES.PANEL_SETTINGS, ROUTES.HELP].some(router.isRoute)"
            :page="router.currentRoute.value"
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
          <slot v-else-if="router.isRoute(ROUTES.HOME)" />
        </div>
      </Transition>
      <UpdateDialog
        v-if="showUpdateDialog"
        v-model="showUpdateDialog"
        :update-info="updateInfo"
        :current-version="currentVersion"
        @close="showUpdateDialog = false"
        @skip-version="skipVersion"
      />
    </div>
  </div>
</template>

<script setup>
import { computed, defineAsyncComponent, toRef, watch, onMounted } from 'vue'
import { MNav, MIconButton } from 'foundation-ui/vue'
import VddSettings from './VddSettings.vue'
import Welcome from './welcome.vue'
import WebStreamSettings from './WebStreamSettings.vue'
import AiAssistant from './AiAssistant.vue'
import HdrEnhancedManager from './HdrEnhancedManager.vue'
import ControlPanelPage from './ControlPanelPage.vue'
import CategoryPage from './m3e/CategoryPage.vue'
import GamepadPage from './m3e/GamepadPage.vue'
import MicrophonePage from './m3e/MicrophonePage.vue'
import UsbPage from './m3e/UsbPage.vue'
import ComponentsPage from './m3e/ComponentsPage.vue'
import DiagnosticsPage from './m3e/DiagnosticsPage.vue'
import { useDeviceRuntime } from '../composables/useDeviceRuntime.js'
import { useSidebarState } from '../composables/useSidebarState.js'
import { useWindowControls } from '../composables/useWindowControls.js'
import { useTools } from '../composables/useTools.js'
import { useTouchWindowDrag } from '../composables/useTouchWindowDrag.js'
import { createUtilityTools } from '../composables/toolsRegistry.js'
import { ROUTES } from '../composables/useRouter.js'
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
  emit('webui-navigate', path)
  router.goHome()
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

const CATEGORY_OF_ROUTE = {
  [ROUTES.CAT_STREAM]: 'stream', [ROUTES.CAT_INPUT]: 'input', [ROUTES.CAT_DISPLAY]: 'display', [ROUTES.CAT_SYSTEM]: 'system',
}
const category = computed(() => CATEGORY_OF_ROUTE[router.currentRoute.value] || '')
const PARENT = {
  [ROUTES.GAMEPAD]: ROUTES.CAT_INPUT, [ROUTES.MICROPHONE]: ROUTES.CAT_INPUT, [ROUTES.USB]: ROUTES.CAT_INPUT,
  [ROUTES.VDD_SETTINGS]: ROUTES.CAT_DISPLAY, [ROUTES.HDR_ENHANCED]: ROUTES.CAT_DISPLAY,
  [ROUTES.COMPONENTS]: ROUTES.CAT_SYSTEM, [ROUTES.DIAGNOSTICS]: ROUTES.CAT_SYSTEM, [ROUTES.AI_ASSISTANT]: ROUTES.CAT_SYSTEM,
}
const back = () => router.navigate(PARENT[router.currentRoute.value] || ROUTES.HOME)

const STREAM_PATHS = /^\/(pin|apps|config|downloads|dualsense)(?:\.html)?(?:[?#/]|$)/
const currentNav = computed(() => {
  const route = router.currentRoute.value
  if (route === ROUTES.HOME) {
    const path = props.webuiPath || '/'
    if (/^\/password(?:\.html)?(?:[?#]|$)/.test(path)) return 'settings'
    if (/^\/troubleshooting(?:\.html)?(?:[?#]|$)/.test(path)) return 'help'
    return STREAM_PATHS.test(path) ? 'stream' : 'overview'
  }
  if (route === ROUTES.PANEL_SETTINGS) return 'settings'
  if (route === ROUTES.HELP) return 'help'
  if (route === ROUTES.WELCOME) return 'overview'
  const top = category.value || { [ROUTES.GAMEPAD]: 'input', [ROUTES.MICROPHONE]: 'input', [ROUTES.USB]: 'input', [ROUTES.VDD_SETTINGS]: 'display', [ROUTES.HDR_ENHANCED]: 'display' }[route]
  return top || 'system'
})
const routeKey = computed(() => (router.isRoute(ROUTES.HOME) ? `home` : router.currentRoute.value))

const dot = (ok) => (ok ? 'ok' : 'none')
const navItems = computed(() => [
  { id: 'overview', label: t.value.m3e.nav.overview, icon: 'home' },
  { id: 'stream', label: t.value.m3e.nav.stream, icon: 'cast_connected' },
  { id: 'input', label: t.value.m3e.nav.input, icon: 'sports_esports', dot: dot(['ready', 'active'].includes(runtime.ds.state)) },
  { id: 'display', label: t.value.m3e.nav.display, icon: 'desktop_windows' },
  { id: 'system', label: t.value.m3e.nav.system, icon: 'build' },
])
const footerItems = computed(() => [
  { id: 'settings', label: t.value.m3e.nav.settings, icon: 'settings' },
  { id: 'help', label: t.value.m3e.nav.help, icon: 'help' },
])
const CATEGORY_ROUTE = { stream: ROUTES.CAT_STREAM, input: ROUTES.CAT_INPUT, display: ROUTES.CAT_DISPLAY, system: ROUTES.CAT_SYSTEM }
const onNavigate = (item) => {
  if (item.id === 'overview') return openWebUi('/')
  if (item.id === 'settings') return router.navigate(ROUTES.PANEL_SETTINGS)
  if (item.id === 'help') return router.navigate(ROUTES.HELP)
  router.navigate(CATEGORY_ROUTE[item.id])
}
const openDestination = (id) => {
  const web = { clients: '/pin', apps: '/apps', settings: '/config', downloads: '/downloads' }
  if (web[id]) return openWebUi(web[id])
  const route = { gamepad: ROUTES.GAMEPAD, microphone: ROUTES.MICROPHONE, usb: ROUTES.USB, vdd: ROUTES.VDD_SETTINGS, quality: ROUTES.HDR_ENHANCED, components: ROUTES.COMPONENTS, diagnostics: ROUTES.DIAGNOSTICS, assistant: ROUTES.AI_ASSISTANT }[id]
  if (route) router.navigate(route)
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
.panel-shell__main { position: relative; }
.panel-route { min-height: 100%; }
.panel-drag { position: fixed; top: 0; left: var(--m-rail-expanded-width); right: 0; height: 40px; z-index: 6; }
@media (max-width: 1099.98px) { .panel-drag { left: var(--m-rail-width); } }
.panel-window { position: fixed; top: 4px; right: 8px; z-index: 30; display: flex; gap: 0; }
.panel-window .m-icon-btn { width: 40px; height: 32px; border-radius: 9999px; font-size: 18px; }
.m-route-enter-active { transition: opacity var(--m-spring-default-effects-duration) var(--m-spring-default-effects); }
.m-route-leave-active { transition: opacity 80ms linear; }
.m-route-enter-from, .m-route-leave-to { opacity: 0; }
</style>
