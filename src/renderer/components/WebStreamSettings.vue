<template>
  <MPage :title="current === 'config' ? t.webStream.serviceConfig : t.webStream.title" :back="!!current" :back-label="t.m3e.back" @back="back">
    <!-- Status -->
    <template v-if="!current">
      <MStatusPill :icon="status.running ? 'play_arrow' : 'stop'" :tone="status.running ? 'ok' : ''" :action="false"
        :label="(status.running ? t.webStream.statusRunning : (status.installed ? t.webStream.statusStopped : t.webStream.statusNotInstalled)) + (status.version ? ' · v' + status.version : '')">
        <template #actions>
          <MButton v-if="!status.installed" icon="download" :loading="downloading" @click="handleInstall">{{ t.webStream.install }}</MButton>
          <MButton v-else-if="status.running" variant="tonal" icon="stop" :loading="stopping" @click="handleStop">{{ t.webStream.stop }}</MButton>
          <MButton v-else icon="play_arrow" :loading="starting" @click="handleStart">{{ t.webStream.start }}</MButton>
        </template>
      </MStatusPill>
      <MProgress v-if="downloading" :value="downloadProgress > 0 ? downloadProgress / 100 : null" :label="t.webStream.install" />

      <MSection v-if="status.running" :title="t.webStream.accessLink">
        <MGroup>
          <MItem icon="link" :title="status.access_url" plain>
            <template #trail>
              <MIconButton icon="content_copy" :label="t.webStream.copy" @click="copyUrl" />
              <MIconButton icon="open_in_new" :label="t.webStream.open" @click="openInBrowser" />
            </template>
          </MItem>
        </MGroup>
        <MFooter>{{ t.webStream.accessTip.replace('{port}', status.port) }}</MFooter>
      </MSection>

      <MGroup v-if="status.installed">
        <MItem action icon="settings" :title="t.webStream.serviceConfig" @click="open('config')" />
        <MItem action icon="refresh" :title="t.webStream.checkUpdate" @click="handleCheckUpdate" />
      </MGroup>
      <MFooter v-if="status.installed">{{ t.webStream.installPath.replace('{path}', status.install_path) }}</MFooter>
      <MFooter>{{ t.webStream.desc }} <a href="https://github.com/MrCreativ3001/moonlight-web-stream" target="_blank" rel="noopener">Moonlight Web</a></MFooter>
    </template>

    <!-- Service configuration -->
    <template v-else-if="current === 'config'">
      <MGroup>
        <MFieldItem v-model="config.web_server.bind_address" :label="t.webStream.bindAddress" placeholder="0.0.0.0:8080" />
        <MSwitchItem v-model="httpsEnabled" icon="lock" :title="t.webStream.httpsCert" :text="httpsEnabled ? t.webStream.httpsEnabled : t.webStream.httpsDisabled" />
        <template v-if="httpsEnabled">
          <MFieldItem v-model="certKeyPath" :label="t.webStream.privateKey" placeholder="./server/key.pem" />
          <MFieldItem v-model="certPemPath" :label="t.webStream.certFile" placeholder="./server/cert.pem" />
          <MItem action icon="key" :title="t.webStream.generateCert" :text="t.webStream.generateCertTip" :disabled="generatingCert" @click="handleGenerateCert" />
        </template>
      </MGroup>
      <MSection :title="t.webStream.defaultStreamSettings">
        <MGroup>
          <MChoiceItem v-model="defaultVideoCodec" icon="videocam" :title="t.webStream.videoCodec" :options="codecOptions" :cancel-label="t.common.cancel" />
          <MChoiceItem v-model="defaultFps" icon="speed" :title="t.webStream.defaultFps" :options="[30, 60, 120].map((v) => ({ value: v, label: String(v) }))" :cancel-label="t.common.cancel" />
          <MSliderItem v-model="defaultBitrate" icon="bolt" :title="t.webStream.bitrate" :min="1000" :max="150000" :step="1000" :decimals="0" unit=" kbps" />
          <MChoiceItem v-model="defaultTransport" icon="swap_horiz" :title="t.webStream.transportMode" :options="transportOptions" :cancel-label="t.common.cancel" />
        </MGroup>
      </MSection>
      <div class="m-actions-row">
        <MButton icon="save" :loading="saving" @click="handleSaveConfig">{{ t.webStream.saveConfig }}</MButton>
        <MButton variant="tonal" icon="refresh" @click="loadConfig">{{ t.webStream.reload }}</MButton>
        <MChip v-if="configDirty" tone="warn" icon="warning">{{ t.webStream.unsavedChanges }}</MChip>
      </div>
    </template>
  </MPage>
</template>

<script setup>
import { ref, reactive, computed, onMounted, onUnmounted, watch } from 'vue'
import { ElMessage, ElMessageBox } from '../composables/notifyMessage.js'
import { MPage, MGroup, MSection, MItem, MSwitchItem, MSliderItem, MChoiceItem, MFieldItem, MFooter, MStatusPill, MProgress, MButton, MIconButton, MChip, useSubpage } from 'foundation-ui/vue'
import { moonlightWeb } from '../tauri-adapter.js'
import { useI18n } from '../desktop/i18n/index.js'

const { t } = useI18n()

const emit = defineEmits(['close'])
const { current, open, back } = useSubpage('webstream')

// 状态
const status = reactive({
  installed: false,
  running: false,
  install_path: '',
  version: '',
  access_url: '',
  port: 8080,
})

const config = reactive({
  web_server: { bind_address: '0.0.0.0:8080' },
  webrtc: null,
  default_settings: null,
})

// UI 状态
const starting = ref(false)
const stopping = ref(false)
const downloading = ref(false)
const downloadProgress = ref(0)
const saving = ref(false)
const configDirty = ref(false)
const generatingCert = ref(false)
const codecOptions = computed(() => [{ value: 'h264', label: t.value.webStream.h264Label }, { value: 'h265', label: 'H.265 / HEVC' }, { value: 'av1', label: 'AV1' }, { value: 'auto', label: t.value.webStream.autoLabel }])
const transportOptions = computed(() => [{ value: 'auto', label: t.value.webStream.transportAuto }, { value: 'webrtc', label: 'WebRTC' }, { value: 'websocket', label: 'WebSocket' }])
const httpsEnabled = ref(false)
const certKeyPath = ref('./server/key.pem')
const certPemPath = ref('./server/cert.pem')
const defaultVideoCodec = ref('h264')
const defaultBitrate = ref(20000)
const defaultFps = ref(60)
const defaultTransport = ref('auto')

// 状态轮询
let pollTimer = null

// ========== 生命周期 ==========

onMounted(async () => {
  await refreshStatus()
  if (status.installed) {
    await loadConfig()
  }
  startPolling()

  // 监听下载进度事件
  try {
    const { listen } = await import('@tauri-apps/api/event')
    const unlisten = await listen('moonlight-web-download-progress', (event) => {
      downloadProgress.value = event.payload.progress || 0
    })
    // 组件销毁时取消监听
    onUnmounted(() => {
      unlisten()
    })
  } catch (e) {
    console.warn('Cannot listen to download progress event:', e)
  }
})

onUnmounted(() => {
  stopPolling()
})

// 监听配置变化标记脏状态
watch([() => config.web_server.bind_address, httpsEnabled, certKeyPath, certPemPath, defaultVideoCodec, defaultBitrate, defaultFps, defaultTransport], () => {
  configDirty.value = true
})

// ========== 方法 ==========

async function refreshStatus() {
  try {
    const s = await moonlightWeb.getStatus()
    Object.assign(status, s)
  } catch (e) {
    console.error('Failed to get status:', e)
  }
}

async function loadConfig() {
  try {
    const c = await moonlightWeb.getConfig()
    Object.assign(config, c)

    // 确保 bind_address 不为空
    if (!config.web_server?.bind_address) {
      config.web_server.bind_address = '0.0.0.0:8080'
    }

    // 解析 HTTPS 状态
    httpsEnabled.value = !!config.web_server?.certificate
    if (config.web_server?.certificate) {
      certKeyPath.value = config.web_server.certificate.private_key_pem || './server/key.pem'
      certPemPath.value = config.web_server.certificate.certificate_pem || './server/cert.pem'
    }

    // 解析默认设置
    const ds = config.default_settings || {}
    defaultVideoCodec.value = ds.videoCodec || 'h264'
    defaultBitrate.value = ds.bitrate || 20000
    defaultFps.value = ds.fps || 60
    defaultTransport.value = ds.dataTransport || 'auto'

    configDirty.value = false
  } catch (e) {
    console.error('Failed to load config:', e)
  }
}

function startPolling() {
  pollTimer = setInterval(refreshStatus, 5000)
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
}

async function handleStart() {
  starting.value = true
  try {
    const msg = await moonlightWeb.start()
    ElMessage.success(msg)
    await refreshStatus()
  } catch (e) {
    ElMessage.error(t.value.webStream.startFailed.replace('{error}', e))
  } finally {
    starting.value = false
  }
}

async function handleStop() {
  stopping.value = true
  try {
    const msg = await moonlightWeb.stop()
    ElMessage.success(msg)
    await refreshStatus()
  } catch (e) {
    ElMessage.error(t.value.webStream.stopFailed.replace('{error}', e))
  } finally {
    stopping.value = false
  }
}

async function handleInstall() {
  downloading.value = true
  downloadProgress.value = 0
  try {
    // 先检查最新版
    const release = await moonlightWeb.checkRelease()
    if (!release.download_url) {
      ElMessage.error(t.value.webStream.noWindowsDownload)
      return
    }

    await ElMessageBox.confirm(
      t.value.webStream.downloadConfirm.replace('{version}', release.version).replace('{filename}', release.download_name),
      t.value.webStream.installTitle,
      { confirmButtonText: t.value.webStream.downloadInstall, cancelButtonText: t.value.webStream.cancelBtn }
    )

    await moonlightWeb.download(release.download_url, release.version)
    ElMessage.success(t.value.webStream.installComplete)
    await refreshStatus()
    if (status.installed) {
      await loadConfig()
    }
  } catch (e) {
    if (e !== 'cancel') {
      ElMessage.error(t.value.webStream.installFailed.replace('{error}', e))
    }
  } finally {
    downloading.value = false
  }
}

async function handleCheckUpdate() {
  try {
    const release = await moonlightWeb.checkRelease()
    const currentVersion = status.version?.replace(/^v/, '') || '0.0.0'
    const latestVersion = release.version?.replace(/^v/, '') || '0.0.0'

    if (latestVersion === currentVersion) {
      ElMessage.info(t.value.webStream.alreadyLatest)
    } else {
      await ElMessageBox.confirm(
        t.value.webStream.newVersionFound.replace('{version}', release.version).replace('{current}', status.version),
        t.value.webStream.updateAvailable,
        { confirmButtonText: t.value.webStream.downloadUpdate, cancelButtonText: t.value.webStream.laterBtn }
      )

      downloading.value = true
      downloadProgress.value = 0

      // 停止服务后再更新
      if (status.running) {
        await moonlightWeb.stop()
      }
      await moonlightWeb.download(release.download_url, release.version)
      ElMessage.success(t.value.webStream.updateComplete)
      await refreshStatus()
    }
  } catch (e) {
    if (e !== 'cancel') {
      ElMessage.error(t.value.webStream.checkUpdateFailed.replace('{error}', e))
    }
  } finally {
    downloading.value = false
  }
}

async function handleGenerateCert() {
  generatingCert.value = true
  try {
    const result = await moonlightWeb.generateCert()
    certKeyPath.value = result.private_key_pem
    certPemPath.value = result.certificate_pem
    // 生成后自动保存配置
    await handleSaveConfig()
    // 如果服务正在运行，提示重启
    if (status.running) {
      try {
        await ElMessageBox.confirm(
          t.value.webStream.certGenRestart,
          t.value.webStream.certTitle,
          { confirmButtonText: t.value.webStream.restartService, cancelButtonText: t.value.webStream.laterManualRestart }
        )
        await moonlightWeb.stop()
        await new Promise(resolve => setTimeout(resolve, 500))
        await moonlightWeb.start()
        await refreshStatus()
        ElMessage.success(t.value.webStream.serviceRestarted)
      } catch {
        ElMessage.info(t.value.webStream.manualRestartHint)
      }
    } else {
      ElMessage.success(t.value.webStream.certGenSuccess)
    }
  } catch (e) {
    ElMessage.error(t.value.webStream.certGenFailed.replace('{error}', e))
  } finally {
    generatingCert.value = false
  }
}

async function handleSaveConfig() {
  saving.value = true
  try {
    // 基于已加载的完整配置进行更新，保留所有上游字段
    const saveConfig = JSON.parse(JSON.stringify(config))

    // 更新用户可编辑的字段
    saveConfig.web_server.bind_address = config.web_server.bind_address

    if (httpsEnabled.value) {
      saveConfig.web_server.certificate = {
        private_key_pem: certKeyPath.value,
        certificate_pem: certPemPath.value,
      }
      saveConfig.web_server.session_cookie_secure = true
    } else {
      delete saveConfig.web_server.certificate
      saveConfig.web_server.session_cookie_secure = false
    }

    saveConfig.default_settings = {
      ...(saveConfig.default_settings || {}),
      videoCodec: defaultVideoCodec.value,
      bitrate: defaultBitrate.value,
      fps: defaultFps.value,
      dataTransport: defaultTransport.value,
    }

    await moonlightWeb.saveConfig(saveConfig)
    ElMessage.success(t.value.webStream.configSaved)
    configDirty.value = false
  } catch (e) {
    ElMessage.error(t.value.webStream.configSaveFailed.replace('{error}', e))
  } finally {
    saving.value = false
  }
}

function copyUrl() {
  navigator.clipboard.writeText(status.access_url).then(() => {
    ElMessage.success(t.value.webStream.copied)
  }).catch(() => {
    ElMessage.warning(t.value.webStream.copyFailed)
  })
}

async function openInBrowser() {
  try {
    const { openExternalUrl } = await import('../tauri-adapter.js')
    await openExternalUrl(status.access_url)
  } catch (e) {
    window.open(status.access_url, '_blank')
  }
}
</script>

