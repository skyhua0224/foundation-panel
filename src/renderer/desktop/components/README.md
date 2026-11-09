# Desktop 组件

大屏桌面（`src/renderer/desktop`）的组件。控件一律用 `foundation-ui/vue` 的 `M*` 组件（按钮、输入、选择、开关、对话框、提示），颜色取 `--md-sys-color-*`；这里没有原生 `<button>`/`<input>`/`<select>`/`<textarea>`（隐藏的文件选择器和取色器除外，它们由 `MButton`/`MIconButton` 触发）。弹层、键盘、启动页放在 `.m-app` 下，减少动态的偏好才会生效。

## 导入

```js
import { DesktopWindow, TitleBar, WindowControls, DesktopSidebar, DesktopCard, DesktopGrid, LaunchHelperIcon } from './components'
import { useWindowControls } from './composables'
```

`components/index.js` 只导出上面这些基础组件；其余（应用库、弹层、设置卡片）由使用它们的页面直接按路径引入。

## 外壳

### DesktopWindow
窗口容器：壁纸层（`--fd-wallpaper`）+ 可选标题栏 + 侧栏 + 主区 + 底部浮层。插槽 `titlebar`、`sidebar`、默认、`footer`。

| 属性 | 默认 | 说明 |
|---|---|---|
| `title` | `'Desktop Application'` | 标题栏标题 |
| `icon` | `null` | 标题栏图标 |
| `showTitleBar` | `true` | 大屏外壳传 `false`，标题栏不显示（组件保留） |
| `hasSidebar` | `false` | 主区是否给侧栏让位 |

### TitleBar / WindowControls
自定义标题栏：`title`、`icon`、`draggable`、`showControls`。`WindowControls` 是最小化/最大化(还原)/关闭三个 `MIconButton`，事件 `minimize`、`maximize`、`close`，`disabled` 可整体禁用；窗口状态来自 `useWindowControls`。只在 Tauri 环境可用，浏览器里不报错。

### DesktopSidebar
导航栏，用 `MIcon`、`MBadge`、`MIconButton`。`items`、`bottomItems`：`[{ id, label, icon, disabled?, badge? }]`，`icon` 是 Material 图标名；`activeItem`；`collapsible` + `collapsed`。事件 `item-click`、`update:activeItem`、`update:collapsed`。禁用项不触发事件。

### DesktopCard / DesktopGrid
`DesktopCard`：标题、图标、`variant`(`default|primary|secondary|success|warning|danger`)、`hoverable`、`clickable`、`showHeader`、`noPadding`，事件 `click`。`DesktopGrid`：`cols`(1-6)、`gap`(`xs..xl`)、`responsive`。当前页面直接用全局类 `.desktop-card`、`.desktop-grid`（`desktop.less`），这两个组件是同样样式的封装。

### DeskPage
设备、串流、工具、设置页的外框：`title`、`wide`、`actions` 插槽。就是 foundation-ui 的 `MPage`（标题行加分组的行），与手机、Web UI、Panel 的页面一样；没有 hero、没有形状。按 `useDeskScale` 放大到沙发尺度（基数 1.25，再随主题编辑器的字号）。

### DeskDock
页面的操作条（保存、重置）：用 `Teleport` 把内容送到窗口里的 `#dsk-dock`（`DesktopWindow` 在滚动区下面留的一行，避开手柄提示条的高度），所以不会盖住任何控件。

### PanelLink / PanelLinkSection
「在 Panel 中打开」：一行 `MItem`（图标、页面名、里面有什么、跳出图标）；按下调用 `openInPanel(page)`（`panelLinks.js`），被拒绝时用 snackbar 说。`PanelLinkSection` 是 `MSection` 加一组 `MGroup`，`links` 是 `PANEL_LINKS` 的键。

## 应用库

| 组件 | 说明 |
|---|---|
| `AppToolbar` | 筛选页签(含计数)、搜索 `MField`、排序、卡片大小、网格/列表；事件 `update:activeFilter`、`update:searchQuery`、`cycleSortMode`、`cycleGridSize`、`toggleViewMode` |
| `AppGridView` / `AppListView` | 卡片与行：封面或字母占位、收藏、助手角标、时长、管理员、启动中；事件 `launch`、`contextmenu`、`toggleFavorite`(网格) |
| `AppRecentStrip` | 最近启动（无搜索且筛选为全部时显示） |
| `AppContextMenu` | 启动、收藏、更新封面、启动助手、复制命令、打开目录；`Esc` 关闭，手柄 Y 打开 |
| `CoverPickerModal` | 按应用名自动搜封面，候选网格，点击上传，`MDialog` 内，事件 `close`、`updated` |
| `LaunchHelperPanel` / `LaunchHelperIcon` | 启动助手：启用、排序、参数(必填校验、路径检查、浏览)、命令预览、保存；`MSheet` 内，事件 `close`、`saved` |

## 弹层与状态

| 组件 | 说明 |
|---|---|
| `ConfirmDialog` + `useConfirm` | 应用内确认(标题、消息、取消/确定、危险态)；`Esc`/B 为取消。不用 `window.confirm`：手柄关不掉它 |
| `LaunchOverlay` | 启动状态：启动中、已启动、未跟踪、冲突(停止/关闭)、已退出、出错；事件 `dismiss`、`stop` |
| `RunningGameBar` | 正在玩：时长、累计、接管、返回游戏、停止；事件 `resume`、`stop` |
| `OnScreenKeyboard` | 屏幕键盘(`useOsk`)：`await openOsk({ title })` 返回输入；Shift、符号、空格、删除、确认 |
| `ThemeEditor` | 主题与壁纸：壁纸、取色种子、预设、颜色、圆角、字号、背景网格/扫描线、导出/导入 |
| `GamepadLegend` | 按键提示条，只显示当前页面真的会响应的键 |
| `BackHoldRing` | 长按 B 的进度环 |
| `PadGlyph` | 一个动作在当前手柄类型上的按键图标（Xbox 字、PlayStation 形状、Nintendo 字、generic 位置），尺寸 md / sm / lg |
| `PadHint` | 带 `{confirm}` `{back}` 记号的一句话，记号画成 `PadGlyph` |
| `SplashScreen` | 启动页，`done` 事件结束 |

## 设置页卡片（`settings/`）
`SettingsCard`（`MSection` 加 `MGroup`：分组名、连成一组的行）、`SettingsRow`（`MItem`）、`SettingsToggleRow`（开关行：整行可按，开着时下面可带一组 `MSeg`）、`SettingsSwitch`、`SettingsSelect`（`MSeg`）、`SettingsToggleSection`、`SettingsNotice`、`SettingsActions`，以及外观、关于两组。文件共享、桌宠、启动助手路径、高级这几项在 Panel 里，大屏上是 `PanelLink` 行。

## Composables

| 文件 | 说明 |
|---|---|
| `useWindowControls` | 窗口最小化、最大化(还原)、关闭、`isMaximized` |
| `useGamepad` / `useGamepadActions` / `useGamepadLayout` / `useGamepadCursor` | 读手柄、页面声明响应的键、按手柄型号换符号、摇杆光标 |
| `useFocusNav` | 焦点作用域栈、方向导航、焦点还原 |
| `useOsk` | 屏幕键盘；`isMaterialTextField`、`isTextEntryElement` 决定 A 键是打开键盘还是点击 |
| `useConfirm` | `requestConfirm`、`acceptConfirm`、`rejectConfirm` |
| `useTheme` / `useDynamicColor` / `materialBridge` | 预设与壁纸取色(HCT 种子)；`--md-sys-color-*` 推到 `--fd-*` |
| `useDeskScale` | 沙发尺度的缩放值 |
| `useApps` / `useLaunchHelpers` / `useGameSession` | 应用库数据、启动助手模板、受跟踪的游戏会话 |
| `useBigScreenSettings` / `useDesktopSettings` / `useSettingsState` | 大屏偏好(localStorage)、桌面设置(Tauri)、设置页状态 |
| `useNavSound` / `useTauri` | WebAudio 导航音、Tauri `invoke` 封装 |

## 样式与动效

- 颜色只用 `--md-sys-color-*`，尺度用 `desktop.less` 里的 `--d-gap`、`--d-radius`、`--d-target`(56px 最小目标)、`--d-focus`(焦点环宽度)。
- 手柄焦点环是实线(远看清楚)，挂在 `<html data-fd-gamepad>`；弹层在窗口之外，也拿得到环。
- 动效时长和缓动取 `--m-spring-*`；`.m-app` 把 `减少` 偏好统一应用到后代。

## 注意

- `-webkit-app-region: drag` 用于标题栏拖拽，按钮区需要 `no-drag`。
- 图标名必须在 foundation-ui 的图标集里，`deskIcons.test.js` 检查设置、主题编辑器、工具、设备、串流页。
- 新增文案同时加到 `i18n/en.js` 和 `i18n/zh.js`。
