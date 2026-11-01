<template>
  <section class="m-status" aria-labelledby="ds5t-status-title">
    <MHero :tone="heroTone" :icon="heroIcon" :title="status ? stateLabel : tx('status.heading')" :text="status ? nextText : (failed ? tx('errors.DS5-HOST-001') : tx('status.loading'))">
      <MButton variant="tonal" size="sm" icon="refresh" :loading="loading" @click="$emit('refresh')">{{ tx('refresh') }}</MButton>
    </MHero>
    <template v-if="status">
      <MGroup class="m-status__facts">
        <MItem v-for="row in rows" :key="row.id" :icon="ROW_ICON[row.tone] || 'info'" :tone="ROW_TONE[row.tone] || ''" :title="tx('status.rows.' + row.id)" :text="row.text" />
      </MGroup>
      <div v-if="needsTransport" class="m-notice m-notice--warn">
        <MIcon name="warning" fill />
        <div class="m-notice__body">
          <span v-if="status.state === 'usbip_missing'" class="m-notice__title">{{ tx('status.install.title') }}</span>
          <ol v-if="status.state === 'usbip_missing'" class="m-status__steps"><li v-for="(step, number) in installSteps" :key="number">{{ step }}</li></ol>
          <div class="m-notice__actions"><MButton size="sm" @click="$emit('manage-components')">{{ tx('status.install.open') }}</MButton></div>
        </div>
      </div>
    </template>
  </section>
</template>

<script setup>
import { MHero, MGroup, MItem, MButton, MIcon } from 'foundation-ui/vue'
import { computed } from 'vue'
import { Connection, Refresh } from '@element-plus/icons-vue'
import { useDualSenseText } from '../../composables/useDualSenseText.js'
import { statusRows, statusTone } from '../../composables/dualsenseStatusView.js'

const props = defineProps({
  status: { type: Object, default: null },
  loading: { type: Boolean, default: false },
  failed: { type: Boolean, default: false },
})
defineEmits(['refresh', 'manage-components'])

const { text, tx } = useDualSenseText()

const tone = computed(() => (props.status ? statusTone(props.status.state) : 'neutral'))
const heroTone = computed(() => ({ good: 'success', ok: 'success', warn: 'tertiary', bad: 'neutral', error: 'neutral' }[tone.value] || 'tertiary'))
const heroIcon = computed(() => (['good', 'ok'].includes(tone.value) ? 'check_circle' : ['bad', 'error'].includes(tone.value) ? 'error' : 'sports_esports'))
const ROW_ICON = { ok: 'check_circle', warn: 'warning', bad: 'error', neutral: 'info' }
const ROW_TONE = { ok: 'success', warn: 'tertiary', bad: 'error', neutral: '' }
const stateLabel = computed(() => (props.status ? tx('status.states.' + props.status.state) : ''))
const rows = computed(() => statusRows(props.status, (path, params) => tx('status.values.' + path, params)))
const pinned = computed(() => props.status?.transport?.pinned_version || '')
const nextText = computed(() => tx('status.next.' + props.status.state, {
  pinned: pinned.value,
  version: props.status.transport?.version || '',
}))
const needsTransport = computed(() =>
  ['usbip_missing', 'usbip_version', 'usbip_unsigned', 'usbip_not_ready'].includes(props.status?.state))
const installSteps = computed(() => text.value.status.install.steps)
</script>

