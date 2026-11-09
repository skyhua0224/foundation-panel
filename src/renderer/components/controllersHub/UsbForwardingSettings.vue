<template>
  <MSection :title="text.forwardingTitle" :description="text.forwardingHint">
    <MSkeleton v-if="loading" :rows="2" role="status" :aria-label="text.loadingConfig" />
    <MNotice v-else-if="loadError" tone="error" :title="text.loadConfigFailed">
      <template #action><MButton size="sm" @click="load">{{ text.retryLoad }}</MButton></template>
    </MNotice>
    <MNotice v-else-if="!supported">{{ text.updateHost }}</MNotice>
    <template v-else>
      <MGroup>
        <MSwitchItem v-model="enabled" icon="usb" :title="text.allowClients" />
        <MItem action icon="settings_ethernet" :title="text.advancedPort" :value="port ? String(port) : '0'" @click="portOpen = !portOpen">
          <template #trail><MIcon :name="portOpen ? 'keyboard_arrow_up' : 'keyboard_arrow_down'" /></template>
        </MItem>
        <MItem v-if="portOpen" plain :title="text.forwardingPort">
          <template #trail>
            <md-outlined-text-field class="m-port-field" type="number" min="0" max="65535" :value="port" :error="!validPort" :aria-label="text.forwardingPort" @input="port = Number($event.target.value)"><span slot="container" class="m-field__fill" aria-hidden="true" /></md-outlined-text-field>
          </template>
        </MItem>
      </MGroup>
      <p v-if="!validPort" aria-live="polite" class="m-section__desc" style="color: var(--md-sys-color-error)">{{ text.invalidForwardingPort }}</p>
      <MNotice v-if="enabled && !transportReady" tone="warn">{{ text.prepareTransport }}</MNotice>
    </template>
  </MSection>
</template>

<script setup>
import { MSection, MGroup, MItem, MSwitchItem, MNotice, MButton, MIcon, MSkeleton } from 'foundation-ui/vue'
import { useUsbPending } from '../../composables/useUsbPending.js'
import { computed, onMounted, ref } from 'vue'
import { usbip } from '../../tauri-adapter.js'
import { useI18n } from '../../desktop/i18n/index.js'

defineProps({ transportReady: Boolean })
const { t, locale } = useI18n()
const text = computed(() => t.value.deviceHub.usb)
const loading = ref(true)
const loadError = ref(false)
const supported = ref(false)
const enabled = ref(false)
const portOpen = ref(false)
const port = ref(0)
const baseline = ref({ enabled: false, port: 0 })
const dirty = computed(() => enabled.value !== baseline.value.enabled || port.value !== baseline.value.port)
const ready = computed(() => !loading.value && !loadError.value && supported.value)
// the changes belong to the window's unsaved state: the bar saves them from any page
useUsbPending({
  state: () => ({ enabled: enabled.value, port: port.value }),
  baseline: () => baseline.value,
  ready,
  locale: () => locale.value,
  valid: () => validPort.value,
  set: (key, value) => { if (key === 'enabled') enabled.value = value; else if (key === 'port') port.value = value; else if (key === 'baseline') baseline.value = { enabled: enabled.value, port: port.value }; else if (key === 'reload') load() },
})
const validPort = computed(() => Number.isInteger(port.value) && (port.value === 0 || (port.value >= 1024 && port.value <= 65535)))

async function load() {
  loading.value = true
  loadError.value = false
  try {
    const result = await usbip.getForwardingConfig()
    if (!result?.success) throw new Error('load failed')
    supported.value = result.data.supported === true
    enabled.value = result.data.enabled === true
    port.value = result.data.port
    baseline.value = { enabled: enabled.value, port: port.value }
  } catch {
    loadError.value = true
  } finally {
    loading.value = false
  }
}

onMounted(load)
</script>

