//! 体检, the panel's own layer: what can be checked without Sunshine running (the service, the installation, the processes and the
//! ports, the drivers), in the JSON of the host's `GET /api/doctor` (version 1: groups, items, fix with an action) with `source: "local"`.
//! The page asks for one group at a time (`doctor_local(only, lang)`) and merges these groups with the host's.
//!
//! Everything here reads. A fix that acts is only described (`repair` with a target, a level and what it will do); the page asks first
//! and the panel runs it through its existing commands. Facts are gathered on Windows; the evaluators are plain functions with tests.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const GROUPS: [&str; 4] = ["service", "install", "process", "drivers"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Zh,
}

impl Lang {
    pub fn parse(value: &str) -> Self {
        if value.starts_with("zh") { Lang::Zh } else { Lang::En }
    }
}

fn tr(lang: Lang, en: impl Into<String>, zh: impl Into<String>) -> String {
    match lang {
        Lang::En => en.into(),
        Lang::Zh => zh.into(),
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Action {
    pub kind: String,
    pub target: String,
    pub confirm: bool,
    /// 1 leads somewhere, 2 acts reversibly, 3 needs administrator rights
    pub level: u8,
    /// what it will do, shown before it runs
    #[serde(skip_serializing_if = "String::is_empty")]
    pub effect: String,
}

#[derive(Clone, Debug, Serialize, Default, PartialEq)]
pub struct Fix {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Item {
    pub id: String,
    pub group: String,
    pub status: &'static str,
    pub title: String,
    pub detail: String,
    pub fix: Fix,
    pub source: &'static str,
}

fn item(id: &str, status: &'static str, title: String, detail: String, fix: Fix) -> Item {
    Item { id: id.to_string(), group: id.split('.').next().unwrap_or("").to_string(), status, title, detail, fix, source: "local" }
}

fn fix(text: String) -> Fix {
    Fix { text, action: None }
}

fn open(text: String, target: &str) -> Fix {
    Fix { text, action: Some(Action { kind: "open".into(), target: target.into(), confirm: false, level: 1, effect: String::new() }) }
}

fn act(text: String, kind: &str, target: &str, level: u8, effect: String) -> Fix {
    Fix { text, action: Some(Action { kind: kind.into(), target: target.into(), confirm: true, level, effect }) }
}

fn skip_not_windows(id: &str, title: String, lang: Lang) -> Item {
    item(id, "skip", title, tr(lang, "Not checked: only on Windows", "未检查:只在 Windows 上"), Fix::default())
}

// ---- Sunshine 服务 ----

#[derive(Clone, Debug, Default)]
pub struct ServiceFacts {
    pub checked: bool,
    pub exists: bool,
    /// SCM state: RUNNING, STOPPED, START_PENDING, STOP_PENDING, PAUSED, …
    pub state: String,
    pub service_pid: u32,
    pub win32_exit: u32,
    pub service_exit: u32,
    /// auto, delayed, demand, disabled; empty when unread
    pub start_type: String,
    pub image_path: String,
    pub image_matches_install: Option<bool>,
    /// sunshine.exe processes (the host under the service wrapper, or in user mode)
    pub host_pids: Vec<u32>,
    /// the host's HTTPS answered (any status); None when not asked
    pub api_answers: Option<bool>,
}

pub fn evaluate_service(f: &ServiceFacts, lang: Lang) -> Vec<Item> {
    let mut items = Vec::new();
    if !f.checked {
        return vec![skip_not_windows("service.state", tr(lang, "Running state", "运行状态"), lang)];
    }
    let start_effect = tr(lang, "Stops and starts SunshineService; Windows may ask for administrator rights. A stream that is live ends.", "停止并重新启动 SunshineService;Windows 可能会请求管理员确认。正在进行的串流会断开。");
    if !f.exists {
        let running_user = !f.host_pids.is_empty();
        items.push(item(
            "service.state",
            if running_user { "warn" } else { "fail" },
            tr(lang, "Running state", "运行状态"),
            if running_user {
                tr(lang, format!("No SunshineService; sunshine.exe runs in user mode (PID {})", f.host_pids[0]), format!("没有 SunshineService;sunshine.exe 以用户模式运行(PID {})", f.host_pids[0]))
            } else {
                tr(lang, "SunshineService is not installed and sunshine.exe is not running", "没有安装 SunshineService,sunshine.exe 也没有运行")
            },
            fix(tr(lang, "Run the Sunshine installer again to register the service.", "重新运行 Sunshine 安装程序以注册服务。")),
        ));
        return items;
    }
    // Three layers: the service manager, the host process, the host's own API; running means all three
    let scm_running = f.state == "RUNNING";
    let host = !f.host_pids.is_empty();
    let api = f.api_answers.unwrap_or(false);
    let layers = format!(
        "{} {} · {} · {}",
        tr(lang, "service", "服务"),
        f.state,
        if host { format!("sunshine.exe PID {}", f.host_pids[0]) } else { tr(lang, "no sunshine.exe", "没有 sunshine.exe") },
        match f.api_answers {
            Some(true) => tr(lang, "Host responds", "主机有应答"),
            Some(false) => tr(lang, "Host does not respond", "主机没有应答"),
            None => tr(lang, "Host not queried", "未查询主机"),
        }
    );
    if scm_running && host && api {
        items.push(item("service.state", "pass", tr(lang, "Running state", "运行状态"), layers, Fix::default()));
    } else if scm_running && !host {
        items.push(item("service.state", "fail", tr(lang, "Running state", "运行状态"),
            tr(lang, format!("{layers}: the service wrapper runs but Sunshine itself has exited"), format!("{layers}:服务包装进程还在,但 Sunshine 本身已经退出")),
            act(tr(lang, "Restart Sunshine; if it exits again, the log says why.", "重启 Sunshine;如果又退出,日志里有原因。"), "repair", "service:start", 2, start_effect)));
    } else if scm_running && host && !api {
        items.push(item("service.state", "warn", tr(lang, "Running state", "运行状态"),
            tr(lang, format!("{layers}: it may still be starting, or a port is taken"), format!("{layers}:可能还在启动,或者端口被占用")),
            open(tr(lang, "Check the ports below, then the log.", "先看下面的端口,再看日志。"), "page:logs")));
    } else if f.state.ends_with("PENDING") {
        items.push(item("service.state", "warn", tr(lang, "Running state", "运行状态"),
            tr(lang, format!("{layers}: changing state"), format!("{layers}:状态正在变化")),
            Fix { text: tr(lang, "Check again in a few seconds.", "几秒后重新检查。"), action: Some(Action { kind: "rerun".into(), target: "service".into(), confirm: false, level: 1, effect: String::new() }) }));
    } else {
        items.push(item("service.state", "fail", tr(lang, "Running state", "运行状态"),
            tr(lang, format!("{layers}: Sunshine is not running"), format!("{layers}:Sunshine 没有运行")),
            act(tr(lang, "Start Sunshine.", "启动 Sunshine。"), "repair", "service:start", 2, start_effect)));
    }

    match f.start_type.as_str() {
        "auto" | "delayed" => items.push(item("service.start_type", "pass", tr(lang, "Start type", "启动方式"),
            if f.start_type == "auto" { tr(lang, "Automatic", "开机自动") } else { tr(lang, "Automatic (delayed)", "开机自动(延迟)") }, Fix::default())),
        "demand" | "disabled" => items.push(item("service.start_type", "warn", tr(lang, "Start type", "启动方式"),
            if f.start_type == "demand" {
                tr(lang, "Manual: after a restart nobody can connect until someone starts it", "手动:电脑重启后没人启动它就连不上")
            } else {
                tr(lang, "Disabled: Sunshine cannot start", "已禁用:Sunshine 无法启动")
            },
            act(tr(lang, "Set the start type to Automatic.", "把启动方式设为“开机自动”。"), "repair", "service:start_type", 3,
                tr(lang, "Sets SunshineService to start automatically with Windows.", "把 SunshineService 设为随 Windows 自动启动。")))),
        _ => items.push(item("service.start_type", "skip", tr(lang, "Start type", "启动方式"), tr(lang, "Not checked: the start type could not be read", "未检查：无法读取启动方式"), Fix::default())),
    }

    if f.image_matches_install == Some(false) {
        items.push(item("service.image_path", "fail", tr(lang, "Service program", "服务程序"),
            tr(lang, format!("The service runs another installation: {}", f.image_path), format!("服务指向另一个安装:{}", f.image_path)),
            fix(tr(lang, "Run the installer of this Sunshine again; it points the service here.", "重新运行这个 Sunshine 的安装程序,服务会指回这里。"))));
    }

    if f.win32_exit != 0 || f.service_exit != 0 {
        let code = if f.win32_exit == 1066 { f.service_exit } else { f.win32_exit };
        items.push(item("service.last_exit", "warn", tr(lang, "Last exit", "上次退出"),
            tr(lang, format!("Exit code {code} (0x{code:x})"), format!("退出码 {code}(0x{code:x})")),
            open(tr(lang, "The log of that run says why.", "那次运行的日志里有原因。"), "page:logs")));
    } else {
        items.push(item("service.last_exit", "pass", tr(lang, "Last exit", "上次退出"), tr(lang, "Exit code 0", "退出码 0"), Fix::default()));
    }
    // the recovery actions (restart on failure) come with the service work's reader
    items.push(item("service.recovery", "skip", tr(lang, "Restart on failure", "失败后自动重启"),
        tr(lang, "Not checked: the recovery actions are read once the service work lands", "未检查:恢复策略的读取等服务工作合入"), Fix::default()));
    items
}

// ---- 安装 ----

/// The build identity of `sunshine.exe --version --json` (audit #1); every field optional until the build work lands.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct BuildInfo {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub commit: String,
    #[serde(default)]
    pub branch: String,
    #[serde(default)]
    pub dirty: bool,
    #[serde(default)]
    pub channel: String,
    #[serde(default)]
    pub build_time_utc: String,
    #[serde(default)]
    pub variant: String,
    #[serde(default)]
    pub package: String,
    #[serde(default)]
    pub builder: String,
}

/// The JSON object in the output of `sunshine --version --json` (the first `{` to the last `}`), if it has a version.
pub fn parse_build_info(output: &str) -> Option<BuildInfo> {
    let start = output.find('{')?;
    let end = output.rfind('}')?;
    let info: BuildInfo = serde_json::from_str(&output[start..=end]).ok()?;
    (!info.version.is_empty()).then_some(info)
}

#[derive(Clone, Debug, Default)]
pub struct InstallFacts {
    pub checked: bool,
    pub dir: String,
    pub exe_found: bool,
    pub file_version: String,
    pub build: Option<BuildInfo>,
    pub config_found: bool,
}

pub fn evaluate_install(f: &InstallFacts, lang: Lang) -> Vec<Item> {
    if !f.checked {
        return vec![skip_not_windows("install.dir", tr(lang, "Installation folder", "安装目录"), lang)];
    }
    let mut items = Vec::new();
    if f.exe_found {
        items.push(item("install.dir", "pass", tr(lang, "Installation folder", "安装目录"), tr(lang, format!("{} · sunshine.exe present", f.dir), format!("{} · sunshine.exe 存在", f.dir)), Fix::default()));
    } else {
        items.push(item("install.dir", "fail", tr(lang, "Installation folder", "安装目录"),
            tr(lang, format!("sunshine.exe is missing from {}", f.dir), format!("{} 里没有 sunshine.exe", f.dir)),
            fix(tr(lang, "Install Sunshine again.", "重新安装 Sunshine。"))));
        return items;
    }
    match &f.build {
        Some(b) => {
            let mut parts = vec![b.version.clone()];
            if !b.channel.is_empty() { parts.push(b.channel.clone()); }
            if !b.commit.is_empty() { parts.push(b.commit.chars().take(10).collect()); }
            if !b.build_time_utc.is_empty() { parts.push(b.build_time_utc.clone()); }
            if !b.variant.is_empty() { parts.push(b.variant.clone()); }
            let status = if b.dirty { "warn" } else { "pass" };
            items.push(item("install.version", status, tr(lang, "Version and build", "版本与构建"), parts.join(" · "),
                if b.dirty { fix(tr(lang, "This build has uncommitted changes; install a release before reporting problems.", "这是含未提交改动的构建;报告问题前请装正式版。")) } else { Fix::default() }));
        }
        None => items.push(item("install.version", if f.file_version.is_empty() { "skip" } else { "pass" }, tr(lang, "Version and build", "版本与构建"),
            if f.file_version.is_empty() {
                tr(lang, "Not checked: the version could not be read", "未检查：无法读取版本")
            } else {
                tr(lang, format!("{} · build details are read from the version string (sunshine --version --json is not there yet)", f.file_version), format!("{} · 构建信息由版本号推断(sunshine --version --json 还没合入)", f.file_version))
            }, Fix::default())),
    }
    if !f.config_found {
        items.push(item("install.config", "warn", tr(lang, "Configuration", "配置"),
            tr(lang, "config\\sunshine.conf is missing; Sunshine starts with its defaults", "没有 config\\sunshine.conf;Sunshine 用默认设置启动"),
            open(tr(lang, "Save the settings once to write it.", "保存一次设置即可生成。"), "page:config")));
    }
    items
}

// ---- 进程与端口 ----

#[derive(Clone, Debug, PartialEq)]
pub struct Listener {
    pub proto: String,
    pub port: u16,
    pub pid: u32,
}

/// The listening TCP sockets and bound UDP sockets of `netstat -ano` (columns: proto, local, foreign, [state], pid). A TCP row listens
/// when its foreign address is the wildcard (0.0.0.0:0 or [::]:0), which does not depend on the language of the state column.
pub fn parse_netstat(text: &str) -> Vec<Listener> {
    let mut out = Vec::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 4 {
            continue;
        }
        let proto = cols[0].to_ascii_lowercase();
        if proto != "tcp" && proto != "udp" {
            continue;
        }
        if proto == "tcp" && !(cols[2] == "0.0.0.0:0" || cols[2] == "[::]:0") {
            continue;
        }
        let Some(port) = cols[1].rsplit(':').next().and_then(|p| p.parse::<u16>().ok()) else { continue };
        let Some(pid) = cols.last().and_then(|p| p.parse::<u32>().ok()) else { continue };
        let listener = Listener { proto, port, pid };
        if !out.contains(&listener) {
            out.push(listener);
        }
    }
    out
}

/// The ports Sunshine uses for a base port (the `port` setting, 47989 by default), as the host maps them.
pub fn sunshine_ports(base: u16) -> Vec<(&'static str, u16, &'static str)> {
    let at = |offset: i32| (base as i32 + offset) as u16;
    vec![
        ("tcp", at(-5), "https"),
        ("tcp", at(0), "http"),
        ("tcp", at(1), "web"),
        ("tcp", at(21), "rtsp"),
        ("udp", at(9), "video"),
        ("udp", at(10), "control"),
        ("udp", at(11), "audio"),
        ("udp", at(13), "mic"),
    ]
}

#[derive(Clone, Debug, Default)]
pub struct ProcessFacts {
    pub checked: bool,
    pub base_port: u16,
    pub listeners: Vec<Listener>,
    /// pid → image name
    pub names: Vec<(u32, String)>,
    pub host_pids: Vec<u32>,
    pub log_found: bool,
    pub log_path: String,
    /// (stamp, level, message) of the last Error or Fatal line of the log
    pub last_error: Option<(String, String, String)>,
    /// the log ends without the shutdown line while no host runs: the last run did not end cleanly
    pub ended_abruptly: bool,
    /// set when the port table could not be read in time; empty listeners are then not "the ports are free"
    pub port_table_error: String,
}

/// The last Error or Fatal line of a Sunshine log ("[stamp]: Level: message") and whether the log ends on a clean shutdown.
pub fn scan_log_tail(text: &str) -> (Option<(String, String, String)>, bool) {
    let mut last = None;
    let mut clean_end = false;
    for line in text.lines() {
        let Some(rest) = line.strip_prefix('[') else { continue };
        let Some((stamp, tail)) = rest.split_once("]: ") else { continue };
        for level in ["Error", "Fatal"] {
            if let Some(message) = tail.strip_prefix(level).and_then(|t| t.strip_prefix(": ")) {
                last = Some((stamp.to_string(), level.to_string(), message.trim_end().to_string()));
            }
        }
        if tail.contains("Goodbye") || tail.contains("Sunshine stopped") {
            clean_end = true;
        } else if !tail.is_empty() {
            clean_end = false;
        }
    }
    (last, clean_end)
}

pub fn evaluate_process(f: &ProcessFacts, lang: Lang) -> Vec<Item> {
    if !f.checked {
        return vec![skip_not_windows("process.ports", tr(lang, "Ports", "端口占用"), lang)];
    }
    let name_of = |pid: u32| f.names.iter().find(|(p, _)| *p == pid).map(|(_, n)| n.clone()).unwrap_or_else(|| format!("PID {pid}"));
    let mut items = Vec::new();
    if f.host_pids.len() > 1 {
        items.push(item(
            "process.instances",
            "fail",
            tr(lang, "Sunshine processes", "Sunshine 进程"),
            tr(
                lang,
                format!("{} sunshine.exe processes are running and compete for the ports and the GPU", f.host_pids.len()),
                format!("{} 个 sunshine.exe 同时在跑,互相抢端口和显卡", f.host_pids.len()),
            ),
            act(
                tr(lang, "Restart Sunshine so only one is left.", "重启 Sunshine,只留一个。"),
                "restart",
                "sunshine",
                2,
                tr(lang, "Stops every Sunshine of this installation and starts one.", "停掉本安装的全部 Sunshine,再启动一个。"),
            ),
        ));
    }
    if !f.port_table_error.is_empty() {
        items.push(item(
            "process.ports",
            "warn",
            tr(lang, "Ports", "端口占用"),
            tr(
                lang,
                format!("The port table did not answer: {}", f.port_table_error),
                format!("端口表没有返回:{}", f.port_table_error),
            ),
            fix(tr(lang, "Run this group again. If it repeats, restart Sunshine.", "重新检查这一组。反复出现时重启 Sunshine。")),
        ));
    } else {
    let mut taken = Vec::new();
    let mut ours = 0;
    let mut free = Vec::new();
    for (proto, port, role) in sunshine_ports(f.base_port) {
        match f.listeners.iter().find(|l| l.proto == proto && l.port == port) {
            Some(l) if f.host_pids.contains(&l.pid) => ours += 1,
            Some(l) => taken.push(format!("{proto} {port} ({role}): {} (PID {})", name_of(l.pid), l.pid)),
            None => free.push(format!("{proto} {port}")),
        }
    }
    if !taken.is_empty() {
        items.push(item("process.ports", "fail", tr(lang, "Ports", "端口占用"),
            tr(lang, format!("Taken by another program: {}", taken.join("; ")), format!("被别的程序占用:{}", taken.join(";"))),
            open(tr(lang, "Close that program or change Sunshine's Port, then restart Sunshine.", "关掉那个程序或改 Sunshine 的「端口」,然后重启 Sunshine。"), "setting:port")));
    } else if f.host_pids.is_empty() {
        items.push(item("process.ports", "pass", tr(lang, "Ports", "端口占用"),
            tr(lang, format!("Free for Sunshine (base port {})", f.base_port), format!("Sunshine 的端口都空闲(基准端口 {})", f.base_port)), Fix::default()));
    } else {
        // the UDP stream ports open only while a stream runs
        let tcp_missing: Vec<_> = free.iter().filter(|p| p.starts_with("tcp")).cloned().collect();
        if tcp_missing.is_empty() {
            items.push(item("process.ports", "pass", tr(lang, "Ports", "端口占用"),
                tr(lang, format!("{ours} held by Sunshine; the UDP stream ports open when a stream starts"), format!("{ours} 个由 Sunshine 占用;UDP 串流端口在开始串流时才打开")), Fix::default()));
        } else {
            items.push(item("process.ports", "warn", tr(lang, "Ports", "端口占用"),
                tr(lang, format!("Sunshine runs but does not listen on {}", tcp_missing.join(", ")), format!("Sunshine 在运行,但没有在 {} 上监听", tcp_missing.join("、"))),
                open(tr(lang, "The log says why it could not bind.", "日志里有绑定失败的原因。"), "page:logs")));
        }
    }

    }
    if !f.log_found {
        items.push(item("process.log", "skip", tr(lang, "End of the log", "日志末尾"),
            tr(lang, format!("Not checked: no log at {}", f.log_path), format!("未检查:{} 没有日志", f.log_path)), Fix::default()));
    } else if let Some((stamp, level, message)) = &f.last_error {
        let short: String = message.chars().take(160).collect();
        items.push(item("process.log", "warn", tr(lang, "End of the log", "日志末尾"),
            tr(lang, format!("Last {level} ({stamp}): {short}"), format!("最后一条 {level}({stamp}):{short}")),
            open(tr(lang, "See the whole log in Logs and diagnostics.", "在「日志与诊断」里看完整日志。"), "page:logs")));
    } else {
        items.push(item("process.log", "pass", tr(lang, "End of the log", "日志末尾"), tr(lang, "No error in the last lines", "最后几百行没有错误"), Fix::default()));
    }
    if f.ended_abruptly && f.host_pids.is_empty() {
        items.push(item("process.last_run", "warn", tr(lang, "Last run", "上次运行"),
            tr(lang, "The log ends without a shutdown: Sunshine stopped abruptly (a crash or a kill)", "日志没有正常退出的记录:Sunshine 是突然停止的(崩溃或被结束)"),
            open(tr(lang, "Export the diagnostics bundle and report it.", "导出诊断包并反馈。"), "page:logs")));
    }
    items
}

// ---- 驱动 ----

#[derive(Clone, Debug, Default)]
pub struct DriverFact {
    /// vdd, vigem, vmouse, usbip
    pub id: String,
    pub name: String,
    pub checked: bool,
    pub installed: bool,
    pub running: bool,
    /// the device's PnP problem code (CM_PROB_*), when it has one
    pub problem: Option<u32>,
    pub version: String,
    pub reboot: bool,
    pub note: String,
    /// absence is a problem (VDD is needed for headless hosts; the others are optional)
    pub required: bool,
    /// the probe itself did not finish; not a statement about the driver
    pub probe_error: String,
}

pub fn driver_effect(id: &str, lang: Lang) -> String {
    match id {
        "vdd" => tr(lang, "Removes and recreates the virtual display device, then installs its driver. On a host without a monitor the picture is gone until it is back; the display a stream uses drops.", "删除并重建虚拟显示器设备,再安装驱动。没有物理显示器的主机在恢复前没有画面;正在串流用的显示器会掉。"),
        "vigem" => tr(lang, "Installs ViGEmBus again from the package that ships with Sunshine (SHA-256 checked). Other programs that use it, such as DS4Windows, lose their gamepads for a moment; a restart may be needed.", "用随产品附带的安装包(已校验 SHA-256)重新安装 ViGEmBus。DS4Windows 等也用它的软件会短暂失去手柄,可能需要重启电脑。"),
        "vmouse" => tr(lang, "Installs the virtual mouse driver again. Its installer stops SunshineService first, so a stream that is live ends.", "重新安装虚拟鼠标驱动。安装脚本会先停掉 SunshineService,正在进行的串流会断开。"),
        "usbip" => tr(lang, "Installs the usbip-win2 transport again through the panel's elevated helper. Every attached USB device is detached, remote keyboards, mice and gamepads included.", "通过面板的加固提权助手重新安装 usbip-win2。所有已挂载的 USB 设备都会断开,包括远程键鼠和手柄。"),
        _ => String::new(),
    }
}

/// Words of the CM_PROB_* codes Device Manager shows most for these drivers.
fn problem_words(code: u32, lang: Lang) -> String {
    let (en, zh) = match code {
        10 => ("the device cannot start", "设备无法启动"),
        14 => ("Windows needs a restart", "需要重启 Windows"),
        18 => ("the driver needs reinstalling", "需要重新安装驱动"),
        22 => ("the device is disabled", "设备已被禁用"),
        28 => ("the driver is not installed", "驱动没有安装"),
        31 => ("the driver could not load", "驱动无法加载"),
        39 => ("the driver is damaged or missing", "驱动损坏或缺失"),
        43 => ("the device reported a problem", "设备报告了故障"),
        45 => ("the device is not connected", "设备没有连接"),
        52 => ("the driver signature cannot be verified", "无法验证驱动签名"),
        _ => ("", ""),
    };
    if en.is_empty() { tr(lang, format!("problem code {code}"), format!("问题代码 {code}")) } else { tr(lang, format!("problem code {code} ({en})"), format!("问题代码 {code}({zh})")) }
}

pub fn evaluate_drivers(drivers: &[DriverFact], lang: Lang) -> Vec<Item> {
    let mut items = Vec::new();
    for d in drivers {
        let id = format!("drivers.{}", d.id);
        if !d.probe_error.is_empty() {
            items.push(item(&id, "warn", d.name.clone(), tr(lang, format!("The check did not finish: {}", d.probe_error), format!("检查没有完成:{}", d.probe_error)), fix(tr(lang, "Run this group again.", "重新检查这一组。"))));
            continue;
        }
        if !d.checked {
            items.push(skip_not_windows(&id, d.name.clone(), lang));
            continue;
        }
        let version = if d.version.is_empty() { String::new() } else { format!(" · {}", d.version) };
        let repair = |text: String| act(text, "repair", &format!("driver:{}", d.id), 3, driver_effect(&d.id, lang));
        if !d.installed {
            if d.required {
                items.push(item(&id, "warn", d.name.clone(), tr(lang, "Not installed", "没有安装"), repair(tr(lang, "Install it.", "安装它。"))));
            } else {
                items.push(item(&id, "skip", d.name.clone(), tr(lang, "Not checked: not installed", "未检查:没有安装"), Fix::default()));
            }
            continue;
        }
        if d.reboot || d.problem == Some(14) {
            items.push(item(&id, "warn", d.name.clone(), tr(lang, format!("Installed; Windows needs a restart to start it{version}"), format!("已安装;需要重启 Windows 才能启用{version}")),
                fix(tr(lang, "Restart Windows.", "重启 Windows。"))));
        } else if let Some(code) = d.problem.filter(|c| *c != 0) {
            let mut text = tr(lang, "Repair the driver.", "修复驱动。");
            if !d.note.is_empty() { text = format!("{text} {}", d.note); }
            items.push(item(&id, "fail", d.name.clone(), format!("{}{version}", problem_words(code, lang)), repair(text)));
        } else if !d.running {
            items.push(item(&id, "fail", d.name.clone(), tr(lang, format!("Installed but not running{version}"), format!("已安装但没有运行{version}")), repair(tr(lang, "Repair the driver.", "修复驱动。"))));
        } else {
            let mut detail = tr(lang, format!("Running · device OK{version}"), format!("运行中 · 设备正常{version}"));
            if !d.note.is_empty() { detail = format!("{detail} · {}", d.note); }
            items.push(item(&id, "pass", d.name.clone(), detail, Fix::default()));
        }
    }
    items
}

// ---- the report ----

/// Chinese punctuation beside Chinese text, the way the host writes it: ",;:" beside a Han character or a full-width mark, "?" and ")"
/// after one, "(" beside one become full width; Latin text keeps its own marks.
pub fn zh_punctuation(text: &str) -> String {
    let wide = |c: char| ('\u{4e00}'..='\u{9fff}').contains(&c) || ('\u{3000}'..='\u{303f}').contains(&c) || ('\u{ff00}'..='\u{ffef}').contains(&c) || c == '“' || c == '”';
    let mut chars: Vec<char> = text.chars().collect();
    for i in 0..chars.len() {
        let before = i > 0 && wide(chars[i - 1]);
        let after = i + 1 < chars.len() && wide(chars[i + 1]);
        chars[i] = match chars[i] {
            ',' if before || after => '，',
            ';' if before || after => '；',
            ':' if before || after => '：',
            '?' if before => '？',
            '(' if before || after => '（',
            ')' if before => '）',
            c => c,
        };
    }
    let mut open = false;
    for c in chars.iter_mut() {
        match *c {
            '（' => open = true,
            ')' if open => { *c = '）'; open = false; }
            '）' => open = false,
            _ => {}
        }
    }
    chars.into_iter().collect()
}

fn localize(mut items: Vec<Item>, lang: Lang) -> Vec<Item> {
    if lang == Lang::Zh {
        for i in &mut items {
            i.title = zh_punctuation(&i.title);
            i.detail = zh_punctuation(&i.detail);
            i.fix.text = zh_punctuation(&i.fix.text);
            if let Some(a) = &mut i.fix.action {
                a.effect = zh_punctuation(&a.effect);
            }
        }
    }
    items
}

fn group_title(id: &str, lang: Lang) -> String {
    match id {
        "service" => tr(lang, "Sunshine service", "Sunshine 服务"),
        "install" => tr(lang, "Installation", "安装"),
        "process" => tr(lang, "Processes and ports", "进程与端口"),
        "drivers" => tr(lang, "Drivers", "驱动"),
        _ => id.to_string(),
    }
}

fn worst(items: &[Item]) -> &'static str {
    let rank = |s: &str| match s { "fail" => 3, "warn" => 2, "pass" => 1, _ => 0 };
    items.iter().map(|i| i.status).max_by_key(|s| rank(s)).unwrap_or("skip")
}

/// One group as the host writes a report (GET /api/doctor), marked as local.
pub fn report(group: &str, items: Vec<Item>, elapsed_ms: u64, lang: Lang, checked_at: &str) -> Value {
    let items = localize(items, lang);
    let mut summary = json!({ "pass": 0, "warn": 0, "fail": 0, "skip": 0 });
    for i in &items {
        summary[i.status] = json!(summary[i.status].as_i64().unwrap_or(0) + 1);
    }
    json!({
        "version": 1,
        "source": "local",
        "checked_at": checked_at,
        "elapsed_ms": elapsed_ms,
        "summary": summary,
        "groups": [{ "id": group, "title": group_title(group, lang), "status": worst(&items), "elapsed_ms": elapsed_ms, "timed_out": false, "items": items.iter().map(|i| i.id.clone()).collect::<Vec<_>>() }],
        "items": items,
    })
}

// ---- collectors (Windows) ----

#[cfg(target_os = "windows")]
mod collect {
    use super::*;
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use std::time::Duration;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    /// `netstat -ano` walks the endpoint tables and does not return on its own when those tables are wedged
    /// (two Sunshine processes holding sockets is the case that left the health check running forever).
    /// Five seconds, then the child is ended and this probe fails on its own [dr-ms-netstat].
    fn output_bounded(mut command: Command, budget: Duration) -> Result<std::process::Output, String> {
        use std::process::Stdio;
        let mut child = command.stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|error| error.to_string())?;
        let started = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => return child.wait_with_output().map_err(|error| error.to_string()),
                Ok(None) if started.elapsed() >= budget => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("no answer within {} s", budget.as_secs()));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(error) => return Err(error.to_string()),
            }
        }
    }

    pub fn host_pids() -> Vec<u32> {
        crate::device_probe::system_processes()
            .map(|list| list.into_iter().filter(|p| p.name.eq_ignore_ascii_case("sunshine.exe")).map(|p| p.pid).collect())
            .unwrap_or_default()
    }

    fn scm() -> Option<(String, u32, u32, u32)> {
        use windows::Win32::System::Services::*;
        use windows::core::{Owned, PCWSTR, w};
        unsafe {
            let manager = Owned::new(OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT).ok()?);
            let service = Owned::new(OpenServiceW(*manager, w!("SunshineService"), SERVICE_QUERY_STATUS).ok()?);
            let mut status = SERVICE_STATUS_PROCESS::default();
            let mut needed = 0;
            let buffer = std::slice::from_raw_parts_mut(std::ptr::from_mut(&mut status).cast::<u8>(), std::mem::size_of::<SERVICE_STATUS_PROCESS>());
            QueryServiceStatusEx(*service, SC_STATUS_PROCESS_INFO, Some(buffer), &mut needed).ok()?;
            let state = match status.dwCurrentState {
                SERVICE_RUNNING => "RUNNING",
                SERVICE_STOPPED => "STOPPED",
                SERVICE_START_PENDING => "START_PENDING",
                SERVICE_STOP_PENDING => "STOP_PENDING",
                SERVICE_PAUSED => "PAUSED",
                SERVICE_PAUSE_PENDING => "PAUSE_PENDING",
                SERVICE_CONTINUE_PENDING => "CONTINUE_PENDING",
                _ => "UNKNOWN",
            };
            Some((state.to_string(), status.dwProcessId, status.dwWin32ExitCode, status.dwServiceSpecificExitCode))
        }
    }

    fn registry() -> (String, String) {
        use winreg::RegKey;
        use winreg::enums::*;
        let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(r"SYSTEM\CurrentControlSet\Services\SunshineService", KEY_READ | KEY_WOW64_64KEY) else {
            return (String::new(), String::new());
        };
        let start: u32 = key.get_value("Start").unwrap_or(0);
        let delayed: u32 = key.get_value("DelayedAutoStart").unwrap_or(0);
        let start_type = match (start, delayed) {
            (2, 1) => "delayed",
            (2, _) => "auto",
            (3, _) => "demand",
            (4, _) => "disabled",
            _ => "",
        };
        (start_type.to_string(), key.get_value::<String, _>("ImagePath").unwrap_or_default())
    }

    async fn api_answers() -> bool {
        let Ok(base) = crate::sunshine::get_local_sunshine_url().await else { return false };
        let Ok(client) = crate::sunshine::create_https_client() else { return false };
        // any HTTP answer (200, 401) means the host's server is up
        matches!(tokio::time::timeout(Duration::from_secs(3), client.get(format!("{}/api/config", base.trim_end_matches('/'))).send()).await, Ok(Ok(_)))
    }

    pub async fn service() -> ServiceFacts {
        let mut f = ServiceFacts { checked: true, host_pids: host_pids(), ..Default::default() };
        if let Some((state, pid, win32, specific)) = scm() {
            f.exists = true;
            f.state = state;
            f.service_pid = pid;
            f.win32_exit = win32;
            f.service_exit = specific;
            let (start_type, image) = registry();
            f.start_type = start_type;
            f.image_path = image;
            f.image_matches_install = crate::desktop_settings::sunshine_service_matches_install(&crate::sunshine::install_dir()).ok();
        }
        f.api_answers = Some(api_answers().await);
        f
    }

    fn build_info(exe: &std::path::Path) -> Option<BuildInfo> {
        let mut child = Command::new(exe).args(["--version", "--json"]).creation_flags(CREATE_NO_WINDOW).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn().ok()?;
        // an older host does not know --json and may start up instead of printing: give it a moment, then end it
        for _ in 0..30 {
            if let Ok(Some(_)) = child.try_wait() {
                let out = child.wait_with_output().ok()?;
                return parse_build_info(&crate::text_encoding::decode_console_bytes(&out.stdout));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = child.kill();
        let _ = child.wait();
        None
    }

    pub fn install() -> InstallFacts {
        let dir = crate::sunshine::install_dir();
        let exe = dir.join("sunshine.exe");
        let exe_found = exe.is_file();
        InstallFacts {
            checked: true,
            dir: dir.display().to_string(),
            exe_found,
            file_version: if exe_found { crate::device_probe::file_version_string(&exe).unwrap_or_default() } else { String::new() },
            build: if exe_found { build_info(&exe) } else { None },
            config_found: crate::sunshine::config_dir().join("sunshine.conf").is_file(),
        }
    }

    pub async fn process() -> ProcessFacts {
        let base_port = crate::sunshine::parse_sunshine_config().await.ok().and_then(|c| c.port).and_then(|p| p.trim().parse().ok()).unwrap_or(47989);
        let (listeners, port_table_error) = match output_bounded({ let mut netstat = Command::new("netstat"); netstat.args(["-ano"]).creation_flags(CREATE_NO_WINDOW); netstat }, Duration::from_secs(5)) {
            Ok(output) => (parse_netstat(&crate::text_encoding::decode_console_bytes(&output.stdout)), String::new()),
            Err(error) => (Vec::new(), error),
        };
        let names = crate::device_probe::system_processes().map(|list| list.into_iter().map(|p| (p.pid, p.name)).collect()).unwrap_or_default();
        let log = crate::sunshine::config_dir().join("sunshine.log");
        let text = read_tail(&log, 256 * 1024);
        let (last_error, clean_end) = text.as_deref().map(scan_log_tail).unwrap_or((None, true));
        ProcessFacts {
            checked: true,
            base_port,
            listeners,
            names,
            host_pids: host_pids(),
            log_found: text.is_some(),
            log_path: log.display().to_string(),
            last_error,
            ended_abruptly: text.is_some() && !clean_end,
            port_table_error,
        }
    }

    fn read_tail(path: &std::path::Path, bytes: u64) -> Option<String> {
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::fs::File::open(path).ok()?;
        let len = file.metadata().ok()?.len();
        file.seek(SeekFrom::Start(len.saturating_sub(bytes))).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;
        Some(String::from_utf8_lossy(&buffer).into_owned())
    }

    pub async fn drivers(lang: Lang) -> Vec<DriverFact> {
        let mut out = Vec::new();
        let vdd_name = tr(lang, "Foundation Virtual Display", "基地虚拟显示器");
        match tokio::time::timeout(Duration::from_secs(8), crate::vdd::get_vdd_status()).await {
            Ok(Ok(s)) => out.push(DriverFact {
                id: "vdd".into(), name: vdd_name.clone(), checked: true, installed: s.installed,
                running: s.running && s.control_available, problem: None, version: s.installed_version.clone(), reboot: s.state == "reboot_required",
                note: if s.installed && !s.version_match && !s.bundled_version.is_empty() { tr(lang, format!("the bundled driver is {}", s.bundled_version), format!("随附驱动是 {}", s.bundled_version)) } else { String::new() },
                required: true, probe_error: String::new(),
            }),
            Err(_) => out.push(DriverFact { id: "vdd".into(), name: vdd_name, checked: true, required: true, probe_error: "no answer within 8 s".into(), ..Default::default() }),
            Ok(Err(_)) => out.push(DriverFact { id: "vdd".into(), name: tr(lang, "Foundation Virtual Display", "基地虚拟显示器"), ..Default::default() }),
        }
        match tokio::time::timeout(Duration::from_secs(8), crate::vigem::get_vigem_status()).await {
            Ok(Ok(s)) => out.push(DriverFact {
                id: "vigem".into(), name: "ViGEmBus".into(), checked: true, installed: s.installed, running: s.running && s.version_ok, problem: None, version: s.version.clone(), reboot: false,
                note: tr(lang, "Upstream no longer maintains ViGEmBus; DualSense can go over USB instead.", "上游已停止维护 ViGEmBus,DualSense 可以改走 USB 方式。"), required: false, probe_error: String::new(),
            }),
            Err(_) => out.push(DriverFact { id: "vigem".into(), name: "ViGEmBus".into(), checked: true, probe_error: "no answer within 8 s".into(), ..Default::default() }),
            Ok(Err(_)) => out.push(DriverFact { id: "vigem".into(), name: "ViGEmBus".into(), ..Default::default() }),
        }
        if crate::vmouse::get_vmouse_support() {
            let vmouse_name = tr(lang, "Virtual mouse", "虚拟鼠标");
            match tokio::time::timeout(Duration::from_secs(8), crate::vmouse::get_vmouse_status()).await {
                Ok(Ok(s)) => out.push(DriverFact { id: "vmouse".into(), name: vmouse_name.clone(), checked: true, installed: s.installed, running: s.running, problem: None, version: String::new(), reboot: false, note: String::new(), required: false, probe_error: String::new() }),
                Err(_) => out.push(DriverFact { id: "vmouse".into(), name: vmouse_name, checked: true, probe_error: "no answer within 8 s".into(), ..Default::default() }),
                Ok(Err(_)) => out.push(DriverFact { id: "vmouse".into(), name: tr(lang, "Virtual mouse", "虚拟鼠标"), ..Default::default() }),
            }
        }
        match tokio::time::timeout(Duration::from_secs(8), crate::usbip::usbip_get_status()).await {
            Ok(Ok(s)) if s.supported => out.push(DriverFact {
                id: "usbip".into(), name: "USB/IP (usbip-win2)".into(), checked: true, installed: s.installed, running: s.ready, problem: None, version: s.version.clone(), reboot: s.reboot_recommended,
                note: if s.error_code.is_empty() { String::new() } else { s.error_code.clone() }, required: false, probe_error: String::new(),
            }),
            Err(_) => out.push(DriverFact { id: "usbip".into(), name: "USB/IP (usbip-win2)".into(), checked: true, probe_error: "no answer within 8 s".into(), ..Default::default() }),
            _ => out.push(DriverFact { id: "usbip".into(), name: "USB/IP (usbip-win2)".into(), ..Default::default() }),
        }
        // the PnP problem code of each device node, as Device Manager shows it
        for d in out.iter_mut().filter(|d| d.installed) {
            d.problem = problem_code(&d.id);
        }
        out
    }

    fn problem_code(id: &str) -> Option<u32> {
        use crate::device_probe::{self as probe, NodeState};
        let (enumerator, wanted): (&str, &[&str]) = match id {
            "vdd" => ("DISPLAY", &["Root\\ZakoVDD", "ZakoVDD"]),
            "vmouse" => ("HIDCLASS", &["Root\\ZakoVirtualMouse"]),
            "usbip" => ("USBIP_WIN2", &["ROOT\\USBIP_WIN2\\UDE"]),
            _ => return None,
        };
        let devices = probe::enum_root_devices(enumerator).ok()?;
        let device = devices.into_iter().find(|d| d.hardware_ids.iter().any(|h| wanted.iter().any(|w| h.eq_ignore_ascii_case(w))))?;
        match probe::node_state(&device.instance_id) {
            NodeState::Error(code) => Some(code),
            _ => None,
        }
    }
}

/// The checks of one group, in the report form of `GET /api/doctor`.
#[tauri::command]
pub async fn doctor_local(only: String, lang: Option<String>) -> Result<Value, String> {
    let lang = Lang::parse(lang.as_deref().unwrap_or("en"));
    if !GROUPS.contains(&only.as_str()) {
        return Err(format!("Unknown group: {only}"));
    }
    let started = std::time::Instant::now();
    #[cfg(target_os = "windows")]
    let items = match only.as_str() {
        "service" => evaluate_service(&collect::service().await, lang),
        "install" => evaluate_install(&tokio::task::spawn_blocking(collect::install).await.map_err(|e| e.to_string())?, lang),
        "process" => evaluate_process(&collect::process().await, lang),
        _ => evaluate_drivers(&collect::drivers(lang).await, lang),
    };
    #[cfg(not(target_os = "windows"))]
    let items = match only.as_str() {
        "service" => evaluate_service(&ServiceFacts::default(), lang),
        "install" => evaluate_install(&InstallFacts::default(), lang),
        "process" => evaluate_process(&ProcessFacts::default(), lang),
        _ => evaluate_drivers(&[DriverFact { id: "vdd".into(), name: tr(lang, "Foundation Virtual Display", "基地虚拟显示器"), ..Default::default() }], lang),
    };
    let checked_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    Ok(report(&only, items, started.elapsed().as_millis() as u64, lang, &checked_at))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find<'a>(items: &'a [Item], id: &str) -> &'a Item {
        items.iter().find(|i| i.id == id).unwrap_or_else(|| panic!("no {id}"))
    }

    /// Every warn and fail has a fix; every acting fix says what it does; every skip says why
    fn contract(items: &[Item]) {
        for i in items {
            assert!(i.id.starts_with(&format!("{}.", i.group)), "{}", i.id);
            assert_eq!(i.source, "local");
            if matches!(i.status, "warn" | "fail") {
                assert!(!i.fix.text.is_empty(), "{} has no fix", i.id);
            }
            if let Some(a) = &i.fix.action {
                if a.level >= 2 {
                    assert!(a.confirm && a.effect.len() > 10, "{} acts without saying what it does", i.id);
                }
            }
            if i.status == "skip" {
                assert!(i.detail.starts_with("未检查") || i.detail.starts_with("Not checked"), "{}: {}", i.id, i.detail);
            }
        }
    }

    fn running() -> ServiceFacts {
        ServiceFacts { checked: true, exists: true, state: "RUNNING".into(), service_pid: 900, start_type: "auto".into(), image_matches_install: Some(true), host_pids: vec![8124], api_answers: Some(true), ..Default::default() }
    }

    #[test]
    fn three_layers_running_passes() {
        let items = evaluate_service(&running(), Lang::Zh);
        assert_eq!(find(&items, "service.state").status, "pass");
        assert!(find(&items, "service.state").detail.contains("PID 8124"));
        assert_eq!(find(&items, "service.start_type").status, "pass");
        contract(&items);
    }

    #[test]
    fn wrapper_alive_but_host_gone_fails_with_a_level_two_start() {
        let f = ServiceFacts { host_pids: vec![], api_answers: Some(false), ..running() };
        let items = evaluate_service(&f, Lang::Zh);
        let state = find(&items, "service.state");
        assert_eq!(state.status, "fail");
        assert!(state.detail.contains("已经退出"));
        let a = state.fix.action.as_ref().unwrap();
        assert_eq!((a.kind.as_str(), a.target.as_str(), a.level), ("repair", "service:start", 2));
        contract(&items);
    }

    #[test]
    fn stopped_service_fails_and_manual_start_warns_with_a_level_three_fix() {
        let f = ServiceFacts { state: "STOPPED".into(), host_pids: vec![], api_answers: Some(false), start_type: "demand".into(), win32_exit: 1066, service_exit: 3, ..running() };
        let items = evaluate_service(&f, Lang::En);
        assert_eq!(find(&items, "service.state").status, "fail");
        let start = find(&items, "service.start_type");
        assert_eq!(start.status, "warn");
        assert_eq!(start.fix.action.as_ref().unwrap().level, 3);
        assert!(find(&items, "service.last_exit").detail.contains("Exit code 3"));
        assert_eq!(find(&items, "service.recovery").status, "skip");
        contract(&items);
    }

    #[test]
    fn no_service_and_another_installation() {
        let items = evaluate_service(&ServiceFacts { checked: true, ..Default::default() }, Lang::Zh);
        assert_eq!(find(&items, "service.state").status, "fail");
        let items = evaluate_service(&ServiceFacts { image_matches_install: Some(false), image_path: "D:\\Old\\sunshinesvc.exe".into(), ..running() }, Lang::Zh);
        assert_eq!(find(&items, "service.image_path").status, "fail");
        contract(&items);
        assert_eq!(evaluate_service(&ServiceFacts::default(), Lang::Zh)[0].status, "skip");
    }

    #[test]
    fn netstat_listeners_do_not_depend_on_the_language() {
        let text = "\r\n活动连接\r\n\r\n  协议  本地地址          外部地址        状态           PID\r\n  TCP    0.0.0.0:47984          0.0.0.0:0              侦听           8124\r\n  TCP    0.0.0.0:47990          0.0.0.0:0              LISTENING       4420\r\n  TCP    192.0.2.5:47984      192.0.2.9:51234      ESTABLISHED     8124\r\n  TCP    [::]:47989             [::]:0                 LISTENING       8124\r\n  UDP    0.0.0.0:47998          *:*                                    8124\r\n";
        let l = parse_netstat(text);
        assert_eq!(l, vec![
            Listener { proto: "tcp".into(), port: 47984, pid: 8124 },
            Listener { proto: "tcp".into(), port: 47990, pid: 4420 },
            Listener { proto: "tcp".into(), port: 47989, pid: 8124 },
            Listener { proto: "udp".into(), port: 47998, pid: 8124 },
        ]);
    }

    #[test]
    fn ports_taken_by_another_program_fail_and_name_it() {
        let f = ProcessFacts {
            checked: true, base_port: 47989,
            listeners: vec![Listener { proto: "tcp".into(), port: 47990, pid: 4420 }, Listener { proto: "tcp".into(), port: 47984, pid: 8124 }],
            names: vec![(4420, "nginx.exe".into()), (8124, "sunshine.exe".into())],
            host_pids: vec![8124], log_found: true, ..Default::default()
        };
        let items = evaluate_process(&f, Lang::Zh);
        let ports = find(&items, "process.ports");
        assert_eq!(ports.status, "fail");
        assert!(ports.detail.contains("tcp 47990 (web): nginx.exe (PID 4420)"), "{}", ports.detail);
        assert_eq!(ports.fix.action.as_ref().unwrap().target, "setting:port");
        contract(&items);
    }

    #[test]
    fn ports_follow_the_base_port_and_udp_waits_for_a_stream() {
        assert_eq!(sunshine_ports(50000).iter().map(|p| p.1).collect::<Vec<_>>(), vec![49995, 50000, 50001, 50021, 50009, 50010, 50011, 50013]);
        let tcp = |port| Listener { proto: "tcp".into(), port, pid: 8124 };
        let f = ProcessFacts { checked: true, base_port: 47989, listeners: vec![tcp(47984), tcp(47989), tcp(47990), tcp(48010)], host_pids: vec![8124], log_found: true, ..Default::default() };
        assert_eq!(find(&evaluate_process(&f, Lang::Zh), "process.ports").status, "pass");
        let f = ProcessFacts { listeners: vec![tcp(47984)], ..f };
        assert_eq!(find(&evaluate_process(&f, Lang::Zh), "process.ports").status, "warn");
        let f = ProcessFacts { checked: true, base_port: 47989, log_found: true, ..Default::default() };
        assert_eq!(find(&evaluate_process(&f, Lang::Zh), "process.ports").status, "pass", "nothing listens and Sunshine is down: the ports are free");
    }

    #[test]
    fn log_tail_finds_the_last_error_and_an_abrupt_end() {
        let log = "[2026-11-04 10:00:00.000]: Info: Sunshine version: x\n[2026-11-04 10:00:01.000]: Error: Couldn't bind http server to ports [47989, 47990]\n[2026-11-04 10:00:02.000]: Info: still going\n";
        let (last, clean) = scan_log_tail(log);
        assert_eq!(last.unwrap(), ("2026-11-04 10:00:01.000".into(), "Error".into(), "Couldn't bind http server to ports [47989, 47990]".into()));
        assert!(!clean);
        let (last, clean) = scan_log_tail("[2026-11-04 10:00:00.000]: Info: Goodbye\n");
        assert!(last.is_none() && clean);
        let f = ProcessFacts { checked: true, base_port: 47989, log_found: true, last_error: scan_log_tail(log).0, ended_abruptly: true, ..Default::default() };
        let items = evaluate_process(&f, Lang::Zh);
        assert_eq!(find(&items, "process.log").status, "warn");
        assert_eq!(find(&items, "process.last_run").status, "warn");
        contract(&items);
    }

    #[test]
    fn two_sunshine_processes_fail_on_their_own_and_a_silent_port_table_is_not_a_pass() {
        let f = ProcessFacts {
            checked: true,
            base_port: 47989,
            host_pids: vec![100, 200],
            log_found: true,
            ..Default::default()
        };
        let items = evaluate_process(&f, Lang::Zh);
        assert_eq!(find(&items, "process.instances").status, "fail");
        assert!(find(&items, "process.instances").detail.contains("2"));
        let silent = ProcessFacts {
            checked: true,
            base_port: 47989,
            port_table_error: "no answer within 5 s".into(),
            log_found: true,
            ..Default::default()
        };
        let silent_items = evaluate_process(&silent, Lang::Zh);
        let ports = find(&silent_items, "process.ports");
        assert_eq!(ports.status, "warn");
        assert!(ports.detail.contains("no answer within 5 s"));
        contract(&items);
    }

    #[test]
    fn a_driver_probe_that_does_not_finish_is_a_warning_not_not_installed() {
        let drivers = [DriverFact {
            id: "vdd".into(),
            name: "基地虚拟显示器".into(),
            checked: true,
            required: true,
            probe_error: "no answer within 8 s".into(),
            ..Default::default()
        }];
        let driver_items = evaluate_drivers(&drivers, Lang::Zh);
        let item = find(&driver_items, "drivers.vdd");
        assert_eq!(item.status, "warn");
        assert!(item.detail.contains("8"));
        assert!(!item.detail.contains("没有安装"));
    }

    #[test]
    fn build_info_parses_the_json_line_and_falls_back() {
        let out = "some banner\n{\"version\":\"2026.1104.1\",\"commit\":\"96a1e2b6c0ffee00112233445566778899aabbcc\",\"channel\":\"beta\",\"dirty\":false,\"build_time_utc\":\"2026-11-04T04:18:13Z\",\"variant\":\"product-zero\"}\n";
        let b = parse_build_info(out).unwrap();
        assert_eq!(b.channel, "beta");
        assert!(parse_build_info("Sunshine version: 2026.1104").is_none());
        let items = evaluate_install(&InstallFacts { checked: true, dir: "C:\\Program Files\\Sunshine".into(), exe_found: true, build: Some(b), config_found: true, ..Default::default() }, Lang::Zh);
        assert_eq!(find(&items, "install.version").status, "pass");
        assert!(find(&items, "install.version").detail.contains("96a1e2b6c0"));
        let items = evaluate_install(&InstallFacts { checked: true, dir: "C:\\S".into(), exe_found: true, file_version: "2026.1104.121813".into(), config_found: false, ..Default::default() }, Lang::Zh);
        assert!(find(&items, "install.version").detail.contains("还没合入"));
        assert_eq!(find(&items, "install.config").status, "warn");
        contract(&items);
    }

    #[test]
    fn drivers_problem_codes_reboot_and_optional_ones() {
        let d = |id: &str, installed, running, problem: Option<u32>, required| DriverFact { id: id.into(), name: id.into(), checked: true, installed, running, problem, required, ..Default::default() };
        let items = evaluate_drivers(&[d("vdd", true, true, None, true), d("vigem", true, false, Some(28), false), d("vmouse", false, false, None, false), d("usbip", true, false, Some(14), false)], Lang::Zh);
        assert_eq!(find(&items, "drivers.vdd").status, "pass");
        let vigem = find(&items, "drivers.vigem");
        assert_eq!(vigem.status, "fail");
        assert!(vigem.detail.contains("问题代码 28(驱动没有安装)"));
        let a = vigem.fix.action.as_ref().unwrap();
        assert_eq!((a.target.as_str(), a.level), ("driver:vigem", 3));
        assert!(a.effect.contains("DS4Windows"));
        assert_eq!(find(&items, "drivers.vmouse").status, "skip");
        assert_eq!(find(&items, "drivers.usbip").status, "warn");
        contract(&items);
        let missing_vdd = evaluate_drivers(&[d("vdd", false, false, None, true)], Lang::En);
        assert_eq!(missing_vdd[0].status, "warn");
        assert!(driver_effect("vmouse", Lang::En).contains("stops SunshineService"));
    }

    #[test]
    fn chinese_punctuation_beside_chinese_text() {
        assert_eq!(zh_punctuation("未检查:只在 Windows 上"), "未检查：只在 Windows 上");
        assert_eq!(zh_punctuation("问题代码 28(驱动没有安装)"), "问题代码 28（驱动没有安装）");
        assert_eq!(zh_punctuation("tcp 47990 (web): nginx.exe (PID 4420)"), "tcp 47990 (web): nginx.exe (PID 4420)");
        assert_eq!(zh_punctuation("用随产品附带的安装包(已校验 SHA-256)重新安装"), "用随产品附带的安装包（已校验 SHA-256）重新安装");
    }

    #[test]
    fn the_report_is_the_host_contract_with_source_local() {
        let items = evaluate_service(&running(), Lang::Zh);
        let r = report("service", items, 12, Lang::Zh, "2026-11-04T00:00:00Z");
        assert_eq!(r["version"], 1);
        assert_eq!(r["source"], "local");
        assert_eq!(r["groups"][0]["id"], "service");
        assert_eq!(r["groups"][0]["title"], "Sunshine 服务");
        assert_eq!(r["groups"][0]["status"], "pass");
        assert_eq!(r["items"][0]["source"], "local");
        assert!(r["items"][0]["fix"].get("action").is_none());
        assert_eq!(r["summary"]["skip"], 1);
    }
}
