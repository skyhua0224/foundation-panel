<script setup>
import { frameOrigin, postToFrame } from '../composables/frameOrigin.js'
import { onBeforeUnmount, onMounted } from 'vue'
import { MDialogHost, MNotifyHost, MPendingHost, mountPendingBridge, usePending } from 'foundation-ui/vue'
import { tools } from '../tauri-adapter.js'
import { defineVddSource } from '../composables/useVddPending.js'
import { defineUsbSource } from '../composables/useUsbPending.js'
import { useI18n } from '../desktop/i18n/index.js'
import { ROUTES } from '../composables/useRouter.js'

// The window's one bar of unsaved changes, its list and the one save confirmation, and the place of the snackbars. The Web UI in the frame reports its own
// edits to this window (mountPendingBridge) and saves them itself when asked; the pages of the panel (the virtual display) report here directly.
defineProps({ locale: { type: String, default: 'en' } })
const emit = defineEmits(['go'])
const { t } = useI18n()
const pending = usePending({ restartHost: () => tools.restartSunshineService(true) /* the pending dialog lists the live sessions that end */ })
defineVddSource({ pageLabel: t.value.m3e.nav.display })
defineUsbSource({ pageLabel: t.value.m3e.input.usb })

const frame = () => document.querySelector('.panel-web__frame iframe')
const originOf = (el) => frameOrigin(el)
let stop = () => {}
let last = null
let timer = 0
onMounted(() => {
  stop = mountPendingBridge(pending, {
    post: (message) => { postToFrame(frame(), message) },
    listen: (fn) => {
      const on = (event) => { const el = frame(); if (el && event.source === el.contentWindow && event.origin === originOf(el)) fn(event.data) }
      window.addEventListener('message', on)
      return () => window.removeEventListener('message', on)
    },
  })
  // the frame loads again when it opens another page of the Web UI: it is asked for what it holds
  timer = setInterval(() => { const w = frame()?.contentWindow; if (w && w !== last && originOf(frame())) { last = w; postToFrame(frame(), { type: 'foundation-pending', op: 'hello' }) } }, 600)
})
onBeforeUnmount(() => { stop(); clearInterval(timer) })

// a page of the changes: the Web UI page that holds them, or the page of the panel
function go(page) {
  const entry = pending.all.value.find((e) => e.page === page)
  if (page.startsWith('web:') || page.startsWith('config:') || page === 'gamepad' || page === 'apps') emit('go', { page, entry })
  else if (page === 'vdd') emit('go', { route: ROUTES.VDD_SETTINGS })
  else if (page === 'usb') emit('go', { route: ROUTES.USB })
}
</script>

<template>
  <MNotifyHost />
  <MDialogHost :locale="locale" />
  <MPendingHost :locale="locale" @go="go" />
</template>
