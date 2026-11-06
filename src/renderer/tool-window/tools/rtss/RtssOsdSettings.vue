<template>
  <MSection :title="t.rtssTool.osdSettings">
    <template #action><MIconButton icon="refresh" :label="t.rtssTool.refresh" @click="load" /></template>
    <MGroup>
      <MSwitchItem :model-value="props.osd_enabled === 1" icon="layers" :title="t.rtssTool.osdEnabled" @update:model-value="(v) => setProp('OSD', v ? '1' : '0')" />
      <MSwitchItem :model-value="props.show_own_stats === 1" icon="monitoring" :title="t.rtssTool.showOwnStats" @update:model-value="(v) => setProp('OSDShowOwnStatistics', v ? '1' : '0')" />
      <MItem stack icon="zoom_in" :title="t.rtssTool.osdZoom"><template #control><MChipSet :model-value="props.zoom" :label="t.rtssTool.osdZoom" :options="[1, 2, 3, 4].map((z) => ({ value: z, label: `${z}x` }))" @update:model-value="(z) => setProp('OnScreenDisplayZoom', String(z))" /></template></MItem>
      <MItem stack icon="picture_in_picture" :title="t.rtssTool.osdPosition">
        <template #control>
          <div class="m-rtss__pos">
            <MField :model-value="positionText.x" type="number" label="X" @update:model-value="(v) => setPosition('OnScreenDisplayX', 'x', v)" />
            <MField :model-value="positionText.y" type="number" label="Y" @update:model-value="(v) => setPosition('OnScreenDisplayY', 'y', v)" />
          </div>
        </template>
      </MItem>
      <MChoiceItem :model-value="props.coordinate_space ?? 0" icon="monitor" :title="t.rtssTool.coordSpace" :options="[{ value: 0, label: t.rtssTool.coordFramebuffer }, { value: 1, label: t.rtssTool.coordScreen }]" :cancel-label="t.common.cancel" @update:model-value="(v) => setProp('OSDCoordinateSpace', String(v))" />
    </MGroup>
  </MSection>
</template>

<script setup>
import { computed, reactive, onMounted } from 'vue'
import { MChipSet, MChoiceItem, MField, MGroup, MItem, MSection, MSwitchItem, MIconButton } from 'foundation-ui/vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from '../../../desktop/i18n/index.js'

const { t } = useI18n()
const emit = defineEmits(['message'])

const props = reactive({
  osd_enabled: null,
  show_own_stats: null,
  position_x: null,
  position_y: null,
  zoom: null,
  coordinate_space: null,
})

const PROP_MAP = {
  OSD: 'osd_enabled',
  OSDShowOwnStatistics: 'show_own_stats',
  OnScreenDisplayX: 'position_x',
  OnScreenDisplayY: 'position_y',
  OnScreenDisplayZoom: 'zoom',
  OSDCoordinateSpace: 'coordinate_space',
}

async function load() {
  try {
    const data = await invoke('rtss_get_osd_properties', { profile: null })
    Object.assign(props, data)
  } catch (e) {
    console.warn('获取 OSD 属性失败:', e)
  }
}

async function setProp(key, value) {
  try {
    await invoke('rtss_set_osd_property', { key, value, profile: null })
    if (PROP_MAP[key]) {
      props[PROP_MAP[key]] = parseInt(value)
    }
    emit('message', t.value.rtssTool.propApplied, 'success')
  } catch (e) {
    emit('message', String(e), 'error')
  }
}

// a position is written when the field holds a whole number of 0 or more; anything else is left for the person to finish typing
const typed = reactive({ x: null, y: null })
const positionText = computed(() => ({ x: typed.x ?? (props.position_x ?? ''), y: typed.y ?? (props.position_y ?? '') }))
async function setPosition(key, axis, value) {
  typed[axis] = value
  if (!/^\d+$/.test(String(value))) return
  await setProp(key, String(value))
  typed[axis] = null
}

onMounted(() => load())

defineExpose({ load })
</script>

<style lang="less" scoped>
.refresh-btn {
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.7);
  font-size: 16px;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 8px;
  transition: all 0.2s;

  &:hover { color: white; background: rgba(255, 255, 255, 0.1); }
}

.prop-grid {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.prop-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;

  > label {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.8);
    flex-shrink: 0;
  }
}

.zoom-btns {
  display: flex;
  gap: 4px;
}

.preset-btn {
  padding: 4px 10px;
  border: 1px solid rgba(255, 255, 255, 0.18);
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.06);
  color: rgba(255, 255, 255, 0.8);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;

  &:hover { background: rgba(255, 255, 255, 0.14); color: white; }
  &.active { background: rgba(147, 197, 253, 0.25); border-color: #93c5fd; color: #c4b5fd; }
  &.small { padding: 3px 8px; font-size: 12px; }
}

.pos-inputs {
  display: flex;
  gap: 8px;
}

.pos-field {
  display: flex;
  align-items: center;
  gap: 4px;
}

.pos-label {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.6);
  font-weight: 600;
}

.pos-input {
  width: 60px;
  padding: 4px 8px;
  background: rgba(0, 0, 0, 0.2);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 8px;
  color: white;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  outline: none;
  text-align: center;

  &:focus { border-color: rgba(96, 165, 250, 0.6); box-shadow: 0 0 0 2px rgba(96, 165, 250, 0.15); }
  &::-webkit-inner-spin-button,
  &::-webkit-outer-spin-button { -webkit-appearance: none; margin: 0; }
}

.fmt-select {
  padding: 4px 8px;
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.2);
  color: rgba(255, 255, 255, 0.9);
  font-size: 12px;
  cursor: pointer;
  outline: none;

  &.wide { min-width: 80px; }
  option { background: #1e1e2e; color: white; }
}
</style>

<style scoped>
.m-rtss__pos { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
</style>
