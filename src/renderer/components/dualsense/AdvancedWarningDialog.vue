<!--
  Warning before an advanced controller section opens: says plainly what a wrong
  value can do, keeps the confirm button locked for a few seconds, and offers a
  per-panel opt-out. The caller supplies the sentences for its own section.
-->
<template>
  <el-dialog
    :model-value="show"
    :title="warning.title"
    width="min(540px, 92vw)"
    class="ds5t-warning"
    align-center
    append-to-body
    @update:model-value="(open) => { if (!open) emit('cancel') }"
  >
    <p class="ds5t-warning-intro">{{ copy.intro }}</p>
    <ul class="ds5t-warning-list">
      <li v-for="risk in copy.risks" :key="risk">{{ risk }}</li>
    </ul>
    <p class="ds5t-warning-outro">{{ copy.outro }}</p>
    <el-checkbox v-model="dontShow">{{ warning.dont_show }}</el-checkbox>
    <template #footer>
      <el-button @click="emit('cancel')">{{ t.common.cancel }}</el-button>
      <el-button type="warning" :disabled="!ready" @click="confirm">
        {{ ready ? warning.confirm : formatText(warning.confirm_wait, { seconds }) }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from '../../desktop/i18n/index.js'
import { formatText } from '../../composables/dualsenseText.js'
import { ADVANCED_WARNING_SECONDS, remainingSeconds } from '../../composables/advancedWarning.js'

const props = defineProps({
  show: { type: Boolean, default: false },
  /** Which sentences to show: a key under dualSense.tuning.advanced_warning. */
  scope: { type: String, default: 'tuning' },
})
// confirm carries the state of the "do not show again" checkbox.
const emit = defineEmits(['confirm', 'cancel'])

const { t } = useI18n()
const warning = computed(() => t.value.dualSense.tuning.advanced_warning)
const copy = computed(() => warning.value[props.scope])

const seconds = ref(ADVANCED_WARNING_SECONDS)
const dontShow = ref(false)
const ready = computed(() => seconds.value === 0)
let startedAt = 0
let timer = null

const stop = () => {
  if (timer !== null) clearInterval(timer)
  timer = null
}

watch(
  () => props.show,
  (show) => {
    stop()
    if (!show) return
    dontShow.value = false
    startedAt = Date.now()
    seconds.value = ADVANCED_WARNING_SECONDS
    timer = setInterval(() => {
      seconds.value = remainingSeconds(Date.now() - startedAt)
      if (seconds.value === 0) stop()
    }, 250)
  },
  { immediate: true },
)

onBeforeUnmount(stop)

const confirm = () => {
  if (ready.value) emit('confirm', dontShow.value)
}
</script>

<style lang="less">
.ds5t-warning {
  .ds5t-warning-intro,
  .ds5t-warning-list,
  .ds5t-warning-outro {
    color: var(--el-text-color-primary);
  }

  .ds5t-warning-intro,
  .ds5t-warning-outro {
    margin: 0 0 0.75rem;
    line-height: 1.6;
  }

  .ds5t-warning-list {
    margin: 0 0 0.75rem;
    padding-left: 1.25rem;
    line-height: 1.6;

    li + li {
      margin-top: 0.4rem;
    }
  }
}
</style>
