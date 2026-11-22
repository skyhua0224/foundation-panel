import { ref } from 'vue'
import { ElMessage, ElNotification, ElMessageBox, UPDATE_CHECK_MESSAGE } from './notifyMessage.js'
import { openExternalUrl, tools, controllerMeta, sunshine } from '@/tauri-adapter.js'
import { latencyClockTexts } from 'foundation-ui/vue'
import { pickStreamMonitor, centredOn } from './streamMonitor.js'
import { useI18n } from '../desktop/i18n/index.js'
import { formatMessage } from '../shared/format-message.js'
import { isCurrentVersionUnknown, isPrereleaseUnavailable, updateCheckFailureText } from './updateCheckError.js'
import { clipboardSyncState, clipboardSyncWorks } from './clipboardStatusView.js'
import { setVisibleInterval, clearVisibleInterval } from './pageVisibility.js'

// Module-scoped reactive flag so all sidebar instances share state.
const clipboardSyncEnabled = ref(false)
const clipboardSyncStateWord = ref('unknown')
let clipboardSyncInitialised = false
let clipboardSyncPollTimer = null
// The gamepad test tool downloads or writes files on disk: while that runs nothing else may interrupt it (a restart would corrupt the write). The flag covers
// the whole action, from its first synchronous statement — before the first await, so a second click starts no second flow — to its end on every path; the text
// drives the panel's full-window blocking layer (PanelBusyLayer), which shows the live progress of the work.
const controllerBusy = ref(false)
const controllerBusyText = ref('')
const startBusy = (text) => {
  controllerBusy.value = true
  controllerBusyText.value = text
  return {
    setText: (words) => { controllerBusyText.value = words },
    close: () => { controllerBusy.value = false; controllerBusyText.value = '' },
  }
}

/**
 * 工具操作 Composable
 */
export function useTools() {
  const { t, locale } = useI18n()

  /**
   * 延迟测试时钟: full screen in a window of its own on the display the stream captures (Sunshine's output_name, streamMonitor.js).
   * The window is made centred on that monitor and then set full screen, so it fills that monitor; with no match it opens on the primary one and says so.
   */
  const openLatencyClockOnStream = async ({ mode = 'dark' } = {}) => {
    const lang = String(locale.value || 'zh').slice(0, 2)
    const T = latencyClockTexts(lang)
    let pick = { monitor: null, found: false }
    try {
      const { availableMonitors, primaryMonitor } = await import('@tauri-apps/api/window')
      const [monitors, primary] = await Promise.all([availableMonitors(), primaryMonitor()])
      let config = {}
      try { const base = await sunshine.getProxyUrl(); const r = await fetch(`${base}/api/config`, { signal: AbortSignal.timeout(3000) }); config = await r.json() } catch { /* the host is not answering: the primary display */ }
      pick = pickStreamMonitor({ outputName: config.output_name, displayDevices: config.display_devices, monitors, primary })
    } catch { /* not in Tauri */ }
    try {
      const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow')
      const rect = pick.monitor ? centredOn(pick.monitor, 960, 600) : { width: 960, height: 600 }
      const win = new WebviewWindow(`latency_${Date.now()}`, {
        url: `${window.location.origin}/latency-clock/index.html?mode=${encodeURIComponent(mode)}&locale=${lang}`,
        title: T.title, ...rect, ...(pick.monitor ? {} : { center: true }), decorations: false, focus: true,
      })
      win.once('tauri://created', async () => { try { await win.setFullscreen(true); await win.setFocus() } catch (e) { console.error('[LatencyClock] full screen failed:', e) } })
      win.once('tauri://error', (e) => { console.error('[LatencyClock] window failed:', e); ElMessage.error(formatMessage(t.value.toolActions.windowCreateFailed, { title: T.title })) })
      if (!pick.found) ElMessage.warning(T.streamFailed)
    } catch (e) {
      console.error('[LatencyClock] window failed:', e)
    }
  }

  /**
   * 按需加载并打开随 GUI 安装的手写笔输入检测插件。
   * 后端只返回稳定错误码，避免在界面中暴露本地安装路径。
   */
  const openStylusInputProbe = async () => {
    try {
      await tools.openNativeTool('alkaidlab.stylus')
      ElMessage.success(t.value.tools.stylusInputProbeStarted)
    } catch (error) {
      const errorCode = String(error || '')
      const missing = errorCode.includes('NATIVE_PLUGIN_NOT_FOUND')
      const cleanupPending = errorCode.includes('NATIVE_PLUGIN_CLEANUP_PENDING')
      const incompatible = [
        'NATIVE_PLUGIN_PATH_INVALID',
        'NATIVE_PLUGIN_LOAD_FAILED',
        'NATIVE_PLUGIN_ENTRY_MISSING',
        'NATIVE_PLUGIN_ABI_MISMATCH',
        'NATIVE_PLUGIN_ID_MISMATCH',
        'NATIVE_PLUGIN_INIT_FAILED',
      ].some((code) => errorCode.includes(code))
      console.error('[StylusInputProbe] launch failed:', errorCode)
      try {
        await ElMessageBox.alert(
          missing
            ? t.value.tools.stylusInputProbeMissing
            : cleanupPending
              ? t.value.tools.stylusInputProbeCleanupPending
              : incompatible
                ? t.value.tools.stylusInputProbeUntrusted
                : t.value.tools.stylusInputProbeLaunchFailed,
          t.value.tools.stylusInputProbeErrorTitle,
          {
            confirmButtonText: t.value.systemTools.confirm,
            type: 'error',
            showClose: false,
            closeOnClickModal: false,
            closeOnPressEscape: false,
          },
        )
      } catch {
        // The launch failure has already been handled and must not escape the UI action.
      }
    }
  }

  /**
   * 打开外部 URL
   * @param {string} url - 要打开的URL
   */
  const openUrl = async (url) => {
    await openExternalUrl(url)
  }

  /**
   * 清理无用的封面图片和临时文件
   */
  const cleanupCovers = async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core')

      // 首先检查是否以管理员权限运行
      const isRunningAsAdmin = await invoke('is_running_as_admin')

      if (!isRunningAsAdmin) {
        // 不是管理员，提示重启
        await ElMessageBox.confirm(t.value.toolActions.cleanupAdminRequired, t.value.toolActions.adminRequiredTitle, {
          confirmButtonText: t.value.toolActions.restartAsAdmin,
          cancelButtonText: t.value.common.cancel,
          type: 'warning',
        })

        // 用户确认后，调用重启为管理员
        await restartAsAdmin()
        return
      }

      // 已经是管理员，继续执行清理
      await ElMessageBox.confirm(
        t.value.toolActions.cleanupConfirm,
        t.value.toolActions.cleanupTitle,
        {
          confirmButtonText: t.value.systemTools.cleanup,
          cancelButtonText: t.value.common.cancel,
          type: 'warning',
          danger: true,
        },
      )

      // 显示加载提示
      const loading = ElMessage({
        message: t.value.toolActions.cleaning,
        type: 'info',
        duration: 0,
      })

      try {
        // 调用 Tauri 命令
        const result = await invoke('cleanup_unused_covers')

        // 显示结果
        if (result.success) {
          if (result.deleted_count > 0) {
            ElMessageBox.alert(
              formatMessage(t.value.toolActions.cleanupSummary, {
                count: result.deleted_count,
                size: (result.freed_space / 1024).toFixed(2),
              }),
              t.value.toolActions.cleanupComplete,
              {
                confirmButtonText: t.value.common.confirm,
                type: 'success',
              },
            )
          } else {
            ElMessage.success(t.value.toolActions.cleanupEmpty)
          }
        } else {
          console.error('Cleanup failed:', result.message)
          ElMessage.error(t.value.toolActions.cleanupFailed)
        }
      } finally {
        // the message with duration 0 ends on every path: a failed clean leaves the queue free for the notices that follow
        loading.close()
      }
    } catch (error) {
      if (error !== 'cancel') {
        console.error('清理文件失败:', error)
        ElMessage.error(t.value.toolActions.cleanupFailed)
      }
    }
  }

  /**
   * 以管理员权限重启 GUI
   */
  const restartAsAdmin = async () => {
    try {
      // 确认对话框
      await ElMessageBox.confirm(t.value.toolActions.elevateConfirm, t.value.toolActions.elevateTitle, {
        confirmButtonText: t.value.common.confirm,
        cancelButtonText: t.value.common.cancel,
        type: 'warning',
      })

      // 显示提示
      ElMessage.info({ message: t.value.toolActions.requestingAdmin, id: 'admin-restart' })

      // 调用 Tauri 命令
      const { invoke } = await import('@tauri-apps/api/core')
      await invoke('restart_as_admin')

      // 如果到这里说明成功请求了重启
      ElMessage.success({ message: t.value.toolActions.restartingAsAdmin, id: 'admin-restart' })
    } catch (error) {
      if (error !== 'cancel') {
        console.error('重启失败:', error)
        ElMessage.error({ message: t.value.toolActions.restartFailed, id: 'admin-restart' })
      }
    }
  }

  /**
   * 检查更新，返回 UpdateInfo（包含 is_latest 标记）由调用方处理展示
   */
  const checkForUpdates = async (channel = null) => {
    let progress = null
    let failed = false
    try {
      const { invoke } = await import('@tauri-apps/api/core')

      // one message for the whole check: the sentence stays until the answer comes (a slow check outlasts the usual 5 s), a failure replaces it where it stands
      // (same id), an answer closes it because the dialog of the answer takes over
      progress = ElMessage({ message: t.value.toolActions.checkingUpdates, id: UPDATE_CHECK_MESSAGE, duration: 0 })

      const hasExplicitChannel = channel === 'stable' || channel === 'prerelease'
      const result = hasExplicitChannel
        ? await invoke('check_for_updates_for_channel', { channel })
        : await invoke('check_for_updates')

      if (result) {
        return result // 返回更新信息（包含 is_latest 标记），让调用者处理
      }
      return null
    } catch (error) {
      console.error('检查更新失败:', error)
      failed = true
      const text = updateCheckFailureText(error, t.value.toolActions)
      if (isPrereleaseUnavailable(error) || isCurrentVersionUnknown(error)) ElMessage.warning({ message: text, id: UPDATE_CHECK_MESSAGE })
      else ElMessage.error({ message: text, id: UPDATE_CHECK_MESSAGE })
      return null
    } finally {
      if (!failed) await progress?.close()
    }
  }

  /**
   * 打开手柄测试工具（ControllerMeta）
   *
   * 流程：
   * 1. 已安装 → 直接启动
   * 2. 未安装 → 询问用户：下载并启动 / 打开网页版 / 取消
   * 3. 下载过程中全窗口阻塞层显示进度（PanelBusyLayer）
   */
  const openGamepadTestFlow = async () => {
    const FALLBACK_WEB = 'https://hardwaretester.com/gamepad'
    const OFFICIAL_SITE = 'https://www.controllermeta.com/'

    // 规范化版本号比较：忽略 v 前缀、按数字段比较
    const normalizeVersion = (v) =>
      String(v || '')
        .trim()
        .replace(/^v/i, '')
    const compareVersion = (a, b) => {
      const pa = normalizeVersion(a)
        .split('.')
        .map((x) => parseInt(x, 10) || 0)
      const pb = normalizeVersion(b)
        .split('.')
        .map((x) => parseInt(x, 10) || 0)
      const len = Math.max(pa.length, pb.length)
      for (let i = 0; i < len; i++) {
        const d = (pa[i] || 0) - (pb[i] || 0)
        if (d !== 0) return d
      }
      return 0
    }

    // 执行下载 + 启动的完整流程（已有 loading 实例时复用）
    const downloadAndLaunch = async (release, loading) => {
      const sizeMb = release.download_size ? (release.download_size / 1024 / 1024).toFixed(1) : '?'
      loading.setText(formatMessage(t.value.controllerMeta.downloading, { version: release.version, size: sizeMb }))

      const { listen } = await import('@tauri-apps/api/event')
      const unlisten = await listen('controllermeta-download-progress', (event) => {
        const p = event.payload?.progress ?? 0
        const downloaded = ((event.payload?.downloaded ?? 0) / 1024 / 1024).toFixed(1)
        loading.setText(formatMessage(t.value.controllerMeta.downloadProgress, { version: release.version, downloaded, size: sizeMb, progress: p }))
      })

      try {
        await controllerMeta.download(release.download_url, release.version)
        unlisten()
        loading.close()
        ElMessage.success(formatMessage(t.value.controllerMeta.installed, { version: release.version }))
        try {
          await controllerMeta.launch()
        } catch (err) {
          console.error('[ControllerMeta] launch failed:', err)
          ElMessage.error(String(err).includes('ControllerMeta 未安装')
            ? t.value.controllerMeta.notInstalled : t.value.controllerMeta.launchFailed)
        }
        return true
      } catch (err) {
        unlisten()
        loading.close()
        console.error('[ControllerMeta] download failed:', err)
        ElMessage.error(String(err).includes('无法覆盖已有版本')
          ? t.value.controllerMeta.closeBeforeUpdate : t.value.controllerMeta.downloadFailed)
        return false
      }
    }

    try {
      const status = await controllerMeta.getStatus()

      if (status.installed) {
        // 已安装：先启动（不阻塞），然后后台检查更新
        try {
          await controllerMeta.launch()
          ElMessage.success(formatMessage(t.value.controllerMeta.launched, { version: status.version ? ' ' + status.version : '' }))
        } catch (err) {
          console.error('[ControllerMeta] launch failed:', err)
          ElMessage.error(String(err).includes('ControllerMeta 未安装')
            ? t.value.controllerMeta.notInstalled : t.value.controllerMeta.launchFailed)
          return
        }

        // 后台静默检查更新，不打扰用户
        ;(async () => {
          try {
            const release = await controllerMeta.checkRelease()
            if (release?.download_url && status.version && compareVersion(release.version, status.version) > 0) {
              const notice = ElNotification({
                title: t.value.controllerMeta.updateTitle,
                message: formatMessage(t.value.controllerMeta.updateMessage, { current: status.version, latest: release.version }),
                type: 'info',
                duration: 0,
                actionText: t.value.controllerMeta.updateAction,
                standing: 'controllermeta-update',
                revision: release.version,
                onClick: async () => {
                  if (controllerBusy.value) return
                  const loading = startBusy(t.value.controllerMeta.preparingUpdate)
                  try {
                    const downloaded = await downloadAndLaunch(release, loading)
                    if (downloaded) notice?.close?.()
                  } catch (err) {
                    console.error('[ControllerMeta] update failed:', err)
                    ElMessage.error(t.value.controllerMeta.downloadFailed)
                  } finally {
                    loading.close()
                  }
                },
              })
            }
          } catch (err) {
            console.warn('[ControllerMeta] 后台检查更新失败（静默）:', err)
          }
        })()

        return
      }

      // 未安装：询问用户操作
      let choice
      try {
        choice = await ElMessageBox({
          title: t.value.controllerMeta.title,
          message:
            t.value.controllerMeta.description,
          showCancelButton: true,
          distinguishCancelAndClose: true,
          confirmButtonText: t.value.controllerMeta.downloadAndLaunch,
          cancelButtonText: t.value.controllerMeta.openWeb,
          closeOnClickModal: false,
          type: 'info',
        })
      } catch (action) {
        if (action === 'cancel') {
          await openExternalUrl(FALLBACK_WEB)
        }
        return
      }

      // 对话框 resolve 的是 action 字符串（如 'confirm'），不是 { action: 'confirm' } 对象。
      // 旧判断会导致点击「下载并启动」后直接返回，进度提示和下载进度监听都不会创建。
      if (choice !== 'confirm') return

      const loading = startBusy(t.value.controllerMeta.checkingRelease)

      let release
      try {
        release = await controllerMeta.checkRelease()
      } catch (err) {
        loading.close()
        console.error('[ControllerMeta] checkRelease failed:', err)
        ElMessage.error(t.value.controllerMeta.releaseFailed)
        await openExternalUrl(OFFICIAL_SITE)
        return
      }

      if (!release?.download_url) {
        loading.close()
        ElMessage.warning(t.value.controllerMeta.noDownload)
        await openExternalUrl(OFFICIAL_SITE)
        return
      }

      try {
        await downloadAndLaunch(release, loading)
      } finally {
        loading.close()
      }
    } catch (err) {
      console.error('[ControllerMeta] openGamepadTest failed:', err)
      ElMessage.error(t.value.controllerMeta.unavailable)
      await openExternalUrl(FALLBACK_WEB)
    }
  }

  // The busy state is taken before the first await, so a second click while the status is still in flight (or while a dialog is open) starts no second flow,
  // and it is given back on every path; the blocking layer and its live text are owned by startBusy and its close.
  const openGamepadTest = async () => {
    if (controllerBusy.value) return
    controllerBusy.value = true
    try {
      await openGamepadTestFlow()
    } finally {
      controllerBusy.value = false
    }
  }

  /**
   * Clipboard sync is enabled by default whenever the user-session agent is
   * alive. Panel settings show connection status; the action reports details.
   */
  const refreshClipboardSyncStatus = async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const status = await invoke('clipboard_sync_status')
      // working = the agent is connected, the host hears it and the host's own setting says on; a setting nobody could read is not "on"
      clipboardSyncEnabled.value = clipboardSyncWorks(status)
      clipboardSyncStateWord.value = clipboardSyncState(status)
      return status
    } catch (err) {
      console.warn('[clipboard] status query failed:', err)
      return null
    }
  }

  const showClipboardSyncStatus = async () => {
    const status = await refreshClipboardSyncStatus()
    const msg = t.value.clipboardSync
    if (!status) {
      ElMessage.warning(msg.statusUnavailable)
      return
    }
    if (!status.agent_active || status.transport_state === 'stopped') {
      ElMessage.warning(msg.agentInactive)
    } else if (status.service_allowed === false) {
      ElMessage.warning(msg.serviceDisabled)
    } else if (status.transport_state === 'degraded') {
      ElMessage.warning(msg.degraded)
    } else if (status.transport_state === 'connected' && status.service_allowed !== true) {
      ElMessage.warning(msg.capabilityUnknown)
    } else if (status.transport_state === 'connected') {
      ElMessage.success(msg.active)
    } else if (status.transport_state === 'connecting') {
      ElMessage.info(msg.connecting)
    } else if (status.transport_state === 'disconnected') {
      const detail = status.last_error ? `: ${status.last_error}` : ''
      ElMessage.warning(`${msg.disconnected}${detail}`)
    } else {
      ElMessage.info(msg.inactive)
    }
  }

  /** Load status once for the panel, then keep the settings page up to date. */
  const initClipboardSyncStatus = async () => {
    if (clipboardSyncInitialised) return
    clipboardSyncInitialised = true
    await refreshClipboardSyncStatus()
    clipboardSyncPollTimer = setVisibleInterval(refreshClipboardSyncStatus, 5000, { resume: true })
    window.addEventListener(
      'beforeunload',
      () => {
        if (clipboardSyncPollTimer) clearVisibleInterval(clipboardSyncPollTimer)
        clipboardSyncPollTimer = null
      },
      { once: true },
    )
  }

  return {
    openLatencyClockOnStream,
    openStylusInputProbe,
    openUrl,
    cleanupCovers,
    restartAsAdmin,
    checkForUpdates,
    openGamepadTest,
    showClipboardSyncStatus,
    initClipboardSyncStatus,
    clipboardSyncEnabled,
    clipboardSyncStateWord,
    controllerBusy,
    controllerBusyText,
  }
}
