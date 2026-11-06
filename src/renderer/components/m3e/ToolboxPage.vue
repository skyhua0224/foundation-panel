<template>
  <MPage :title="t.sidebar.toolbox">
    <MGroup>
      <MItem action icon="sports_esports" :title="t.controllersHub.peripherals.meta.title" :text="metaLine" @click="byId('gamepad-test')?.action()">
        <template #trail><span class="panel-tool__go">{{ t.controllersHub.peripherals.meta.launch }}</span></template>
      </MItem>
      <MItem action icon="edit" :title="t.controllersHub.peripherals.stylus.title" @click="byId('stylus-input-probe')?.action()">
        <template #trail><span class="panel-tool__go">{{ t.controllersHub.peripherals.stylus.launch }}</span></template>
      </MItem>
      <MItem v-for="tool in others" :key="tool.id" action :icon="ICONS[tool.id] || 'build'" :title="tool.label" :text="tool.description" @click="tool.action()">
        <template v-if="tool.external" #trail><MIcon name="open_in_new" /></template>
      </MItem>
    </MGroup>
  </MPage>
</template>

<script setup>
import { computed, onMounted, reactive, ref } from 'vue'
import { MPage, MGroup, MItem, MIcon } from 'foundation-ui/vue'
import { controllerMeta } from '../../tauri-adapter.js'
import { useI18n } from '../../desktop/i18n/index.js'

// 工具箱: the small tools as tiles (ControllerMeta controller test, stylus probe, stream timer, latency test). The controller test shows whether
// ControllerMeta is installed and its version, as the diagnostics card did.
const props = defineProps({ tools: { type: Array, default: () => [] } })
const { t } = useI18n()
const ICONS = { 'test-tools': 'science', 'latency-clock': 'speed', 'stream-timer': 'timer', 'latency-test': 'public' }
const byId = (id) => props.tools.find((tool) => tool.id === id)
const others = computed(() => props.tools.filter((tool) => !['gamepad-test', 'stylus-input-probe'].includes(tool.id)))
const meta = reactive({ installed: false, version: '' })
const failed = ref(false)
const metaBadge = computed(() => (failed.value ? '' : meta.installed ? 'ok' : 'info'))
const metaLine = computed(() => (failed.value ? t.value.deviceHub.probeUnavailable : meta.installed ? (meta.version ? `v${meta.version}` : t.value.controllersHub.peripherals.installed) : t.value.controllersHub.peripherals.notInstalled))
onMounted(async () => {
  try { const r = await controllerMeta.probeStatus(); failed.value = !r?.success; if (r?.success) Object.assign(meta, r.data) } catch { failed.value = true }
})
</script>

<style>
.panel-tool__go { font: var(--m-label-large); color: color-mix(in srgb, var(--md-sys-color-primary) 65%, var(--md-sys-color-on-surface)); }
</style>
