<template>
  <MSection :title="t.deviceHub.tabs.tools">
    <MSkeleton v-if="!initialized" :rows="2" />
    <MGroup v-else>
      <MItem icon="sports_esports" :title="t.controllersHub.peripherals.meta.title" :text="t.controllersHub.peripherals.meta.hint">
        <template #trail>
          <MChip :tone="!probeFailed && metaStatus.installed ? 'ok' : 'neutral'">{{ probeFailed ? t.deviceHub.probeUnavailable : (metaStatus.installed ? (metaStatus.version ? 'v' + metaStatus.version : t.controllersHub.peripherals.installed) : t.controllersHub.peripherals.notInstalled) }}</MChip>
          <MButton size="sm" :disabled="refreshing || probeFailed" @click="emit('open-controller-meta')">{{ t.controllersHub.peripherals.meta.launch }}</MButton>
        </template>
      </MItem>
      <MItem icon="edit" :title="t.controllersHub.peripherals.stylus.title" :text="t.controllersHub.peripherals.stylus.hint">
        <template #trail><MButton size="sm" :disabled="refreshing" @click="emit('open-stylus-input-probe')">{{ t.controllersHub.peripherals.stylus.launch }}</MButton></template>
      </MItem>
    </MGroup>
  </MSection>
</template>

<script setup>
import { MSection, MGroup, MItem, MChip, MButton, MSkeleton } from 'foundation-ui/vue'
import { onMounted, reactive, ref } from 'vue'
import { controllerMeta } from '../../tauri-adapter.js'
import { useI18n } from '../../desktop/i18n/index.js'

const emit = defineEmits(['open-controller-meta', 'open-stylus-input-probe'])
const { t } = useI18n()
const metaStatus = reactive({ installed: false, version: '' })
const probeFailed = ref(false)
const initialized = ref(false)
const refreshing = ref(false)

async function refreshAll() {
  if (refreshing.value) return
  refreshing.value = true
  try {
    const result = await controllerMeta.probeStatus()
    probeFailed.value = !result?.success
    if (result?.success) Object.assign(metaStatus, result.data)
  } catch {
    probeFailed.value = true
  } finally {
    initialized.value = true
    refreshing.value = false
  }
}
onMounted(refreshAll)
</script>
