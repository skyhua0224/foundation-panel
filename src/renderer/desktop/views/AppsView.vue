<template>
  <div class="apps-view">
    <AppToolbar
      :filterTabs="filterTabs"
      :activeFilter="activeFilter"
      :searchQuery="searchQuery"
      :sortLabel="sortLabel"
      :gridSize="gridSize"
      :viewMode="viewMode"
      :totalCount="displayApps.length"
      @update:activeFilter="activeFilter = $event"
      @update:searchQuery="searchQuery = $event"
      @cycleSortMode="cycleSortMode"
      @cycleGridSize="cycleGridSize"
      @toggleViewMode="viewMode = viewMode === 'grid' ? 'list' : 'grid'"
    />

    <AppRecentStrip
      v-if="recentApps.length > 0 && !searchQuery && activeFilter === 'all'"
      :apps="recentApps"
      :getAppImageUrl="getAppImageUrl"
      :handleImageError="handleImageError"
      @launch="launchApp"
      @contextmenu="openContextMenu"
    />

    <!-- 加载状态 -->
    <div v-if="loading" class="apps-loading" role="status">
      <MLoading size="64px" />
      <span>{{ t.apps.loading }}</span>
    </div>

    <!-- 加载失败状态 -->
    <MEmpty v-else-if="loadFailed" class="apps-empty apps-error" role="alert" icon="error" :title="t.apps.loadFailed">
      <MButton size="lg" icon="refresh" data-focusable data-focus-key="apps-retry" @click="loadApps">{{ t.apps.retry }}</MButton>
    </MEmpty>

    <!-- 空状态 -->
    <MEmpty
      v-else-if="displayApps.length === 0"
      class="apps-empty"
      :icon="searchQuery ? 'search_off' : activeFilter === 'favorites' ? 'star' : 'apps'"
      :title="searchQuery ? t.apps.searchNoMatch : activeFilter === 'favorites' ? t.apps.noFavorites : t.apps.noApps"
      :text="!searchQuery && activeFilter === 'all' ? t.apps.addHint : ''"
    />

    <AppGridView
      v-else-if="viewMode === 'grid'"
      :apps="displayApps"
      :gridSize="gridSize"
      :launchingApp="launchingApp"
      :isFavorite="isFavorite"
      :getAppImageUrl="getAppImageUrl"
      :handleImageError="handleImageError"
      :helperIds="getActiveHelperIds"
      :stats="gameStats"
      @launch="launchApp"
      @contextmenu="openContextMenu"
      @toggleFavorite="toggleFavorite"
    />

    <AppListView
      v-else
      :apps="displayApps"
      :launchingApp="launchingApp"
      :isFavorite="isFavorite"
      :getAppImageUrl="getAppImageUrl"
      :handleImageError="handleImageError"
      :stats="gameStats"
      @launch="launchApp"
      @contextmenu="openContextMenu"
    />

    <AppContextMenu
      :visible="contextMenu.visible"
      :x="contextMenu.x"
      :y="contextMenu.y"
      :isFavorited="isFavorite(contextMenu.app?.name)"
      :hasCmd="!!contextMenu.app?.cmd"
      :hasWorkingDir="!!contextMenu.app?.['working-dir']"
      @launch="ctxLaunch"
      @toggleFavorite="ctxToggleFavorite"
      @copyCmd="ctxCopyCmd"
      @openDir="ctxOpenDir"
      @configHelpers="ctxConfigHelpers"
      @updateCover="ctxUpdateCover"
      @close="closeContextMenu"
    />

    <LaunchHelperPanel
      :open="helperPanel.open"
      :appName="helperPanel.appName"
      :app="helperPanel.app"
      :proxyUrl="proxyUrl"
      @close="helperPanel.open = false"
      @saved="loadApps"
    />

    <CoverPickerModal
      :open="coverPicker.open"
      :appName="coverPicker.appName"
      :proxyUrl="proxyUrl"
      @close="coverPicker.open = false"
      @updated="onCoverUpdated"
    />
  </div>
</template>

<script setup>
import { ref, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { MButton, MEmpty, MLoading, useNotify } from 'foundation-ui/vue'
import { useApps } from '../composables/useApps'
import { useLaunchHelpers } from '../composables/useLaunchHelpers'
import { tauriInvoke } from '../composables/useTauri'
import { gamepadConnected, onGamepadAction } from '../composables/useGamepad.js'
import { clearViewActions, setViewActions } from '../composables/useGamepadActions.js'
import { editInputWithOsk } from '../composables/useOsk.js'
import { focusElement } from '../composables/useFocusNav.js'
import { useI18n } from '../i18n/index.js'

const { t } = useI18n()
const notify = useNotify()
import AppToolbar from '../components/AppToolbar.vue'
import AppRecentStrip from '../components/AppRecentStrip.vue'
import AppGridView from '../components/AppGridView.vue'
import AppListView from '../components/AppListView.vue'
import AppContextMenu from '../components/AppContextMenu.vue'
import LaunchHelperPanel from '../components/LaunchHelperPanel.vue'
import CoverPickerModal from '../components/CoverPickerModal.vue'

const {
  loading,
  loadFailed,
  searchQuery,
  launchingApp,
  launchError,
  viewMode,
  gridSize,
  activeFilter,
  filterTabs,
  sortLabel,
  proxyUrl,
  recentApps,
  displayApps,
  gameStats,
  isFavorite,
  toggleFavorite,
  cycleSortMode,
  cycleGridSize,
  getAppImageUrl,
  handleImageError,
  invalidateAppImage,
  loadApps,
  launchApp,
} = useApps()

// 启动失败 / 打开目录失败：用 foundation-ui 的 snackbar 提示，读完清掉
watch(launchError, (message) => {
  if (!message) return
  notify.snack(message, { timeout: 5000 })
  launchError.value = ''
})

// 右键菜单
const contextMenu = ref({ visible: false, x: 0, y: 0, app: null })

// 启动助手面板
const helperPanel = ref({ open: false, appName: '', app: null })
const { getActiveHelperIds, helperPanelOpen } = useLaunchHelpers(t)

// 同步面板状态到共享 composable（供 DesktopApp 控制器返回使用）
watch(() => helperPanel.value.open, (v) => { helperPanelOpen.value = v })
watch(helperPanelOpen, (v) => { if (!v) helperPanel.value.open = false })

function openContextMenu(event, app) {
  contextMenu.value = {
    visible: true,
    x: event.clientX,
    y: event.clientY,
    app,
  }
}

function closeContextMenu() {
  contextMenu.value.visible = false
}

function ctxLaunch() {
  if (contextMenu.value.app) launchApp(contextMenu.value.app)
  closeContextMenu()
}

function ctxToggleFavorite() {
  if (contextMenu.value.app) toggleFavorite(contextMenu.value.app.name)
  closeContextMenu()
}

function ctxConfigHelpers() {
  if (contextMenu.value.app) {
    helperPanel.value = {
      open: true,
      appName: contextMenu.value.app.name,
      app: contextMenu.value.app,
    }
  }
  closeContextMenu()
}

async function ctxCopyCmd() {
  if (contextMenu.value.app?.cmd) {
    try {
      await navigator.clipboard.writeText(contextMenu.value.app.cmd)
    } catch (e) {
      console.error('Copy failed:', e)
    }
  }
  closeContextMenu()
}

async function ctxOpenDir() {
  const dir = contextMenu.value.app?.['working-dir']
  if (dir) {
    try {
      const opened = await tauriInvoke('open_local_path', {
        path: dir,
        appName: contextMenu.value.app.name,
      })
      if (!opened) throw new Error('Open directory returned false')
    } catch (e) {
      console.error('Open dir failed:', e)
      launchError.value = `${t.value.appContext.openDirectory}: ${e}`
      setTimeout(() => { launchError.value = '' }, 4000)
    }
  }
  closeContextMenu()
}

// 封面选择弹窗
const coverPicker = ref({ open: false, appName: '' })

async function ctxUpdateCover() {
  const app = contextMenu.value.app
  closeContextMenu()
  if (!app) return

  coverPicker.value = { open: true, appName: app.name }
}

function onCoverUpdated(appName) {
  invalidateAppImage(appName)
}

function onDocClick() {
  closeContextMenu()
}

// === 手柄动作 ===

/** 当前焦点落在哪个应用卡片上。网格、列表、最近条都带 data-app-name。 */
function focusedApp() {
  const host = document.activeElement?.closest?.('[data-app-name]')
  if (!host) return null
  const name = host.dataset.appName
  return {
    element: host,
    app: displayApps.value.find((entry) => entry.name === name)
      || recentApps.value.find((entry) => entry.name === name)
      || null,
  }
}

/** Y 键：在焦点卡片下方打开上下文菜单，不再依赖鼠标坐标。 */
function openContextMenuForFocus() {
  const focused = focusedApp()
  if (!focused?.app) return false
  const rect = focused.element.getBoundingClientRect()
  contextMenu.value = {
    visible: true,
    x: rect.left,
    y: rect.bottom + 6,
    app: focused.app,
  }
  return true
}

function toggleFavoriteForFocus() {
  const focused = focusedApp()
  if (!focused?.app) return false
  toggleFavorite(focused.app.name)
  return true
}

async function focusSearchWithOsk() {
  const input = document.querySelector('[data-focus-key="apps-search"]')
  if (!input) return
  focusElement(input)
  await editInputWithOsk(input, { title: t.value.apps.searchPlaceholder })
}

function handleGamepadAction(action) {
  // B 键：从内到外逐层关闭本视图拥有的弹层
  if (action === 'back' || action === 'backRoot') {
    if (contextMenu.value.visible) closeContextMenu()
    else if (coverPicker.value.open) coverPicker.value.open = false
    else if (helperPanel.value.open) helperPanel.value.open = false
    return
  }

  // 弹层打开时其余按键留给弹层自己处理
  if (contextMenu.value.visible || coverPicker.value.open || helperPanel.value.open) return

  switch (action) {
    case 'menu':
      openContextMenuForFocus()
      break
    case 'favorite':
      toggleFavoriteForFocus()
      break
    case 'search':
      focusSearchWithOsk()
      break
  }
}

/**
 * 首屏把焦点放在第一张卡片上（有「最近启动」时就是上次玩的那个），
 * 这样手柄第一次按方向键是从有意义的位置开始移动，而不是跳到元素 0。
 * 没插手柄时不做，避免干扰鼠标用户。
 */
async function focusFirstApp() {
  if (!gamepadConnected.value) return
  await nextTick()
  if (document.activeElement && document.activeElement !== document.body) return
  const first = document.querySelector('.apps-view [data-app-name]')
  if (first) focusElement(first)
}

let disposeGamepadActions = null

onMounted(async () => {
  document.addEventListener('click', onDocClick)
  disposeGamepadActions = onGamepadAction(handleGamepadAction)
  // 声明本视图支持的手柄动作，按键提示条据此显示（其他页面不会显示这几项）
  setViewActions(['search', 'favorite', 'menu'])
  await loadApps()
  await focusFirstApp()
})

onUnmounted(() => {
  document.removeEventListener('click', onDocClick)
  clearViewActions()
  disposeGamepadActions?.()
})
</script>

<style lang="less" scoped>
.apps-view {
  max-width: 1760px;
  margin: 0 auto;
}

.apps-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 120px 0;
  gap: 20px;
  font: 500 18px/28px var(--md-ref-typeface-brand, system-ui);
  color: var(--md-sys-color-on-surface-variant);
}

.apps-empty {
  padding: 80px 0;
}
</style>
