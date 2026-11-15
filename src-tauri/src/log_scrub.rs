//! Anonymisation of a session log or a log tail, on the client. Port of the Worker's `src/logscrub.ts`; the two are held together by ONE shared vector file
//! (`testdata/log-scrub-vectors.json`, identical copies in the Worker, the host and the Panel): every implementation must remove each `forbidden` string and keep each `required` one.
//!
//! What goes: user and computer names, the user segment of paths, IPv4/IPv6 and MAC addresses (each distinct value becomes a stable placeholder `<ip#1>`, `<mac#1>` ... inside one report, so lines can
//! still be correlated), e-mail addresses, serial numbers and device instance ids, tokens, keys, cookies, pairing PINs, Wi-Fi network names, game account names.
//! What stays: timestamps, levels, components, codes, numbers, versions, GPU and driver strings and every config value that is not a secret.
//!
//! Why `fancy-regex`: the rules need look-around (an address is not part of a longer token, a name is not followed by `=`), which the linear-time `regex` crate does not have. The patterns have no nested
//! quantifiers over the same text and each line is cut at `MAX_LINE_IN` characters before it is scanned, so a backtracking blow-up has no input to grow on; if the engine reports an error anyway
//! (backtrack limit) the whole line is replaced by `<line removed>`: the scrubber fails closed.
//!
//! This module has no Tauri or OS dependency: it is the one scrubber of the Panel shell, shared by the error capture (`error_capture.rs`) and by log upload.
use fancy_regex::{Captures, Regex};
use std::collections::HashMap;
use std::sync::LazyLock;

/// A line is cut to this many characters before any rule runs (bounds the work of the look-around rules).
const MAX_LINE_IN: usize = 4000;
const LINE_REMOVED: &str = "<line removed>";

#[derive(Debug, Clone)]
pub struct Options {
    /// The tail of the log that is kept, in characters.
    pub max_chars: usize,
    /// A single line is cut here.
    pub max_line: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self { max_chars: 2_000_000, max_line: 600 }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Scrubbed {
    pub text: String,
    pub lines: usize,
    pub truncated: bool,
    pub replaced: HashMap<String, usize>,
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|e| panic!("log_scrub pattern {pattern:?}: {e}"))
}

// the words of the key part that make a value a secret: the key may carry a prefix or suffix (api_key, authToken, pairing_pin)
const SECRET_KEY: &str = r"(?:[\w.-]*(?:token|secret|passwd|password|passphrase|pwd|apikey|api[_-]?key|private[_-]?key|credential|cookie|authorization|auth|passcode|cdk|salt|nonce)[\w.-]*|(?:[\w.-]*[_.-])?pin|pairing[_-]?pin)";
const SERIAL_KEY: &str = r"(?:[\w.-]*serial(?:[_ -]?(?:number|no))?|sn|s/n|[\w.-]*(?:device|hardware|machine|instance|unique|monitor|display)[_ -]?id|udid|imei|edid[_ -]?(?:serial|hash))";
const ACCOUNT_KEY: &str = r"(?:[\w.-]*(?:account|nickname|nick[_ -]?name|player[_ -]?name|role[_ -]?name|user[_ -]?name|username|login|owner|steam[_ -]?id|uid|user[_ -]?id|profile[_ -]?name)[\w.-]*)";
const SSID_KEY: &str = r"(?:ssid|essid|wi-?fi(?:[_ -]?(?:name|network))?|wlan[_ -]?name|network[_ -]?name|profile)";
const HOST_KEY: &str = r"(?:host[_ -]?name|computer[_ -]?name|machine[_ -]?name|device[_ -]?name|node[_ -]?name|netbios[_ -]?name|pc[_ -]?name)";

// key [:=] value, where the value is "quoted", 'quoted' or runs to a delimiter
fn kv(key: &str) -> Regex {
    re(&format!(r#"(?i)\b({key})(["']?\s*[:=]\s*)("[^"\r\n]*"|'[^'\r\n]*'|[^\s,;&"'}}\]]+)"#))
}

static RE_SECRET: LazyLock<Regex> = LazyLock::new(|| kv(SECRET_KEY));
static RE_SERIAL: LazyLock<Regex> = LazyLock::new(|| kv(SERIAL_KEY));
static RE_ACCOUNT: LazyLock<Regex> = LazyLock::new(|| kv(ACCOUNT_KEY));
static RE_SSID: LazyLock<Regex> = LazyLock::new(|| kv(SSID_KEY));
static RE_HOSTKEY: LazyLock<Regex> = LazyLock::new(|| kv(HOST_KEY));

const IPV4: &str = r"(?<![\w.])(?:25[0-5]|2[0-4]\d|1?\d?\d)(?:\.(?:25[0-5]|2[0-4]\d|1?\d?\d)){3}(?![\w.]*\d)";
const IPV6: &str = r"(?<![\w:.])(?:(?:[0-9a-f]{1,4}:){7}[0-9a-f]{1,4}|(?:[0-9a-f]{1,4}(?::[0-9a-f]{1,4}){0,6})?::(?:[0-9a-f]{1,4}(?::[0-9a-f]{1,4}){0,6})?)(?![\w:])";
static IP_ANY: LazyLock<Regex> = LazyLock::new(|| re(&format!("(?i){IPV6}|{IPV4}")));
static MAC: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)(?<![\w:-])(?:[0-9a-f]{2}[:-]){5}[0-9a-f]{2}(?![\w:-])"));
static VERSIONISH: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)(?:version|ver\.?|v|driver|build|dll|sdk|api|firmware|fw)[\s:=/_-]*$"));
const KEEP_IPV4: [&str; 4] = ["0.0.0.0", "127.0.0.1", "255.255.255.255", "255.255.255.0"];

// values that are not a secret even under a secret-looking key: empty, booleans, numbers, "none", our own placeholders
static HARMLESS: LazyLock<Regex> = LazyLock::new(|| re(r#"(?i)^(?:["']?(?:|true|false|null|none|nil|empty|n/a|unknown|default|disabled|enabled|on|off|yes|no|-?\d{1,6}|<[^>]*>)["']?)$"#));
// a number under one of these keys is a credential (password=123456), unless the key says it counts or measures something (token_count, max_password_length)
static SECRET_NUM_KEY: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)pass|pwd|secret|token|credential|cookie|authorization|pin|salt|nonce"));
static METRIC_KEY: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)(?:^|[_.-])(?:count|len|length|size|timeout|ttl|ms|secs?|seconds|max|min|limit|interval|retries|retry|attempts|version|port|enabled|expires?|age|bytes|bits|rate|percent)(?:$|[_.-])"));
static NUMBER_VALUE: LazyLock<Regex> = LazyLock::new(|| re(r#"^["']?-?\d{1,6}["']?$"#));
static COOKIE_LINE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)(\b(?:set-)?cookie2?\b\s*:\s*)[^\r\n]+"));

static NAME_FROM_PATH: LazyLock<[Regex; 3]> = LazyLock::new(|| {
    [
        re(r#"\b[A-Za-z]:[\\/]Users[\\/]([^\\/\s"'<>|:*?]+(?: [^\\/\s"'<>|:*?]+)*?)(?=[\\/])"#),
        re(r#"/Users/([^/\s"'<>|]+)"#),
        re(r#"/home/([^/\s"'<>|]+)"#),
    ]
});
const PUBLIC_NAMES: [&str; 17] = [
    "public", "default", "default user", "all users", "defaultuser0", "system", "system32", "localservice", "networkservice", "administrator", "administrators", "guest", "shared", "user", "users", "root", "windows",
];
const PUBLIC_NAMES_MORE: [&str; 2] = ["appdata", "programdata"];

fn is_harmless(value: &str) -> bool {
    HARMLESS.is_match(value.trim()).unwrap_or(false)
}

fn is_control(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{8}' | '\u{b}' | '\u{c}' | '\u{e}'..='\u{1f}' | '\u{7f}'..='\u{9f}' | '\u{2028}' | '\u{2029}' | '\u{feff}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// Stable placeholders: the same value is the same `<kind#n>` inside one report (case-insensitive), another value the next number.
#[derive(Default)]
struct Labels {
    maps: HashMap<String, HashMap<String, usize>>,
    count: HashMap<String, usize>,
}

impl Labels {
    fn of(&mut self, kind: &str, value: &str) -> String {
        let map = self.maps.entry(kind.to_string()).or_default();
        let next = map.len() + 1;
        let n = *map.entry(value.to_lowercase()).or_insert(next);
        *self.count.entry(kind.to_string()).or_insert(0) += 1;
        format!("<{kind}#{n}>")
    }

    fn bump(&mut self, kind: &str) {
        *self.count.entry(kind.to_string()).or_insert(0) += 1;
    }
}

/// `regex.replace_all` with a closure that may fail: an engine error aborts the whole line (the caller removes it).
fn sub(regex: &Regex, text: &str, mut f: impl FnMut(&Captures) -> String) -> Result<String, fancy_regex::Error> {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for caps in regex.captures_iter(text) {
        let caps = caps?;
        let whole = caps.get(0).expect("group 0");
        out.push_str(&text[last..whole.start()]);
        out.push_str(&f(&caps));
        last = whole.end();
    }
    out.push_str(&text[last..]);
    Ok(out)
}

fn group<'a>(caps: &'a Captures, n: usize) -> &'a str {
    caps.get(n).map_or("", |m| m.as_str())
}

fn strip_quotes(value: &str) -> &str {
    value.trim_matches(|c| c == '"' || c == '\'')
}

fn escape(text: &str) -> String {
    fancy_regex::escape(text).into_owned()
}

/// Names the log itself reveals: the user segment of profile paths and the value of "user ..." / computer name keys.
fn learn_names(text: &str) -> (Vec<String>, Vec<String>) {
    let mut users: Vec<String> = Vec::new();
    let mut hosts: Vec<String> = Vec::new();
    let account_user = re(r"(?i)^(?:[\w.-]*(?:user[_ -]?name|username|login|owner))$");
    for pattern in NAME_FROM_PATH.iter() {
        for caps in pattern.captures_iter(text).flatten() {
            users.push(group(&caps, 1).to_string());
        }
    }
    for caps in RE_ACCOUNT.captures_iter(text).flatten() {
        if account_user.is_match(group(&caps, 1)).unwrap_or(false) {
            users.push(strip_quotes(group(&caps, 3)).to_string());
        }
    }
    for caps in RE_HOSTKEY.captures_iter(text).flatten() {
        hosts.push(strip_quotes(group(&caps, 3)).to_string());
    }
    for caps in re(r"\\\\([A-Za-z0-9_-]{2,63})(?=\\)").captures_iter(text).flatten() {
        hosts.push(group(&caps, 1).to_string());
    }
    for caps in re(r"\b((?:DESKTOP|LAPTOP|WIN|PC|NOTEBOOK)-[A-Z0-9]{5,13})\b").captures_iter(text).flatten() {
        hosts.push(group(&caps, 1).to_string());
    }
    let placeholder = re(r"^<.*>$");
    let clean = |list: Vec<String>| -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        for x in list {
            let x = x.trim().to_string();
            let low = x.to_lowercase();
            let n = x.chars().count();
            if (3..=64).contains(&n)
                && !PUBLIC_NAMES.contains(&low.as_str())
                && !PUBLIC_NAMES_MORE.contains(&low.as_str())
                && low != "windows"
                && !is_harmless(&x)
                && !placeholder.is_match(&x).unwrap_or(false)
                && !seen.contains(&x)
            {
                seen.push(x);
            }
        }
        seen
    };
    (clean(users), clean(hosts))
}

/// Anonymise `raw`. Never panics; a line the engine cannot scan is replaced, not passed.
pub fn scrub_log(raw: &str, options: &Options) -> Scrubbed {
    let mut text: String = raw.replace("\r\n", "\n").replace('\r', "\n").chars().filter(|c| !is_control(*c) || *c == '\n').collect();
    let mut truncated = false;
    let mut in_pem_at_start = false;
    let total = text.chars().count();
    if total > options.max_chars {
        // the cut may fall inside a key block: its body must not turn into ordinary lines
        let cut_byte = text.char_indices().nth(total - options.max_chars).map_or(text.len(), |(i, _)| i);
        let before = &text[..cut_byte];
        let begin = re(r"-----BEGIN [A-Z ]*(?:PRIVATE KEY|CERTIFICATE|PUBLIC KEY)[A-Z ]*-----");
        let end = re(r"-----END [A-Z ]+-----");
        let last_begin = begin.find_iter(before).flatten().last().map(|m| m.start());
        let last_end = end.find_iter(before).flatten().last().map(|m| m.start());
        in_pem_at_start = match (last_begin, last_end) {
            (Some(b), Some(e)) => e < b,
            (Some(_), None) => true,
            _ => false,
        };
        text = text[cut_byte..].to_string();
        text = match text.find('\n') {
            Some(i) => text[i + 1..].to_string(),
            None => String::new(),
        };
        truncated = true;
    }
    let mut labels = Labels::default();
    let (users, hosts) = learn_names(&text);

    // names first: every occurrence of a learned user or computer name, wherever it stands
    for (list, kind) in [(&users, "user"), (&hosts, "host")] {
        if list.is_empty() {
            continue;
        }
        let mut sorted = list.clone();
        sorted.sort_by_key(|s| std::cmp::Reverse(s.chars().count()));
        let alt = sorted.iter().map(|s| escape(s)).collect::<Vec<_>>().join("|");
        // an occurrence followed by = or : is a key (username=password, then password=...), not the name: it stays, so the key rules still see it
        let pattern = re(&format!(r"(?i)(?<![\w])(?:{alt})(?![\w])(?!\s*[:=])"));
        text = match sub(&pattern, &text, |caps| labels.of(kind, group(caps, 0))) {
            Ok(t) => t,
            Err(_) => return fail_closed(&text),
        };
    }

    let mut out: Vec<String> = Vec::new();
    let mut in_pem = in_pem_at_start;
    let pem_begin = re(r"-----BEGIN [A-Z ]*(?:PRIVATE KEY|CERTIFICATE|PUBLIC KEY)[A-Z ]*-----");
    let pem_end = re(r"-----END [A-Z ]+-----");
    let pem_tail = re(r"-----BEGIN[\s\S]*$");
    for line in text.split('\n') {
        if in_pem {
            if pem_end.is_match(line).unwrap_or(true) {
                in_pem = false;
            }
            continue;
        }
        if pem_begin.is_match(line).unwrap_or(true) {
            labels.bump("pem");
            if !pem_end.is_match(line).unwrap_or(true) {
                in_pem = true;
            }
            let head = pem_tail.replace(line, "").into_owned();
            out.push(scrub_line_safe(&head, &mut labels) + "<pem>");
            continue;
        }
        let mut done = scrub_line_safe(line, &mut labels);
        if done.chars().count() > options.max_line {
            done = done.chars().take(options.max_line - 1).collect::<String>() + "…";
        }
        out.push(done);
    }
    Scrubbed { lines: out.len(), text: out.join("\n"), truncated, replaced: labels.count }
}

fn fail_closed(text: &str) -> Scrubbed {
    let lines = text.split('\n').count();
    Scrubbed { text: vec![LINE_REMOVED; lines].join("\n"), lines, truncated: false, replaced: HashMap::from([("removed".to_string(), lines)]) }
}

fn scrub_line_safe(line: &str, labels: &mut Labels) -> String {
    let bounded: String = if line.chars().count() > MAX_LINE_IN { line.chars().take(MAX_LINE_IN).collect() } else { line.to_string() };
    match scrub_line(&bounded, labels) {
        Ok(done) => done,
        Err(_) => {
            labels.bump("removed");
            LINE_REMOVED.to_string()
        }
    }
}

fn scrub_line(line: &str, labels: &mut Labels) -> Result<String, fancy_regex::Error> {
    static QUERY: LazyLock<Regex> = LazyLock::new(|| re(r#"([?&][A-Za-z0-9_.-]+)=[^&\s#"']*"#));
    static URL_CRED: LazyLock<Regex> = LazyLock::new(|| re(r#"(?i)\bhttps?://[^\s/@"']+@"#));
    static JWT: LazyLock<Regex> = LazyLock::new(|| re(r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{4,}\b"));
    static BEARER: LazyLock<Regex> = LazyLock::new(|| re(r"\b(Bearer|Basic)\s+[A-Za-z0-9._~+/=-]{8,}"));
    static TOKEN_SHAPE: LazyLock<Regex> = LazyLock::new(|| re(r"\b(?:sk-[A-Za-z0-9_-]{12,}|gh[pousr]_[A-Za-z0-9]{20,}|xox[abp]-[A-Za-z0-9-]{10,}|AKIA[0-9A-Z]{16})\b"));
    static PIN_PHRASE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\b(PIN|passcode|pairing code)\b(\s+(?:is\s+)?)(\d{4,8})\b"));
    static INSTANCE: LazyLock<Regex> = LazyLock::new(|| {
        re(r#"(?i)\b((?:PCI|USB|HID|ACPI|SWD|ROOT|DISPLAY|MONITOR|BTHENUM|BTHLE|HDAUDIO|MMDEVAPI|SCSI|STORAGE|IDE|NVME)[\\#][^\\#\s"']+)[\\#](?!<)([^\\#\s"']+)"#)
    });
    static DEVICE_PATH: LazyLock<Regex> = LazyLock::new(|| re(r#"\b(\\\\\?\\)(?:[A-Za-z]+#)[^\s"']+"#));
    static UUID: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b"));
    static SID: LazyLock<Regex> = LazyLock::new(|| re(r"\bS-1-\d+(?:-\d+){2,}\b"));
    static STEAM: LazyLock<Regex> = LazyLock::new(|| re(r"\b7656119\d{10}\b"));
    static SSID_PHRASE: LazyLock<Regex> = LazyLock::new(|| re(r#"(?i)\b(SSID|Wi-?Fi)\b(\s+)("[^"\r\n]{1,64}"|'[^'\r\n]{1,64}')"#));
    static WIN_PROFILE: LazyLock<Regex> = LazyLock::new(|| re(r#"(?i)\b([A-Za-z]):([\\/])Users\2(?!(?:Public|Default|All Users|<)\b)[^\\/\s"'<>|]+"#));
    static UNIX_PROFILE: LazyLock<Regex> = LazyLock::new(|| re(r#"/(Users|home)/(?!(?:Shared|Guest|<)\b)[^/\s"'<>|]+"#));
    static SHARE: LazyLock<Regex> = LazyLock::new(|| re(r#"\\\\(?!\?\\)(?:<host#\d+>|[^\s\\"'<>|]+)\\[^\s\\"'<>|]+"#));
    static EMAIL: LazyLock<Regex> = LazyLock::new(|| re(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"));
    static LOCAL_HOST: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\b[A-Za-z0-9-]+\.(?:local|lan|home|internal|localdomain|home\.arpa)\b"));
    static CLOCK: LazyLock<Regex> = LazyLock::new(|| re(r"^\d{1,2}:\d{2}(?::\d{2})?$"));
    static OPAQUE: LazyLock<Regex> = LazyLock::new(|| re(r"\b[A-Za-z0-9_-]{32,}\b"));
    static HEX40: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^[0-9a-f]{40}$"));
    static BUILD_WORD: LazyLock<Regex> = LazyLock::new(|| re(r#"(?i)(?:commit|sha1?|hash|build|rev|revision)[\s:="']*$"#));
    static ALPHA_ONLY: LazyLock<Regex> = LazyLock::new(|| re(r"^[A-Za-z]+$"));
    static PIN_KEY: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)pin|passcode"));
    static DIGIT: LazyLock<Regex> = LazyLock::new(|| re(r"\d"));

    // urls: the host of a private address goes with the IP rule below; every query value goes
    let mut line = sub(&QUERY, line, |c| {
        labels.bump("query");
        format!("{}=<hidden>", group(c, 1))
    })?;
    line = sub(&URL_CRED, &line, |c| {
        labels.bump("credential");
        let m = group(c, 0);
        format!("{}<hidden>@", &m[..m.find("//").map_or(0, |i| i + 2)])
    })?;

    // PEM-like and token-shaped values
    line = sub(&JWT, &line, |_| {
        labels.bump("jwt");
        "<jwt>".to_string()
    })?;
    line = sub(&BEARER, &line, |c| {
        labels.bump("token");
        format!("{} <hidden>", group(c, 1))
    })?;
    line = sub(&TOKEN_SHAPE, &line, |_| {
        labels.bump("token");
        "<hidden>".to_string()
    })?;

    // a Cookie header is one secret, whatever it holds
    line = sub(&COOKIE_LINE, &line, |c| {
        labels.bump("secret");
        format!("{}<hidden>", group(c, 1))
    })?;
    // secrets by their key (pairing PIN included; a number under a credential key is the credential)
    line = sub(&RE_SECRET, &line, |c| {
        let (k, sep, v) = (group(c, 1), group(c, 2), group(c, 3));
        let hide_anyway = (PIN_KEY.is_match(k).unwrap_or(true) && DIGIT.is_match(v).unwrap_or(true))
            || (NUMBER_VALUE.is_match(v.trim()).unwrap_or(true) && SECRET_NUM_KEY.is_match(k).unwrap_or(true) && !METRIC_KEY.is_match(k).unwrap_or(false));
        if is_harmless(v) && !hide_anyway {
            return group(c, 0).to_string();
        }
        labels.bump("secret");
        format!("{k}{sep}<hidden>")
    })?;
    line = sub(&PIN_PHRASE, &line, |c| {
        labels.bump("secret");
        format!("{}{}<hidden>", group(c, 1), group(c, 2))
    })?;

    // serial numbers and device identifiers by key; Windows device instance ids by shape
    line = sub(&RE_SERIAL, &line, |c| {
        if is_harmless(group(c, 3)) {
            return group(c, 0).to_string();
        }
        labels.bump("serial");
        format!("{}{}<serial>", group(c, 1), group(c, 2))
    })?;
    line = sub(&INSTANCE, &line, |c| {
        labels.bump("serial");
        format!("{}\\<instance>", group(c, 1))
    })?;
    line = sub(&DEVICE_PATH, &line, |c| {
        labels.bump("serial");
        format!("{}<device-path>", group(c, 1))
    })?;
    line = sub(&UUID, &line, |_| {
        labels.bump("id");
        "<id>".to_string()
    })?;
    line = sub(&SID, &line, |_| {
        labels.bump("sid");
        "<sid>".to_string()
    })?;
    line = sub(&STEAM, &line, |_| {
        labels.bump("id");
        "<id>".to_string()
    })?;

    // Wi-Fi names, game accounts, computer name keys
    line = sub(&RE_SSID, &line, |c| {
        if is_harmless(group(c, 3)) {
            return group(c, 0).to_string();
        }
        labels.bump("ssid");
        format!("{}{}\"<ssid>\"", group(c, 1), group(c, 2))
    })?;
    line = sub(&SSID_PHRASE, &line, |c| {
        labels.bump("ssid");
        format!("{}{}\"<ssid>\"", group(c, 1), group(c, 2))
    })?;
    line = sub(&RE_ACCOUNT, &line, |c| {
        if is_harmless(group(c, 3)) {
            return group(c, 0).to_string();
        }
        labels.bump("account");
        format!("{}{}<account>", group(c, 1), group(c, 2))
    })?;
    line = sub(&RE_HOSTKEY, &line, |c| {
        if is_harmless(group(c, 3)) {
            return group(c, 0).to_string();
        }
        labels.bump("host");
        format!("{}{}<host>", group(c, 1), group(c, 2))
    })?;

    // profile paths (the learned names are already gone; this catches a path whose name was too short or public-looking)
    line = sub(&WIN_PROFILE, &line, |c| {
        labels.bump("user");
        format!("{}:{}Users{}<user>", group(c, 1), group(c, 2), group(c, 2))
    })?;
    line = sub(&UNIX_PROFILE, &line, |c| {
        labels.bump("user");
        format!("/{}/<user>", group(c, 1))
    })?;
    line = sub(&SHARE, &line, |_| {
        labels.bump("share");
        "\\\\<host>\\<share>".to_string()
    })?;

    // contact and network addresses
    line = sub(&EMAIL, &line, |_| {
        labels.bump("email");
        "<email>".to_string()
    })?;
    line = sub(&LOCAL_HOST, &line, |_| {
        labels.bump("host");
        "<host>".to_string()
    })?;
    line = sub(&MAC, &line, |c| labels.of("mac", &group(c, 0).replace('-', ":")))?;
    // IPv6 and IPv4 in one scan, so the numbers follow the order of appearance in the report
    let whole = line.clone();
    line = sub(&IP_ANY, &whole, |c| {
        let m = c.get(0).expect("group 0");
        let text = m.as_str();
        if text.contains(':') {
            if text == "::" || text == "::1" {
                return text.to_string();
            }
            // no decimal digit: C++ scopes such as a::b look the same; an address is made of groups of 3 or 4 hex characters (fdab::cafe)
            if !text.chars().any(|ch| ch.is_ascii_digit()) && text.split(':').any(|g| !g.is_empty() && g.len() < 3) {
                return text.to_string();
            }
            if CLOCK.is_match(text).unwrap_or(false) {
                return text.to_string();
            }
            return labels.of("ip", text);
        }
        if KEEP_IPV4.contains(&text) {
            return text.to_string();
        }
        let before: Vec<char> = whole[..m.start()].chars().rev().take(14).collect();
        let before: String = before.into_iter().rev().collect();
        if VERSIONISH.is_match(&before).unwrap_or(false) {
            return text.to_string();
        }
        labels.of("ip", text)
    })?;

    // long opaque values: keys, hashes of secrets, session ids. 40 hex characters after commit/sha/hash/build stay (build identity).
    let whole = line.clone();
    line = sub(&OPAQUE, &whole, |c| {
        let m = c.get(0).expect("group 0");
        let text = m.as_str();
        let before: Vec<char> = whole[..m.start()].chars().rev().take(20).collect();
        let before: String = before.into_iter().rev().collect();
        if HEX40.is_match(text).unwrap_or(false) && BUILD_WORD.is_match(&before).unwrap_or(false) {
            return text.to_string();
        }
        if ALPHA_ONLY.is_match(text).unwrap_or(false) {
            return text.to_string();
        }
        labels.bump("id");
        "<id>".to_string()
    })?;
    Ok(line)
}

/// The last `max_lines` lines of `raw`, anonymised, each at most `max_line` characters. Reads only the tail of a long text.
pub fn scrubbed_tail(raw: &str, max_lines: usize, max_line: usize) -> Vec<String> {
    let lines: Vec<&str> = raw.lines().collect();
    let start = lines.len().saturating_sub(max_lines.saturating_mul(2).max(max_lines));
    let slice = lines[start..].join("\n");
    let scrubbed = scrub_log(&slice, &Options { max_chars: 200_000, max_line });
    let all: Vec<String> = scrubbed.text.split('\n').map(str::to_string).collect();
    let from = all.len().saturating_sub(max_lines);
    all[from..].iter().filter(|l| !l.trim().is_empty()).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    struct Vectors {
        version: u32,
        vectors: Vec<Vector>,
    }
    #[derive(serde::Deserialize)]
    struct Vector {
        name: String,
        input: String,
        forbidden: Vec<String>,
        required: Vec<String>,
    }

    #[test]
    fn every_shared_vector_holds() {
        let doc: Vectors = serde_json::from_str(include_str!("../testdata/log-scrub-vectors.json")).expect("vectors parse");
        assert_eq!(doc.version, 1);
        assert!(doc.vectors.len() >= 40);
        let mut failures = Vec::new();
        for v in &doc.vectors {
            let out = scrub_log(&v.input, &Options::default()).text;
            for f in &v.forbidden {
                if out.contains(f.as_str()) {
                    failures.push(format!("[{}] leaked {f:?}\n{out}", v.name));
                }
            }
            for r in &v.required {
                if !out.contains(r.as_str()) {
                    failures.push(format!("[{}] lost {r:?}\n{out}", v.name));
                }
            }
        }
        assert!(failures.is_empty(), "{} vector failures:\n{}", failures.len(), failures.join("\n---\n"));
    }

    #[test]
    fn the_same_address_is_the_same_placeholder_in_one_report() {
        let t = scrub_log("client 10.0.0.5 up\nclient 10.0.0.9 up\nclient 10.0.0.5 down", &Options::default()).text;
        assert_eq!(t, "client <ip#1> up\nclient <ip#2> up\nclient <ip#1> down");
        assert_eq!(scrub_log("peer 172.16.4.4", &Options::default()).text, "peer <ip#1>");
    }

    #[test]
    fn an_ipv4_an_ipv6_and_a_mac_count_in_their_own_series() {
        let t = scrub_log("a 10.1.1.1 b fe80::1:2:3:4 c aa:bb:cc:dd:ee:ff d aa-bb-cc-dd-ee-ff", &Options::default()).text;
        assert_eq!(t, "a <ip#1> b <ip#2> c <mac#1> d <mac#1>");
    }

    #[test]
    fn a_cut_inside_a_key_block_drops_the_rest_of_the_block() {
        let mut raw = String::from("Info: start\n-----BEGIN PRIVATE KEY-----\n");
        for i in 0..60 {
            raw.push_str(&format!("ABCDEFGHijklmnop{i}\n"));
        }
        raw.push_str("-----END PRIVATE KEY-----\nInfo: after the key\n");
        let r = scrub_log(&raw, &Options { max_chars: raw.chars().count() - 400, max_line: 600 });
        assert!(r.truncated);
        assert!(!r.text.contains("ABCDEFGH"));
        assert!(r.text.contains("after the key"));
    }

    #[test]
    fn a_line_is_cut_and_a_hostile_line_does_not_hang() {
        let long = format!("Info: {}", "a:".repeat(3000));
        let started = std::time::Instant::now();
        let out = scrub_log(&long, &Options::default());
        assert!(started.elapsed().as_secs() < 5, "scrubbing one hostile line took {:?}", started.elapsed());
        assert!(out.text.chars().count() <= 600);
    }

    #[test]
    fn the_tail_keeps_the_last_lines_scrubbed() {
        let raw = (0..100).map(|i| format!("line {i} from 192.168.1.{}", i % 7 + 2)).collect::<Vec<_>>().join("\n");
        let tail = scrubbed_tail(&raw, 5, 200);
        assert_eq!(tail.len(), 5);
        assert!(tail.last().unwrap().starts_with("line 99 from <ip#"));
        assert!(tail.iter().all(|l| !l.contains("192.168")));
    }
}
