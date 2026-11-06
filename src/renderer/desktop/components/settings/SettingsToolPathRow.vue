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
      <MField
        class="dsk-path__field"
        :label="param.label || browseLabel"
        :placeholder="param.placeholder || ''"
        :model-value="currentPath(param)"
        data-focusable
        :data-focus-key="`tool-path-${helper.id}`"
        @update:model-value="emit('update-path', helper.id, param.key, $event)"
      />
      <MIconButton
        v-if="hasTauri"
        icon="folder_open"
        tonal
        :label="browseLabel"
        data-focusable
        @click="emit('browse', helper.id, param.key)"
      />
    </div>
  </SettingsRow>
</template>

<script setup>
import { computed } from 'vue'
import { MField, MIconButton } from 'foundation-ui/vue'
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
</script>

<style scoped>
.dsk-path { display: flex; align-items: center; gap: 8px; width: 100%; }
.dsk-path__field { flex: 1; min-width: 0; --md-outlined-text-field-input-text-font: var(--m-font-mono, ui-monospace, 'Consolas', monospace); }
</style>
