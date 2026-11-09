<!-- Stand-in for foundation-ui src/vue/installer/MInstaller.vue while that page set is built: the same bridge, the
     title bar the real page draws (drag region, Minimise, Close), and one button per action of each page. -->
<script setup>
import { onMounted, onBeforeUnmount, ref, computed } from 'vue'
import { MButton } from 'foundation-ui/vue'

const props = defineProps({ bridge: { type: Object, required: true } })
const view = ref(null)
let off = null
onMounted(async () => {
  off = props.bridge.subscribe((v) => { view.value = v })
  view.value = (await props.bridge.view()) || view.value
  requestAnimationFrame(() => requestAnimationFrame(() => props.bridge.send({ type: 'ready' })))
})
onBeforeUnmount(() => off?.())
const send = (type, extra = {}) => props.bridge.send({ type, ...extra })
const actions = computed(() => ({
  welcome: ['install', 'customize', 'exit'],
  maintenance: ['install', 'customize', 'uninstall'],
  components: ['install', 'back', 'select-all'],
  uac: [],
  installing: ['minimise'],
  failed: ['retry', 'copy-details', 'open-log-folder'],
  done: ['open-panel', 'restart-pc'],
}[view.value?.page] || []))
</script>

<template>
  <div class="ph">
    <header class="ph__bar" data-tauri-drag-region>
      <span class="ph__title" data-tauri-drag-region>Foundation Sunshine</span>
      <button class="ph__cap" aria-label="Minimise" @click="send('minimise')">&#x2013;</button>
      <button class="ph__cap" aria-label="Close" @click="send('exit')">&#x2715;</button>
    </header>
    <main class="ph__body" v-if="view">
      <h1 class="ph__page">{{ view.page }}</h1>
      <div class="ph__spin" v-if="view.busy" />
      <p v-if="view.maintenance">{{ view.maintenance.installedVersion }} → {{ view.maintenance.packageVersion }}</p>
      <p v-if="view.page === 'installing' || view.page === 'uac'">
        <span v-for="g in view.progress.groups" :key="g.id" class="ph__group">{{ g.id }}: {{ g.state }}</span>
      </p>
      <div v-if="view.page === 'components'" class="ph__comps">
        <label v-for="(on, id) in view.components" :key="id"><input type="checkbox" :checked="on" @change="send('toggle-component', { id })"> {{ id }}</label>
      </div>
      <div class="ph__actions">
        <MButton v-for="(a, i) in actions" :key="a" :variant="i ? 'tonal' : 'filled'" @click="send(a)">{{ a }}</MButton>
      </div>
    </main>
  </div>
</template>

<style scoped>
.ph { display: flex; flex-direction: column; height: 100%; background: var(--md-sys-color-surface, #fdf8fd); color: var(--md-sys-color-on-surface, #1c1b1f); font-family: var(--md-ref-typeface-plain, 'Segoe UI Variable', 'Segoe UI', sans-serif); }
.ph__bar { display: flex; align-items: center; height: 40px; padding-left: 16px; }
.ph__title { flex: 1; font-size: 13px; }
.ph__cap { width: 46px; height: 40px; border: 0; background: transparent; color: inherit; font-size: 14px; }
.ph__cap:hover { background: color-mix(in srgb, currentColor 8%, transparent); }
.ph__body { flex: 1; padding: 24px 40px; display: flex; flex-direction: column; gap: 12px; }
.ph__page { margin: 0; font-size: 32px; font-weight: 400; text-transform: capitalize; }
.ph__group { margin-right: 16px; }
.ph__comps { display: grid; grid-template-columns: repeat(2, 1fr); gap: 6px; }
.ph__actions { margin-top: auto; display: flex; gap: 8px; justify-content: flex-end; }
.ph__spin { width: 40px; height: 40px; border-radius: 50%; border: 4px solid var(--md-sys-color-primary, #6750a4); border-right-color: transparent; animation: ph-spin 0.9s linear infinite; }
@keyframes ph-spin { to { transform: rotate(360deg); } }
</style>
