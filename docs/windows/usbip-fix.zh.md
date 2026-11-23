# USB/IP 一键修复：状态清单

此文件由 `scripts/usbip-fix-inventory-doc.mjs` 从 `src/renderer/composables/usbipFixInventory.js` 生成，不要手改。清单里每一行都有测试（`usbipFix.test.js`）：能由状态判定的行，测试检查修复计划；只在修复过程中出现的行，测试检查它的错误给出哪份处理步骤。

一个按钮、一条流程：读取状态并判定 → 用大白话列出会结束、释放、移除的东西（只问一次）→ 结束串流 → 按顺序修复（释放已挂载端口、清理共享传输或只移除多余控制器、替换用户自己装的完整版、安装配套的 USB/IP）→ 检查结果（传输就绪，且 DualSense 原生通道不再报告 USB/IP 问题）→ 需要重启时说明并提供重启，重启后继续剩下的步骤。软件无法处理的情况（系统策略拦截、根集线器回滚、系统盘在 USB 上、用户自己的安装不允许替换）给出一步一步的处理说明，并保留原始诊断。

| 行 | 代码或状态 | 怎么发现 | 修复做什么 | 确认里写什么 | 重启 | 怎么验证 |
| --- | --- | --- | --- | --- | --- | --- |
| `not-installed` | `USBIP-SETUP-001` | 状态：未安装、无残留 | 安装配套的 USB/IP（主机附带的受保护安装脚本，含根集线器检查与自动回滚） | 安装 USB/IP；需要管理员授权 | 安装器可能建议重启 | 读取传输状态：就绪、无残留、无错误码；再读 DualSense 原生通道状态，不得是 usbip_* 状态 |
| `install-resume` | `install_resume` | 上次安装等待重启，系统已重启，安装还没完成 | 继续安装 | 重启前已确认；重启后自动继续 | 否 | 读取传输状态：就绪、无残留、无错误码；再读 DualSense 原生通道状态，不得是 usbip_* 状态 |
| `damaged-install` | `USBIP-SETUP-002` | 有登记但 usbip.exe 或安装目录缺失、不安全 | 统一清理后重新安装 | 清理共享 USB/IP 并重装；DualSense 原生通道与 USB 透传暂不可用 | 清理可能需要重启 | 读取传输状态：就绪、无残留、无错误码；再读 DualSense 原生通道状态，不得是 usbip_* 状态 |
| `old-version` | `USBIP-SETUP-003`、`USBIP-SETUP-008` | 已安装的版本不是配套版本（面板自己装的） | 统一清理后安装配套版本 | 清理共享 USB/IP 并重装 | 清理可能需要重启 | 读取传输状态：就绪、无残留、无错误码；再读 DualSense 原生通道状态，不得是 usbip_* 状态 |
| `mismatch-hash-abi` | `USBIP-EXEC-006` | usbip.exe 或内核驱动的 SHA-256 与固定组合不一致，或 usbip.exe 报 ABI mismatch | 统一清理后安装配套版本 | 清理共享 USB/IP 并重装 | 清理可能需要重启 | 读取传输状态：就绪、无残留、无错误码；再读 DualSense 原生通道状态，不得是 usbip_* 状态 |
| `residual-broken` | `USBIP-CLEAN-002` | 设备树里有控制器节点，但没有存活的控制器 | 统一清理（卸载程序加节点清扫）后重装 | 清理共享 USB/IP 并重装 | Windows 接受但要重启才完成时（USBIP-CLEAN-006） | 读取传输状态：就绪、无残留、无错误码；再读 DualSense 原生通道状态，不得是 usbip_* 状态 |
| `stale-controller` | `Multiple instances of VHCI device interface found` | 传输可用，但旧版留下了多余的控制器 | 只移除多余的控制器，保留存活的那个，不重装 | 移除多余控制器 | 节点要等重启才删时 | 读取传输状态：就绪、无残留、无错误码；再读 DualSense 原生通道状态，不得是 usbip_* 状态 |
| `attached-ports` | `USBIP-CLEAN-008`、`USBIP-REPLACE-005` | 要清理或替换时仍有挂载的 USB 设备（端口没释放） | 先释放所有已挂载的端口，再继续 | 列出会释放的设备数量 | 否 | 读取传输状态：就绪、无残留、无错误码；再读 DualSense 原生通道状态，不得是 usbip_* 状态 |
| `unsigned-driver` | `usbip_unsigned`、`signature unsigned/invalid` | 已登记驱动的签名无效或缺失（DualSense 状态读出） | 统一清理后重装官方签名版本 | 清理共享 USB/IP 并重装 | 清理可能需要重启 | 读取传输状态：就绪、无残留、无错误码；再读 DualSense 原生通道状态，不得是 usbip_* 状态 |
| `cleanup-resume` | `cleanup_resume` | 清理等重启，系统已重启，残留还在 | 继续清理，再安装 | 重启前已确认；重启后自动继续 | 否 | 读取传输状态：就绪、无残留、无错误码；再读 DualSense 原生通道状态，不得是 usbip_* 状态 |
| `tool-failed` | `USBIP-EXEC-001`、`USBIP-EXEC-002`、`USBIP-EXEC-003`、`USBIP-EXEC-007` | usbip.exe 起不来、超时、输出超限或本机操作失败，传输未就绪 | 统一清理后重装 | 清理共享 USB/IP 并重装 | 清理可能需要重启 | 读取传输状态：就绪、无残留、无错误码；再读 DualSense 原生通道状态，不得是 usbip_* 状态 |
| `external-incompatible-replaceable` | `USBIP-SETUP-008`、`USBIP-REPLACE-*` | 外部安装的完整 usbip-win2（含 GUI/SDK）版本或驱动不兼容，主机助手判定可替换 | 经确认后用主机助手替换为精简配套版本（失败自动回滚） | 明确写出会卸载现有 usbip-win2 及其图形界面和 SDK | 否 | 读取传输状态：就绪、无残留、无错误码；再读 DualSense 原生通道状态，不得是 usbip_* 状态 |
| `external-incompatible-manual` | `USBIP-SETUP-008`、`USBIP-REPLACE-001` | 外部安装不兼容，且主机助手不允许替换 | 不改动；给出在「已安装的应用」里手动卸载的步骤 | 无（不会改动） | 用户手动卸载后需要 | 不适用 |
| `external-stale-controller` | `Multiple instances of VHCI device interface found` | 外部安装可用，但旧版留下了多余的控制器 | 只移除多余的控制器（外部安装本身保留） | 移除多余控制器 | 节点要等重启才删时 | 读取传输状态：就绪、无残留、无错误码；再读 DualSense 原生通道状态，不得是 usbip_* 状态 |
| `external-compatible-healthy` | — | 外部安装的 0.9.8.1，工具和驱动哈希一致，就绪 | 不需要；保留用户自己的安装 | 无 | 否 | 不适用 |
| `pending-restart-cleanup` | `USBIP-CLEAN-006`、`cleanup_pending_restart` | 清理已被 Windows 接受，重启后才完成 | 说明原因并提供立即重启；同时保存计划（清理、检查），重启后继续；清理完成后要装的 USB/IP 要重新确认 | 确认重启；重启后若还要安装，再问一次 | 需要 | 重启后继续并检查 |
| `pending-restart-install` | `USBIP-SETUP-015`、`install_pending_restart` | Windows 有待完成的重启，安装不能完成 | 说明原因并提供立即重启；同时保存计划（安装、检查），重启后自动继续安装 | 确认重启；重启前的安装请求已经是用户的决定 | 需要 | 重启后继续并检查 |
| `reboot-recommended` | `reboot_recommended` | 安装器建议重启（退出码 3010） | 说明原因并提供立即重启；同时保存计划（检查），重启后做检查 | 确认重启 | 建议 | 重启后检查 |
| `healthy` | — | 传输就绪，无残留，无错误码，签名有效 | 不需要 | 无 | 否 | 不适用 |
| `unsupported-os` | `USBIP-SETUP-004` | 不是 Windows | 不做任何事 | 无 | 否 | 不适用 |
| `unexplained` | — | 已安装但未就绪，没有错误码，没有重启建议 | 不改动；打开原始诊断，建议导出诊断 | 无 | 否 | 不适用 |
| `stream-unknown` | `USBIP-SETUP-006` | 主机读不到会话（Sunshine 没有响应）：不知道有没有串流在用传输 | 读不到不等于没有串流：有限时间内重试，仍读不到就停下，什么都不动；说明要先启动 Sunshine | 无 | 否 | 不适用 |
| `stream-active` | `USBIP-SETUP-006`、`USBIP-CLEAN-001` | 有串流在用传输 | 经主机接口结束串流并等它真正结束；读不到就一直等到时限，不继续 | 写明会结束串流 | 否 | 主机返回空的会话列表 |
| `driver-op-busy` | `USBIP-SETUP-018` | 另一项驱动操作（VB-CABLE、虚拟鼠标等）还在进行 | 不开始，提示稍后重试 | 无 | 否 | 不适用 |
| `uac-cancelled` | `USBIP-UAC-001`、`USBIP-UAC-002`、`USBIP-UAC-003` | 管理员授权被取消、助手连不上或超时 | 停在当前步骤，提示重试 | 无 | 否 | 不适用 |
| `root-hub-rolled-back` | `USBIP-SETUP-011` | 安装时 USB 根集线器检查没通过（AMD 等 USB 3 控制器上出现过），安装器已自动回滚 | 不再重试；给出更新芯片组驱动、重启、再修复的步骤 | 无 | 步骤里要求 | 不适用 |
| `root-hub-rollback-incomplete` | `USBIP-SETUP-012`、`USBIP-REPLACE-003` | 回滚后仍有 USB 根集线器没恢复 | 不再重试；给出重启、检查设备管理器、导出诊断的步骤 | 无 | 步骤里要求 | 不适用 |
| `usb-boot-disk` | `USBIP-SETUP-013` | Windows 从 USB 磁盘启动，安装会断开系统盘 | 不安装；给出说明 | 无 | 否 | 不适用 |
| `guard-could-not-run` | `USBIP-SETUP-014` | 根集线器保护程序没能运行 | 不安装；给出重启后重试的步骤 | 无 | 步骤里要求 | 不适用 |
| `driver-policy-blocked` | `USBIP-SETUP-016` | 内存完整性（HVCI）、易受攻击驱动阻止列表、签名校验或 CM_PROB 48/52/39 拦下了驱动 | 软件不能绕过；给出关闭内存完整性、重启、再修复的步骤 | 无 | 步骤里要求 | 不适用 |
| `smart-app-control` | `USBIP-SETUP-017` | 智能应用控制或应用控制策略拦下了安装程序（0x800711C7 / 4551） | 软件不能绕过；给出关闭智能应用控制的步骤 | 无 | 否 | 不适用 |
| `removal-access-denied` | `USBIP-CLEAN-007` | Windows 拒绝移除主控制器（CR_ACCESS_DENIED / ERROR_ACCESS_DENIED） | 给出以管理员运行、设备管理器手动卸载的步骤 | 无 | 步骤里要求 | 不适用 |
| `removal-in-use` | `USBIP-CLEAN-008` | 控制器被占用（否决者在错误行里） | 已先结束串流并释放端口；仍占用时提示关闭使用 USB/IP 的程序或先重启 | 无 | 可能 | 不适用 |
| `installer-generic-failure` | `USBIP-SETUP-010`、`USBIP-SETUP-005`、`USBIP-REPLACE-004`、`USBIP-CLEAN-001`、`USBIP-CLEAN-003`、`USBIP-CLEAN-004` | 安装器、卸载程序或服务清理失败，原因不属于以上任何一种 | 给出重启后重试并导出诊断的步骤，原始诊断里有安装器与 setupapi 摘录 | 无 | 步骤里要求 | 不适用 |
| `replace-rolled-back` | `USBIP-REPLACE-002` | 替换失败并已回滚；根集线器正常，USB/IP 当前未安装 | 状态变为未安装；再点一键修复就是安装 | 无 | 否 | 不适用 |
| `verify-failed` | `USBIP-VERIFY-001`、`USBIP-VERIFY-002` | 步骤都完成了，但传输仍未就绪，或 DualSense 原生通道仍报告 USB/IP 问题 | 不宣称已恢复；给出重启后重试并导出诊断的步骤 | 无 | 步骤里要求 | 不适用 |
| `windows-10` | — | 面板里没有 Windows 10 专用的分支：版本差异由同一条路径处理（挂起重启、快速启动下要选“重启”而不是“关机”、驱动策略拦截）；智能应用控制只存在于 Windows 11 | 走与 Windows 11 相同的流程和步骤 | 同上 | 同上 | 读取传输状态：就绪、无残留、无错误码；再读 DualSense 原生通道状态，不得是 usbip_* 状态 |
| `host-not-running` | `host_unreachable` | 修复后 Sunshine 没有响应，DualSense 原生通道无法检查 | 传输检查通过则宣布传输已恢复，并说明要先启动 Sunshine | 无 | 否 | 只检查传输 |
