<template>
  <MUpdateSheet
    v-model="visible"
    :phase="phase"
    :current="isLatest ? '' : currentVersion"
    :next="updateInfo?.version || ''"
    :channel="channel"
    :notes="sheet?.notes || null"
    :drivers="sheet?.drivers || null"
    :progress="download.progress"
    :downloaded="download.downloaded"
    :total="download.total || updateInfo?.download_size || 0"
    :source="download.source"
    :download-phase="download.downloadPhase"
    :install-stage="installStage"
    :install-detail="installDetail"
    :wait-hint="installWaitHint"
    :error-text="errorText"
    :error-step="errorStep"
    :mirror-host="mirrorHost"
    :has-release-page="!!sheet?.releasePage"
    :can-skip="!isLatest"
    :locale="locale"
    :close-label="t.updateDialog.close"
    @update="handleDownload"
    @restart="handleInstall(downloadedFilePath)"
    @later="handleCancel"
    @skip="handleSkipVersion"
    @retry="handleRetry"
    @open-release="handleOpenBrowser()"
    @open-changelog="handleOpenBrowser"
  >
    <template v-if="latestNote" #notice>
      <MBanner tone="info" icon="info">{{ latestNote }}</MBanner>
    </template>
  </MUpdateSheet>
</template>

<script setup>
import { ref, computed, watch, onBeforeUnmount } from 'vue'
import { ElMessage, ElMessageBox } from '../composables/notifyMessage.js'
import { MBanner, MUpdateSheet, sheetFromPanelInfo } from 'foundation-ui/vue'
import { channelOfVersion, fetchReleaseMetadata, orderedDownloadUrls } from 'foundation-ui/update'
import { useUpdateSource } from '../composables/useUpdateSource.js'
import { useUpdateDownload } from '../composables/updateDownload.js'
import { useI18n } from '../desktop/i18n/index.js'

const { t, locale } = useI18n()
const updateSource = useUpdateSource()

const DOWNLOAD_ERROR_MESSAGE_KEYS = Object.freeze({
  setup_failed: 'downloadSetupFailed',
  file_preparation_failed: 'downloadFilePreparationFailed',
  file_finalization_failed: 'downloadFileFinalizationFailed',
  sources_exhausted: 'downloadSourcesFailed',
})

const props = defineProps({
  modelValue: { type: Boolean, default: false },
  updateInfo: { type: Object, default: null },
  currentVersion: { type: String, default: '0.0.0' },
})

const emit = defineEmits(['update:modelValue', 'close', 'skip-version'])

const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
})

const updateDownload = useUpdateDownload()
const isDownloading = computed(() => updateDownload.state.active)
const download = computed(() => updateDownload.state)
const downloadedFilePath = computed(() => updateDownload.state.path)
const isInstalling = ref(false)
const installStage = ref('idle')
const installDetail = ref('')
const installWaitHint = ref('')
const installLastEventAt = ref(0)
const errorText = ref('')
const errorStep = ref('download')
const channel = ref('')
const mirrorHost = ref('')
let installProgressUnlisten = null
let installWaitTimer = null

const isLatest = computed(() => !!props.updateInfo?.is_latest)
// "up to date" plus what else is true: the installed version is a prerelease and nothing newer exists
const latestNote = computed(() => (isLatest.value && props.updateInfo?.latest_note === 'installed_prerelease_no_newer' ? t.value.updateDialog.latestNoteInstalledPrerelease : ''))

// Notes, driver notice, release page and installer facts of the update the host reported.
const sheet = computed(() => sheetFromPanelInfo(props.updateInfo, { current: props.currentVersion }))

const phase = computed(() => {
  if (isLatest.value) return 'latest'
  if (errorText.value) return 'error'
  if (isInstalling.value) return 'installing'
  if (isDownloading.value) return 'downloading'
  // 安装包已就绪：本次下载完成或上次会话已完整下载，可直接安装
  if (downloadedFilePath.value) return 'ready'
  return 'available'
})

const resetDownloadState = () => updateDownload.discard()

const clearInstallProgressListener = () => {
  if (installProgressUnlisten) {
    installProgressUnlisten()
    installProgressUnlisten = null
  }
}

const clearInstallWaitTimer = () => {
  if (installWaitTimer) {
    clearInterval(installWaitTimer)
    installWaitTimer = null
  }
}

const markInstallProgressSeen = () => {
  installLastEventAt.value = Date.now()
  installWaitHint.value = ''
}

const startInstallWaitTimer = () => {
  clearInstallWaitTimer()
  markInstallProgressSeen()
  installWaitTimer = setInterval(() => {
    if (!isInstalling.value || installStage.value === 'failed') return
    const elapsedMs = Date.now() - installLastEventAt.value
    if (elapsedMs >= 60000) {
      installWaitHint.value = t.value.updateDialog.installLongWaitHint
    } else if (elapsedMs >= 15000) {
      installWaitHint.value = t.value.updateDialog.installWaitHint
    } else {
      installWaitHint.value = ''
    }
  }, 1000)
}

const resetInstallProgress = () => {
  installStage.value = 'idle'
  installDetail.value = ''
  installWaitHint.value = ''
  installLastEventAt.value = 0
  clearInstallWaitTimer()
}

const getTauriApis = async () => {
  const [{ invoke }, { listen }] = await Promise.all([import('@tauri-apps/api/core'), import('@tauri-apps/api/event')])
  return { invoke, listen }
}

// The update info carries no channel; the metadata the updater read says which channel the version belongs to.
// Both hosts are asked at once, the answer of the mirror is reported when the main host did not answer.
let metadataId = 0
const resolveChannel = async () => {
  const id = ++metadataId
  channel.value = ''
  mirrorHost.value = ''
  const version = props.updateInfo?.version
  if (!version || isLatest.value) return
  const result = await fetchReleaseMetadata()
  if (id !== metadataId || !result.ok) return
  channel.value = channelOfVersion(result.metadata, version)
  mirrorHost.value = result.fallback ? result.host : ''
}

// 打开对话框时检查 temp 目录是否已有完整下载的安装包（上次取消安装或关闭应用时遗留）
let cacheCheckId = 0
const checkCachedInstaller = async () => {
  if (isLatest.value || !props.updateInfo?.download_name) return
  const checkId = ++cacheCheckId
  const filename = props.updateInfo.download_name
  try {
    const { invoke } = await getTauriApis()
    const cached = await invoke('check_cached_update', {
      filename,
      expectedSize: props.updateInfo.download_size || null,
      expectedSha256: props.updateInfo.download_sha256 || null,
      assetType: props.updateInfo.download_type || null,
    })
    if (
      cached &&
      checkId === cacheCheckId &&
      visible.value &&
      props.updateInfo?.download_name === filename &&
      !isDownloading.value &&
      !isInstalling.value
    ) {
      updateDownload.state.path = cached
    }
  } catch (error) {
    console.warn('Failed to check cached installer:', error)
  }
}

const downloadErrorMessage = (error) => {
  const code = error && typeof error === 'object' ? error.code : ''
  const messageKey = DOWNLOAD_ERROR_MESSAGE_KEYS[code] || 'downloadFailed'
  console.error('Download update failed', error)
  return t.value.updateDialog[messageKey]
}

const handleDownload = async () => {
  const downloadUrl = props.updateInfo?.download_url
  if (!downloadUrl) {
    ElMessage.warning(t.value.updateDialog.noDownloadUrl)
    return
  }

  resetDownloadState()
  errorText.value = ''

  try {
    const filename = props.updateInfo.download_name || `sunshine-update-${props.updateInfo.version}.exe`
    // the host that is tried first follows the interface language and the source of the update settings; the other is the one tried when it fails
    const urls = orderedDownloadUrls({ url: downloadUrl, fallbackUrl: props.updateInfo.download_fallback_url || '' }, { source: updateSource.source.value, hasCdk: updateSource.hasCdk.value, locale: locale.value })
    const result = await updateDownload.start({
      url: urls.url,
      fallbackUrl: urls.fallbackUrl,
      filename,
      expectedSize: props.updateInfo.download_size || null,
      expectedSha256: props.updateInfo.download_sha256 || null,
      assetType: props.updateInfo.download_type || null,
    })

    if (!result.ok) {
      console.error('Download update returned an unsuccessful result', result)
      errorStep.value = 'download'
      errorText.value = downloadErrorMessage({ code: result.errorCode })
    }
  } catch (error) {
    errorStep.value = 'download'
    errorText.value = downloadErrorMessage(error)
  }
}

const handleInstall = async (filePath) => {
  try {
    await ElMessageBox.confirm(
      t.value.updateDialog.installConfirm,
      t.value.updateDialog.installTitle,
      { confirmButtonText: t.value.updateDialog.installConfirmBtn, cancelButtonText: t.value.updateDialog.cancelBtn, type: 'warning' }
    )

    errorText.value = ''
    isInstalling.value = true
    resetInstallProgress()
    installStage.value = 'downloaded'
    startInstallWaitTimer()

    const { invoke, listen } = await getTauriApis()
    clearInstallProgressListener()
    installProgressUnlisten = await listen('install-progress', (event) => {
      const payload = event.payload || {}
      if (payload.stage) installStage.value = payload.stage
      installDetail.value = payload.detail || ''
      markInstallProgressSeen()
      if (payload.terminal) clearInstallWaitTimer()
    })

    await invoke('install_update', {
      filePath,
      targetVersion: props.updateInfo?.version || null,
      expectedSize: props.updateInfo?.download_size || null,
      expectedSha256: props.updateInfo?.download_sha256 || null,
      darkTheme: document.documentElement?.getAttribute('data-bs-theme') === 'dark',
    })

    ElMessage.success(t.value.updateDialog.installStarted)
  } catch (error) {
    if (error !== 'cancel') {
      errorStep.value = 'install'
      errorText.value = t.value.updateDialog.installError.replace('{error}', error)
      clearInstallWaitTimer()
      clearInstallProgressListener()
      isInstalling.value = false
    }
  }
}

const handleRetry = () => {
  errorText.value = ''
    if (downloadedFilePath.value) handleInstall(downloadedFilePath.value)
  else handleDownload()
}

const handleOpenBrowser = async (url) => {
  // the release page, or the address a link of the sheet asks for (the full changelog)
  const releasePage = typeof url === 'string' ? url : sheet.value?.releasePage
  if (!releasePage) {
    ElMessage.warning(t.value.updateDialog.noReleasePage)
    return
  }

  try {
    const { invoke } = await getTauriApis()
    await invoke('open_external_url', { url: releasePage })
  } catch (error) {
    ElMessage.error(t.value.updateDialog.openBrowserError.replace('{error}', error))
  }
}

const handleCancel = () => {
  visible.value = false
  emit('close')
}

const handleSkipVersion = () => {
  const version = props.updateInfo?.version
  if (version) {
    emit('skip-version', version)
  }
  visible.value = false
  emit('close')
}

const resetState = () => {
  // An update download outlives this sheet: reopening it must show the
  // transfer, not erase it.
  if (!isDownloading.value && !downloadedFilePath.value) resetDownloadState()
  isInstalling.value = false
  errorText.value = updateDownload.state.errorCode
    ? downloadErrorMessage({ code: updateDownload.state.errorCode })
    : ''
  resetInstallProgress()
  clearInstallProgressListener()
}

onBeforeUnmount(() => {
  metadataId += 1
  clearInstallWaitTimer()
  clearInstallProgressListener()
})

watch(
  () => props.modelValue,
  (newVal) => {
    if (newVal) {
      resetState()
      updateDownload.refresh()
      checkCachedInstaller()
      resolveChannel()
    }
  },
  { immediate: true },
)
</script>
