//! Anonymisation of a whole session log before it is attached to a crash report: the Rust port of the host's `src/crash/log_scrub.cpp`.
//!
//! Removed: user and computer names, the user segment of paths, IPv4/IPv6/MAC addresses (one value is one placeholder inside a log: `<ip#1>`,
//! `<mac#1>`), e-mail, serial numbers and device instance ids, tokens, keys, cookies, pairing PINs, Wi-Fi names, game account names. Kept:
//! timestamps, levels, components, codes, numbers, versions, GPU and driver strings and every config value that is not a secret.
//!
//! The rules, their order and the placeholders are those of the shared test vectors (`log-scrub-vectors.json`, the same file the host, the
//! report service and the decode workflow run); the tests at the end run every vector. A line the regular-expression engine cannot finish
//! (its backtracking limit) is dropped, never passed on: the scrub fails closed.

use fancy_regex::{Captures, Regex};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

pub(crate) struct Scrubbed {
    pub text: String,
    pub lines: usize,
    /// The beginning was cut: only the tail of a very long log is kept.
    pub truncated: bool,
}

/// `\w`, `\d`, `\s` and `\b` of the shared patterns mean ASCII (as in the other implementations); `fancy-regex` is Unicode by default.
fn ascii(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len() + 16);
    let mut chars = pattern.chars().peekable();
    let mut in_class = false;
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                let Some(n) = chars.next() else {
                    out.push('\\');
                    break;
                };
                match (n, in_class) {
                    ('w', true) => out.push_str("A-Za-z0-9_"),
                    ('w', false) => out.push_str("[A-Za-z0-9_]"),
                    ('d', true) => out.push_str("0-9"),
                    ('d', false) => out.push_str("[0-9]"),
                    ('s', true) => out.push_str(" \\t\\r\\n\\x0b\\x0c"),
                    ('s', false) => out.push_str("[ \\t\\r\\n\\x0b\\x0c]"),
                    // an ASCII word boundary (the engine's own \b is Unicode: a letter next to a Chinese character is not a boundary there)
                    ('b', false) => out.push_str("(?:(?<=[A-Za-z0-9_])(?![A-Za-z0-9_])|(?<![A-Za-z0-9_])(?=[A-Za-z0-9_]))"),
                    _ => {
                        out.push('\\');
                        out.push(n);
                    }
                }
            }
            '[' if !in_class => {
                in_class = true;
                out.push('[');
                if chars.peek() == Some(&'^') {
                    out.push(chars.next().unwrap());
                }
                if chars.peek() == Some(&']') {
                    out.push_str("\\]");
                    chars.next();
                }
            }
            ']' if in_class => {
                in_class = false;
                out.push(']');
            }
            _ => out.push(c),
        }
    }
    out
}

fn rx(pattern: &str, ignore_case: bool) -> Regex {
    let p = ascii(pattern);
    Regex::new(&if ignore_case { format!("(?i){p}") } else { p }).expect("a fixed scrub pattern")
}

const SECRET_KEY: &str = r"(?:[\w.-]*(?:token|secret|passwd|password|passphrase|pwd|apikey|api[_-]?key|private[_-]?key|credential|cookie|authorization|auth|passcode|cdk|salt|nonce)[\w.-]*|(?:[\w.-]*[_.-])?pin|pairing[_-]?pin)";
const SERIAL_KEY: &str = r"(?:[\w.-]*serial(?:[_ -]?(?:number|no))?|sn|s/n|[\w.-]*(?:device|hardware|machine|instance|unique|monitor|display)[_ -]?id|udid|imei|edid[_ -]?(?:serial|hash))";
const ACCOUNT_KEY: &str = r"(?:[\w.-]*(?:account|nickname|nick[_ -]?name|player[_ -]?name|role[_ -]?name|user[_ -]?name|username|login|owner|steam[_ -]?id|uid|user[_ -]?id|profile[_ -]?name)[\w.-]*)";
const SSID_KEY: &str = r"(?:ssid|essid|wi-?fi(?:[_ -]?(?:name|network))?|wlan[_ -]?name|network[_ -]?name|profile)";
const HOST_KEY: &str = r"(?:host[_ -]?name|computer[_ -]?name|machine[_ -]?name|device[_ -]?name|node[_ -]?name|netbios[_ -]?name|pc[_ -]?name)";

/// `key [:=] value`, the value "quoted", 'quoted' or running to a delimiter
fn kv(key: &str) -> Regex {
    rx(&format!(r#"\b({key})(["']?\s*[:=]\s*)("[^"\r\n]*"|'[^'\r\n]*'|[^\s,;&"'}}\]]+)"#), true)
}

struct Rules {
    sep_follows: Regex,
    /// a learned name that is itself a key word
    name_is_key: Regex,
    secret: Regex,
    serial: Regex,
    account: Regex,
    ssid: Regex,
    host: Regex,
    harmless: Regex,
    login_key: Regex,
    pin_key: Regex,
    secret_num_key: Regex,
    metric_key: Regex,
    number_value: Regex,
    cookie_line: Regex,
    query: Regex,
    url_cred: Regex,
    jwt: Regex,
    bearer: Regex,
    known_tokens: Regex,
    pin_words: Regex,
    device_instance: Regex,
    device_path: Regex,
    guid: Regex,
    sid: Regex,
    steam: Regex,
    wifi_quoted: Regex,
    win_user_bs: Regex,
    win_user_fs: Regex,
    posix_user: Regex,
    unc: Regex,
    email: Regex,
    local_host: Regex,
    mac: Regex,
    ip_any: Regex,
    versionish: Regex,
    clock: Regex,
    opaque: Regex,
    hex40: Regex,
    letters: Regex,
    commit_ctx: Regex,
    pem_begin: Regex,
    pem_end: Regex,
    path_win: Regex,
    path_mac: Regex,
    path_linux: Regex,
    unc_host: Regex,
    win_host: Regex,
    quotes: Regex,
    digit: Regex,
}

static RULES: Lazy<Rules> = Lazy::new(|| Rules {
    sep_follows: rx(r"^\s*[:=]", false),
    name_is_key: rx(&format!("^(?:{SECRET_KEY}|{SERIAL_KEY}|{ACCOUNT_KEY}|{SSID_KEY}|{HOST_KEY})$"), true),
    secret: kv(SECRET_KEY),
    serial: kv(SERIAL_KEY),
    account: kv(ACCOUNT_KEY),
    ssid: kv(SSID_KEY),
    host: kv(HOST_KEY),
    harmless: rx(r#"^(?:["']?(?:|true|false|null|none|nil|empty|n/a|unknown|default|disabled|enabled|on|off|yes|no|-?\d{1,6}|<[^>]*>)["']?)$"#, true),
    login_key: rx(r"^(?:[\w.-]*(?:user[_ -]?name|username|login|owner))$", true),
    pin_key: rx(r"pin|passcode", true),
    // a number under one of these keys is a credential (password=123456), unless the key says it counts or measures something (token_count, max_password_length)
    secret_num_key: rx(r"pass|pwd|secret|token|credential|cookie|authorization|pin|salt|nonce", true),
    metric_key: rx(r"(?:^|[_.-])(?:count|len|length|size|timeout|ttl|ms|secs?|seconds|max|min|limit|interval|retries|retry|attempts|version|port|enabled|expires?|age|bytes|bits|rate|percent)(?:$|[_.-])", true),
    number_value: rx(r#"^["']?-?\d{1,6}["']?$"#, false),
    cookie_line: rx(r"(\b(?:set-)?cookie2?\b\s*:\s*)[^\r\n]+", true),
    query: rx(r#"([?&][A-Za-z0-9_.-]+)=[^&\s#"']*"#, false),
    url_cred: rx(r#"\bhttps?://[^\s/@"']+@"#, true),
    jwt: rx(r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{4,}\b", false),
    bearer: rx(r"\b(Bearer|Basic)\s+[A-Za-z0-9._~+/=-]{8,}", false),
    known_tokens: rx(r"\b(?:sk-[A-Za-z0-9_-]{12,}|gh[pousr]_[A-Za-z0-9]{20,}|xox[abp]-[A-Za-z0-9-]{10,}|AKIA[0-9A-Z]{16})\b", false),
    pin_words: rx(r"\b(PIN|passcode|pairing code)\b(\s+(?:is\s+)?)(\d{4,8})\b", true),
    device_instance: rx(r#"\b((?:PCI|USB|HID|ACPI|SWD|ROOT|DISPLAY|MONITOR|BTHENUM|BTHLE|HDAUDIO|MMDEVAPI|SCSI|STORAGE|IDE|NVME)[\\#][^\\#\s"']+)[\\#](?!<)([^\\#\s"']+)"#, true),
    device_path: rx(r#"\b(\\\\\?\\)(?:[A-Za-z]+#)[^\s"']+"#, false),
    guid: rx(r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b", true),
    sid: rx(r"\bS-1-\d+(?:-\d+){2,}\b", false),
    steam: rx(r"\b7656119\d{10}\b", false),
    wifi_quoted: rx(r#"\b(SSID|Wi-?Fi)\b(\s+)("[^"\r\n]{1,64}"|'[^'\r\n]{1,64}')"#, true),
    // the host's pattern has a back-reference to the separator; one pattern per separator does the same
    win_user_bs: rx(r#"\b([A-Za-z]):(\\)Users\\(?!(?:Public|Default|All Users|<)\b)[^\\/\s"'<>|]+"#, true),
    win_user_fs: rx(r#"\b([A-Za-z]):(/)Users/(?!(?:Public|Default|All Users|<)\b)[^\\/\s"'<>|]+"#, true),
    posix_user: rx(r#"/(Users|home)/(?!(?:Shared|Guest|<)\b)[^/\s"'<>|]+"#, false),
    unc: rx(r#"\\\\(?!\?\\)(?:<host#\d+>|[^\s\\"'<>|]+)\\[^\s\\"'<>|]+"#, false),
    email: rx(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}", false),
    local_host: rx(r"\b[A-Za-z0-9-]+\.(?:local|lan|home|internal|localdomain|home\.arpa)\b", true),
    mac: rx(r"(?<![\w:-])(?:[0-9a-f]{2}[:-]){5}[0-9a-f]{2}(?![\w:-])", true),
    ip_any: rx(
        r"(?<![\w:.])(?:(?:[0-9a-f]{1,4}:){7}[0-9a-f]{1,4}|(?:[0-9a-f]{1,4}(?::[0-9a-f]{1,4}){0,6})?::(?:[0-9a-f]{1,4}(?::[0-9a-f]{1,4}){0,6})?)(?![\w:])|(?<![\w.])(?:25[0-5]|2[0-4]\d|1?\d?\d)(?:\.(?:25[0-5]|2[0-4]\d|1?\d?\d)){3}(?![\w.]*\d)",
        true,
    ),
    versionish: rx(r"(?:version|ver\.?|v|driver|build|dll|sdk|api|firmware|fw)[\s:=/_-]*$", true),
    clock: rx(r"^\d{1,2}:\d{2}(?::\d{2})?$", false),
    opaque: rx(r"\b[A-Za-z0-9_-]{32,}\b", false),
    hex40: rx(r"^[0-9a-f]{40}$", true),
    letters: rx(r"^[A-Za-z]+$", false),
    commit_ctx: rx(r#"(?:commit|sha1?|hash|build|rev|revision)[\s:="']*$"#, true),
    pem_begin: rx(r"-----BEGIN [A-Z ]*(?:PRIVATE KEY|CERTIFICATE|PUBLIC KEY)[A-Z ]*-----", false),
    pem_end: rx(r"-----END [A-Z ]+-----", false),
    path_win: rx(r#"\b[A-Za-z]:[\\/]Users[\\/]([^\\/\s"'<>|:*?]+(?: [^\\/\s"'<>|:*?]+)*?)(?=[\\/])"#, false),
    path_mac: rx(r#"/Users/([^/\s"'<>|]+)"#, false),
    path_linux: rx(r#"/home/([^/\s"'<>|]+)"#, false),
    unc_host: rx(r"\\\\([A-Za-z0-9_-]{2,63})(?=\\)", false),
    win_host: rx(r"\b((?:DESKTOP|LAPTOP|WIN|PC|NOTEBOOK)-[A-Z0-9]{5,13})\b", false),
    quotes: rx(r#"^["']|["']$"#, false),
    digit: rx(r"\d", false),
});

const PUBLIC_NAMES: [&str; 19] = [
    "public", "default", "default user", "all users", "defaultuser0", "system", "system32", "localservice", "networkservice", "administrator", "administrators", "guest", "shared", "user", "users",
    "root", "appdata", "programdata", "windows",
];
const KEEP_IPV4: [&str; 4] = ["0.0.0.0", "127.0.0.1", "255.255.255.255", "255.255.255.0"];

/// The line could not be finished: it is dropped, not passed on.
struct Failed;

fn is_dropped_control(c: char) -> bool {
    let u = c as u32;
    u <= 0x08 || u == 0x0B || u == 0x0C || (0x0E..=0x1F).contains(&u) || (0x7F..=0x9F).contains(&u) || u == 0x2028 || u == 0x2029 || u == 0xFEFF || (0x202A..=0x202E).contains(&u) || (0x2066..=0x2069).contains(&u)
}

/// Invalid UTF-8 becomes U+FFFD, control characters go (the same set as the other implementations); CR and CRLF become LF.
pub(crate) fn sanitize(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            out.push('\n');
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
        } else if !is_dropped_control(c) {
            out.push(c);
        }
    }
    out
}

#[derive(Default)]
struct Labels {
    maps: HashMap<&'static str, HashMap<String, usize>>,
}

impl Labels {
    fn of(&mut self, kind: &'static str, value: &str) -> String {
        let map = self.maps.entry(kind).or_default();
        let next = map.len() + 1;
        let n = *map.entry(value.to_lowercase()).or_insert(next);
        format!("<{kind}#{n}>")
    }
}

fn replace_all<F: FnMut(&Captures) -> String>(re: &Regex, text: &str, mut f: F) -> Result<String, Failed> {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for caps in re.captures_iter(text) {
        let caps = caps.map_err(|_| Failed)?;
        let whole = caps.get(0).ok_or(Failed)?;
        if whole.start() == whole.end() {
            continue;
        }
        out.push_str(&text[last..whole.start()]);
        out.push_str(&f(&caps));
        last = whole.end();
    }
    out.push_str(&text[last..]);
    Ok(out)
}

fn matches(re: &Regex, text: &str) -> bool {
    re.is_match(text).unwrap_or(true)
}

fn is_harmless(v: &str) -> bool {
    matches(&RULES.harmless, v.trim())
}

fn unquote(v: &str) -> String {
    RULES.quotes.replace_all(v, "").into_owned()
}

/// Every match of `re` in `line` (captures as owned strings: group 1, group 3). The engine gives up on a line it cannot finish (its backtracking limit); that ends the list,
/// since the iterator would hand out the same error again and again.
fn captured(re: &Regex, line: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for caps in re.captures_iter(line) {
        let Ok(caps) = caps else { break };
        out.push((caps.get(1).map_or(String::new(), |m| m.as_str().to_string()), caps.get(3).map_or(String::new(), |m| m.as_str().to_string())));
    }
    out
}

/// The names the log reveals. Line by line: no rule spans a line, and the backtracking limit of the engine counts per search, so a whole log in one search would run into it.
fn learn_names(text: &str) -> (Vec<String>, Vec<String>) {
    let r = &*RULES;
    let mut users: HashSet<String> = HashSet::new();
    let mut hosts: HashSet<String> = HashSet::new();
    for line in text.split('\n') {
        for re in [&r.path_win, &r.path_mac, &r.path_linux] {
            for (name, _) in captured(re, line) {
                users.insert(name);
            }
        }
        for (key, value) in captured(&r.account, line) {
            if matches(&r.login_key, &key) {
                users.insert(unquote(&value));
            }
        }
        for (_, value) in captured(&r.host, line) {
            hosts.insert(unquote(&value));
        }
        for re in [&r.unc_host, &r.win_host] {
            for (name, _) in captured(re, line) {
                hosts.insert(name);
            }
        }
    }
    let clean = |set: HashSet<String>| -> Vec<String> {
        let mut v: Vec<String> = set
            .into_iter()
            .map(|x| x.trim().to_string())
            .filter(|x| (3..=64).contains(&x.chars().count()) && !PUBLIC_NAMES.contains(&x.to_lowercase().as_str()) && !is_harmless(x) && !(x.starts_with('<') && x.ends_with('>')))
            .collect();
        v.sort();
        v
    };
    (clean(users), clean(hosts))
}

/// A quote that cannot be balanced: the value was opened by a quote (or cut short by one) and the rest of the line holds an odd number of that quote character. What follows such a
/// quote cannot be told from the rest of the value (`'Players' choice - Budget.xlsx'`, `Bob' phone`), so it goes with it.
fn quote_unbalanced(value: &str, rest: &str) -> bool {
    let is_quote = |c: char| c == '\'' || c == '"';
    let q = match (value.chars().next(), rest.chars().next()) {
        (Some(c), _) if is_quote(c) => c,
        (_, Some(c)) if is_quote(c) => c,
        _ => return false,
    };
    rest.matches(q).count() % 2 == 1
}

/// `key [:=] value` rules: the value is replaced by `fmt` unless it is harmless (and `must_hide` does not say otherwise). A quote that cannot be balanced drops the rest of the line.
fn kv_sub(line: &str, re: &Regex, must_hide: &dyn Fn(&str, &str) -> bool, fmt: &dyn Fn(&str, &str) -> String) -> Result<String, Failed> {
    let mut out = String::with_capacity(line.len());
    let mut last = 0;
    for caps in re.captures_iter(line) {
        let caps = caps.map_err(|_| Failed)?;
        let whole = caps.get(0).ok_or(Failed)?;
        if whole.start() == whole.end() {
            continue;
        }
        let (k, sep, v) = (caps.get(1).map_or("", |m| m.as_str()), caps.get(2).map_or("", |m| m.as_str()), caps.get(3).map_or("", |m| m.as_str()));
        let rest = &line[whole.end()..];
        if is_harmless(v) && !must_hide(k, v) {
            // a value that is one of our own placeholders (a learned name was replaced first) has the same unbalanced quote behind it
            if v.len() > 1 && v.starts_with('<') && v.ends_with('>') && quote_unbalanced(v, rest) {
                out.push_str(&line[last..whole.end()]);
                return Ok(out);
            }
            continue;
        }
        out.push_str(&line[last..whole.start()]);
        out.push_str(&fmt(k, sep));
        last = whole.end();
        if quote_unbalanced(v, rest) {
            return Ok(out);
        }
    }
    out.push_str(&line[last..]);
    Ok(out)
}

fn scrub_line(line: &str, labels: &mut Labels) -> Result<String, Failed> {
    let r = &*RULES;
    let never = |_: &str, _: &str| false;
    let mut line = replace_all(&r.query, line, |c| format!("{}=<hidden>", &c[1]))?;
    line = replace_all(&r.url_cred, &line, |c| {
        let m = c.get(0).map_or("", |m| m.as_str());
        format!("{}<hidden>@", &m[..m.find("//").map_or(0, |i| i + 2)])
    })?;
    line = replace_all(&r.jwt, &line, |_| "<jwt>".to_string())?;
    line = replace_all(&r.bearer, &line, |c| format!("{} <hidden>", &c[1]))?;
    line = replace_all(&r.known_tokens, &line, |_| "<hidden>".to_string())?;
    // a Cookie header is one secret, whatever it holds
    line = replace_all(&r.cookie_line, &line, |c| format!("{}<hidden>", &c[1]))?;
    line = kv_sub(
        &line,
        &r.secret,
        &|k, v| (matches(&r.pin_key, k) && matches(&r.digit, v)) || (matches(&r.number_value, v.trim()) && matches(&r.secret_num_key, k) && !matches(&r.metric_key, k)),
        &|k, sep| format!("{k}{sep}<hidden>"),
    )?;
    line = replace_all(&r.pin_words, &line, |c| format!("{}{}<hidden>", &c[1], &c[2]))?;
    line = kv_sub(&line, &r.serial, &never, &|k, sep| format!("{k}{sep}<serial>"))?;
    line = replace_all(&r.device_instance, &line, |c| format!("{}\\<instance>", &c[1]))?;
    line = replace_all(&r.device_path, &line, |c| format!("{}<device-path>", &c[1]))?;
    line = replace_all(&r.guid, &line, |_| "<id>".to_string())?;
    line = replace_all(&r.sid, &line, |_| "<sid>".to_string())?;
    line = replace_all(&r.steam, &line, |_| "<id>".to_string())?;
    line = kv_sub(&line, &r.ssid, &never, &|k, sep| format!("{k}{sep}\"<ssid>\""))?;
    line = kv_sub(&line, &r.wifi_quoted, &|_, _| true, &|k, sep| format!("{k}{sep}\"<ssid>\""))?;
    line = kv_sub(&line, &r.account, &never, &|k, sep| format!("{k}{sep}<account>"))?;
    line = kv_sub(&line, &r.host, &never, &|k, sep| format!("{k}{sep}<host>"))?;
    for re in [&r.win_user_bs, &r.win_user_fs] {
        line = replace_all(re, &line, |c| format!("{}:{}Users{}<user>", &c[1], &c[2], &c[2]))?;
    }
    line = replace_all(&r.posix_user, &line, |c| format!("/{}/<user>", &c[1]))?;
    line = replace_all(&r.unc, &line, |_| "\\\\<host>\\<share>".to_string())?;
    line = replace_all(&r.email, &line, |_| "<email>".to_string())?;
    line = replace_all(&r.local_host, &line, |_| "<host>".to_string())?;
    line = replace_all(&r.mac, &line, |c| labels.of("mac", &c[0].replace('-', ":")))?;
    let src = line.clone();
    line = replace_all(&r.ip_any, &src, |c| {
        let m = c.get(0).unwrap();
        let v = m.as_str();
        if v.contains(':') {
            if v == "::" || v == "::1" {
                return v.to_string();
            }
            // no decimal digit: C++ scopes such as a::b look the same; an address is made of groups of 3 or 4 hex characters (fdab::cafe)
            if !v.bytes().any(|b| b.is_ascii_digit()) && v.split(':').any(|g| !g.is_empty() && g.len() < 3) {
                return v.to_string();
            }
            if matches(&r.clock, v) {
                return v.to_string();
            }
            return labels.of("ip", v);
        }
        if KEEP_IPV4.contains(&v) {
            return v.to_string();
        }
        let from = m.start().saturating_sub(14);
        let from = (from..=m.start()).find(|i| src.is_char_boundary(*i)).unwrap_or(m.start());
        if matches(&r.versionish, &src[from..m.start()]) {
            return v.to_string();
        }
        labels.of("ip", v)
    })?;
    // long opaque values: keys, hashes of secrets, session ids; 40 hex characters after commit/sha/hash/build stay (build identity)
    let src = line.clone();
    replace_all(&r.opaque, &src, |c| {
        let m = c.get(0).unwrap();
        let v = m.as_str();
        let from = m.start().saturating_sub(20);
        let from = (from..=m.start()).find(|i| src.is_char_boundary(*i)).unwrap_or(m.start());
        if matches(&r.hex40, v) && matches(&r.commit_ctx, &src[from..m.start()]) {
            return v.to_string();
        }
        if matches(&r.letters, v) { v.to_string() } else { "<id>".to_string() }
    })
}

fn cut_chars(text: &str, max: usize) -> &str {
    if text.chars().count() <= max {
        return text;
    }
    let skip = text.chars().count() - max;
    let at = text.char_indices().nth(skip).map_or(text.len(), |(i, _)| i);
    &text[at..]
}

/// Scrubs `raw`. `max_chars` bounds the kept tail (whole lines), `max_line` cuts a single line.
pub(crate) fn scrub(raw: &[u8], max_chars: usize, max_line: usize) -> Scrubbed {
    let mut text = sanitize(raw);
    let mut truncated = false;
    let mut in_pem = false;
    if text.chars().count() > max_chars {
        // the cut may fall inside a key block: its body must not turn into ordinary lines
        let kept = cut_chars(&text, max_chars).to_string();
        let before = &text[..text.len() - kept.len()];
        // line by line (the engine's backtracking limit counts per search); a line it cannot finish is taken to be inside the block
        let (mut begin, mut end, mut unsure, mut at) = (None, None, false, 0usize);
        for l in before.split_inclusive('\n') {
            match RULES.pem_begin.is_match(l) {
                Ok(true) => begin = Some(at),
                Ok(false) => {}
                Err(_) => unsure = true,
            }
            match RULES.pem_end.is_match(l) {
                Ok(true) => end = Some(at),
                Ok(false) => {}
                Err(_) => unsure = true,
            }
            at += l.len();
        }
        in_pem = unsure
            || match (begin, end) {
                (Some(b), Some(e)) => e < b,
                (Some(_), None) => true,
                _ => false,
            };
        text = kept;
        text = text.find('\n').map_or_else(String::new, |i| text[i + 1..].to_string());
        truncated = true;
    }
    let mut labels = Labels::default();
    let (users, hosts) = learn_names(&text);
    // The learned names go line by line, after the key blocks are told apart: a name must not change the markers of a key block (a user called "Key" and
    // -----BEGIN PRIVATE KEY-----), and the lines of a key block are dropped, not renamed.
    let mut name_rules: Vec<(Regex, &'static str)> = Vec::new();
    for (names, kind) in [(users, "user"), (hosts, "host")] {
        if names.is_empty() {
            continue;
        }
        let mut sorted = names;
        sorted.sort_by_key(|n| std::cmp::Reverse(n.len()));
        let alt: Vec<String> = sorted.iter().map(|n| fancy_regex::escape(n).into_owned()).collect();
        name_rules.push((rx(&format!(r"(?<![\w])(?:{})(?![\w])", alt.join("|")), true), kind));
    }
    // An occurrence followed by = or : stays only when the name is itself a key word (username=password, then password=...): the key rules need it.
    // Any other name before a colon is part of a message ("Tanaka: disconnected") and goes.
    let rename = |line: &str, labels: &mut Labels| -> String {
        let mut line = line.to_string();
        for (re, kind) in &name_rules {
            let src = line;
            line = match replace_all(re, &src, |c| {
                let name = &c[0];
                let end = c.get(0).map_or(0, |m| m.end());
                if matches(&RULES.sep_follows, &src[end..]) && RULES.name_is_key.is_match(name).unwrap_or(false) {
                    name.to_string()
                } else {
                    labels.of(kind, name)
                }
            }) {
                Ok(t) => t,
                Err(Failed) => "<line dropped>".to_string(),
            };
        }
        line
    };
    let mut out: Vec<String> = Vec::new();
    for line in text.split('\n') {
        if in_pem {
            if matches(&RULES.pem_end, line) {
                in_pem = false;
            }
            continue;
        }
        if matches(&RULES.pem_begin, line) {
            if !matches(&RULES.pem_end, line) {
                in_pem = true;
            }
            let before = line.find("-----BEGIN").map_or(line, |i| &line[..i]);
            out.push(format!("{}<pem>", scrub_line(&rename(before, &mut labels), &mut labels).unwrap_or_default()));
            continue;
        }
        let mut piece = scrub_line(&rename(line, &mut labels), &mut labels).unwrap_or_else(|_| "<line dropped>".to_string());
        if piece.chars().count() > max_line {
            piece = format!("{}…", piece.chars().take(max_line - 1).collect::<String>());
        }
        out.push(piece);
    }
    let lines = out.len();
    Scrubbed { text: out.join("\n"), lines, truncated }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VECTORS: &str = include_str!("log-scrub-vectors.json");

    #[test]
    fn every_shared_vector_holds() {
        let doc: serde_json::Value = serde_json::from_str(VECTORS).unwrap();
        assert_eq!(doc["version"], 1);
        let vectors = doc["vectors"].as_array().unwrap();
        assert!(vectors.len() >= 50);
        for v in vectors {
            let name = v["name"].as_str().unwrap();
            let out = scrub(v["input"].as_str().unwrap().as_bytes(), 2_000_000, 600).text;
            for f in v["forbidden"].as_array().unwrap() {
                assert!(!out.contains(f.as_str().unwrap()), "{name} leaked {f} -> {out}");
            }
            for r in v["required"].as_array().unwrap() {
                assert!(out.contains(r.as_str().unwrap()), "{name} lost {r} -> {out}");
            }
        }
    }

    #[test]
    fn placeholders_are_stable_per_value_and_per_log() {
        assert_eq!(scrub(b"client 10.0.0.5 up\nclient 10.0.0.9 up\nclient 10.0.0.5 down", 1_000_000, 600).text, "client <ip#1> up\nclient <ip#2> up\nclient <ip#1> down");
        assert_eq!(scrub(b"a 10.1.1.1 b fe80::1:2:3:4 c aa:bb:cc:dd:ee:ff d aa-bb-cc-dd-ee-ff", 1_000_000, 600).text, "a <ip#1> b <ip#2> c <mac#1> d <mac#1>");
    }

    #[test]
    fn a_scrubbed_log_is_stable() {
        let raw: String = (0..300).map(|i| format!("[2026-11-14 21:03:11.482]: Info: frame {i}, client 192.168.1.{} C:\\Users\\Sky\\x.log\n", i % 200)).collect();
        let once = scrub(raw.as_bytes(), 2_000_000, 600).text;
        assert_eq!(scrub(once.as_bytes(), 2_000_000, 600).text, once);
    }

    #[test]
    fn a_cut_inside_a_key_block_drops_the_rest_of_the_block() {
        let mut raw = String::from("Info: start\n-----BEGIN PRIVATE KEY-----\n");
        for i in 0..60 {
            raw.push_str(&format!("ABCDEFGHijklmnop{i}\n"));
        }
        raw.push_str("-----END PRIVATE KEY-----\nInfo: after the key\n");
        let r = scrub(raw.as_bytes(), raw.len() - 400, 600);
        assert!(r.truncated);
        assert!(!r.text.contains("ABCDEFGH"), "{}", r.text);
        assert!(r.text.contains("Info: after the key"));
    }

    #[test]
    fn bounds_utf8_and_control_characters() {
        let raw: String = (0..1000).map(|i| format!("line {} {}\n", 1000 + i, "x".repeat(40))).collect();
        let r = scrub(raw.as_bytes(), 5000, 600);
        assert!(r.truncated && r.text.len() <= 5000 && r.text.starts_with("line "));
        let cut = scrub(format!("a\u{7}b {}", "z".repeat(2000)).as_bytes(), 2_000_000, 100).text;
        assert!(cut.chars().count() <= 100 && !cut.contains('\u{7}') && cut.ends_with('…'));
        assert_eq!(sanitize(b"ab\xC3(cd"), "ab\u{FFFD}(cd");
        let cn = format!("{}中文中文", "a".repeat(98));
        assert!(scrub(cn.as_bytes(), 2_000_000, 100).text.is_char_boundary(0));
    }
}
