# USB/IP 主控制器残留：判定、清理与重启后继续

旧版 usbip-win2（0.9.7.x）的卸载程序会在系统里留下主控制器设备节点（硬件 ID `ROOT\USBIP_WIN2\UDE`，实例在 `ROOT\USB` 或 `ROOT\ALKAIDUSB` 下）。新版装上后，旧节点要么拖着「就绪」不放，要么清理时报「无法移除残留的 USB/IP 主控制器设备」。本文记录控制面板怎么判定、怎么清、清不掉时怎么说。依据见 `docs/authority/source-index.tsv` 里 `usbip-clean-*` 与 `panel-ms-webview2-*` 各行。

## 以前为什么清不掉

0. **旧版自己就要求重启。** 0.9.7.7 的 Inno 脚本里 `UninstallNeedRestart` 恒为真，卸载时调用 `devnode.exe remove`，它用 `DiUninstallDevice` 删除控制器节点并在 `NeedReboot` 时提醒重启。静默卸载带 `/NORESTART`（控制面板和宿主助手都是这样调用的），节点于是留在设备树里等重启；这时再装 0.9.8.1，会出现第二个控制器。
1. **清理根本没动节点。** 控制面板在宿主的安装助手存在时，把卸载交给助手；助手对「没有任何登记的安装」直接返回成功，不碰节点。旧版卸载程序恰好把登记删掉、把节点留下，于是清理「成功」，状态仍报残留，界面只剩一句「无法移除」。
2. **删除节点的调用本身不合规范。** 原来只调 `CM_Locate_DevNode(PHANTOM)` 加 `CM_Uninstall_DevNode`。官方对这个函数的说明：它删除设备的全部持久状态，**不发 `IRP_MN_QUERY_REMOVE_DEVICE`、不经类安装程序**；桌面应用应改用 `SetupDiCallClassInstaller(DIF_REMOVE)` 或 `DiUninstallDevice`；若一定要用，顺序是先 `CM_Get_DevNode_Status`（设备在不在），在就先 `CM_Query_And_Remove_SubTree`，最后才 `CM_Uninstall_DevNode`。对仍在设备树里的节点直接调它，设备并没有被停掉。
3. **失败原因被吞掉。** 只返回 USBIP-CLEAN-002，`CR_*` 返回码不进界面也不进诊断。
4. **旧节点被新驱动接管。** 实测：0.9.7.7 的节点 `ROOT\USB\0000` 留着，装上 0.9.8.1 后新驱动把它和新建的 `0001` 都启动了，两个存活接口，`usbip.exe` 报 `Multiple instances of VHCI device interface found`，状态是「已安装、版本对、就绪 ⚠、残留 ⚠」。

## 现在的判定（`usbip/nodes.rs`）

`usbip.exe` 只在「存在的 VHCI 接口多于一个」时报 `Multiple instances of VHCI device interface found`。所以只有**在设备树里**的节点才可能成为第二个控制器：

| 状态 | 判定 |
| --- | --- |
| 有存活接口，旁边还有另一个启用且在树里的节点，或有两个以上存活接口 | 阻挡，类型 `stale`：保留一个存活控制器，移除其余，不重装 |
| 没有存活接口，但有节点在树里 | 阻挡，类型 `broken`：统一清理后重装 |
| 只剩不在树里的幽灵节点；被禁用的另一个枚举器的旧节点 | 不阻挡；下次安装或修复时顺手清掉 |

## 清理（`usbip/devnode.rs`、`usbip/device.rs`）

- 先 `DiUninstallDevice`（`DIF_REMOVE`，会停掉运行中的设备，需要重启时用 `NeedReboot` 告诉调用方）。失败再走官方文档给的 `CM_*` 序列，并先打开 `SeLoadDriverPrivilege`（`CM_Query_And_Remove_SubTree` 的调用要求）。
- 否决（`CR_REMOVE_VETOED`）时取出否决类型和否决者名称；拒绝访问分 `CR_ACCESS_DENIED` 与 Win32 `ERROR_ACCESS_DENIED`。
- 清理始终在提权的助手里进行（没提权就弹一次 UAC），所以拒绝访问是真拒绝，不会静默重试。
- 异步拆除：列表里还在的节点在 30 次、每次 1 秒的预算内重问；Windows 接受但要等重启的节点单独记下。
- 安装和「清理旧版」之后，助手会把旧版留在存活控制器旁边的节点清掉，不让人看到「已安装、未就绪」。

返回给人的有三种具体说法：

| 代码 | 含义 | 下一步 |
| --- | --- | --- |
| `USBIP-CLEAN-006` | Windows 已接受，重启后完成 | 重启（不是关机）；重启后控制面板自动继续一次 |
| `USBIP-CLEAN-007` | 拒绝访问 | 以管理员运行控制面板重试；仍不行在设备管理器手动卸载 |
| `USBIP-CLEAN-008` | 被占用（有否决者） | 结束串流、释放已挂载设备、关掉用 USB/IP 的程序再试；仍不行先重启 |

返回码（`cr=`、`win32=`、`veto_type=`、`veto_name=`）写进错误行，并保存在登录用户自己的清理记录里，诊断导出和「查看原始诊断」都显示它。

## 重启后继续

Windows 接受但要重启才完成的清理，会在用户自己的记录里留一个重启标记。下一次启动控制面板时，如果系统已重启过、残留还在，就自动继续一次清理（弹一次 UAC）；只继续一次，再失败就给出手动步骤，不会循环。标记在残留消失后自动删除。快速启动（混合关机）下「关机再开机」不是重启，所以文案要求选「重启」。

## 一键导出诊断

「导出诊断」写出一个文本文件并在资源管理器里选中：Windows 版本与快速启动开关、控制面板版本、面板计算的状态、存活的 VHCI 接口、两个根枚举器下的节点（是否在树里、是否禁用）、三个驱动服务及 `sc query`、已登记的安装、`pnputil /enum-drivers` 中含 usbip 的驱动包、上次清理的返回，以及 `setupapi.dev.log` 里与 usbip 相关的行（`DiUninstallDevice` 文档指明移除的细节在这份日志里）。不含串流内容和账号数据。

## 窗口白屏自愈

WebView2 的渲染进程退出（`RenderProcessExited`）后，官方要求应用自己恢复：`Reload`，或关闭并重建控件；GPU 进程退出（`GpuProcessExited`）可自动恢复，但官方提示反复退出要查原因。控制面板的做法（`crash/recover.rs`、`crash/fallback.rs`）：

1. 渲染进程退出：重新加载页面。
2. 五分钟内第三次退出，或 GPU 进程五分钟内三次退出：关掉 GPU 加速重启一次（WebView2 浏览器标志 `--disable-gpu`，官方说明为「关闭 GPU 硬件加速」；官方同时说明这类标志用于排查、不建议长期用于生产，所以只作最后一步）。这个选择保存十四天，WebView2 运行时更新后立即作废；设置环境变量 `SUNSHINE_PANEL_GPU=1` 可强制保持 GPU 加速。
3. 关掉 GPU 加速仍然反复退出：弹一个原生对话框（窗口自己已经没法说话），告诉人把 `panel.log`、Windows 版本和显卡型号发过来，并提供打开日志文件夹的按钮。
4. 日志现在记录每次失败的原因（崩溃、内存不足、启动失败……）和退出码。

## 需要用户提供什么

Windows 版本（Win+R 输入 `winver`）、显卡型号与驱动版本、`panel.log`、USB/IP「导出诊断」生成的文件。
