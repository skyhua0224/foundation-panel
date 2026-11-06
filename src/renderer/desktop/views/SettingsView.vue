<template>
  <DeskPage class="settings-view" :title="t.settings.pageTitle" wide>
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

        <SettingsAdvancedCard
          :values="settings"
          @update-value="setSettingValue"
        />
      </div>

      <div class="dsk-settings__col">
        <SettingsLaunchAssistantCard :has-tauri="hasTauri" />

        <SettingsFileSharingCard />

        <SettingsPetCard />
      </div>
    </div>

    <SettingsAboutCard
      :app-version="appVersion"
      :checking="checking"
      @open-link="openLink"
      @check-update="checkUpdate"
    />

    <div class="dsk-settings__foot">
      <SettingsNotice :notice="statusNotice" />
      <SettingsActions @reset="resetSettings" @save="saveSettings" />
    </div>
  </DeskPage>
</template>

<script setup>
import { computed, onMounted, onUnmounted } from 'vue'
import { useSettingsState } from '../composables/useSettingsState'
import { bigScreenSettings, setBigScreenSetting } from '../composables/useBigScreenSettings.js'
import { useI18n } from '../i18n/index.js'
import DeskPage from '../components/DeskPage.vue'
import SettingsActions from '../components/settings/SettingsActions.vue'
import SettingsAboutCard from '../components/settings/SettingsAboutCard.vue'
import SettingsAdvancedCard from '../components/settings/SettingsAdvancedCard.vue'
import SettingsAppearanceCard from '../components/settings/SettingsAppearanceCard.vue'
import SettingsFileSharingCard from '../components/settings/SettingsFileSharingCard.vue'
import SettingsLaunchAssistantCard from '../components/settings/SettingsLaunchAssistantCard.vue'
import SettingsNotice from '../components/settings/SettingsNotice.vue'
import SettingsPetCard from '../components/settings/SettingsPetCard.vue'
import SettingsToggleSection from '../components/settings/SettingsToggleSection.vue'

const { t } = useI18n()

const {
  appVersion,
  checking,
  hasTauri,
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

<style scoped>
.dsk-settings { display: grid; grid-template-columns: minmax(0, 1fr); gap: 24px; margin-bottom: 24px; }
.dsk-settings__col { display: flex; flex-direction: column; gap: 24px; min-width: 0; }
@media (min-width: 1500px) {
  .dsk-settings { grid-template-columns: repeat(2, minmax(0, 1fr)); align-items: start; }
}
.dsk-settings__foot { position: sticky; bottom: 16px; z-index: 8; display: flex; flex-direction: column; align-items: center; gap: 8px; margin-top: 16px; }
</style>
