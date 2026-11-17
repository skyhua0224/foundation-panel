# USB/IP 安装布局兼容性

兼容清单保留上游 Inno 布局并添加 usbip-slim-v1。新布局由固定卸载项键和
InstallLayout 标识，不靠 DisplayName；旧名字继续识别，冲突版本继续拒绝。
新布局在执行前验证固定 payload 清单哈希及其中全部文件，已有内核驱动哈希检查保留。
安装优先调用主机附带的受保护安装脚本（包含根集线器 guard）；旧主机没有该脚本时
保留原版安装器下载路径。两者均需用户明确确认及无活动串流。

新卸载命令是 PowerShell，直接使用注册参数，不能追加 Inno flags。仅新布局允许
3010 作为卸载成功且需要重启。新旧两种布局最终都要移除自己的安装记录。
Windows 保持现有 Rust 与 Win32 注册表查询路径，解析/完整性检查使用现有
serde_json 和 SHA-256 支持；未增加 GC 运行时或网络热路径开销。

参考来源 usbip-slim-ms-reg-uninstall-20261117 的 DisplayVersion/InstallLocation，
usbip-slim-ms-powershell-20261117 的 Windows 签名语义。读取日期 2026-11-17。
新旧安装、卸载与真实设备结果以 Windows CI 和独立设备验收为准；Linux 单元测试
只覆盖版本策略和退出码。macOS、Linux、Android、iOS、HarmonyOS 未增加 Backend；
吞吐、延迟、资源授权和跨租户隔离没有本卡测量或验收，不宣称已核实。
