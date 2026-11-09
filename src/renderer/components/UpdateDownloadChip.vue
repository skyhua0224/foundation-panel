<script setup>
import { computed, onBeforeUnmount, onMounted } from 'vue'
import { MIcon } from 'foundation-ui/vue'
import { useUpdateDownload } from '../composables/updateDownload.js'
import { useI18n } from '../desktop/i18n/index.js'

// A download that continues after its sheet closes stays visible in the panel:
// the chip is the way back to the sheet, and it survives page moves because it
// lives in the caption bar.
const emit = defineEmits(['open'])
const { t } = useI18n()
const download = useUpdateDownload()

const shown = computed(() => download.visible.value)
const label = computed(() => {
  if (download.ready.value) return t.value.updateDialog.downloadChipReady
  const percent = download.percent.value
  const template = t.value.updateDialog.downloadChipLabel
  return percent == null ? template.replace('{progress}', '') : template.replace('{progress}', String(percent))
})

let poll = 0
onMounted(async () => {
  // A transfer started before this window was closed is still owned by the
  // native task; read it once, then follow it while it is active.
  await download.refresh()
  poll = window.setInterval(() => {
    if (download.state.active) download.refresh()
  }, 1000)
})
onBeforeUnmount(() => clearInterval(poll))
</script>

<template>
  <button
    v-if="shown"
    type="button"
    class="panel-download-chip"
    :class="{ 'is-ready': download.ready.value }"
    :aria-label="label"
    :data-tip="label"
    @click="emit('open')"
  >
    <MIcon :name="download.ready.value ? 'check_circle' : 'download'" />
    <span v-if="!download.ready.value && download.percent.value != null">{{ download.percent.value }}%</span>
  </button>
</template>

<style scoped>
.panel-download-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 32px;
  padding: 0 10px;
  border: 0;
  border-radius: 16px;
  background: color-mix(in srgb, var(--md-sys-color-primary) 16%, transparent);
  color: var(--md-sys-color-primary);
  font: var(--m-label-large);
  cursor: pointer;
}

.panel-download-chip.is-ready {
  background: color-mix(in srgb, var(--md-sys-color-tertiary) 18%, transparent);
  color: var(--md-sys-color-tertiary);
}
</style>
