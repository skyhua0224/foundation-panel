//! What the clipboard sync agent learnt about its own connection to the host, as fixed words and counts.
//!
//! Three readers: the status command (a state word that tells "the stream is open but the host stopped hearing the agent" from "connected"),
//! the exported diagnostics, and one small statistics record per run (`clipboard_health`, sent through the host). Nothing here holds clipboard
//! content, its length or its type; a failure is one of a fixed list of words, never the text of an error (that text can carry an address).

use std::sync::atomic::{AtomicI64, AtomicU64, AtomicU8, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// How long the host may go without a successful heartbeat answer before the agent is no longer called connected.
/// The host forgets the agent after 30 s without a sign of life (clipboard_bridge kGuiAliveWindow) and the heartbeat is every 15 s,
/// so 45 s is one missed beat on top of the host's own window.
pub const HEARTBEAT_FRESH_MS: i64 = 45_000;

/// The class of a failure. The words leave the computer (clipboard_health.last_error), so the list is closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ErrClass {
    None = 0,
    ConnectRefused = 1,
    Timeout = 2,
    Tls = 3,
    Http401 = 4,
    Http403 = 5,
    Http5xx = 6,
    StreamEnded = 7,
    CredentialMissing = 8,
    Heartbeat = 9,
    Other = 10,
}

impl ErrClass {
    pub fn word(self) -> &'static str {
        match self {
            ErrClass::None => "none",
            ErrClass::ConnectRefused => "connect_refused",
            ErrClass::Timeout => "timeout",
            ErrClass::Tls => "tls",
            ErrClass::Http401 => "http_401",
            ErrClass::Http403 => "http_403",
            ErrClass::Http5xx => "http_5xx",
            ErrClass::StreamEnded => "stream_ended",
            ErrClass::CredentialMissing => "credential_missing",
            ErrClass::Heartbeat => "heartbeat",
            ErrClass::Other => "other",
        }
    }

    fn from_u8(value: u8) -> ErrClass {
        match value {
            0 => ErrClass::None,
            1 => ErrClass::ConnectRefused,
            2 => ErrClass::Timeout,
            3 => ErrClass::Tls,
            4 => ErrClass::Http401,
            5 => ErrClass::Http403,
            6 => ErrClass::Http5xx,
            7 => ErrClass::StreamEnded,
            8 => ErrClass::CredentialMissing,
            9 => ErrClass::Heartbeat,
            _ => ErrClass::Other,
        }
    }
}

/// An HTTP status the host answered with -> the class.
pub fn classify_status(status: u16) -> ErrClass {
    match status {
        401 => ErrClass::Http401,
        403 => ErrClass::Http403,
        500..=599 => ErrClass::Http5xx,
        _ => ErrClass::Other,
    }
}

/// A request error of the HTTP client -> the class, from what the client knows about it (not from its text).
pub fn classify_reqwest(error: &reqwest::Error) -> ErrClass {
    if error.is_timeout() {
        return ErrClass::Timeout;
    }
    if let Some(status) = error.status() {
        return classify_status(status.as_u16());
    }
    // the connect error and the TLS error hide in the chain of causes; look at it for words the stacks use, and keep none of it
    let mut chain = String::new();
    let mut source: Option<&dyn std::error::Error> = Some(error);
    while let Some(cause) = source {
        chain.push_str(&cause.to_string().to_ascii_lowercase());
        chain.push('|');
        source = cause.source();
    }
    if chain.contains("certificate") || chain.contains("tls") || chain.contains("ssl") || chain.contains("handshake") {
        return ErrClass::Tls;
    }
    if error.is_connect() || chain.contains("refused") || chain.contains("connect") {
        return ErrClass::ConnectRefused;
    }
    ErrClass::Other
}

/// The text of one of this agent's own errors (a message made in clipboard.rs or sunshine.rs) -> the class. Only used for text this code wrote.
pub fn classify_text(text: &str) -> ErrClass {
    let t = text.to_ascii_lowercase();
    if t.contains("credential") {
        return ErrClass::CredentialMissing;
    }
    if t.contains("401") {
        return ErrClass::Http401;
    }
    if t.contains("403") {
        return ErrClass::Http403;
    }
    if t.contains("returned 5") || t.contains("status 5") {
        return ErrClass::Http5xx;
    }
    if t.contains("timed out") || t.contains("timeout") {
        return ErrClass::Timeout;
    }
    if t.contains("ended") {
        return ErrClass::StreamEnded;
    }
    if t.contains("refused") || t.contains("connect") {
        return ErrClass::ConnectRefused;
    }
    ErrClass::Other
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---------- the counters of one run ----------

static RUN_STARTED_MS: AtomicI64 = AtomicI64::new(0);
static FIRST_CONNECT_MS: AtomicI64 = AtomicI64::new(-1);
static CONNECTED_SINCE_MS: AtomicI64 = AtomicI64::new(0);
static CONNECTED_TOTAL_MS: AtomicI64 = AtomicI64::new(0);
static EVER_CONNECTED: AtomicU8 = AtomicU8::new(0);
static RECONNECTS: AtomicU64 = AtomicU64::new(0);
static HEARTBEAT_FAILS: AtomicU64 = AtomicU64::new(0);
static LAST_HEARTBEAT_OK_MS: AtomicI64 = AtomicI64::new(0);
/// What the host said about the agent in its last heartbeat answer: -1 unknown, 0 not alive, 1 alive.
static HOST_SAYS_ALIVE: std::sync::atomic::AtomicI8 = std::sync::atomic::AtomicI8::new(-1);
static HOST_SESSION_COUNT: AtomicI64 = AtomicI64::new(-1);
/// The class of the latest failure of this run (kept for the statistics record, whatever happened after it).
static LAST_ERROR_CLASS: AtomicU8 = AtomicU8::new(0);
/// The class of the failure that is going on now: cleared when the stream opens again (what the status shows).
static CURRENT_ERROR_CLASS: AtomicU8 = AtomicU8::new(0);
static LAST_HEARTBEAT_CLASS: AtomicU8 = AtomicU8::new(0);
static APPLY_OK: AtomicU64 = AtomicU64::new(0);
static APPLY_FAIL: AtomicU64 = AtomicU64::new(0);
static POST_OK: AtomicU64 = AtomicU64::new(0);
static POST_FAIL: AtomicU64 = AtomicU64::new(0);
static LAST_SENT_MS: AtomicI64 = AtomicI64::new(0);

/// A new run of the agent: the counters start again.
pub fn run_started() {
    RUN_STARTED_MS.store(now_ms(), Ordering::Release);
    FIRST_CONNECT_MS.store(-1, Ordering::Release);
    CONNECTED_SINCE_MS.store(0, Ordering::Release);
    CONNECTED_TOTAL_MS.store(0, Ordering::Release);
    EVER_CONNECTED.store(0, Ordering::Release);
    for counter in [&RECONNECTS, &HEARTBEAT_FAILS, &APPLY_OK, &APPLY_FAIL, &POST_OK, &POST_FAIL] {
        counter.store(0, Ordering::Release);
    }
    LAST_HEARTBEAT_OK_MS.store(0, Ordering::Release);
    HOST_SAYS_ALIVE.store(-1, Ordering::Release);
    HOST_SESSION_COUNT.store(-1, Ordering::Release);
    LAST_ERROR_CLASS.store(0, Ordering::Release);
    CURRENT_ERROR_CLASS.store(0, Ordering::Release);
    LAST_HEARTBEAT_CLASS.store(0, Ordering::Release);
    LAST_SENT_MS.store(now_ms(), Ordering::Release);
}

/// The stream to the host opened.
pub fn connected() {
    let now = now_ms();
    if EVER_CONNECTED.swap(1, Ordering::AcqRel) == 0 {
        let started = RUN_STARTED_MS.load(Ordering::Acquire);
        FIRST_CONNECT_MS.store((now - started).max(0), Ordering::Release);
    } else {
        RECONNECTS.fetch_add(1, Ordering::AcqRel);
    }
    CONNECTED_SINCE_MS.store(now, Ordering::Release);
    // the failure that is over leaves the status, and stays in the record of the run
    CURRENT_ERROR_CLASS.store(ErrClass::None as u8, Ordering::Release);
}

/// The stream to the host closed (any reason): the time it was open is added up.
pub fn left_connected() {
    let since = CONNECTED_SINCE_MS.swap(0, Ordering::AcqRel);
    if since > 0 {
        CONNECTED_TOTAL_MS.fetch_add((now_ms() - since).max(0), Ordering::AcqRel);
    }
}

pub fn note_error(class: ErrClass) {
    LAST_ERROR_CLASS.store(class as u8, Ordering::Release);
    CURRENT_ERROR_CLASS.store(class as u8, Ordering::Release);
}

/// The failure that is going on now (what the status shows).
pub fn current_error_class() -> ErrClass {
    ErrClass::from_u8(CURRENT_ERROR_CLASS.load(Ordering::Acquire))
}

/// A heartbeat was answered: `host_alive` and `sessions` are what the host said (`gui_alive`, `session_count` of its answer).
pub fn heartbeat_ok(host_alive: Option<bool>, sessions: Option<u64>) {
    LAST_HEARTBEAT_OK_MS.store(now_ms(), Ordering::Release);
    LAST_HEARTBEAT_CLASS.store(ErrClass::None as u8, Ordering::Release);
    HOST_SAYS_ALIVE.store(
        match host_alive {
            Some(true) => 1,
            Some(false) => 0,
            None => -1,
        },
        Ordering::Release,
    );
    HOST_SESSION_COUNT.store(sessions.map(|n| n as i64).unwrap_or(-1), Ordering::Release);
}

pub fn heartbeat_failed(class: ErrClass) {
    HEARTBEAT_FAILS.fetch_add(1, Ordering::AcqRel);
    LAST_HEARTBEAT_CLASS.store(class as u8, Ordering::Release);
    // a failed heartbeat is also the latest failure of the run: the record must not say "none" beside a count of failed heartbeats
    LAST_ERROR_CLASS.store(class as u8, Ordering::Release);
}

pub fn apply(ok: bool) {
    (if ok { &APPLY_OK } else { &APPLY_FAIL }).fetch_add(1, Ordering::AcqRel);
}

pub fn post(ok: bool) {
    (if ok { &POST_OK } else { &POST_FAIL }).fetch_add(1, Ordering::AcqRel);
}

/// A run shorter than this says nothing (a one-shot launch of the panel for a file to send): no record.
pub fn worth_sending() -> bool {
    let started = RUN_STARTED_MS.load(Ordering::Acquire);
    started > 0 && now_ms() - started >= 10_000
}

/// The record was sent (or tried): the next one is due in 24 hours.
pub fn mark_sent() {
    LAST_SENT_MS.store(now_ms(), Ordering::Release);
}

/// A long run sends one record per 24 hours (the run's end sends the last one).
pub fn due(now: i64) -> bool {
    let last = LAST_SENT_MS.load(Ordering::Acquire);
    last > 0 && now - last >= 24 * 3_600_000
}

// ---------- the state word ----------

/// What the status reports as the transport: the stream's own state, except that an open stream whose heartbeat the host stopped answering is "degraded".
/// `stream_state` is the word of the stream ("connected", "connecting", "disconnected", "stopped"); `last_connected_ms` when it opened (0: never).
pub fn effective_state(stream_state: &'static str, last_connected_ms: i64, last_heartbeat_ok_ms: i64, now: i64) -> &'static str {
    if stream_state != "connected" {
        return stream_state;
    }
    let beat_fresh = last_heartbeat_ok_ms > 0 && now - last_heartbeat_ok_ms <= HEARTBEAT_FRESH_MS;
    // the first beat is due at most one interval after the stream opened; until a minute has passed there is no complaint
    let just_opened = last_connected_ms > 0 && now - last_connected_ms <= HEARTBEAT_FRESH_MS;
    if beat_fresh || just_opened {
        "connected"
    } else {
        "degraded"
    }
}

/// The share of the run the stream was open, as the statistics' fixed bucket.
pub fn connected_bucket(connected_ms: i64, runtime_ms: i64) -> &'static str {
    if runtime_ms <= 0 {
        return "lt50";
    }
    let pct = connected_ms.clamp(0, runtime_ms) * 100 / runtime_ms;
    if pct < 50 {
        "lt50"
    } else if pct <= 90 {
        "50_90"
    } else {
        "gt90"
    }
}

/// What the status command adds to its answer.
pub struct StatusFacts {
    pub last_heartbeat_ok_ms: Option<i64>,
    pub heartbeat_error: &'static str,
    pub host_gui_alive: Option<bool>,
    pub host_session_count: Option<u64>,
    pub last_error_class: &'static str,
}

pub fn status_facts() -> StatusFacts {
    let ok = LAST_HEARTBEAT_OK_MS.load(Ordering::Acquire);
    let alive = HOST_SAYS_ALIVE.load(Ordering::Acquire);
    let sessions = HOST_SESSION_COUNT.load(Ordering::Acquire);
    StatusFacts {
        last_heartbeat_ok_ms: (ok > 0).then_some(ok),
        heartbeat_error: ErrClass::from_u8(LAST_HEARTBEAT_CLASS.load(Ordering::Acquire)).word(),
        host_gui_alive: match alive {
            1 => Some(true),
            0 => Some(false),
            _ => None,
        },
        host_session_count: (sessions >= 0).then_some(sessions as u64),
        last_error_class: current_error_class().word(),
    }
}

/// The `clipboard_health` record of the run so far: fixed words and counts (docs: privacy-data-inventory, "clipboard_health").
pub fn health_event() -> serde_json::Value {
    let now = now_ms();
    let started = RUN_STARTED_MS.load(Ordering::Acquire);
    let runtime_ms = if started > 0 { (now - started).max(0) } else { 0 };
    let since = CONNECTED_SINCE_MS.load(Ordering::Acquire);
    let connected_ms = CONNECTED_TOTAL_MS.load(Ordering::Acquire) + if since > 0 { (now - since).max(0) } else { 0 };
    let first = FIRST_CONNECT_MS.load(Ordering::Acquire);
    let mut props = serde_json::json!({
        "connected_pct": connected_bucket(connected_ms, runtime_ms),
        "reconnects": RECONNECTS.load(Ordering::Acquire),
        "heartbeat_fail": HEARTBEAT_FAILS.load(Ordering::Acquire),
        "last_error": ErrClass::from_u8(LAST_ERROR_CLASS.load(Ordering::Acquire)).word(),
        "apply_ok": APPLY_OK.load(Ordering::Acquire),
        "apply_fail": APPLY_FAIL.load(Ordering::Acquire),
        "post_ok": POST_OK.load(Ordering::Acquire),
        "post_fail": POST_FAIL.load(Ordering::Acquire),
        "runtime_s": runtime_ms / 1000,
    });
    if first >= 0 {
        props["first_connect_ms"] = serde_json::json!(first);
    }
    serde_json::json!({ "kind": "experience", "name": "clipboard_health", "props": props })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // the counters are process-wide: the tests that read them one after the other
    static COUNTERS: Mutex<()> = Mutex::new(());

    #[test]
    fn the_words_of_a_failure_are_the_closed_list_of_the_contract() {
        let words = [
            ErrClass::None,
            ErrClass::ConnectRefused,
            ErrClass::Timeout,
            ErrClass::Tls,
            ErrClass::Http401,
            ErrClass::Http403,
            ErrClass::Http5xx,
            ErrClass::StreamEnded,
            ErrClass::CredentialMissing,
            ErrClass::Heartbeat,
            ErrClass::Other,
        ]
        .map(|c| c.word());
        assert_eq!(
            words,
            ["none", "connect_refused", "timeout", "tls", "http_401", "http_403", "http_5xx", "stream_ended", "credential_missing", "heartbeat", "other"]
        );
        for class in 0..=10u8 {
            assert_eq!(ErrClass::from_u8(class) as u8, class);
        }
    }

    #[test]
    fn a_status_the_host_answered_is_a_class_and_nothing_else() {
        assert_eq!(classify_status(401), ErrClass::Http401);
        assert_eq!(classify_status(403), ErrClass::Http403);
        assert_eq!(classify_status(503), ErrClass::Http5xx);
        assert_eq!(classify_status(404), ErrClass::Other);
    }

    #[test]
    fn the_text_of_the_agents_own_errors_is_sorted_by_what_it_says() {
        assert_eq!(classify_text("Local host credential is unavailable"), ErrClass::CredentialMissing);
        assert_eq!(classify_text("service returned 401"), ErrClass::Http401);
        assert_eq!(classify_text("service returned 503 Service Unavailable"), ErrClass::Http5xx);
        assert_eq!(classify_text("event stream ended"), ErrClass::StreamEnded);
        assert_eq!(classify_text("operation timed out"), ErrClass::Timeout);
        assert_eq!(classify_text("something unexpected at C:\\Users\\someone"), ErrClass::Other);
    }

    #[test]
    fn an_open_stream_without_a_recent_heartbeat_is_degraded_not_connected() {
        let now = 10_000_000;
        // fresh beat: connected
        assert_eq!(effective_state("connected", now - 600_000, now - 10_000, now), "connected");
        // no beat for over 45 s on a stream that has been open for minutes: degraded
        assert_eq!(effective_state("connected", now - 600_000, now - 50_000, now), "degraded");
        // never a beat on a stream open for minutes: degraded
        assert_eq!(effective_state("connected", now - 600_000, 0, now), "degraded");
        // just opened, the first beat is not due yet: not a complaint
        assert_eq!(effective_state("connected", now - 5_000, 0, now), "connected");
        // the other states pass through
        assert_eq!(effective_state("connecting", now - 600_000, 0, now), "connecting");
        assert_eq!(effective_state("disconnected", 0, 0, now), "disconnected");
        assert_eq!(effective_state("stopped", 0, 0, now), "stopped");
    }

    #[test]
    fn the_connected_share_falls_into_the_three_buckets() {
        assert_eq!(connected_bucket(0, 0), "lt50");
        assert_eq!(connected_bucket(49, 100), "lt50");
        assert_eq!(connected_bucket(50, 100), "50_90");
        assert_eq!(connected_bucket(90, 100), "50_90");
        assert_eq!(connected_bucket(91, 100), "gt90");
        assert_eq!(connected_bucket(500, 100), "gt90");
    }

    #[test]
    fn the_record_holds_words_and_counts_only() {
        let _guard = COUNTERS.lock().unwrap_or_else(|e| e.into_inner());
        run_started();
        connected();
        heartbeat_failed(ErrClass::Timeout);
        apply(true);
        apply(false);
        post(true);
        note_error(ErrClass::Http403);
        let event = health_event();
        assert_eq!(event["kind"], "experience");
        assert_eq!(event["name"], "clipboard_health");
        let props = event["props"].as_object().expect("props");
        assert_eq!(props["apply_ok"], 1);
        assert_eq!(props["apply_fail"], 1);
        assert_eq!(props["post_ok"], 1);
        assert_eq!(props["heartbeat_fail"], 1);
        assert_eq!(props["last_error"], "http_403");
        assert!(props.contains_key("first_connect_ms"));
        // every value is a number or one of the fixed words
        for (key, value) in props {
            assert!(value.is_number() || ["connected_pct", "last_error"].contains(&key.as_str()), "{key}");
        }
    }

    #[test]
    fn a_failure_that_is_over_leaves_the_status_and_stays_in_the_record() {
        let _guard = COUNTERS.lock().unwrap_or_else(|e| e.into_inner());
        run_started();
        note_error(ErrClass::Http5xx);
        connected();
        assert_eq!(current_error_class(), ErrClass::None);
        assert_eq!(health_event()["props"]["last_error"], "http_5xx");
        heartbeat_failed(ErrClass::Timeout);
        assert_eq!(health_event()["props"]["last_error"], "timeout");
        assert_eq!(health_event()["props"]["heartbeat_fail"], 1);
    }

    #[test]
    fn a_long_run_sends_one_record_a_day() {
        let _guard = COUNTERS.lock().unwrap_or_else(|e| e.into_inner());
        run_started();
        let now = now_ms();
        assert!(!due(now));
        assert!(due(now + 24 * 3_600_000 + 1));
    }
}
