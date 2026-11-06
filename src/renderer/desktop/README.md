# 大屏桌面（Foundation Desktop）

沙发上用手柄操作的 10-foot 界面，Tauri + Vue 3。外观是 Material 3 Expressive：控件、对话框、提示都来自 foundation-ui（`foundation-ui/vue` 的 `M*` 组件），颜色只取 `--md-sys-color-*`，动效只取 `--m-spring-*`。这里不再有自绘的按钮、输入框、下拉框。

## 目录结构

```
desktop/
├── DesktopApp.vue         # 外壳：导航栏、页面切换、弹层、手柄路由、浅色/深色/跟随系统
├── main.js / index.html   # 入口
├── desktop.less           # 全局：10-foot 尺度、手柄焦点环、共用块
├── panelLinks.js          # 「在 Panel 中打开」的表和 openInPanel()
├── components/            # 外壳与应用库的组件（见 components/README.md）
│   └── settings/          # 设置页的卡片与行
├── composables/           # 手柄、焦点、屏幕键盘、主题、应用库、游戏会话
├── views/                 # 五个页面 + 设置
├── i18n/                  # en / zh 文案
└── *.test.js              # 图标名检查等
```

## 页面

| 导航 id | 文件 | 内容 |
|---|---|---|
| `apps` | `views/AppsView.vue` | 应用库：筛选、搜索、排序、网格/列表、最近、右键菜单、封面、启动助手 |
| `dashboard` | `views/DashboardView.vue` | 状态、快捷操作（网页控制台、重启服务、主机性能、诊断与日志）、系统信息、内存监控 |
| `devices` | `views/DevicesView.vue` | 配对（二维码；PIN 用 `MPinPad`）、已配对设备、重命名、取消配对 |
| `stream` | `views/StreamView.vue` | 码率（大数字）、编解码器、HDR、自动启动大屏；改一项存一项（`saveSettingKey`），并写明何时生效 |
| `tools` | `views/ToolsView.vue` | 系统检查（GPU、编码器、网络、防火墙）、重启显卡驱动；工具箱各项是 Panel 的页面 |
| `settings` | `views/SettingsView.vue` | 外观、启动、大屏、通知、关于；其余是 Panel 的页面 |

底部导航：语言（EN/中文）、主题编辑器、设置、退出。`desktop-navigate` 事件、托盘 `open_desktop`、`--desktop`/`-d` 仍然进入这里。

页面（设备、串流、工具、设置）都放在 `components/DeskPage.vue` 里：一条 hero（有形状的图标、页面名、一条真实的事实、标签）加内容，按 `useDeskScale` 放大 1.25 倍（再随主题编辑器的字号）。页面的操作条（保存、重置）用 `components/DeskDock.vue`，停在页面下面独立一行，不浮在内容上。

## 在 Panel 中打开

沙发上不常用的东西留在主 Panel：每项是一块磁贴（`components/PanelLink.vue`），手柄按下就通过宿主命令 `open_panel_page` 打开 Panel 的对应页面（`windows::open_main_panel_page`）。
表在 `panelLinks.js`；页面名是 `composables/panelParts.js` 的 `PANEL_PAGES`，宿主允许表在 `src-tauri/src/commands.rs`，`panelLinks.test.js` 让两份和这张表保持一致。
哪些功能搬到了哪一页，见 `/var/tmp/dispatch/md-desk3-panel-map.md` 一表（每项都对过 Panel 的代码；Panel 没有对应页面的，例如系统检查和重启显卡驱动，留在大屏）。
留在大屏的设置和 Panel 走同一份保存代码：`useDesktopSettings.js`、`useBigScreenSettings.js`、串流项用 `saveSettingKey`。

## 入口

- 主机的应用列表里有「大屏 / Big Screen」（新装默认有，旧版本升级后一次性补上，删了不再回来）：启动它 = 桌面串流 + `sunshine-gui.exe --desktop`（已经开着就把窗口带到前台，不开第二个），串流结束时 `sunshine-gui.exe --close-desktop` 关掉大屏。大屏自己的应用库不列这个条目（`composables/bigScreenEntry.js` 按命令认，不按名字）。
- 面板里：概览的「打开大屏」磁贴（默认不在智能列表里，可固定）、搜索「大屏」、帮助里的一行、面板设置里的一行，都走 `open_desktop` 命令，与托盘「打开桌面 UI」是同一条路。
- Web UI 在主机本机（Windows）上有同样的磁贴和搜索动作，经 `/api/tray/action` 的 `open_desktop`；别处没有，不是置灰。

页面（设备、串流、工具、设置）都放在 `components/DeskPage.vue` 里：一个标题加内容，按 `useDeskScale` 放大 1.25 倍（再随主题编辑器的字号）。

## 外观与颜色

- 浅色、深色、跟随系统由 foundation-ui 的外观决定；`DesktopApp.vue` 用 `registerModeHandler` 把结果写到 `<html>` 的 `color-scheme`，`跟随系统` 时监听系统变化。
- 壁纸（主题编辑器设置）只提供种子色：`useDynamicColor` 用 HCT 生成色阶，对比度由结构保证。
- `composables/materialBridge.js` 把 `--md-sys-color-*` 推给少数仍读 `--fd-*` 的代码（主题编辑器的取值、窗口字号），新样式不依赖 `--fd-*`。
- 动画：弹簧缓动取自 foundation-ui，`跟随系统 / 完整 / 减少` 由 `.m-app` 的规则统一处理。弹层、屏幕键盘、启动页必须在 `.m-app` 里面，所以 `DesktopApp.vue` 的根就是 `.m-app`。

## 手柄与焦点

- `useGamepad` 读手柄、`useGamepadActions` 让每个页面声明自己响应哪些键，底部 `GamepadLegend` 只显示真的会响应的键。
- `useFocusNav` 是焦点作用域栈：弹层打开后，方向键和焦点陷阱只在栈顶元素里，关闭后焦点回到打开它的元素。
- B 键按层退出：屏幕键盘 > 确认框 > 启动覆盖 > 主题编辑器 > 启动助手 > 页面内弹层 > 光标模式 > 回到应用库 > 退出确认。
- 文本框（`md-outlined-text-field` / `md-filled-text-field`，由 `isMaterialTextField` 识别）按 A 打开屏幕键盘 `OnScreenKeyboard`，不是点击。
- `useGamepadCursor` 用左摇杆驱动指针，覆盖方向键到不了的密集表单。

## 按键提示与手柄类型

提示条、屏幕键盘、确认框和设置里带按键的句子，都按手里那台手柄印着的符号显示，换手柄即时跟着变：

| 类型 | 面键（下 右 左 上） | 肩键 / 扳机 |
|---|---|---|
| `xbox` | A B X Y | LB RB / LT RT |
| `ps`（DualSense 与 DualShock 4 一样） | ✕ ○ □ △ | L1 R1 / L2 R2 |
| `nintendo` | B A Y X | L R / ZL ZR |
| `generic`（认不出且映射不标准的手柄） | 用位置说话（下 右 左 上） | L1 R1 / L2 R2 |

- `composables/padType.js` 决定类型：Gamepad API 的 `id` 与 `mapping`；串流时（Moonlight 连着这台主机）大屏看到的是主机的虚拟手柄，所以 `useHostPadType` 每 5 秒读 `/api/runtime/sessions` 的 `gamepad_type`（`x360` → xbox，`ds4`/`ds5` → ps），有报告就以它为准，没有就回到本机手柄。
- `composables/padGlyphs.js` 是唯一的映射：动作（`confirm` `back` `favorite` `search` `menu` `pages` `scroll` `cursor`）→ 哪几颗键 → 这类手柄上印着什么。确认永远是下方那颗面键，返回永远是右边那颗，只变「印着什么」，不变行为（`useGamepad` 仍按标准映射的按钮号）。
- `components/PadGlyph.vue` 用简单形状和 `--md-sys-color-*` 画键（Xbox 画字，PlayStation 画 ✕ ○ □ △ 的形状，generic 画位置），不用任何品牌图；`PadHint.vue` 把句子里的 `{confirm}` `{back}` 记号画成图标。文案里不写键名，写记号。
- 只能放字的地方（`title`、`aria-label`、设置说明）用 `fillHint(text, padType.value)`。

## 开发规则

- 需要按钮、输入、选择、开关、对话框、提示时用 `foundation-ui/vue` 的组件，不写原生 `<button>`/`<input>`/`<select>`/`<textarea>`。唯一的例外是隐藏的原生文件选择器和取色器，它们由一个 `MButton`/`MIconButton` 触发。
- 确认走 `useConfirm`/`ConfirmDialog`（`window.confirm` 手柄点不掉），消息走 `useNotify`。
- 图标用 Material 名字，名字必须在 foundation-ui 的图标集里，`deskIcons.test.js` 会检查。
- 新页面：在 `views/` 建文件，放进 `DesktopApp.vue` 的 `viewMap` 和导航表，在 `i18n/en.js`、`zh.js` 加文案，用 `useGamepadActions` 声明手柄动作。
- 测试：`npm run test:renderer`；构建：`npm run build:renderer`。
