// Every USB/IP state the panel and the host can detect, and what the one-click repair does about it. The tests walk this table: a row with a
// `status` must plan exactly `expect`, a row with an `error` must give exactly `guide`. docs/windows/usbip-fix.zh.md is made from it
// (node scripts/usbip-fix-inventory-doc.mjs), so the document cannot drift from what the code does.
//
// Fields: id, codes (the USBIP-* codes or host states), detect (how it is found), repair (what the repair does), confirm (what the one question
// names), reboot (whether Windows may have to restart), verify (how success is checked).
const VERIFY = '读取传输状态：就绪、无残留、无错误码；再读 DualSense 原生通道状态，不得是 usbip_* 状态'
const base = { supported: true, installed: true, version_valid: true, ready: true, externally_managed: false, replaceable: false, vhci_residual: false, attached_devices: [] }
const device = { port: 1 }

export const USBIP_FIX_INVENTORY = [
  {
    id: 'not-installed', codes: ['USBIP-SETUP-001'], detect: '状态：未安装、无残留', repair: '安装配套的 USB/IP（主机附带的受保护安装脚本，含根集线器检查与自动回滚）',
    confirm: '安装 USB/IP；需要管理员授权', reboot: '安装器可能建议重启', verify: VERIFY,
    status: { ...base, installed: false, ready: false }, expect: { kind: 'install', steps: ['install', 'verify'] },
  },
  {
    id: 'install-resume', codes: ['install_resume'], detect: '上次安装等待重启，系统已重启，安装还没完成', repair: '继续安装', confirm: '重启前已确认；重启后自动继续',
    reboot: '否', verify: VERIFY,
    status: { ...base, installed: false, ready: false, install_resume: true }, expect: { kind: 'install', steps: ['install', 'verify'] },
  },
  {
    id: 'damaged-install', codes: ['USBIP-SETUP-002'], detect: '有登记但 usbip.exe 或安装目录缺失、不安全', repair: '统一清理后重新安装', confirm: '清理共享 USB/IP 并重装；DualSense 原生通道与 USB 透传暂不可用',
    reboot: '清理可能需要重启', verify: VERIFY,
    status: { ...base, ready: false, vhci_residual: true, error_code: 'USBIP-SETUP-002', detail: 'USBIP-SETUP-002: usbip.exe is missing' }, expect: { kind: 'clean-install', steps: ['cleanup', 'install', 'verify'] },
  },
  {
    id: 'old-version', codes: ['USBIP-SETUP-003', 'USBIP-SETUP-008'], detect: '已安装的版本不是配套版本（面板自己装的）', repair: '统一清理后安装配套版本', confirm: '清理共享 USB/IP 并重装',
    reboot: '清理可能需要重启', verify: VERIFY,
    status: { ...base, ready: false, version_valid: false, error_code: 'USBIP-SETUP-003' }, expect: { kind: 'clean-install', steps: ['cleanup', 'install', 'verify'] },
  },
  {
    id: 'mismatch-hash-abi', codes: ['USBIP-EXEC-006'], detect: 'usbip.exe 或内核驱动的 SHA-256 与固定组合不一致，或 usbip.exe 报 ABI mismatch', repair: '统一清理后安装配套版本',
    confirm: '清理共享 USB/IP 并重装', reboot: '清理可能需要重启', verify: VERIFY,
    status: { ...base, ready: false, error_code: 'USBIP-EXEC-006' }, expect: { kind: 'clean-install', steps: ['cleanup', 'install', 'verify'] },
  },
  {
    id: 'residual-broken', codes: ['USBIP-CLEAN-002'], detect: '设备树里有控制器节点，但没有存活的控制器', repair: '统一清理（卸载程序加节点清扫）后重装', confirm: '清理共享 USB/IP 并重装',
    reboot: 'Windows 接受但要重启才完成时（USBIP-CLEAN-006）', verify: VERIFY,
    status: { ...base, installed: false, ready: false, vhci_residual: true, residual_kind: 'broken' }, expect: { kind: 'clean-install', steps: ['cleanup', 'install', 'verify'] },
  },
  {
    id: 'stale-controller', codes: ['Multiple instances of VHCI device interface found'], detect: '传输可用，但旧版留下了多余的控制器', repair: '只移除多余的控制器，保留存活的那个，不重装',
    confirm: '移除多余控制器', reboot: '节点要等重启才删时', verify: VERIFY,
    status: { ...base, ready: false, vhci_residual: true, residual_kind: 'stale' }, expect: { kind: 'remove-stale', steps: ['remove-stale', 'verify'] },
  },
  {
    id: 'attached-ports', codes: ['USBIP-CLEAN-008', 'USBIP-REPLACE-005'], detect: '要清理或替换时仍有挂载的 USB 设备（端口没释放）', repair: '先释放所有已挂载的端口，再继续',
    confirm: '列出会释放的设备数量', reboot: '否', verify: VERIFY,
    status: { ...base, ready: false, error_code: 'USBIP-EXEC-006', attached_devices: [device] }, expect: { kind: 'clean-install', steps: ['detach', 'cleanup', 'install', 'verify'] },
  },
  {
    id: 'unsigned-driver', codes: ['usbip_unsigned', 'signature unsigned/invalid'], detect: '已登记驱动的签名无效或缺失（DualSense 状态读出）', repair: '统一清理后重装官方签名版本',
    confirm: '清理共享 USB/IP 并重装', reboot: '清理可能需要重启', verify: VERIFY,
    status: { ...base, driver_signature: 'invalid' }, expect: { kind: 'clean-install', steps: ['cleanup', 'install', 'verify'] },
  },
  {
    id: 'cleanup-resume', codes: ['cleanup_resume'], detect: '清理等重启，系统已重启，残留还在', repair: '继续清理，再安装', confirm: '重启前已确认；重启后自动继续', reboot: '否', verify: VERIFY,
    status: { ...base, installed: false, ready: false, vhci_residual: true, residual_kind: 'broken', cleanup_resume: true }, expect: { kind: 'clean-install', steps: ['cleanup', 'install', 'verify'] },
  },
  {
    id: 'tool-failed', codes: ['USBIP-EXEC-001', 'USBIP-EXEC-002', 'USBIP-EXEC-003', 'USBIP-EXEC-007'], detect: 'usbip.exe 起不来、超时、输出超限或本机操作失败，传输未就绪', repair: '统一清理后重装', confirm: '清理共享 USB/IP 并重装',
    reboot: '清理可能需要重启', verify: VERIFY,
    status: { ...base, ready: false, error_code: 'USBIP-EXEC-007' }, expect: { kind: 'clean-install', steps: ['cleanup', 'install', 'verify'] },
  },
  {
    id: 'external-incompatible-replaceable', codes: ['USBIP-SETUP-008', 'USBIP-REPLACE-*'], detect: '外部安装的完整 usbip-win2（含 GUI/SDK）版本或驱动不兼容，主机助手判定可替换',
    repair: '经确认后用主机助手替换为精简配套版本（失败自动回滚）', confirm: '明确写出会卸载现有 usbip-win2 及其图形界面和 SDK', reboot: '否', verify: VERIFY,
    status: { ...base, ready: false, externally_managed: true, replaceable: true, version_valid: false }, expect: { kind: 'replace', steps: ['replace', 'verify'], replacesExternal: true },
  },
  {
    id: 'external-incompatible-manual', codes: ['USBIP-SETUP-008', 'USBIP-REPLACE-001'], detect: '外部安装不兼容，且主机助手不允许替换', repair: '不改动；给出在「已安装的应用」里手动卸载的步骤',
    confirm: '无（不会改动）', reboot: '用户手动卸载后需要', verify: '不适用',
    status: { ...base, ready: false, externally_managed: true, replaceable: false, version_valid: false }, expect: { kind: 'blocked', reason: 'external', guide: 'external' },
  },
  {
    id: 'external-stale-controller', codes: ['Multiple instances of VHCI device interface found'], detect: '外部安装可用，但旧版留下了多余的控制器', repair: '只移除多余的控制器（外部安装本身保留）',
    confirm: '移除多余控制器', reboot: '节点要等重启才删时', verify: VERIFY,
    status: { ...base, ready: false, externally_managed: true, vhci_residual: true, residual_kind: 'stale' }, expect: { kind: 'remove-stale', steps: ['remove-stale', 'verify'] },
  },
  {
    id: 'external-compatible-healthy', codes: [], detect: '外部安装的 0.9.8.1，工具和驱动哈希一致，就绪', repair: '不需要；保留用户自己的安装', confirm: '无', reboot: '否', verify: '不适用',
    status: { ...base, externally_managed: true }, expect: { kind: 'healthy', steps: [] },
  },
  {
    id: 'pending-restart-cleanup', codes: ['USBIP-CLEAN-006', 'cleanup_pending_restart'], detect: '清理已被 Windows 接受，重启后才完成', repair: '说明原因并提供立即重启；同时保存计划（清理、检查），重启后继续；清理完成后要装的 USB/IP 要重新确认',
    confirm: '确认重启；重启后若还要安装，再问一次', reboot: '需要', verify: '重启后继续并检查',
    status: { ...base, ready: false, cleanup_pending_restart: true, vhci_residual: true }, expect: { kind: 'reboot', reason: 'cleanup', steps: [] },
  },
  {
    id: 'pending-restart-install', codes: ['USBIP-SETUP-015', 'install_pending_restart'], detect: 'Windows 有待完成的重启，安装不能完成', repair: '说明原因并提供立即重启；同时保存计划（安装、检查），重启后自动继续安装',
    confirm: '确认重启；重启前的安装请求已经是用户的决定', reboot: '需要', verify: '重启后继续并检查',
    status: { ...base, installed: false, ready: false, install_pending_restart: true }, expect: { kind: 'reboot', reason: 'install', steps: [] },
  },
  {
    id: 'reboot-recommended', codes: ['reboot_recommended'], detect: '安装器建议重启（退出码 3010）', repair: '说明原因并提供立即重启；同时保存计划（检查），重启后做检查', confirm: '确认重启', reboot: '建议', verify: '重启后检查',
    status: { ...base, reboot_recommended: true }, expect: { kind: 'reboot', reason: 'install', steps: [] },
  },
  {
    id: 'healthy', codes: [], detect: '传输就绪，无残留，无错误码，签名有效', repair: '不需要', confirm: '无', reboot: '否', verify: '不适用',
    status: { ...base }, expect: { kind: 'healthy', steps: [] },
  },
  {
    id: 'unsupported-os', codes: ['USBIP-SETUP-004'], detect: '不是 Windows', repair: '不做任何事', confirm: '无', reboot: '否', verify: '不适用',
    status: { ...base, supported: false }, expect: { kind: 'blocked', reason: 'unsupported', steps: [] },
  },
  {
    id: 'unexplained', codes: [], detect: '已安装但未就绪，没有错误码，没有重启建议', repair: '不改动；打开原始诊断，建议导出诊断', confirm: '无', reboot: '否', verify: '不适用',
    status: { ...base, ready: false }, expect: { kind: 'blocked', reason: 'nothing', steps: [] },
  },
  // ---- found while the repair runs ----
  { id: 'stream-unknown', codes: ['USBIP-SETUP-006'], detect: '主机读不到会话（Sunshine 没有响应）：不知道有没有串流在用传输', repair: '读不到不等于没有串流：有限时间内重试，仍读不到就停下，什么都不动；说明要先启动 Sunshine', confirm: '无', reboot: '否', verify: '不适用', error: 'USBIP-SETUP-006: unable to verify stream ownership: the host did not answer', guide: '' },
  { id: 'stream-active', codes: ['USBIP-SETUP-006', 'USBIP-CLEAN-001'], detect: '有串流在用传输', repair: '经主机接口结束串流并等它真正结束；读不到就一直等到时限，不继续', confirm: '写明会结束串流', reboot: '否', verify: '主机返回空的会话列表', error: 'USBIP-SETUP-006: finish the active Sunshine stream', guide: '' },
  { id: 'driver-op-busy', codes: ['USBIP-SETUP-018'], detect: '另一项驱动操作（VB-CABLE、虚拟鼠标等）还在进行', repair: '不开始，提示稍后重试', confirm: '无', reboot: '否', verify: '不适用', error: 'USBIP-SETUP-018: another component driver operation is still running', guide: '' },
  { id: 'uac-cancelled', codes: ['USBIP-UAC-001', 'USBIP-UAC-002', 'USBIP-UAC-003'], detect: '管理员授权被取消、助手连不上或超时', repair: '停在当前步骤，提示重试', confirm: '无', reboot: '否', verify: '不适用', error: 'USBIP-UAC-001: canceled', guide: '' },
  { id: 'root-hub-rolled-back', codes: ['USBIP-SETUP-011'], detect: '安装时 USB 根集线器检查没通过（AMD 等 USB 3 控制器上出现过），安装器已自动回滚', repair: '不再重试；给出更新芯片组驱动、重启、再修复的步骤', confirm: '无', reboot: '步骤里要求', verify: '不适用', error: 'USBIP-SETUP-011: USB root hub check failed and was rolled back (exit code 102)', guide: 'roothub' },
  { id: 'root-hub-rollback-incomplete', codes: ['USBIP-SETUP-012', 'USBIP-REPLACE-003'], detect: '回滚后仍有 USB 根集线器没恢复', repair: '不再重试；给出重启、检查设备管理器、导出诊断的步骤', confirm: '无', reboot: '步骤里要求', verify: '不适用', error: 'USBIP-SETUP-012: USB root hub check failed and rollback was incomplete (exit code 103)', guide: 'roothub-broken' },
  { id: 'usb-boot-disk', codes: ['USBIP-SETUP-013'], detect: 'Windows 从 USB 磁盘启动，安装会断开系统盘', repair: '不安装；给出说明', confirm: '无', reboot: '否', verify: '不适用', error: 'USBIP-SETUP-013: Windows appears to boot from a USB disk (exit code 104)', guide: 'usb-boot' },
  { id: 'guard-could-not-run', codes: ['USBIP-SETUP-014'], detect: '根集线器保护程序没能运行', repair: '不安装；给出重启后重试的步骤', confirm: '无', reboot: '步骤里要求', verify: '不适用', error: 'USBIP-SETUP-014: USB root hub guard could not run (exit code 105)', guide: 'guard' },
  { id: 'driver-policy-blocked', codes: ['USBIP-SETUP-016'], detect: '内存完整性（HVCI）、易受攻击驱动阻止列表、签名校验或 CM_PROB 48/52/39 拦下了驱动', repair: '软件不能绕过；给出关闭内存完整性、重启、再修复的步骤', confirm: '无', reboot: '步骤里要求', verify: '不适用', error: 'USBIP-SETUP-007: transport installation failed: USBIP-SETUP-016: Windows driver policy blocked (exit code 1)', guide: 'policy' },
  { id: 'smart-app-control', codes: ['USBIP-SETUP-017'], detect: '智能应用控制或应用控制策略拦下了安装程序（0x800711C7 / 4551）', repair: '软件不能绕过；给出关闭智能应用控制的步骤', confirm: '无', reboot: '否', verify: '不适用', error: 'USBIP-SETUP-017: Windows Smart App Control blocked the installer', guide: 'sac' },
  { id: 'removal-access-denied', codes: ['USBIP-CLEAN-007'], detect: 'Windows 拒绝移除主控制器（CR_ACCESS_DENIED / ERROR_ACCESS_DENIED）', repair: '给出以管理员运行、设备管理器手动卸载的步骤', confirm: '无', reboot: '步骤里要求', verify: '不适用', error: 'USBIP-CLEAN-007: Windows refused to remove the USB/IP host controller', guide: 'access' },
  { id: 'removal-in-use', codes: ['USBIP-CLEAN-008'], detect: '控制器被占用（否决者在错误行里）', repair: '已先结束串流并释放端口；仍占用时提示关闭使用 USB/IP 的程序或先重启', confirm: '无', reboot: '可能', verify: '不适用', error: 'USBIP-CLEAN-008: the USB/IP host controller is in use', guide: '' },
  { id: 'installer-generic-failure', codes: ['USBIP-SETUP-010', 'USBIP-SETUP-005', 'USBIP-REPLACE-004', 'USBIP-CLEAN-001', 'USBIP-CLEAN-003', 'USBIP-CLEAN-004'], detect: '安装器、卸载程序或服务清理失败，原因不属于以上任何一种', repair: '给出重启后重试并导出诊断的步骤，原始诊断里有安装器与 setupapi 摘录', confirm: '无', reboot: '步骤里要求', verify: '不适用', error: 'USBIP-SETUP-010: the USB/IP installer failed with exit code 1', guide: 'generic' },
  { id: 'replace-rolled-back', codes: ['USBIP-REPLACE-002'], detect: '替换失败并已回滚；根集线器正常，USB/IP 当前未安装', repair: '状态变为未安装；再点一键修复就是安装', confirm: '无', reboot: '否', verify: '不适用', error: 'USBIP-REPLACE-002: the replacement was rolled back', guide: '' },
  { id: 'verify-failed', codes: ['USBIP-VERIFY-001', 'USBIP-VERIFY-002'], detect: '步骤都完成了，但传输仍未就绪，或 DualSense 原生通道仍报告 USB/IP 问题', repair: '不宣称已恢复；给出重启后重试并导出诊断的步骤', confirm: '无', reboot: '步骤里要求', verify: '不适用', error: 'USBIP-VERIFY-001: not ready after the repair', guide: 'generic' },
  { id: 'windows-10', codes: [], detect: '面板里没有 Windows 10 专用的分支：版本差异由同一条路径处理（挂起重启、快速启动下要选“重启”而不是“关机”、驱动策略拦截）；智能应用控制只存在于 Windows 11', repair: '走与 Windows 11 相同的流程和步骤', confirm: '同上', reboot: '同上', verify: VERIFY },
  { id: 'host-not-running', codes: ['host_unreachable'], detect: '修复后 Sunshine 没有响应，DualSense 原生通道无法检查', repair: '传输检查通过则宣布传输已恢复，并说明要先启动 Sunshine', confirm: '无', reboot: '否', verify: '只检查传输' },
]
