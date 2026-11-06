<template>
  <DeskPage class="settings-view" :title="t.settings.pageTitle" icon="settings" :text="heroText" wide>
    <template #chips>
      <MChip v-for="chip in heroChips" :key="chip.label" :tone="chip.on === chip.total ? 'ok' : 'neutral'">{{ chip.label }} {{ chip.on }}/{{ chip.total }}</MChip>
    </template>

    <div class="dsk-settings">
      <div class="dsk-settings__col">
        <SettingsAppearanceCard @open-theme-editor="emit('openThemeEditor')" />

        <SettingsToggleSection
          :title="t.settings.startup"
          icon="rocket_launch"
          :items="startupSettings"
          :values="settings"
          @update-value="setSettingValue"
        />
      </div>

      <div class="dsk-settings__col">
        <SettingsToggleSection
          :title="t.settings.bigScreen"
          icon="tv_gen"
          :items="bigScreenItems"
          :values="bigScreenSettings"
          @update-value="setBigScreenSetting"
        />

        <SettingsToggleSection
          :title="t.settings.notifications"
          icon="notifications"
          :items="notificationSettings"
          :values="settings"
          @update-value="setSettingValue"
        />
      </div>
    </div>

    <SettingsAboutCard
      :app-version="appVersion"
      :checking="checking"
      @open-link="openLink"
      @check-update="checkUpdate"
    />

    <!-- what a sofa does not need often: tool paths, file sharing, the pet, the advanced switches -->
    <PanelLinkSection :links="['toolPaths', 'fileSharing', 'pet', 'advanced']" />

    <DeskDock>
      <SettingsNotice :notice="statusNotice" />
      <SettingsActions @reset="resetSettings" @save="saveSettings" />
    </DeskDock>
  </DeskPage>
</template>

<script setup>
import { computed, onMounted, onUnmounted } from 'vue'
import { MChip } from 'foundation-ui/vue'
import { useSettingsState } from '../composables/useSettingsState'
import { bigScreenSettings, setBigScreenSetting } from '../composables/useBigScreenSettings.js'
import { useI18n } from '../i18n/index.js'
import DeskPage from '../components/DeskPage.vue'
import DeskDock from '../components/DeskDock.vue'
import PanelLinkSection from '../components/PanelLinkSection.vue'
import SettingsActions from '../components/settings/SettingsActions.vue'
import SettingsAboutCard from '../components/settings/SettingsAboutCard.vue'
import SettingsAppearanceCard from '../components/settings/SettingsAppearanceCard.vue'
import SettingsNotice from '../components/settings/SettingsNotice.vue'
import SettingsToggleSection from '../components/settings/SettingsToggleSection.vue'

const { t } = useI18n()

const {
  appVersion,
  checking,
  settings,
  statusNotice,
  checkUpdate,
  disposeSettingsState,
  initializeSettingsState,
  openLink,
  resetSettings,
  saveSettings,
} = useSettingsState(t)

const startupSettings = computed(() => [
  {
    key: 'autoStart',
    name: t.value.settings.autoStart,
    description: t.value.settings.autoStartDesc,
  },
  {
    key: 'startMinimized',
    name: t.value.settings.startMinimized,
    description: t.value.settings.startMinimizedDesc,
  },
  {
    key: 'autoStartSunshine',
    name: t.value.settings.autoStartSunshine,
    description: t.value.settings.autoStartSunshineDesc,
  },
])

const bigScreenItems = computed(() => [
  {
    key: 'autoYieldOnLaunch',
    name: t.value.settings.autoYield,
    description: t.value.settings.autoYieldDesc,
  },
  {
    key: 'gamepadCursorEnabled',
    name: t.value.settings.gamepadCursor,
    description: t.value.settings.gamepadCursorDesc,
  },
  {
    key: 'oskAutoOpen',
    name: t.value.settings.oskAutoOpen,
    description: t.value.settings.oskAutoOpenDesc,
  },
  {
    key: 'navSounds',
    name: t.value.settings.navSounds,
    description: t.value.settings.navSoundsDesc,
  },
])

const notificationSettings = computed(() => [  {
    key: 'notifications',
    name: t.value.settings.desktopNotifications,
    description: t.value.settings.desktopNotificationsDesc,
  },
  {
    key: 'connectionNotify',
    name: t.value.settings.connectionNotify,
    description: t.value.settings.connectionNotifyDesc,
    disabled: !settings.value.notifications,
  },
  {
    key: 'updateNotify',
    name: t.value.settings.updateNotify,
    description: t.value.settings.updateNotifyDesc,
    disabled: !settings.value.notifications,
  },
])

function setSettingValue(key, value) {
  settings.value = {
    ...settings.value,
    [key]: value,
  }
}

const emit = defineEmits(['openThemeEditor'])

// the hero: the version, and how much of each group is on
const onCount = (items, values) => items.filter((item) => Boolean(values[item.key])).length
const heroText = computed(() => `${t.value.settings.version} ${appVersion.value}`)
const heroChips = computed(() => [
  { label: t.value.settings.startup, on: onCount(startupSettings.value, settings.value), total: startupSettings.value.length },
  { label: t.value.settings.bigScreen, on: onCount(bigScreenItems.value, bigScreenSettings.value), total: bigScreenItems.value.length },
  { label: t.value.settings.notifications, on: onCount(notificationSettings.value, settings.value), total: notificationSettings.value.length },
])

onMounted(async () => {
  await initializeSettingsState()
})

onUnmounted(() => {
  disposeSettingsState()
})
</script>

<style scoped>
.dsk-settings { display: grid; grid-template-columns: minmax(0, 1fr); gap: 20px; margin-bottom: 20px; }
.dsk-settings__col { display: flex; flex-direction: column; gap: 20px; min-width: 0; }
@media (min-width: 1500px) {
  .dsk-settings { grid-template-columns: repeat(2, minmax(0, 1fr)); align-items: start; }
}
</style>
