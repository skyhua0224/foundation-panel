<script setup>
import { MButton, MProgress } from 'foundation-ui/vue'
import { VIVI } from 'foundation-ui/about/mascot.js'
import { useI18n } from '../desktop/i18n/index.js'

// The panel cannot reach its own Web UI through the local proxy (a proxy or TUN tool may be catching loopback traffic): a state of the page, in the language of the offline card of
// the host (m3e m-offline: the same surface, the type scale, the ring and the button). It checks again by itself and goes away by itself; the button only asks for the next check now.
defineProps({ attempts: { type: Number, default: 0 }, checking: { type: Boolean, default: false } })
const emit = defineEmits(['retry'])
const { t } = useI18n()
</script>

<template>
  <section class="m-offline proxy-blocked" role="alert" aria-live="polite" data-proxy-blocked>
    <div class="m-offline__inner">
      <div class="m-offline__art"><img class="m-offline__vivi" :src="VIVI.bg" alt="" width="512" height="768" decoding="async"></div>
      <div class="m-offline__copy">
        <h2 class="m-offline__title">{{ t.sunshineFrame.proxyTitle }}</h2>
        <p class="m-offline__line">{{ t.sunshineFrame.proxyUnavailable }}</p>
        <div class="m-offline__wait">
          <MProgress circular :value="null" :label="t.sunshineFrame.proxyChecking" class="m-offline__ring" />
          <div class="m-offline__waittext"><b>{{ checking ? t.sunshineFrame.proxyChecking : t.sunshineFrame.proxyWaiting }}</b><span>{{ t.sunshineFrame.proxyAttempts.replace('{n}', String(attempts)) }}</span></div>
        </div>
        <div class="m-offline__actions" role="group">
          <MButton size="lg" icon="refresh" :disabled="checking" @click="emit('retry')">{{ t.sunshineFrame.retry }}</MButton>
        </div>
      </div>
    </div>
  </section>
</template>
