# 右键菜单窗口（超出主窗口的右键）

主窗口里的 Material 右键菜单原来画在 WebView 里面，贴近窗口边缘时被截断或只能向内翻。现在在 Panel 里它是一个独立的无边框透明窗口，可以像系统菜单一样伸出主窗口，
只受当前显示器工作区（不含任务栏）限制。外观仍是 m3e 的 Material 菜单（同一个 `MMenu` 组件、同一套 token、同一个主题和强调色）。

## 组成

| 部分 | 位置 |
| --- | --- |
| 放置算法（物理像素、工作区翻转、多显示器多 DPI） | `src-tauri/src/menu_place.rs`（单元测试在同一文件） |
| 窗口、命令、关闭规则 | `src-tauri/src/ctx_popup.rs` |
| 菜单窗口页面 | `src/renderer/ctxmenu/`（只加载 `foundation-ui/popup`，不加载整个库） |
| 主窗口一侧：把菜单交给原生一侧、收回选择 | `src/renderer/composables/menuPopup.js` |
| 组件、presenter、框架桥、主题快照 | foundation-ui：`src/contextmenu/presenter.js`、`popup.js`、`src/vue/MMenuPopup.vue` |

## 一次右键

1. 主窗口的 `MContextProvider`/`MContextMenu` 先问 presenter：菜单（只含数据，`run` 之类的函数留在主窗口）、主窗口主题快照、点击位置交给 `ctx_popup_open`。
2. Rust 找到指针所在显示器，把菜单窗口（`DWMWA_CLOAK`：渲染但不可见）移到该显示器、放大到工作区，再把菜单发给菜单页。
3. 菜单页画出并量好尺寸，调用 `ctx_popup_ready`；Rust 按 `menu_place` 把窗口定位到指针（右/下放不下就向左/向上翻，仍放不下就贴边，比工作区还高就在菜单内滚动），取消遮蔽并获得焦点。
4. 结束：选择项（返回 `[顶层序号, 子菜单序号…]`）、Esc、点窗口外、焦点离开、主窗口移动/缩放/最小化/隐藏。窗口隐藏，焦点还给主窗口（焦点去了别的程序则不抢），主窗口收到 `ctx-popup:result`，
   由主窗口自己的项目动作运行（含确认对话框）。
5. Web UI 框架里的右键：框架用 `postMessage`（`foundation-menu-open`）请求外层主窗口代为显示，结果同样回到框架。需要框架内的 Web UI 带上新版 foundation-ui（`MContextProvider` 自动安装）。

回退：菜单窗口不存在 / 未加载 / 900 ms 内没回应 / `SUNSHINE_GUI_CTX_POPUP=off` → `ctx_popup_open` 报错，页面用原来的窗口内菜单（m3e `placeMenu`：量、翻转、移位、内部滚动）。浏览器里的 Web UI 永远用这条路。
窗口打开期间主窗口有模态对话框时仍然不出应用右键（`MContextProvider` 的模态规则没变）。

## 真窗口验证步骤（需要有桌面的会话，由控制者执行）

构建出的 exe：`C:\alk-scratch\perfp\product-ctxpop\sunshine-gui.exe`，用受控实例（`runl.ps1`、`SUNSHINE_GUI_ALLOW_PARALLEL_INSTANCE=1`、独立 `WEBVIEW2_USER_DATA_FOLDER`、带 `--remote-debugging-port`）。

1. 把窗口拖到屏幕右下角，在窗口右下角附近右键：菜单应越过窗口边缘显示在屏幕上，不被截断，不盖住任务栏。
2. 副屏不同缩放比（例如 100 % 与 150 %）：在两块屏的边缘各右键一次，菜单大小与位置应和各自屏的缩放一致。
3. 键盘：右键后 ↑/↓/Home/End、Enter 选择；→ 进入子菜单、← 返回；Esc 关闭，焦点回到主窗口。
4. 点菜单外任意处、点任务栏、Alt+Tab：菜单关闭；点到别的程序后焦点留在那个程序。
5. 菜单打开时拖动/最小化主窗口：菜单关闭。
6. 延迟：CDP 读 `window.__ctxPopupTimings`（右键请求到窗口显示，毫秒）；受控实例上也可调用 `ctx_popup_selftest`（`SUNSHINE_GUI_TEST_OFFSCREEN=1` 时，窗口保持遮蔽，不会出现在桌面上），返回 p50/p95。
7. 空闲占用：对比 `SUNSHINE_GUI_CTX_POPUP=off` 与默认启动后，`msedgewebview2.exe` 工作集之差。
