# USB/IP 安装布局兼容性

兼容清单保留上游 Inno 布局并添加 usbip-slim-v1。新布局由固定卸载项键和
InstallLayout 标识，不靠 DisplayName；旧名字继续识别，冲突版本继续拒绝。
新布局在执行前验证固定 payload 清单哈希及其中全部文件，已有内核驱动哈希检查保留。
安装优先调用主机附带的受保护安装脚本（包含根集线器 guard）；旧主机没有该脚本时
提示更新主机，不再下载大体积上游安装器。已有旧布局仍可识别与使用。安装需用户明确确认及无活动串流。

新卸载命令是 PowerShell，直接使用注册参数，不能追加 Inno flags。仅新布局允许
3010 作为卸载成功且需要重启。新旧两种布局最终都要移除自己的安装记录。
Windows 保持现有 Rust 与 Win32 注册表查询路径，解析/完整性检查使用现有
serde_json 和 SHA-256 支持；未增加 GC 运行时或网络热路径开销。

参考来源 usbip-slim-ms-reg-uninstall-20261117 的 DisplayVersion/InstallLocation，
usbip-slim-ms-powershell-20261117 的 Windows 签名语义。读取日期 2026-11-17。
新旧安装、卸载与真实设备结果以 Windows CI 和独立设备验收为准；Linux 单元测试
只覆盖版本策略和退出码。macOS、Linux、Android、iOS、HarmonyOS 未增加 Backend；
吞吐、延迟、资源授权和跨租户隔离没有本卡测量或验收，不宣称已核实。

固定库版本：winreg 0.55.0、Tokio 1.52.3、serde_json 1.0.149，依据分别见
`panel-usbip-winreg-20261117`、`panel-usbip-tokio-20261117`、`panel-usbip-serde-json-20261117`。
新布局的安装和卸载子进程在界面关闭或调用超时后继续完成 guard 事务，避免中断驱动变更后的恢复；
进程间互斥量仍由子进程持有。取消中的真实 OS 行为未核实，不把策略测试作为设备验收。

## 归属优先（2026-11-17）

外部完整安装（带 GUI 或 SDK）、其他版本、缺少可靠记录和任何检查失败均按外部管理。
自有 receipt 必须匹配布局、来源哈希和同版本；旧包只在同版本、无 GUI/SDK、同一标准
安装目录、主机保留固定哈希原安装器时才允许接管。普通成功 journal 不能证明来源。
规则来源 usbip-policy-reg-20261117（DisplayVersion/InstallLocation）、
usbip-policy-winreg-20261117（get_value、只读 registry view）、usbip-policy-rust-path-20261117（try_exists）。

唯一兼容版本仍为 0.9.8.1，继续验证 CLI 与注册 UDE 驱动固定哈希，再执行有界 port
探测。发行版本不同于驱动文件版本，不把“版本较新”视为 ABI 兼容。完整上游版的
GUI/SDK 不影响复用，但永远阻止自动接管、清理、卸载。清理的提权端再次判定归属，
不能仅靠按钮隐藏保护。版本不符时说明外部安装被保留，由用户自行选择是否先在
Windows 已安装的应用中替换或修复；当前面板不替用户卸载它。

状态追加 externally_managed，兼容安装仍可用于远程 USB、原生手柄和 USB 麦克风。
“不阻止使用”不是实机功能通过声明；这些设备功能仍待分别验证。CI 使用 HKCU
隔离注册项与临时文件布局，覆盖五种必需状态及双卸载器完整安装记录；没有更改 INF
或签名。Windows Rust/winreg 保留既有有界命令与哈希校验，避免 shell 拼接；UI 用
既有 Vue 状态。没有更换协议、驱动或数据热路径，无运行时性能和跨实现互通测量。
其他系统没有变更的 Backend，未构建或运行；真实设备、资源授权、租户隔离未核实。
