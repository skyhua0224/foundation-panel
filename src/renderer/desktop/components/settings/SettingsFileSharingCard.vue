<template>
  <SettingsCard :title="text.title" icon="folder_shared">
    <SettingsRow :name="text.status" :description="text.statusDesc">
      <MChip class="dsk-share__status" :tone="statusTone" :icon="statusTone === 'ok' ? 'check_circle' : ''">{{ statusText }}</MChip>
      <MIconButton icon="refresh" tonal :label="text.refresh" :disabled="refreshDisabled" @click="loadMappings" />
    </SettingsRow>

    <SettingsRow :name="text.sharedFolders" :description="text.sharedFoldersDesc">
      <MButton icon="add" :disabled="actionDisabled" @click="addFolder">{{ text.add }}</MButton>
    </SettingsRow>

    <SettingsRow :name="text.explorerMenu" :description="text.explorerMenuDesc">
      <MButton variant="tonal" icon="link" :disabled="actionDisabled" @click="installMenu">{{ text.enable }}</MButton>
      <MButton variant="text" :disabled="actionDisabled" @click="uninstallMenu">{{ text.disable }}</MButton>
    </SettingsRow>

    <template #extra>
      <div class="dsk-share__policy">
        <MChip tone="ok" icon="lock">{{ text.readOnly }}</MChip>
        <MChip tone="ok" icon="devices">{{ text.pairedOnly }}</MChip>
        <MChip tone="ok" icon="link_off">{{ text.blockLinks }}</MChip>
      </div>

      <Transition name="dsk-share-notice">
        <MNotice v-if="notice.text" class="dsk-share__notice" icon="check_circle">
          {{ notice.text }}
          <template #action><MIconButton icon="close" :label="text.close" @click="clearNotice" /></template>
        </MNotice>
      </Transition>

      <MNotice v-if="error" tone="error">{{ error }}</MNotice>

      <div v-if="!runtimeChecked || loading" class="dsk-share__wait">
        <MLoading size="32px" :label="runtimeChecked ? text.loadingShares : text.detectingRuntime" />
        <span>{{ runtimeChecked ? text.loadingShares : text.detectingRuntime }}</span>
      </div>

      <MEmpty v-else-if="!hasTauri" icon="desktop_windows" :title="text.desktopOnly" />

      <MEmpty v-else-if="mappings.length === 0" icon="folder_off" :title="text.empty">
        <MButton icon="add" :disabled="actionDisabled" @click="addFolder">{{ text.chooseFolder }}</MButton>
      </MEmpty>

      <MGroup v-else class="dsk-share__list">
        <MItem v-for="mapping in mappings" :key="mapping.id" icon="folder" :title="mapping.name || mapping.id" :text="mapping.path">
          <template #trail>
            <MChip>{{ mapping.mode === 'readwrite' ? text.readWrite : text.readOnly }}</MChip>
            <MChip icon="devices">{{ clientLabel(mapping) }}</MChip>
            <MChip v-if="!mapping.follow_reparse_points" tone="ok" icon="link_off">{{ text.blockLinks }}</MChip>
            <MIconButton icon="delete" :label="text.revoke" :disabled="actionDisabled" @click="removeMapping(mapping)" />
          </template>
        </MItem>
      </MGroup>
    </template>
  </SettingsCard>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { MButton, MChip, MEmpty, MGroup, MIconButton, MItem, MLoading, MNotice } from 'foundation-ui/vue'
import { open } from '@tauri-apps/plugin-dialog'
import { isTauriRuntime } from '../../composables/useTauri.js'
import { useI18n } from '../../i18n/index.js'
import { fileMapping } from '../../../tauri-adapter.js'
import { requestConfirm } from '../../composables/useConfirm.js'
import SettingsCard from './SettingsCard.vue'
import SettingsRow from './SettingsRow.vue'

const mappings = ref([])
const { t } = useI18n()
const loading = ref(false)
const busy = ref(false)
const runtimeChecked = ref(false)
const hasTauri = ref(false)
const error = ref('')
const notice = ref({ type: 'success', text: '' })

const text = computed(() => t.value.fileSharing)
const canUseSharing = computed(() => runtimeChecked.value && hasTauri.value)
const actionDisabled = computed(() => busy.value || !canUseSharing.value)
const refreshDisabled = computed(() => loading.value || !canUseSharing.value)

const statusTone = computed(() => {
  if (!runtimeChecked.value || loading.value) return 'info'
  if (!hasTauri.value) return 'neutral'
  return mappings.value.length > 0 ? 'ok' : 'neutral'
})

const statusText = computed(() => {
  if (!runtimeChecked.value) return text.value.detecting
  if (!hasTauri.value) return text.value.desktopOnlyShort
  if (loading.value) return text.value.syncing
  if (mappings.value.length === 0) return text.value.notShared
  return text.value.shareCount.replace('{count}', mappings.value.length)
})

async function loadMappings() {
  if (!canUseSharing.value) {
    mappings.value = []
    loading.value = false
    error.value = ''
    return
  }

  loading.value = true
  error.value = ''
  try {
    mappings.value = await fileMapping.list()
  } catch (err) {
    mappings.value = []
    error.value = friendlyError(err)
  } finally {
    loading.value = false
  }
}

async function addFolder() {
  if (actionDisabled.value) return
  busy.value = true
  error.value = ''
  try {
    const path = await open({
      directory: true,
      multiple: false,
    })
    if (!path) return
    const mapping = await fileMapping.quickShareFolder(path)
    await loadMappings()
    showNotice('success', text.value.sharedSuccess.replace('{name}', mapping?.name || folderName(path)))
  } catch (err) {
    error.value = friendlyError(err)
  } finally {
    busy.value = false
  }
}

async function removeMapping(mapping) {
  if (!mapping?.id || actionDisabled.value) return
  // The window's own confirm dialog: a native one cannot be answered with a pad.
  const revoked = await requestConfirm({
    title: text.value.revoke,
    message: text.value.revokeConfirm.replace('{name}', mapping.name || mapping.id),
    confirmLabel: text.value.revoke,
    cancelLabel: text.value.close,
    danger: true,
  })
  if (!revoked || busy.value) return

  busy.value = true
  error.value = ''
  try {
    await fileMapping.remove(mapping.id)
    mappings.value = mappings.value.filter(item => item.id !== mapping.id)
    showNotice('success', text.value.revokedSuccess.replace('{name}', mapping.name || mapping.id))
  } catch (err) {
    error.value = friendlyError(err)
  } finally {
    busy.value = false
  }
}

async function installMenu() {
  if (actionDisabled.value) return
  busy.value = true
  error.value = ''
  try {
    await fileMapping.installMenu()
    showNotice('success', text.value.menuInstalled)
  } catch (err) {
    error.value = friendlyError(err)
  } finally {
    busy.value = false
  }
}

async function uninstallMenu() {
  if (actionDisabled.value) return
  busy.value = true
  error.value = ''
  try {
    await fileMapping.uninstallMenu()
    showNotice('success', text.value.menuRemoved)
  } catch (err) {
    error.value = friendlyError(err)
  } finally {
    busy.value = false
  }
}

function clientLabel(mapping) {
  return mapping.clients?.length
    ? text.value.deviceCount.replace('{count}', mapping.clients.length)
    : text.value.allPairedDevices
}

function folderName(path) {
  return String(path || '')
    .split(/[\\/]/)
    .filter(Boolean)
    .pop() || text.value.folderFallback
}

function showNotice(type, text) {
  notice.value = { type, text }
}

function clearNotice() {
  notice.value = { type: 'success', text: '' }
}

function friendlyError(err) {
  const errorText = String(err || '').trim()
  const msg = t.value.fileSharing
  if (!errorText) return msg.operationFailed
  if (errorText.includes('Connection') || errorText.includes('connection') || errorText.includes('refused')) {
    return msg.connectionFailed
  }
  if (errorText.includes('not a folder')) return msg.chooseFolderError
  if (errorText.includes('does not exist') || errorText.includes('cannot be accessed')) return msg.folderUnavailable
  if (errorText.includes('readwrite mode is not supported')) return msg.readOnlyOnly
  if (errorText.includes('allow_delete')) return msg.deleteUnsupported
  if (errorText.includes('follow_reparse_points')) return msg.reparseUnsupported
  return errorText
}

onMounted(async () => {
  hasTauri.value = await isTauriRuntime()
  runtimeChecked.value = true
  if (hasTauri.value) {
    await loadMappings()
  }
})
</script>


<style scoped>
.dsk-share__policy { display: flex; flex-wrap: wrap; gap: 8px; padding: 0 4px; }
.dsk-share__wait { display: flex; align-items: center; justify-content: center; gap: 12px; padding: 20px; font: var(--m-body-large); color: var(--md-sys-color-on-surface-variant); }
.dsk-share__list { margin: 0; }
.dsk-share-notice-enter-active, .dsk-share-notice-leave-active { transition: opacity var(--m-spring-default-effects-duration) var(--m-spring-default-effects); }
.dsk-share-notice-enter-from, .dsk-share-notice-leave-to { opacity: 0; }
</style>
