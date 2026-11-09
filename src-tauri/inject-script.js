// Sunshine Web UI 注入脚本
// 用于主题同步、导航检测和拖放功能

;(function () {
  window.isTauri = true
  window.electron ??= {}

  const parentOrigins = new Set([
    'http://tauri.localhost',
    'https://tauri.localhost',
    'tauri://localhost',
    'http://localhost:8080',
    'https://localhost:8080',
    'http://127.0.0.1:8080',
    'https://127.0.0.1:8080',
  ])
  const parentOrigin = [document.referrer, location.ancestorOrigins?.[0]]
    .map((value) => {
      try {
        const url = new URL(value)
        return url.origin === 'null' ? `${url.protocol}//${url.host}` : url.origin
      } catch {
        return ''
      }
    })
    .find((origin) => parentOrigins.has(origin))
  const adaptedTabs = new Map()

  // Keep these integration details in the native proxy, not in browser-served WebUI source.
  const adaptEmbeddedControls = () => {
    if (document.documentElement.dataset.sunshineNavigation !== 'unified') return
    document.querySelectorAll('.config-tabs .nav-link').forEach((tab) => {
      if (adaptedTabs.has(tab)) return
      adaptedTabs.set(tab, tab.scrollIntoView)
      tab.scrollIntoView = ({ behavior = 'auto' } = {}) => {
        const list = tab.closest('.config-tabs')
        if (!list || list.scrollWidth <= list.clientWidth) return
        const tabRect = tab.getBoundingClientRect()
        const listRect = list.getBoundingClientRect()
        const left = list.scrollLeft + tabRect.left - listRect.left - (list.clientWidth - tabRect.width) / 2
        list.scrollTo({ left: Math.max(0, left), behavior })
      }
    })
  }

  const handleNavigationContext = (event) => {
    if (
      window.parent === window ||
      !parentOrigin ||
      event.source !== window.parent ||
      event.origin !== parentOrigin ||
      event.data?.source !== 'sunshine-control-panel'
    ) {
      return
    }
    if (event.data.unified === true) {
      document.documentElement.dataset.sunshineNavigation = 'unified'
      adaptEmbeddedControls()
    } else {
      delete document.documentElement.dataset.sunshineNavigation
      adaptedTabs.forEach((original, tab) => {
        tab.scrollIntoView = original
      })
      adaptedTabs.clear()
    }
  }

  // 消息处理器映射
  const messageHandlers = {
    'theme-sync': ({ theme }) => {
      document.documentElement.setAttribute('data-bs-theme', theme)
      localStorage.setItem('theme', theme)
    },
    'set-background': ({ dataUrl, filePath }) => {
      document.body.style.backgroundImage = `url("${dataUrl}")`
      filePath && localStorage.setItem('WEBUI-BGSRC-PATH', filePath)
    },
  }

  // 消息监听器
  window.addEventListener('message', (event) => {
    const { data } = event
    if (data?.type === 'native-navigation-context') {
      handleNavigationContext(event)
      return
    }
    data?.type && messageHandlers[data.type]?.(data)
  })

  // 发送消息到父窗口
  const postToParent = (type, payload = {}) => {
    window.parent.postMessage({ type, ...payload }, '*')
  }

  // Tauri API 辅助函数
  const invoke = (cmd, args) => window.__TAURI__?.core?.invoke?.(cmd, args)
  const openDialog = (options) => window.__TAURI__?.dialog?.open?.(options)

  // Electron 兼容 API（使用 Tauri API）
  window.electron.webUtils = {
    async getPathForFile(file) {
      if (!file) return ''
      if (file.path) return file.path

      try {
        const selected = await openDialog({
          title: '选择文件',
          filters: [{ name: '所有文件', extensions: ['*'] }],
          multiple: false,
          directory: false,
        })
        return selected || file.name
      } catch {
        return file.name
      }
    },
  }

  // ICC 文件列表 API
  const getIccFileList = async (callback) => {
    try {
      const result = await invoke('get_icc_file_list')
      if (result) {
        callback?.(result)
        return result
      }
    } catch {
      // 忽略错误
    }
    callback?.([])
    return []
  }
  window.getIccFileList = window.electron.getIccFileList = getIccFileList

  // 读取目录 API
  const emptyDir = { files: [], dirs: [] }
  window.readDirectory = async (path, callback) => {
    try {
      const result = await invoke('read_directory', { path })
      if (result) {
        callback?.(result)
        return result
      }
    } catch {
      // 忽略错误
    }
    callback?.(emptyDir)
    return emptyDir
  }

  // 虚拟鼠标驱动管理 API
  window.vmouseDriver = {
    async getStatus() {
      try {
        return await invoke('get_vmouse_status')
      } catch (e) {
        return { installed: false, running: false, status_text: '检测失败', driver_path: '', config_enabled: false }
      }
    },
    async install() {
      return await invoke('install_vmouse_driver')
    },
    async uninstall() {
      return await invoke('uninstall_vmouse_driver')
    },
  }

  // ViGEmBus 虚拟手柄驱动管理 API
  window.vigemDriver = {
    async getStatus() {
      try {
        return await invoke('get_vigem_status')
      } catch (e) {
        return {
          installed: false,
          running: false,
          version: '',
          version_ok: false,
          status_text: '检测失败',
          driver_path: '',
        }
      }
    },
    async install(force = false) {
      return await invoke('install_vigem_driver', { force: !!force })
    },
    async uninstall() {
      return await invoke('uninstall_vigem_driver')
    },
  }

  // ZakoVDD virtual-display driver management API
  window.vddDriver = {
    async getStatus() {
      try {
        return await invoke('get_vdd_status')
      } catch (e) {
        return {
          state: 'unknown',
          installed: false,
          running: false,
          control_available: false,
          installed_version: '',
          bundled_version: '',
          version_match: false,
          monitor_active: false,
          status_text: String(e || 'VDD status check failed'),
        }
      }
    },
    async install() {
      return await invoke('install_vdd_driver')
    },
    async uninstall() {
      return await invoke('uninstall_vdd_driver')
    },
  }

  // The right-click menu, the browser keys and the drag ghosts are closed by inject-traces.js, which the proxy puts in front of this script (it keeps the paste menu of a text field).

  // 导航检测
  const initNavigation = () => {
    const getFullPath = () => location.pathname + location.search + location.hash
    let lastPath = location.pathname + location.search
    let lastFull = getFullPath()

    postToParent('path-update', { path: getFullPath() })

    window.navigation?.addEventListener('navigate', (e) => {
      if (!e.canIntercept || e.downloadRequest) return

      const { pathname, search, hash } = new URL(e.destination.url)
      const newPath = pathname + search

      // a place set by the page's own hash (/config#tab=audio&sub=sound) is a move of the page: the panel's list follows it (SidebarMenu.followFrame). It was ignored here, and the list stayed on the old row.
      if (e.hashChange) {
        const full = newPath + hash
        if (full !== lastFull) { lastFull = full; postToParent('navigation-start', { path: full }) }
        return
      }
      if (newPath !== lastPath) {
        postToParent('navigation-start', { path: newPath + hash })
        lastPath = newPath
        lastFull = newPath + hash
      }
    })
  }

  // What happens inside the page is reported to the panel, which writes it to its log (`[renderer] frame-<kind> <page> <ms> ms ...`): a page that is blank or slow in the panel's
  // frame leaves an error with its place, a long frame with its script, a slow call with its path. At most 20 reports a minute; the long poll of the pairing list is not slow.
  const initReports = () => {
    if (window.parent === window) return
    let windowStart = 0
    let sent = 0
    const report = (kind, name, ms, detail) => {
      const now = Date.now()
      if (!windowStart || now - windowStart >= 60000) { windowStart = now; sent = 0 }
      if (++sent > 20) return
      postToParent('panel-frame-report', { kind, name: String(name || '').slice(0, 160), ms: Math.round(ms || 0), detail: String(detail || '').slice(0, 300), page: location.pathname })
    }
    const frames = (stack) => String(stack || '').split('\n').slice(1, 4).join(' | ')
    window.addEventListener('error', (e) => {
      if (/ResizeObserver loop/.test(e.message || '')) return
      report('error', e.message || 'error', 0, `${String(e.filename || '').split('/').pop()}:${e.lineno || 0}:${e.colno || 0} ${frames(e.error && e.error.stack)}`)
    })
    window.addEventListener('unhandledrejection', (e) => {
      const r = e.reason
      report('error', `unhandledrejection: ${(r && r.message) || r}`, 0, frames(r && r.stack))
    })
    try {
      new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) {
          if (entry.duration < 300) continue
          const scripts = (entry.scripts || []).filter((x) => x.duration >= 20).map((x) => `${x.invokerType}:${String(x.invoker || '').slice(0, 40)}:${x.duration | 0}ms@${String(x.sourceURL || '').split('/').pop()}:${x.sourceCharPosition}`).slice(0, 3).join(' ; ')
          report('frame', entry.entryType === 'longtask' ? 'long-task' : 'long-animation-frame', entry.duration, `blocking ${entry.blockingDuration | 0} ms ${scripts}`)
        }
      }).observe({ type: 'long-animation-frame', buffered: false })
    } catch {
      try {
        new PerformanceObserver((list) => { for (const entry of list.getEntries()) if (entry.duration >= 300) report('frame', 'long-task', entry.duration, '') }).observe({ type: 'longtask' })
      } catch { /* an engine without either entry type */ }
    }
    const nativeFetch = window.fetch
    if (typeof nativeFetch === 'function') {
      window.fetch = function (...args) {
        const started = performance.now()
        const target = typeof args[0] === 'string' ? args[0] : args[0] && args[0].url
        const result = nativeFetch.apply(this, args)
        const done = () => {
          const ms = performance.now() - started
          if (ms < 1500 || /\/api\/pairing\/pending/.test(String(target)) || /[?&]wait=\d+/.test(String(target))) return
          let path = String(target)
          try { path = new URL(path, location.href).pathname } catch { /* keep the text */ }
          report('fetch', path, ms, '')
        }
        if (result && typeof result.then === 'function') result.then(done, done)
        return result
      }
    }
  }

  // The Web page writes the colour scheme and a background colour on its root in the first lines of its <head> (the system's light or dark, to avoid a flash) and never again; the choice of
  // the panel arrives later, as data-bs-theme. A frame whose colour scheme differs from the panel's is painted on an opaque dark canvas: a black pane in the light theme, after the theme was
  // changed with a page open, or at the start when the system is dark and the panel is light. In the frame the root follows data-bs-theme and paints no background (the panel paints behind it).
  const followPanelScheme = () => {
    if (window.parent === window) return
    const root = document.documentElement
    const apply = () => {
      const theme = root.getAttribute('data-bs-theme')
      if (theme !== 'light' && theme !== 'dark') return
      if (root.style.colorScheme !== theme) root.style.colorScheme = theme
      if (root.style.backgroundColor) root.style.backgroundColor = ''
    }
    apply()
    new MutationObserver(apply).observe(root, { attributes: true, attributeFilter: ['data-bs-theme', 'style'] })
  }

  // 初始化
  const init = () => {
    initReports()
    followPanelScheme()
    initNavigation()
    postToParent('request-theme')
    // the panel keeps the frame covered until the page has painted: its load event comes 50 to 100 ms before the first frame (measured, Skyindows panel)
    try {
      new PerformanceObserver((list, observer) => {
        if (list.getEntries().some((entry) => entry.name === 'first-contentful-paint')) {
          observer.disconnect()
          postToParent('panel-painted', { fcp: Math.round(performance.now()) })
        }
      }).observe({ type: 'paint', buffered: true })
    } catch {
      postToParent('panel-painted')
    }
    if (window.parent !== window && parentOrigin) {
      window.parent.postMessage(
        { type: 'native-navigation-context-request', source: 'sunshine-webui' },
        parentOrigin
      )
      const observer = new MutationObserver(adaptEmbeddedControls)
      observer.observe(document.body, { childList: true, subtree: true })
      window.addEventListener('hashchange', adaptEmbeddedControls)
    }
  }

  document.readyState === 'loading' ? document.addEventListener('DOMContentLoaded', init) : init()
})()
