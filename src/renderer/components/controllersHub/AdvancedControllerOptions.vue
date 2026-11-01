<template>
  <div class="m-advanced">
    <MSection :title="t.controllersHub.emulation.dsuTitle" :description="t.controllersHub.emulation.dsuHint">
      <MGroup>
        <MSwitchItem :model-value="config.enable_dsu_server" icon="sensors" :title="t.controllersHub.emulation.dsuEnable" :disabled="busyKey !== ''" @update:model-value="(v) => saveKey('enable_dsu_server', v)" />
        <MItem icon="settings_ethernet" :title="t.controllersHub.emulation.dsuPort" :disabled="!config.enable_dsu_server || busyKey !== ''">
          <template #trail>
            <md-outlined-text-field class="m-port-field" type="number" min="1024" max="65535" :value="config.dsu_server_port" :disabled="!config.enable_dsu_server || busyKey !== ''" @change="(e) => saveKey('dsu_server_port', Number(e.target.value))" />
          </template>
        </MItem>
      </MGroup>
    </MSection>
    <MSection :title="t.controllersHub.emulation.ds4Title" :description="t.controllersHub.emulation.ds4Hint">
      <MGroup>
        <MSwitchItem :model-value="config.ds4_back_as_touchpad_click" icon="touch_app" :title="t.controllersHub.emulation.backAsTouchpadClick" :disabled="busyKey !== ''" @update:model-value="(v) => saveKey('ds4_back_as_touchpad_click', v)" />
        <MSwitchItem :model-value="config.motion_as_ds4" icon="sensors" :title="t.controllersHub.emulation.motionAsDs4" :disabled="busyKey !== ''" @update:model-value="(v) => saveKey('motion_as_ds4', v)" />
        <MSwitchItem :model-value="config.touchpad_as_ds4" icon="touch_app" :title="t.controllersHub.emulation.touchpadAsDs4" :disabled="busyKey !== ''" @update:model-value="(v) => saveKey('touchpad_as_ds4', v)" />
      </MGroup>
    </MSection>
  </div>
</template>

<script setup>
import { MSection, MGroup, MItem, MSwitchItem } from 'foundation-ui/vue'
import { computed, onMounted, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { useControllerHubConfig } from '../../composables/useControllerHubConfig.js'
import { useI18n } from '../../desktop/i18n/index.js'

const { t } = useI18n()

// 与 GamepadTypePicker 共享同一份配置副本；字段不相交，任一处保存双方即刻可见。
const { config: sharedConfig, load, save } = useControllerHubConfig()
const DEFAULTS = {
  motion_as_ds4: true,
  touchpad_as_ds4: true,
  ds4_back_as_touchpad_click: true,
  enable_dsu_server: false,
  dsu_server_port: 26760,
}
const config = computed(() => ({ ...DEFAULTS, ...(sharedConfig.value || {}) }))
const busyKey = ref('')

async function saveKey(key, value) {
  if (busyKey.value !== '' || config.value[key] === value) return
  // 回滚值取生效值（含 DEFAULTS 兜底）：加载完成前改默认开启项且保存失败时，
  // 写回 undefined 会盖掉默认值使选项错误保持关闭。
  const prev = config.value[key]
  sharedConfig.value = { ...(sharedConfig.value || {}), [key]: value }
  busyKey.value = key
  try {
    const result = await save({ [key]: value })
    if (result?.success) {
      ElMessage.success(t.value.controllersHub.emulation.advancedSaveSuccess)
    } else {
      sharedConfig.value = { ...(sharedConfig.value || {}), [key]: prev }
      ElMessage.error(result?.message || t.value.controllersHub.emulation.saveFailed)
    }
  } catch (error) {
    sharedConfig.value = { ...(sharedConfig.value || {}), [key]: prev }
    ElMessage.error(String(error))
  } finally {
    busyKey.value = ''
  }
}

onMounted(load)
</script>
