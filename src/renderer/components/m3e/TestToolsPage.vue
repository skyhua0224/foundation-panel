<template>
  <MPage :title="T.title">
    <MTestTools :locale="locale" :host-rumble="hostRumble" />
  </MPage>
</template>

<script setup>
import { computed } from 'vue'
import { MPage, MTestTools, testToolTexts } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'
import { sunshine } from '../../tauri-adapter.js'
import { hostBase, hostFetch } from '../../composables/hostBase.js'

// 测试工具 (toolbox): 画面测试图, 帧率与卡顿, 音画同步, 声道, 键盘鼠标触控, 防休眠; the shared page of foundation-ui.
const { locale } = useI18n()
const T = computed(() => testToolTexts(locale.value))

// The gamepad tool's rumble: the panel's WebView2 does not reach the virtual pad of a stream by itself (the host log never shows a motor value from it), so the host sends one short pulse through the stream.
// The answer is the host's own (sent / no_pad / busy / unsupported); a refusal or a host from before the route throws with its HTTP status.
async function hostRumble(body = {}) {
  const response = await hostFetch(() => hostBase(() => '', () => sunshine.getProxyUrl()), '/api/dualsense/rumble-test', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })
  const data = await response.json().catch(() => ({}))
  if (!response.ok) throw Object.assign(new Error(typeof data?.error === 'string' ? data.error : `HTTP ${response.status}`), { status: response.status })
  return data
}
</script>
