<template>
  <MDialog :model-value="show" :title="warning.title" icon="warning" @update:model-value="(open) => { if (!open) emit('cancel') }" @cancel="emit('cancel')">
    <p>{{ copy.intro }}</p>
    <ul><li v-for="risk in copy.risks" :key="risk">{{ risk }}</li></ul>
    <p>{{ copy.outro }}</p>
    <label class="m-dialog__check"><md-checkbox touch-target="wrapper" :checked="dontShow" @change="dontShow = $event.target.checked" />{{ warning.dont_show }}</label>
    <template #actions>
      <MButton variant="text" @click="emit('cancel')">{{ t.common.cancel }}</MButton>
      <MButton variant="warning" :disabled="!ready" @click="confirm">{{ ready ? warning.confirm : formatText(warning.confirm_wait, { seconds }) }}</MButton>
    </template>
  </MDialog>
</template>

<script setup>
import { MDialog, MButton } from 'foundation-ui/vue'
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

