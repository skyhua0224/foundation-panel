<!--
  Truthful status of the native DualSense path. Everything shown here is read
  from the host and the system; nothing is installed from this card. When the
  USB/IP transport is missing it explains how to install it and points to the
  Component management tab, where the person confirms the installation.
-->
<template>
  <section class="ds5t-surface ds5t-status" :class="'tone-' + tone" aria-labelledby="ds5t-status-title">
    <div class="ds5t-card-head">
      <span class="ds5t-chip-icon"><el-icon><Connection /></el-icon></span>
      <h2 id="ds5t-status-title" class="ds5t-card-title">{{ tx('status.heading') }}</h2>
      <span class="ds5t-state" role="status">
        <span class="ds5t-dot-state" aria-hidden="true"></span>{{ stateLabel }}
      </span>
      <button type="button" class="ds5t-btn ds5t-btn-ghost ds5t-btn-sm ds5t-head-action" :disabled="loading" @click="$emit('refresh')">
        <el-icon :class="{ 'is-loading': loading }"><Refresh /></el-icon>
        {{ tx('refresh') }}
      </button>
    </div>

    <div v-if="!status && !failed" class="ds5t-status-wait">{{ tx('status.loading') }}</div>
    <p v-else-if="!status" class="ds5t-next" role="alert">{{ tx('errors.DS5-HOST-001') }}</p>

    <template v-else>
      <p class="ds5t-intro">{{ tx('status.intro') }}</p>
      <dl class="ds5t-facts">
        <div v-for="row in rows" :key="row.id" class="ds5t-fact" :class="'is-' + row.tone">
          <dt>{{ tx('status.rows.' + row.id) }}</dt>
          <dd><span class="ds5t-fact-dot" aria-hidden="true"></span>{{ row.text }}</dd>
        </div>
      </dl>
      <p class="ds5t-next">{{ nextText }}</p>

      <div v-if="needsTransport" class="ds5t-howto">
        <template v-if="status.state === 'usbip_missing'">
          <strong>{{ tx('status.install.title') }}</strong>
          <ol>
            <li v-for="(step, number) in installSteps" :key="number">{{ step }}</li>
          </ol>
        </template>
        <button type="button" class="ds5t-btn ds5t-btn-primary" @click="$emit('manage-components')">
          {{ tx('status.install.open') }}
        </button>
      </div>
    </template>
  </section>
</template>

<script setup>
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

<style scoped>
.ds5t-status {
  padding: 1rem 1.25rem 1.125rem;
}

.ds5t-state {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  margin-left: 0.25rem;
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--ds5t-tone);
}

.ds5t-dot-state,
.ds5t-fact-dot {
  flex: none;
  width: 0.5rem;
  height: 0.5rem;
  border-radius: 50%;
  background: var(--ds5t-tone);
}

.tone-ok { --ds5t-tone: var(--el-color-success); }
.tone-warn { --ds5t-tone: var(--el-color-warning); }
.tone-bad { --ds5t-tone: var(--el-color-danger); }
.tone-neutral { --ds5t-tone: var(--ds5t-muted); }

.ds5t-status-wait {
  color: var(--ds5t-muted);
  font-size: 0.8125rem;
}

.ds5t-intro {
  margin: 0 0 0.75rem;
  color: var(--ds5t-muted);
  font-size: 0.8125rem;
  line-height: 1.55;
}

.ds5t-facts {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.625rem;
  margin: 0;
}

.ds5t-fact {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
  padding: 0.625rem 0.875rem;
  border-radius: 0.75rem;
  background: var(--ds5t-sunken);
}

.ds5t-fact dt {
  color: var(--ds5t-muted);
  font-size: 0.75rem;
}

.ds5t-fact dd {
  display: flex;
  align-items: baseline;
  gap: 0.45rem;
  margin: 0;
  min-width: 0;
  color: var(--ds5t-text);
  font-size: 0.875rem;
  font-weight: 500;
  line-height: 1.4;
}

.ds5t-fact-dot {
  position: relative;
  top: -0.0625rem;
  background: var(--ds5t-muted);
}

.ds5t-fact.is-ok .ds5t-fact-dot { background: var(--el-color-success); }
.ds5t-fact.is-warn .ds5t-fact-dot { background: var(--el-color-warning); }
.ds5t-fact.is-bad .ds5t-fact-dot { background: var(--el-color-danger); }

.ds5t-next {
  margin: 0.875rem 0 0;
  color: var(--ds5t-text);
  font-size: 0.875rem;
  line-height: 1.6;
}

.ds5t-howto {
  display: grid;
  gap: 0.5rem;
  justify-items: start;
  margin-top: 0.875rem;
  padding: 0.875rem 1rem;
  border-radius: 0.75rem;
  background: var(--ds5t-sunken);
  font-size: 0.8125rem;
}

.ds5t-howto ol {
  margin: 0;
  padding-left: 1.25rem;
  color: var(--ds5t-muted);
  line-height: 1.7;
}

@container ds5t-card (max-width: 46rem) {
  .ds5t-facts { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}
</style>
