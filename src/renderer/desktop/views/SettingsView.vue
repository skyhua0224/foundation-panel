<template>
  <DeskPage class="settings-view" :title="t.settings.pageTitle" wide>
    <SettingsAppearanceCard @open-theme-editor="emit('openThemeEditor')" />

    <SettingsToggleSection
      :title="t.settings.startup"
      :items="startupSettings"
      :values="settings"
      @update-value="setSettingValue"
    />

    <SettingsToggleSection
      :title="t.settings.bigScreen"
      :items="bigScreenItems"
      :values="bigScreenSettings"
      @update-value="setBigScreenSetting"
    />

    <SettingsToggleSection
      :title="t.settings.notifications"
      :items="notificationSettings"
      :values="settings"
      @update-value="setSettingValue"
    />

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

onMounted(async () => {
  await initializeSettingsState()
})

onUnmounted(() => {
  disposeSettingsState()
})
</script>
