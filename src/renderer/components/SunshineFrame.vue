<template>
  <SidebarMenu
    ref="sidebarMenuRef"
    :webui-path="currentPath"
    @route-change="handleRouteChange"
    @webui-navigate="handleWebuiNavigate"
  >
    <div class="iframe-container">
      <!-- no loading page: a page switch shows the new shell at once and the page fills in (its own skeletons); only a proxy that does not answer says so -->
      <iframe
        :key="frameKey"
        ref="sunshineIframe"
        :src="sunshineUrl"
        class="sunshine-iframe"
        @load="onLoad"
        frameborder="0"
        allow="autoplay; clipboard-read; clipboard-write; fullscreen"
      ></iframe>
    </div>
  </SidebarMenu>
</template>

<script setup>
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { followRoot } from '../composables/frameRoot.js'
import { ElMessage } from '../composables/notifyMessage.js'
import { invoke } from '@tauri-apps/api/core'
import { deliverPendingPanelPage } from '../composables/panelParts.js'
import { deliverPendingStartGuide } from '../composables/startGuide.js'
import { sunshine } from '@/tauri-adapter.js'
import { checkLocalProxyHealth } from '../utils/proxyHealth.js'
import { isNativeControlPanelMessage, isTrustedNativeControlPanelMessage } from '../composables/nativeBridge.js'
import { useI18n } from '../desktop/i18n/index.js'
import SidebarMenu from './SidebarMenu.vue'
import { proxyBlock } from '../composables/proxyBlock.js'
import { createProxyGate } from '../composables/proxyGate.js'
import { isWebuiNavigationPath } from '../composables/webuiNavigation.js'
import { BLANK, framePageGo, routeChanged, webuiNavigated } from '../composables/frameRoute.js'
import { webPathOf, routeForWebPath } from '../composables/useRouter.js'
import { createReportLimiter, normalizeFrameReport } from '../composables/frameReport.js'
import { parseSettingsHash } from 'foundation-ui/settings/index.js'

invoke('main_panel_loading').catch(() => {})
const { t } = useI18n()

// Refs
const loading = ref(true)
const sunshineUrl = ref('')
// A frame whose address is set again navigates, and a navigation adds an entry to the window's joint history: the frame's step is then the first thing Back (the mouse button, Alt+Left) undoes,
// no popstate comes to the panel, and the person stays on the page. Every address gets a frame of its own: the first navigation of a new frame replaces its empty page and adds no entry.
const frameKey = ref(0)
// The Web in the frame says (once, after it mounted) that it can be told the page: from then on a request for another framed page is a message to this frame (handleWebuiNavigate), not a new address. A new frame has not said it yet.
let frameGoReady = false
watch(sunshineUrl, () => { frameKey.value++; frameGoReady = false }, { flush: 'sync' })
// A link inside a Web page moves the frame without `sunshineUrl` (the address the panel last gave it) changing. Asked later for the address it remembers (Back to the page the link was on), the same
// value would change nothing and the frame would stay on the page the link led to, with the route and the list already on the other one. `setFrameUrl` loads it again in that case.
let frameMovedAway = false
const setFrameUrl = (url) => {
  const again = sunshineUrl.value === url && frameMovedAway
  frameMovedAway = false
  if (again) { frameKey.value++; frameGoReady = false } else sunshineUrl.value = url
}
const notePath = (path) => {
  try { const asked = new URL(sunshineUrl.value, window.location.href); if (/^https?:$/.test(asked.protocol) && path.split('#')[0] !== asked.pathname + asked.search) frameMovedAway = true } catch { /* about:blank: nothing to compare */ }
}
const currentPath = ref('/')
const sunshineIframe = ref(null)
const sidebarMenuRef = ref(null)

// State
let pollTimer = null
let unlistenVddSettings = null
let unlistenDragDrop = null
let messageHandler = null
let visibilityHandlerRef = null
let proxyBase = '' // 代理服务器基础 URL，用于恢复 iframe 到正确页面
let loadedFrameRevealTimer = null
let awaitingAppReady = true
let localProxyVerified = false
let requestedWebuiPath = ''

// Constants
const ALLOWED_IMAGE_EXTENSIONS = ['.png', '.jpg', '.jpeg', '.gif', '.webp', '.bmp']
const POLL_INTERVAL = 3000
const LOADED_FRAME_REVEAL_DELAY = 300

const clearLoadedFrameReveal = () => {
  if (!loadedFrameRevealTimer) return
  clearTimeout(loadedFrameRevealTimer)
  loadedFrameRevealTimer = null
}

const beginLoading = () => {
  clearLoadedFrameReveal()
  awaitingAppReady = true
  loading.value = true
}

const finishLoading = () => {
  clearLoadedFrameReveal()
  awaitingAppReady = false
  loading.value = false
}

const scheduleLoadedFrameReveal = () => {
  clearLoadedFrameReveal()
  if (!awaitingAppReady) return

  loadedFrameRevealTimer = setTimeout(() => {
    console.debug('[SunshineFrame] revealing loaded iframe without WebUI ready signal')
    finishLoading()
  }, LOADED_FRAME_REVEAL_DELAY)
}

// Utility functions
const extractPathFromUrl = (url) => {
  try {
    const { pathname, search, hash } = new URL(url)
    return pathname + search + hash
  } catch {
    return '/'
  }
}

const isImageFile = (path) => {
  const ext = path.toLowerCase().slice(path.lastIndexOf('.'))
  return ALLOWED_IMAGE_EXTENSIONS.includes(ext)
}

const setAnimationsPaused = (paused) => {
  document.body?.classList.toggle('paused-animations', paused)
}

const isWelcomePath = (url) => {
  if (!url) return false
  try {
    const path = new URL(url).pathname.toLowerCase()
    return path === '/welcome' || path.startsWith('/welcome/') || path === '/welcome.html'
  } catch {
    return url.includes('/welcome') || url.includes('welcome.html')
  }
}

const openWelcome = () => sidebarMenuRef.value?.openWelcome?.()

const refreshProxyTarget = async () => {
  const refreshedTarget = await sunshine.refreshTarget()
  if (!refreshedTarget) {
    console.warn('[SunshineFrame] refresh proxy target failed')
  }
}

const verifyLocalProxy = async () => {
  try {
    const healthCheck = await invoke('get_proxy_health_check')
    return checkLocalProxyHealth(healthCheck)
  } catch (error) {
    console.warn('[SunshineFrame] local proxy health check failed:', error)
    return false
  }
}

// One try: the proxy listens (the first start takes a moment) and the WebView reaches it. The gate tries again with a backoff before the page says anything, and goes on in the
// background while it does (composables/proxyGate.js): a slow start never blanks the session.
const proxyGate = createProxyGate({
  verify: async () => {
    proxyBlock.checking = true
    try {
      try { proxyBase = await invoke('wait_for_proxy_ready') } catch (error) { console.warn('[SunshineFrame] the local proxy is not ready:', error); return false }
      return await verifyLocalProxy()
    } finally { proxyBlock.checking = false }
  },
  onAttempt: (n) => { proxyBlock.attempts = n },
  onShown: () => {
    proxyBlock.blocked = true
    sunshineUrl.value = 'about:blank'
    invoke('main_panel_ready').catch(() => {})
  },
  onCleared: () => { proxyBlock.blocked = false },
})
proxyBlock.retryNow = () => proxyGate.retryNow()

// Navigation handler
const handleNavigateFrame = (event) => {
  const url = event.detail?.url
  if (!url) return

  if (isWelcomePath(url)) {
    console.log('🔄 拦截 welcome 页面加载，打开 Vue welcome 组件')
    openWelcome()
    return
  }

  beginLoading()
  setFrameUrl(url)
}

// 路由切换：离开 HOME 时休眠 iframe，返回时恢复
let savedIframeUrl = ''
// 窗口隐藏/最小化时的 iframe URL 保存（与 tab 切换分开追踪）
let windowSuspendedUrl = ''

// A place inside the page that is already open (/config#tab=av&focus=hevc_mode): the page is told, nothing reloads
const settingsPlaceMessage = (path) => {
  const at = path.indexOf('#')
  if (at < 0 || path.slice(0, at) !== '/config' || !sunshineUrl.value || sunshineUrl.value === 'about:blank') return false
  if (currentPath.value.split(/[?#]/, 1)[0].replace(/\.html$/, '') !== '/config') return false
  const place = parseSettingsHash(path.slice(at))
  const frame = sunshineIframe.value?.contentWindow
  if (!place || !frame) return false
  // a frame that is not on an http(s) address has the origin 'null', and postMessage(message, 'null') throws: nothing is sent to it
  const origin = new URL(sunshineUrl.value, window.location.href).origin
  if (!/^https?:\/\//.test(origin)) return false
  frame.postMessage({ type: 'foundation-settings-go', ...place }, origin)
  currentPath.value = path
  return true
}
// the address of a page of the frame; none until the proxy is known and verified
const frameUrlOf = (path) => (proxyBase && localProxyVerified && path ? proxyBase + path : '')
const handleWebuiNavigate = (path) => {
  if (!isWebuiNavigationPath(path)) return
  if (settingsPlaceMessage(path)) return
  requestedWebuiPath = path
  windowSuspendedUrl = ''
  const next = webuiNavigated(path, { currentPath: currentPath.value, url: sunshineUrl.value, saved: savedIframeUrl }, { urlOf: frameUrlOf })
  // the frame is one document for the framed pages: it is told the page and swaps the view (nothing reloads, no cover; the Web keeps the chunks of the other pages ready)
  if (next.url && framePageGo(path, { goReady: frameGoReady, url: sunshineUrl.value, currentPath: currentPath.value })) {
    const origin = frameOrigin()
    const frame = sunshineIframe.value?.contentWindow
    if (origin && frame) {
      frame.postMessage({ type: 'foundation-frame-go', path }, origin)
      savedIframeUrl = next.saved
      currentPath.value = next.currentPath
      return
    }
  }
  savedIframeUrl = next.saved
  currentPath.value = next.currentPath
  if (next.url === null && sunshineUrl.value && sunshineUrl.value !== BLANK) return
  beginLoading()
  if (next.url) setFrameUrl(next.url)
}

// the overview and the pages of the panel itself: the frame sleeps behind them and wakes with the overview; a Web page is never shown over a sleeping frame (frameRoute.js)
const handleRouteChange = ({ from, to }) => {
  const decision = routeChanged({ from, to }, { currentPath: currentPath.value, url: sunshineUrl.value, saved: savedIframeUrl }, { urlOf: frameUrlOf, webPathOf, routeForWebPath })
  if (decision) {
    if (decision.kind !== 'resume') beginLoading()
    setFrameUrl(decision.url)
    savedIframeUrl = decision.saved
    if (decision.currentPath) currentPath.value = decision.currentPath
    console.log(`[SunshineFrame] route ${from} -> ${to}: iframe ${decision.kind}`)
  }
  if (decision?.kind === 'suspend' && pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  } else if (from !== 'home' && to === 'home' && !pollTimer && checkWindowStateFn) {
    pollTimer = setInterval(checkWindowStateFn, POLL_INTERVAL)
  }
}

// Background image handling
const loadAndSetBackground = async (imagePath) => {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const dataUrl = await invoke('read_image_as_data_url', { path: imagePath })

    sunshineIframe.value?.contentWindow?.postMessage(
      {
        type: 'set-background',
        dataUrl,
        filePath: imagePath,
      },
      '*'
    )
  } catch (error) {
    console.error('❌ 读取背景图片失败:', error)
  }
}

const handleTauriFileDrop = async (paths) => {
  const imagePath = paths?.find(isImageFile)
  if (imagePath) await loadAndSetBackground(imagePath)
}

// Message handling
const frameLimiter = createReportLimiter({ perMinute: 30 })
const createMessageHandler = () => {
  const getSunshineOrigin = () => {
    try {
      const url = new URL(sunshineUrl.value || sunshineIframe.value?.src || '', window.location.href)
      // only an http(s) address has an origin a message can be sent to (about:blank, a custom scheme: 'null')
      return /^https?:$/.test(url.protocol) ? url.origin : ''
    } catch {
      return ''
    }
  }

  const isTrustedSunshineEvent = (event) => (
    event.source === sunshineIframe.value?.contentWindow
    && event.origin === getSunshineOrigin()
  )

  const replyToSunshine = (event, message) => {
    const origin = getSunshineOrigin()
    if (!origin || !isTrustedSunshineEvent(event)) return
    event.source.postMessage(message, origin)
  }

  const handlers = {
    // a Web page in the frame asks for the panel's colours and scheme (composables/frameRoot.js)
    'foundation-root-request': (_data, event) => replyToSunshine(event, rootFollow ? rootFollow.snapshot() : { type: 'foundation-root', props: {}, classes: [], theme: '' }),
    'native-navigation-context-request': (_data, event) => {
      replyToSunshine(event, {
        type: 'native-navigation-context',
        source: 'sunshine-control-panel',
        unified: true,
      })
    },
    'path-update': (data) => {
      currentPath.value = data.path
      notePath(data.path)
      // the document that was loaded is the page that is shown: the panel's route is made to agree (SidebarMenu.followFrame)
      sidebarMenuRef.value?.followFrame?.(data.path)
    },
    'navigation-start': (data) => {
      if (data.path) {
        if (isWelcomePath(data.path) || data.path.toLowerCase().includes('welcome')) {
          console.log('🔄 拦截导航到 welcome 页面，打开 Vue welcome 组件')
          openWelcome()
          return
        }
        currentPath.value = data.path
        notePath(data.path)
        // a link, an address or a place of the page's hash moved the frame by itself: the rail, the list and the title follow (SidebarMenu.followFrame)
        sidebarMenuRef.value?.followFrame?.(data.path)
      }
      beginLoading()
    },
    'webui-ready': finishLoading,
    'foundation-frame-go-ready': (_data, event) => { if (isTrustedSunshineEvent(event)) frameGoReady = true },
    // errors, long frames and slow calls inside the page, written to the panel's log (frameReport.js); only the frame that is shown may say them
    'panel-frame-report': (data, event) => {
      if (!isTrustedSunshineEvent(event) || !frameLimiter.allow()) return
      const line = normalizeFrameReport(data)
      if (line) invoke('frontend_slow', line).catch(() => {})
    },
    'restore-background': (data) => loadAndSetBackground(data.path),
    'show-message': (data) => {
      // 处理来自 Web UI 的消息显示请求
      if (data.source === 'sunshine-webui' && data.message) {
        const messageType = data.messageType || 'info'
        switch (messageType) {
          case 'success':
            ElMessage.success(data.message)
            break
          case 'error':
            ElMessage.error(data.message)
            break
          case 'warning':
            ElMessage.warning(data.message)
            break
          default:
            ElMessage.info(data.message)
        }
      }
    },
    'native-updater-context-request': (_data, event) => {
      replyToSunshine(event, {
        type: 'native-updater-context',
        source: 'sunshine-control-panel',
        available: true,
      })
    },
    'native-update-request': async (data, event) => {
      const requestId = typeof data.requestId === 'string' ? data.requestId : ''
      if (data.channel !== 'stable' && data.channel !== 'prerelease') {
        replyToSunshine(event, {
          type: 'native-update-result',
          source: 'sunshine-control-panel',
          requestId,
          ok: false,
          error: 'Unsupported update channel',
        })
        return
      }

      const opened = await sidebarMenuRef.value?.checkForUpdates?.(data.channel)
      replyToSunshine(event, {
        type: 'native-update-result',
        source: 'sunshine-control-panel',
        requestId,
        ok: opened === true,
      })
    },
    'native-rtx-hdr-context-request': (_data, event) => {
      replyToSunshine(event, {
        type: 'native-rtx-hdr-context',
        source: 'sunshine-control-panel',
        available: true,
      })
    },
    'native-rtx-hdr-open-request': (data, event) => {
      const requestId = typeof data.requestId === 'string' ? data.requestId : ''
      sidebarMenuRef.value?.openRtxHdr?.()
      replyToSunshine(event, {
        type: 'native-rtx-hdr-open-result',
        source: 'sunshine-control-panel',
        requestId,
        ok: true,
      })
    },
  }

  return async (event) => {
    const { data } = event
    if (
      isNativeControlPanelMessage(data)
      && !isTrustedNativeControlPanelMessage(data, isTrustedSunshineEvent(event))
    ) {
      return
    }
    if (data?.type && handlers[data.type]) {
      if (['path-update', 'navigation-start', 'webui-ready'].includes(data.type) && !isTrustedSunshineEvent(event)) return
      await handlers[data.type](data, event)
    }
  }
}


// Window state monitoring
let checkWindowStateFn = null
let visibilityGeneration = 0

const setupWindowStateMonitor = async (currentWindow) => {
  let lastMinimized = false
  let lastHidden = false

  checkWindowStateFn = async () => {
    try {
      const [isMinimized, isVisible] = await Promise.all([currentWindow.isMinimized(), currentWindow.isVisible()])

      if (isMinimized !== lastMinimized || !isVisible !== lastHidden) {
        lastMinimized = isMinimized
        lastHidden = !isVisible
        setAnimationsPaused(isMinimized || !isVisible)
      }
    } catch (e) {
      console.warn('⚠️ 检测窗口状态失败:', e)
    }
  }

  pollTimer = setInterval(checkWindowStateFn, POLL_INTERVAL)
  await checkWindowStateFn()

  const visibilityHandler = async () => {
    const generation = ++visibilityGeneration
    const hidden = document.hidden
    setAnimationsPaused(hidden)

    if (hidden) {
      // 窗口被最小化/隐藏 → 休眠 iframe（仅当 iframe 有实际内容时）
      if (sunshineUrl.value && sunshineUrl.value !== 'about:blank') {
        // 保存当前实际页面路径（而非初始 URL），恢复时能回到用户所在的页面
        windowSuspendedUrl = proxyBase ? proxyBase + currentPath.value : sunshineUrl.value
        // Prepare the shell while the native window is still hidden so the
        // about:blank iframe is never exposed during the next activation.
        beginLoading()
        sunshineUrl.value = 'about:blank'
        console.log('[SunshineFrame] 窗口隐藏 → iframe 休眠, saved:', windowSuspendedUrl)
      }
      if (pollTimer) {
        clearInterval(pollTimer)
        pollTimer = null
      }
    } else {
      // 窗口恢复 → 唤醒 iframe（仅当是窗口隐藏导致的休眠时恢复）
      if (windowSuspendedUrl) {
        beginLoading()
        try {
          await refreshProxyTarget()
        } catch (error) {
          console.warn('[SunshineFrame] refresh proxy target on resume failed:', error)
        }

        // A second visibility transition may have happened while the proxy
        // refresh was in flight. Keep the iframe asleep if we are hidden again.
        if (generation !== visibilityGeneration || document.hidden) return

        const resumeUrl = windowSuspendedUrl
        windowSuspendedUrl = ''
        sunshineUrl.value = resumeUrl
        console.log('[SunshineFrame] 窗口恢复 → iframe 唤醒')
      }
      if (!pollTimer && checkWindowStateFn) {
        pollTimer = setInterval(checkWindowStateFn, POLL_INTERVAL)
      }
    }
  }
  document.addEventListener('visibilitychange', visibilityHandler)
  visibilityHandlerRef = visibilityHandler

  return visibilityHandler
}

// 语言切换时刷新 iframe 以应用新 locale
const handleLocaleChanged = () => {
  if (sunshineUrl.value && sunshineUrl.value !== 'about:blank') {
    beginLoading()
    // 记住当前页面路径，通过 about:blank 中转刷新（避免跨域限制）
    const targetUrl = proxyBase ? proxyBase + currentPath.value : sunshineUrl.value
    sunshineUrl.value = 'about:blank'
    setTimeout(() => { sunshineUrl.value = targetUrl }, 50)
  }
}

// Lifecycle
// the frame follows the panel's root element: a new wallpaper colour, light or dark, the picture on or off
let rootFollow = null
// the origin a message to the frame can be sent to: an http(s) address only ('' for about:blank and any custom scheme, whose origin is 'null': postMessage(message, 'null') throws a SyntaxError)
const frameOrigin = () => { try { const url = new URL(sunshineUrl.value || sunshineIframe.value?.src || '', window.location.href); return /^https?:$/.test(url.protocol) ? url.origin : '' } catch { return '' } }

onUnmounted(() => {
  rootFollow?.stop()
  window.removeEventListener('navigate-frame', handleNavigateFrame)
  window.removeEventListener('locale-changed', handleLocaleChanged)
  if (messageHandler) window.removeEventListener('message', messageHandler)
  if (visibilityHandlerRef) document.removeEventListener('visibilitychange', visibilityHandlerRef)
  if (pollTimer) clearInterval(pollTimer)
  clearLoadedFrameReveal()
  proxyGate.stop()
  proxyBlock.blocked = false
  unlistenVddSettings?.()
  unlistenDragDrop?.()
})

onMounted(async () => {
  rootFollow = followRoot(() => sunshineIframe.value, frameOrigin)
  window.addEventListener('navigate-frame', handleNavigateFrame)
  window.addEventListener('locale-changed', handleLocaleChanged)

  try {
    if (!await proxyGate.run()) return
    const proxyBaseUrl = proxyBase
    localProxyVerified = true
    await refreshProxyTarget()

    const cmdLineUrl = await sunshine.getCommandLineUrl()

    messageHandler = createMessageHandler()
    window.addEventListener('message', messageHandler)

    if (cmdLineUrl || requestedWebuiPath) {
      const targetPath = requestedWebuiPath || extractPathFromUrl(cmdLineUrl)
      const fullUrl = proxyBaseUrl + targetPath

      if (isWelcomePath(fullUrl)) {
        console.log('🔄 启动时检测到 welcome 页面，打开 Vue welcome 组件')
        openWelcome()
        sunshineUrl.value = 'about:blank'
        currentPath.value = '/'
        finishLoading()
        return
      }

      sunshineUrl.value = fullUrl
      currentPath.value = targetPath
    } else {
      sunshineUrl.value = proxyBaseUrl + '/'
      currentPath.value = '/'
    }

    const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow')
    const currentWindow = getCurrentWebviewWindow()

    unlistenDragDrop = await currentWindow.onDragDropEvent((event) => {
      if (event.payload.type === 'drop') {
        handleTauriFileDrop(event.payload.paths)
      }
    })

    await setupWindowStateMonitor(currentWindow)

    unlistenVddSettings = await currentWindow.listen('open-vdd-settings', () => {
      sidebarMenuRef.value?.openVddSettings?.()
    })

    // a page the toolbar or the tray asked for (panel_page.rs): taken on the ping, and once here after the listeners, so a window that
    // was only being created still opens it, exactly once
    const takePanelPage = () => deliverPendingPanelPage(invoke, (page) => sidebarMenuRef.value?.openPanelPage?.(page))
    await currentWindow.listen('panel-page-pending', takePanelPage)

    const takeStartGuide = () => deliverPendingStartGuide(invoke, () => sidebarMenuRef.value?.openStartGuide?.())
    await currentWindow.listen('start-guide-pending', takeStartGuide)

    await currentWindow.listen('open-web-stream', () => {
      sidebarMenuRef.value?.openWebStream?.()
    })

    await currentWindow.listen('open-welcome', openWelcome)
    await takePanelPage()
    await takeStartGuide()
    await invoke('main_panel_ready')
  } catch (error) {
    console.error('初始化失败:', error)
    if (!localProxyVerified) return
    sunshineUrl.value = proxyBase + '/'
  }
})

const onLoad = () => {
  try {
    const iframe = sunshineIframe.value
    const newUrl = iframe?.contentWindow?.location?.href

    // 休眠导航到 about:blank 时不处理
    if (!newUrl || newUrl === 'about:blank') return

    const path = extractPathFromUrl(newUrl)

    if (isWelcomePath(newUrl) || path.toLowerCase().includes('welcome')) {
      console.log('🔄 检测到 welcome 页面加载，拦截并打开 Vue welcome 组件')
      openWelcome()
      sunshineUrl.value = 'about:blank'
      currentPath.value = '/'
      finishLoading()
      return
    }

    currentPath.value = path

    // Proxy and error documents do not mount the WebUI application.
    if (!iframe.contentDocument?.getElementById('app')) {
      finishLoading()
      return
    }
  } catch {
    // 跨域时无法读取，保持当前路径
  }

  // Prefer the WebUI ready signal, but never hold a loaded iframe behind the
  // overlay for long when the signal is unavailable.
  scheduleLoadedFrameReveal()
}
</script>

<style scoped lang="less">
.iframe-container {
  width: 100%;
  height: 100%;
  position: relative;
  overflow: hidden;
}

.sunshine-iframe {
  width: 100%;
  height: 100%;
  border: none;
  transition: opacity 0.3s ease;
}

.fade-loading-enter-active,
.fade-loading-leave-active {
  transition: opacity 0.3s ease;
}

.fade-loading-enter-from,
.fade-loading-leave-to {
  opacity: 0;
}

</style>

<style>
.paused-animations *,
.paused-animations *::before,
.paused-animations *::after {
  animation: none !important;
  transition: none !important;
}
</style>
