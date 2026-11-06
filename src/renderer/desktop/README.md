# 大屏桌面（Foundation Desktop）

沙发上用手柄操作的 10-foot 界面，Tauri + Vue 3。外观是 Material 3 Expressive：控件、对话框、提示都来自 foundation-ui（`foundation-ui/vue` 的 `M*` 组件），颜色只取 `--md-sys-color-*`，动效只取 `--m-spring-*`。这里不再有自绘的按钮、输入框、下拉框。

## 目录结构

```
desktop/
├── DesktopApp.vue         # 外壳：导航栏、页面切换、弹层、手柄路由、浅色/深色/跟随系统
├── main.js / index.html   # 入口
├── desktop.less           # 全局：10-foot 尺度、手柄焦点环、共用块
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
| `devices` | `views/DevicesView.vue` | 配对（PIN、二维码）、已配对设备、重命名、取消配对 |
| `stream` | `views/StreamView.vue` | 编解码器、码率、HDR、显示与采集、启动方式、虚拟显示器、虚拟鼠标 |
| `tools` | `views/ToolsView.vue` | 嵌入工具、GPU/编码器/网络/防火墙诊断、重启显卡驱动 |
| `settings` | `views/SettingsView.vue` | 外观、启动、大屏、启动助手、文件共享、通知、高级、桌宠、关于 |

底部导航：语言（EN/中文）、主题编辑器、设置、退出。`desktop-navigate` 事件、托盘 `open_desktop`、`--desktop`/`-d` 仍然进入这里。

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

## 开发规则

- 需要按钮、输入、选择、开关、对话框、提示时用 `foundation-ui/vue` 的组件，不写原生 `<button>`/`<input>`/`<select>`/`<textarea>`。唯一的例外是隐藏的原生文件选择器和取色器，它们由一个 `MButton`/`MIconButton` 触发。
- 确认走 `useConfirm`/`ConfirmDialog`（`window.confirm` 手柄点不掉），消息走 `useNotify`。
- 图标用 Material 名字，名字必须在 foundation-ui 的图标集里，`deskIcons.test.js` 会检查。
- 新页面：在 `views/` 建文件，放进 `DesktopApp.vue` 的 `viewMap` 和导航表，在 `i18n/en.js`、`zh.js` 加文案，用 `useGamepadActions` 声明手柄动作。
- 测试：`npm run test:renderer`；构建：`npm run build:renderer`。
