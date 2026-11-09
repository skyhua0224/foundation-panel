# 托盘菜单窗口（右键 / 单击 / 双击托盘图标）

托盘图标原来挂 Tauri 的原生菜单（`tray/menu.rs`），单击打开主窗口。现在三个手势各自由用户在「个性化 › 托盘图标」里选：

| 手势 | 可选 | 默认 |
| --- | --- | --- |
| 右键 | V1 紧凑列表 / V2 状态卡 + 列表 / V3 快捷面板 | V1 |
| 单击 | 打开主窗口 / V1 / V2 / V3 / 没有动作 | 打开主窗口（和以前一样） |
| 双击 | 没有动作 / 打开主窗口 / V1 / V2 / V3 | 没有动作 |
| 始终使用系统原生菜单 | 开 / 关 | 关 |

菜单画在右键菜单已经在用的无边框透明窗口里（`ctx_popup.rs`、`docs/context-menu-window.md`），外观是 m3e 的 `MTrayMenu`（`foundation-ui/src/tray/`、`docs/tray-menu.md`）。

## 组成

| 部分 | 位置 |
| --- | --- |
| 手势 → 动作、单击/双击的门（`ClickGate`）、设置文件 `tray-prefs.json` | `src-tauri/src/tray_prefs.rs`（纯逻辑，有单元测试） |
| 托盘图标事件、状态数据、条目标签、原生菜单回退 | `src-tauri/src/tray/popup_input.rs` |
| 窗口：打开、定位、关闭规则 | `src-tauri/src/ctx_popup.rs`（`open_tray`、`pick_tray`，和右键菜单共用同一个窗口和同一套关闭规则） |
| 放在图标旁边（任务栏在哪一边、工作区、多显示器多 DPI） | `src-tauri/src/menu_place.rs`（`taskbar_edge`、`place_beside_icon`，单元测试每个边） |
| 窗口里的页面 | `src/renderer/ctxmenu/main.js`（一个窗口两个页面：右键菜单 `MMenuPopup`、托盘菜单 `MTrayPopup`） |
| 窗口页的传输 | `src/renderer/composables/menuPopupTransport.js` `trayTransport` |
| 主窗口把主题告诉原生一侧 | `src/renderer/composables/trayTheme.js`（托盘菜单打开时可能没有主窗口可问） |
| 设置页 | `src/renderer/components/m3e/AppearancePage.vue` 里的 `MTraySettings` |

## 一次点击

1. 图标回调（`tray.rs` → `popup_input::on_icon_event`）：左键抬起 / 双击 / 右键抬起。`ClickGate` 决定：立刻执行、等一个系统双击间隔再执行（只有给双击设了动作时，单击才等；双击来了就取消它）、忽略（双击的第二次抬起不算单击）。
2. 动作是「打开主窗口」就走原来的 `open_main_panel_from_tray`；是菜单就组装状态（见下）并调用 `ctx_popup::open_tray`。
3. `open_tray`：按图标矩形选显示器，按「工作区比显示器小的那一边」得到任务栏在哪一边（任务栏自动隐藏时按图标离哪边近），把窗口（遮蔽着）放大到工作区，把状态发给页面（`tray-popup:open`）。
4. 页面画好卡片、量出尺寸，`ctx_popup_ready`；Rust 用 `place_beside_icon` 放置：下方任务栏→图标上方、水平居中于图标并夹在工作区内；上方任务栏→任务栏下方；左/右任务栏→贴着任务栏、下沿对齐图标；比工作区高就在卡内滚动。窗口取消遮蔽并取得焦点。
5. 换页（V1/V2 下钻、V3 翻页）使卡片变高：变高时先给窗口再动画，变矮时动画结束后再缩窗口（`MTrayPopup` 340 ms）；卡片贴着任务栏一侧，向远离任务栏的方向长。
6. 选一项：页面调用 `tray_popup_act(token, id)`，窗口先关，然后 `handle_tray_menu_event(id)`（原生菜单今天用的同一个处理函数，37 个 id 一个不少，行为不变；确认对话框等照旧）。
7. 关闭规则与右键菜单窗口相同：Esc、点窗口外、焦点离开、被另一个菜单替换。再点一次托盘图标等于关闭（失去焦点那次关闭之后 350 ms 内的抬起不再打开）。

## 状态（菜单里的真数据）

`popup_input::build_state`：核心连接 / 主机状态（`TRAY_RUNTIME_STATE`）、串流（`get_active_sessions`：设备、应用、码率 Mbps、帧率；0.45 秒内没回来就用核心托盘状态里的名字，码率显示「—」）、基地虚拟显示器（是否已创建、冷却、保留、无头；模式取第一路串流的分辨率@帧率，没有串流时不显示模式）、
更新（上一次检查的结果，`update::last_seen_update`）、DS5 已连接（`dualsense_get_status` 的 `active`）、通知（核心的通知文案）、开机自启 / 阻止休眠 / 用户模式的当前值、RTSS 是否安装、是否 beta/debug 构建。条目的文字是原生菜单的 `TrayStrings`（中/英/日），所以两种菜单用词一致；菜单打开期间状态变化会推给页面（`tray-popup:state`）。

## V3 的滑块（分辨率 / 刷新率 / 缩放 / 码率上限）

页面松开滑块时调用 `tray_popup_set(token, key, value)`（`tray.rs` → `popup_input::set`），菜单不关；命令等到值设好、把设好之后的状态推给页面（`tray-popup:state`）才返回，失败返回原因，页面把它写在滑块下面。`state.sliders` 由 `popup_input::slider_state` 组装（`tray/display_sliders.rs`）：

| key | 值 | 做法 |
| --- | --- | --- |
| `resolution` | `"2560x1440"` | 目标显示器 `ChangeDisplaySettingsExW`（先 `CDS_TEST`，再 `CDS_UPDATEREGISTRY`，像系统设置一样），保留当前刷新率，没有就取最近的 |
| `refresh` | Hz | 同上，当前分辨率下的刷新率 |
| `dpi` | 100–300，25 的倍数 | `system::set_desktop_dpi`（SetDpi.exe 启动后不等待；约 1.2 秒后重新读状态）；SetDpi 不接受显示器参数，所以只在目标是主显示器时给这条 |
| `bitrate` | Kbps，0 = 自动 | 读完整配置 → 改 `max_bitrate` → `post_sunshine_config`（`/api/config` 整表替换，和 `save_microphone_backend` 同一把 `DEVICE_CONFIG_LOCK`） |

分辨率和刷新率之前面板里没有调用、主机也没有端点（主机的 display_device 只在串流开始和结束时改模式），所以这两个是面板自己用 Win32 做的，没有新增主机端点。
目标显示器（`display_sliders::choose`）：串流中——配置的 `output_name` 是基地虚拟显示器（`ZakoHDR`）就是它，否则是当前模式等于串流模式的物理显示器（刷新率最近的优先），都不是则主显示器；没有串流——基地虚拟显示器已创建就是它；否则没有目标。基地虚拟显示器在 Win32 里靠显示器名 `Zako HDR` 认出（主机 `ZAKO_NAME`）。
每次打开菜单要枚举一遍显示器的全部模式（`spawn_blocking`，1.5 秒超时，超时则不给显示器滑块）。

## 回退到原生菜单

图标上挂着原生菜单时，Windows 会在右键时自己弹它，所以使用窗口菜单期间图标**不挂**菜单（`set_menu(None)`）。原生菜单在这些情况下回到图标上（`popup_input::apply_menu_mode`）：菜单窗口还没加载好（启动后约 1.5 秒内）、环境变量 `SUNSHINE_GUI_CTX_POPUP=off`、设置里「始终使用系统原生菜单」、窗口连续两次打不开（改设置会重置）。
原生菜单不能被左键「弹出」：此时左键/双击被设成菜单的话，改为打开主窗口（和以前一样）。

## 真窗口验证步骤（需要有桌面的会话，由控制者执行；本轮没有做）

构建出的 exe：`C:\alk-scratch\perfp\product-traymenu\sunshine-gui.exe`。受控实例：`runl.ps1`、`SUNSHINE_GUI_ALLOW_PARALLEL_INSTANCE=1`、独立 `WEBVIEW2_USER_DATA_FOLDER`、`--remote-debugging-port`；**不要装，不要在 owner 的交互会话里运行**。

1. 右键托盘图标：默认 V1 出现在图标上方、离任务栏 12 px、右侧不越过屏幕；浅色/深色与面板当前一致；外观与 `…/20261109-tray-impl/02-after-*` 一致。
2. 关闭规则：Esc、点菜单外、点任务栏、Alt+Tab 都关闭；菜单开着再点一次托盘图标是关闭（不会立刻再开）。
3. 键盘：↑/↓/Home/End 在按钮间移动，Enter 选择；V1/V2 ←/Backspace 返回、Esc 先返回再关闭；V3 ←/→ 翻页。
4. 个性化 › 托盘图标：右键换成 V2、V3 后立即生效（不用重启）；单击改成 V3、双击改成「打开主窗口」：单击约半秒后才出菜单，双击直接开主窗口且不出菜单；双击设成「没有动作」时单击立刻执行。
5. 37 个条目逐个点（每个变体各一遍，含有确认对话框的几项）：行为与原生菜单一致；服务断开时的「重新启动 Sunshine 服务」、有通知时的通知行、装了 RTSS 才有的 RTSS 行。
6. 任务栏位置：Windows 11 只有底部；上/左/右用 Windows 10 或 ExplorerPatcher 验证。副屏不同缩放（100 % 与 150 %）：图标在副屏任务栏时，菜单出现在副屏的该图标旁，大小与缩放一致。任务栏自动隐藏、图标在溢出区（隐藏图标）时的位置。
7. 回退：打开「始终使用系统原生菜单」→ 右键是原生菜单；关掉 → 回到窗口菜单。`SUNSHINE_GUI_CTX_POPUP=off` 启动 → 原生菜单。
8. 空闲占用：对比 `SUNSHINE_GUI_CTX_POPUP=off` 与默认启动后 `msedgewebview2.exe` 工作集之差（窗口本来就为右键菜单创建，托盘菜单没有增加窗口）。
9. 应用只在托盘、没有开过主窗口时：右键菜单的颜色是上次主窗口的主题（`tray-theme.json`）。

## 没有验证的

真窗口里的位置、焦点、动画、各边任务栏、150 % 缩放、英文/日文的宽度；滑块在真机上的 `ChangeDisplaySettingsExW`（物理屏、基地虚拟显示器；串流中改分辨率主机怎么跟）、`Zako HDR` 显示器名的识别、改分辨率时菜单窗口会不会被显示变化关掉（组件按 360 px 内截断，没有逐页看）、`tray-icon` 在溢出区给出的图标矩形。
