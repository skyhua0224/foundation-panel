<template>
  <SettingsRow
    stack
    name-class="tool-path-name"
    :name="helper.name"
    :description="helper.description"
  >
    <template #name>
      <LaunchHelperIcon :template-id="helper.id" :size="22" />
      <span>{{ helper.name }}</span>
    </template>

    <div v-for="param in pathParams" :key="param.key" class="dsk-path">
      <MButton
        variant="outlined"
        class="dsk-path__value"
        :class="{ 'is-empty': !currentPath(param) }"
        icon="edit"
        @click="editPath(param)"
      >{{ currentPath(param) || param.placeholder || browseLabel }}</MButton>
      <MIconButton
        v-if="hasTauri"
        icon="folder_open"
        tonal
        :label="browseLabel"
        @click="emit('browse', helper.id, param.key)"
      />
    </div>
  </SettingsRow>
</template>

<script setup>
import { computed } from 'vue'
import { MButton, MIconButton } from 'foundation-ui/vue'
import { openOsk } from '../../composables/useOsk.js'
import { useI18n } from '../../i18n/index.js'
import LaunchHelperIcon from '../LaunchHelperIcon.vue'
import SettingsRow from './SettingsRow.vue'

const props = defineProps({
  helper: { type: Object, required: true },
  hasTauri: { type: Boolean, required: true },
  getPath: { type: Function, required: true },
})

const emit = defineEmits(['update-path', 'browse'])
const { t } = useI18n()

const pathParams = computed(() => props.helper.params.filter(param => param.key === 'path'))
const browseLabel = computed(() => t.value.launchHelper?.browse || 'Browse')
const currentPath = (param) => props.getPath(props.helper.id, param.key) || ''

// A path is typed on the on-screen keyboard (the pad has no other way to type); the value goes out when it is confirmed.
async function editPath(param) {
  const next = await openOsk({
    title: props.helper.name,
    value: currentPath(param),
    placeholder: param.placeholder || '',
  })
  if (next !== null) emit('update-path', props.helper.id, param.key, next)
}
</script>

<style scoped>
.dsk-path { display: flex; align-items: center; gap: 8px; width: 100%; }
.dsk-path__value { flex: 1; min-width: 0; justify-content: flex-start; font-family: var(--m-font-mono, ui-monospace, 'Consolas', monospace); }
.dsk-path__value :deep(span), .dsk-path__value { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.dsk-path__value.is-empty { color: var(--md-sys-color-on-surface-variant); }
</style>
