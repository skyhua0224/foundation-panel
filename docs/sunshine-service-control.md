# Sunshine 服务控制（SunshineService）

面板对 `SunshineService` 的读取和修改都走这里描述的路径。代码在 `src-tauri/src/sunshine_service/`。

## 为什么改

旧做法（`desktop_settings.rs` 的 `configure_sunshine_service_start`）先把 `sc.exe config … start=` 写进
`%TEMP%\sunshine-set-service-start.bat`，再用 `bat_runner::run_elevated` 提权执行。`%TEMP%` 归当前用户所有：
同一用户下的任何进程都可以在 UAC 弹窗之前换掉这个脚本，然后脚本以管理员身份运行（TOCTOU）。
托盘里把服务改成手动也没有任何确认，远程用户一点就会在下次重启后连不上。

## 现在的做法

### 读（不需要提权）

| 读什么 | API | 说明 |
|---|---|---|
| 启动类型 | `QueryServiceConfigW` → `dwStartType` | 2 自动、3 手动、4 禁用；0/1 是驱动专用，出现即报错 |
| 延迟自动 | `QueryServiceConfig2W(SERVICE_CONFIG_DELAYED_AUTO_START_INFO)` | 只在 `dwStartType = 2` 时有意义 |
| 服务程序路径 | `QUERY_SERVICE_CONFIGW.lpBinaryPathName` | 用来确认服务属于本安装目录，取代读注册表 `ImagePath` |

服务默认 DACL 允许交互用户 `SERVICE_QUERY_CONFIG` / `SERVICE_QUERY_STATUS`，所以这些读取不需要提权。

### 写（加固的提权路径）

1. 被提权的进程是**已安装的面板自己**（`<安装目录>\assets\gui\sunshine-gui.exe`），只带一个固定动词
   `--elevated-sunshine-service` 和一个白名单操作：
   - `set-start-type auto|delayed-auto|manual|disabled`
   - `start` / `stop` / `restart`

   其他任何命令行（大小写不同、多一个参数、`boot`/`system`、`config binPath=` 之类）都会在打开 SCM 句柄之前被拒绝，退出码 2。
2. 父进程（未提权）在请求同意**之前**检查：
   - 面板 exe 位于 `<root>\assets\gui\`，而 SCM 里登记的服务程序是同一个 `<root>\tools\sunshinesvc.exe`；
     便携副本、下载目录里的副本、开发构建都不会提权去改别的安装登记的服务；
   - 当前用户**不能**修改这个 exe（`FILE_WRITE_DATA | DELETE | WRITE_DAC | WRITE_OWNER` 打开被拒绝），
     也**不能**在它所在目录里新建文件。否则同一用户可以替换映像，或在程序目录里放一个 DLL——
     程序目录是 DLL 搜索顺序的第一位，提权进程会加载它。任何不是“拒绝访问”的结果都按“可写”处理（失败即关闭）。
3. 用 `ShellExecuteExW`、动词 `runas`、`SEE_MASK_NOCLOSEPROCESS` 启动上一步确认过的**绝对路径**，等待进程句柄退出。
   不写任何文件，不经过 `cmd.exe` 或 PowerShell。
4. 提权进程再做一次：按同一白名单解析命令行；确认自己确实是管理员令牌；确认服务程序路径属于自己的安装目录；
   然后**直接**调用：
   - 启动类型：`ChangeServiceConfigW(dwStartType)`，其他字段都是 `SERVICE_NO_CHANGE` / `NULL`；
     再 `ChangeServiceConfig2W(SERVICE_CONFIG_DELAYED_AUTO_START_INFO)` 写延迟标志（非自动时清零，避免以后切回自动时继承旧的延迟标志）；
   - 启动：`StartServiceW`，轮询 `QueryServiceStatusEx` 直到 `SERVICE_RUNNING`（30 秒）；
   - 停止：`ControlService(SERVICE_CONTROL_STOP)`，轮询到 `SERVICE_STOPPED`（30 秒）；
   - 重启：停止后启动。
   结果只通过退出码返回：0 成功，2 参数，3 未提权，4 服务属于别的安装，6 超时，`0x20000000 | Win32 错误码` 为 SCM 错误。
5. 父进程用 `QueryServiceConfigW` 读回，确认 SCM 里存的就是目标值。

如果面板本身已经是管理员进程，就不再弹 UAC，在本进程里执行同一段 SCM 代码（包括同安装目录检查）。

剩余风险：同一用户的恶意进程仍然可以自己用 `runas` 启动这个面板并带上白名单参数，但这同样会弹出 UAC，
并且能做的只有白名单里这几件对本服务的事，不比它直接对 `sc.exe` 请求提权更多。

### 手动 / 禁用要确认

`Manual` 和 `Disabled` 会让这台电脑重启后 Sunshine 不再自动运行，远程设备连不上。
后端 `set_start_type_blocking` 要求调用方传 `acknowledge_unreachable_after_reboot = true`，否则返回 `SVC-CONFIRM-001`，
在任何提权之前就拒绝。托盘的“开机自动运行 Sunshine”在关闭时先弹确认框；设置页的启动方式选择同理。

### 托盘和设置页共用一个后端

- 托盘“开机自动运行 Sunshine”：本安装有服务时，只在 `Auto` ↔ `Manual` 之间切换服务启动类型，走上面的后端；
  没有服务时（用户模式安装），改的是面板自己的开机启动项和“面板打开时确保 Sunshine 在运行”。
- 设置页（系统 › Sunshine 服务）的启动方式：同一个 `set_sunshine_service_start_type` 命令。

## 依据的 Microsoft 文档

- ChangeServiceConfigW：https://learn.microsoft.com/windows/win32/api/winsvc/nf-winsvc-changeserviceconfigw
- ChangeServiceConfig2W：https://learn.microsoft.com/windows/win32/api/winsvc/nf-winsvc-changeserviceconfig2w
- SERVICE_DELAYED_AUTO_START_INFO：https://learn.microsoft.com/windows/win32/api/winsvc/ns-winsvc-service_delayed_auto_start_info
- QueryServiceConfigW：https://learn.microsoft.com/windows/win32/api/winsvc/nf-winsvc-queryserviceconfigw
- QueryServiceConfig2W：https://learn.microsoft.com/windows/win32/api/winsvc/nf-winsvc-queryserviceconfig2w
- QueryServiceStatusEx / SERVICE_STATUS_PROCESS：https://learn.microsoft.com/windows/win32/api/winsvc/nf-winsvc-queryservicestatusex
- StartServiceW：https://learn.microsoft.com/windows/win32/api/winsvc/nf-winsvc-startservicew
- ControlService / Stopping a Service：https://learn.microsoft.com/windows/win32/services/stopping-a-service
- Service Security and Access Rights：https://learn.microsoft.com/windows/win32/services/service-security-and-access-rights
- ShellExecuteExW / SHELLEXECUTEINFOW（`runas` 动词、`SEE_MASK_NOCLOSEPROCESS`）：
  https://learn.microsoft.com/windows/win32/api/shellapi/nf-shellapi-shellexecuteexw 、
  https://learn.microsoft.com/windows/win32/api/shellapi/ns-shellapi-shellexecuteinfow
- How User Account Control works：https://learn.microsoft.com/windows/security/application-security/application-control/user-account-control/how-it-works
- Dynamic-Link Library Security（程序目录在搜索顺序第一位；提权进程从可被控制的目录加载 DLL 会导致本地提权）：
  https://learn.microsoft.com/windows/win32/dlls/dynamic-link-library-security

## 重启和运行模式切换（离线页、托盘、首页、搜索）

原来的 `restart_sunshine_service` / `toggle_sunshine_mode` 用 `utils::execute_powershell_command` 以管理员身份跑
`net stop SunshineService; taskkill /IM sunshine.exe /F; net start …` 或 `Start-Process -Verb RunAs`。现在都走同一个提权 helper，
PowerShell 提权入口 `execute_powershell_command` 已删除，用户模式检测也不再解析 `sc query` / `tasklist` 文本。

| 入口 | 做法 |
|---|---|
| 重启（离线页「重启服务」、托盘恢复、首页、搜索动作、待生效更改、旧页面） | 本安装有服务：helper `restart` = 停止服务 → 结束本安装残留的 sunshine.exe（用户模式或孤儿进程，只认 `<安装目录>\sunshine.exe` 这个映像）→ 启动服务。没有服务（便携版）：以当前用户结束并重新启动本安装的 sunshine.exe，不提权 |
| 切到用户模式（离线页「用户模式」、托盘） | helper `user-mode` = 停止服务 → 结束残留 → 由已提权的 helper 直接启动 `<安装目录>\sunshine.exe`（相当于原来的 RunAs） |
| 从用户模式切回服务 | helper `restart` |

模式判断（`status::plan_toggle`）：本安装的 sunshine.exe 在跑、不是服务拉起的、服务也没在运行，就是用户模式，切回服务；否则切到用户模式。

确认：
- 离线页和搜索动作用 `composables/hostRestart.js`：先读 `get_sunshine_service_status` 拿到串流数，弹出与服务页相同的确认框（需要管理员授权、有几路串流会断；切用户模式还会说明重启电脑后不会自动运行）。
- 托盘「以用户模式运行 Sunshine」先弹确认框，写明会先停再启、串流会断、需要管理员授权。
- 首页「重启主机」和待生效更改有各自的确认框（已写明串流会中断 / 列出会结束的会话），确认后带上确认标记。
- 后端：有直播串流时，`restart_sunshine_service` / `toggle_sunshine_mode` 没有 `acknowledge_live_stream` 就拒绝（`SVC-CONFIRM-002`）。
