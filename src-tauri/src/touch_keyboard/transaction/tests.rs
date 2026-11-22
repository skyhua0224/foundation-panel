//! In-memory fakes only: these tests never open a Windows registry key.

use super::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Default)]
struct Fixture {
    values: BTreeMap<String, RawValue>,
    journal: Option<Vec<u8>>,
    locked: bool,
    deny_read: bool,
    deny_write: bool,
    fail_flush: bool,
    fail_snapshot_sync: bool,
    fail_intent_sync_at: Option<usize>,
    appended_intents: usize,
    fail_set_name: Option<String>,
    writes: usize,
    flushes: usize,
    notification_value: u32,
    session_id: u32,
    logon_time: Option<i64>,
    session_query_failure: bool,
    notification_read_failure: bool,
    notification_fail_value: Option<u32>,
    notification_fail_after_publish: bool,
    notification_ignore_publish: bool,
    notification_unpublished: bool,
    notifications: Vec<(u32, BTreeMap<String, RawValue>)>,
}

type Shared = Rc<RefCell<Fixture>>;

struct FakeRegistry(Shared);

fn permission() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "simulated permission failure",
    )
}

impl FakeRegistry {
    fn assert_intent(fixture: &Fixture, name: &str) {
        let journal = parse_journal(
            fixture
                .journal
                .as_ref()
                .expect("write before durable snapshot"),
        )
        .expect("write before durable intent");
        let index = KEYS.iter().position(|entry| entry.name == name).unwrap();
        assert!(
            journal.intents.contains(&index),
            "write without a durable intent"
        );
    }
}

impl Registry for FakeRegistry {
    fn read(&mut self, name: &str) -> io::Result<Option<RawValue>> {
        let fixture = self.0.borrow();
        if fixture.deny_read {
            return Err(permission());
        }
        Ok(fixture.values.get(name).cloned())
    }

    fn set(&mut self, name: &str, value: &RawValue) -> io::Result<()> {
        let mut fixture = self.0.borrow_mut();
        Self::assert_intent(&fixture, name);
        if fixture.deny_write || fixture.fail_set_name.as_deref() == Some(name) {
            return Err(permission());
        }
        fixture.values.insert(name.to_string(), value.clone());
        fixture.writes += 1;
        Ok(())
    }

    fn delete(&mut self, name: &str) -> io::Result<()> {
        let mut fixture = self.0.borrow_mut();
        Self::assert_intent(&fixture, name);
        if fixture.deny_write {
            return Err(permission());
        }
        fixture.values.remove(name);
        fixture.writes += 1;
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut fixture = self.0.borrow_mut();
        fixture.flushes += 1;
        if fixture.fail_flush {
            Err(permission())
        } else {
            Ok(())
        }
    }
}

struct FakeJournal {
    fixture: Shared,
    held: bool,
}

impl Drop for FakeJournal {
    fn drop(&mut self) {
        self.unlock();
    }
}

impl Journal for FakeJournal {
    fn lock(&mut self) -> io::Result<()> {
        let mut fixture = self.fixture.borrow_mut();
        if fixture.locked {
            return Err(permission());
        }
        fixture.locked = true;
        self.held = true;
        Ok(())
    }

    fn unlock(&mut self) {
        if self.held {
            self.fixture.borrow_mut().locked = false;
            self.held = false;
        }
    }

    fn read(&mut self) -> io::Result<Option<Vec<u8>>> {
        Ok(self.fixture.borrow().journal.clone())
    }

    fn create(&mut self, contents: &[u8]) -> io::Result<()> {
        let mut fixture = self.fixture.borrow_mut();
        if fixture.journal.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "journal exists",
            ));
        }
        if fixture.fail_snapshot_sync {
            return Err(permission());
        }
        fixture.journal = Some(contents.to_vec());
        Ok(())
    }

    fn append(&mut self, contents: &[u8]) -> io::Result<()> {
        let mut fixture = self.fixture.borrow_mut();
        let fail = fixture.fail_intent_sync_at == Some(fixture.appended_intents);
        fixture.appended_intents += 1;
        let to_write = if fail {
            &contents[..contents.len() / 2]
        } else {
            contents
        };
        fixture
            .journal
            .as_mut()
            .unwrap()
            .extend_from_slice(to_write);
        if fail {
            let journal = fixture.journal.as_mut().unwrap();
            journal.truncate(journal.len() - to_write.len());
            return Err(permission());
        }
        Ok(())
    }

    fn remove(&mut self) -> io::Result<()> {
        self.fixture.borrow_mut().journal = None;
        Ok(())
    }
}

struct FakeNotification(Shared);

impl Notification for FakeNotification {
    fn current_session(&mut self) -> io::Result<u32> {
        Ok(self.0.borrow().session_id)
    }

    fn logon_time(&mut self, _: u32) -> io::Result<Option<i64>> {
        let fixture = self.0.borrow();
        if fixture.session_query_failure {
            Err(permission())
        } else {
            Ok(fixture.logon_time)
        }
    }

    fn read(&mut self, session_id: u32) -> io::Result<Option<u32>> {
        let mut fixture = self.0.borrow_mut();
        fixture.session_id = session_id;
        if fixture.notification_read_failure {
            return Err(permission());
        }
        if fixture.notification_unpublished {
            return Ok(None);
        }
        Ok(Some(fixture.notification_value))
    }

    fn publish(&mut self, _: u32, value: u32) -> io::Result<()> {
        let mut fixture = self.0.borrow_mut();
        let parsed = parse_journal(
            fixture
                .journal
                .as_deref()
                .expect("notification without snapshot"),
        )
        .expect("notification without durable intent");
        assert!(parsed.intents.contains(&NOTIFICATION_INDEX));
        if fixture.notification_fail_value == Some(value) {
            return Err(permission());
        }
        if fixture.notification_ignore_publish {
            return Ok(());
        }
        fixture.notification_value = value;
        fixture.notification_unpublished = false;
        let values = fixture.values.clone();
        fixture.notifications.push((value, values));
        if fixture.notification_fail_after_publish {
            fixture.notification_fail_after_publish = false;
            return Err(permission());
        }
        Ok(())
    }
}

fn transaction(fixture: &Shared) -> Transaction<FakeRegistry, FakeJournal, FakeNotification> {
    Transaction::new(
        FakeRegistry(fixture.clone()),
        FakeJournal {
            fixture: fixture.clone(),
            held: false,
        },
        FakeNotification(fixture.clone()),
    )
}

fn fixture() -> Shared {
    let mut data = Fixture::default();
    data.notification_value = 2;
    data.logon_time = Some(100);
    data.values.insert(KEYS[0].name.into(), dword(1));
    Rc::new(RefCell::new(data))
}

#[test]
fn restores_all_raw_types_and_missing_values_for_both_modes() {
    for enabled in [true, false] {
        for value in [
            None,
            Some(dword(1)),
            Some(RawValue {
                kind: 1,
                bytes: vec![65, 0, 0, 0],
            }),
            Some(RawValue {
                kind: 3,
                bytes: vec![],
            }),
        ] {
            let shared = fixture();
            match value {
                Some(value) => {
                    shared
                        .borrow_mut()
                        .values
                        .insert(KEYS[0].name.into(), value);
                }
                None => {
                    shared.borrow_mut().values.remove(KEYS[0].name);
                }
            }
            let original = shared.borrow().values.clone();
            let mut tx = transaction(&shared);
            tx.reconcile(Some(enabled), 7).unwrap();
            let mut expected = original.clone();
            expected.insert(KEYS[0].name.into(), dword(notification_target(enabled)));
            assert_eq!(shared.borrow().values, expected);
            tx.reconcile(None, 7).unwrap();
            assert_eq!(shared.borrow().values, original);
            assert!(shared.borrow().journal.is_none());
            assert!(!shared.borrow().locked);
        }
    }
}

#[test]
fn corrupt_snapshot_is_preserved_and_never_used_to_write() {
    let shared = fixture();
    shared.borrow_mut().journal = Some(b"{broken\n".to_vec());
    let original = shared.borrow().values.clone();
    let mut tx = transaction(&shared);
    assert!(tx.recover_on_start().is_err());
    assert!(tx.reconcile(Some(true), 11).is_err());
    assert_eq!(shared.borrow().values, original);
    assert_eq!(shared.borrow().journal.as_deref(), Some(&b"{broken\n"[..]));
    assert_eq!(shared.borrow().writes, 0);
}

#[test]
fn permission_or_snapshot_sync_failure_never_writes_registry_values() {
    for deny_read in [true, false] {
        let shared = fixture();
        shared.borrow_mut().deny_read = deny_read;
        shared.borrow_mut().fail_snapshot_sync = !deny_read;
        let original = shared.borrow().values.clone();
        let mut tx = transaction(&shared);
        assert!(tx.reconcile(Some(true), 11).is_err());
        assert_eq!(shared.borrow().values, original);
        assert_eq!(shared.borrow().writes, 0);
        assert!(shared.borrow().journal.is_none());
        shared.borrow_mut().deny_read = false;
        shared.borrow_mut().fail_snapshot_sync = false;
        tx.reconcile(Some(true), 11).unwrap();
        tx.reconcile(None, 11).unwrap();
        assert_eq!(shared.borrow().values, original);
    }
}

#[test]
fn partial_apply_permission_failure_rolls_back_the_completed_prefix() {
    let shared = fixture();
    let original = shared.borrow().values.clone();
    shared.borrow_mut().fail_set_name = Some(KEYS[0].name.into());
    let mut tx = transaction(&shared);
    assert!(tx.reconcile(Some(true), 11).is_err());
    assert_eq!(shared.borrow().values, original);
    assert!(shared.borrow().journal.is_none());
}

#[test]
fn failed_registry_flush_is_retried_before_journal_deletion() {
    let shared = fixture();
    let original = shared.borrow().values.clone();
    let mut tx = transaction(&shared);
    tx.reconcile(Some(true), 11).unwrap();
    shared.borrow_mut().fail_flush = true;
    assert!(tx.reconcile(None, 11).is_err());
    assert_eq!(shared.borrow().values, original);
    assert!(shared.borrow().journal.is_some());
    shared.borrow_mut().fail_flush = false;
    tx.reconcile(None, 11).unwrap();
    assert!(shared.borrow().flushes >= 2);
    assert!(shared.borrow().journal.is_none());
}

#[test]
fn a_reconnected_same_mode_does_not_skip_a_partially_failed_restore() {
    let shared = fixture();
    let original = shared.borrow().values.clone();
    let mut tx = transaction(&shared);
    tx.reconcile(Some(true), 11).unwrap();
    shared.borrow_mut().fail_set_name = Some(KEYS[0].name.into());
    assert!(tx.reconcile(None, 11).is_err());
    assert!(shared.borrow().journal.is_some());
    shared.borrow_mut().fail_set_name = None;
    tx.reconcile(Some(true), 11).unwrap();
    assert_eq!(shared.borrow().values.get(KEYS[0].name), Some(&dword(2)));
    tx.reconcile(None, 11).unwrap();
    assert_eq!(shared.borrow().values, original);
    assert!(shared.borrow().journal.is_none());
}

#[test]
fn externally_changed_value_is_preserved_while_notification_is_restored() {
    let shared = fixture();
    let mut tx = transaction(&shared);
    tx.reconcile(Some(false), 11).unwrap();
    let external = RawValue {
        kind: 3,
        bytes: vec![4, 5, 6],
    };
    shared
        .borrow_mut()
        .values
        .insert(KEYS[0].name.into(), external.clone());
    tx.reconcile(None, 11).unwrap();
    assert_eq!(shared.borrow().values.get(KEYS[0].name), Some(&external));
    assert_eq!(shared.borrow().notification_value, 2);
    assert!(shared.borrow().journal.is_none());
}

#[test]
fn simultaneous_transactions_do_not_overwrite_each_others_journal() {
    let shared = fixture();
    let original = shared.borrow().values.clone();
    let mut first = transaction(&shared);
    let mut second = transaction(&shared);
    first.reconcile(Some(true), 1).unwrap();
    let saved = shared.borrow().journal.clone();
    assert!(second.reconcile(Some(false), 2).is_err());
    assert_eq!(shared.borrow().journal, saved);
    first.reconcile(None, 1).unwrap();
    second.reconcile(Some(false), 2).unwrap();
    second.reconcile(None, 2).unwrap();
    assert_eq!(shared.borrow().values, original);
}

#[test]
fn notification_follows_registry_apply_and_restore_for_all_original_modes() {
    for original_mode in 0..=2 {
        for enabled in [false, true] {
            let shared = fixture();
            shared.borrow_mut().notification_value = original_mode;
            let original = shared.borrow().values.clone();
            let mut tx = transaction(&shared);
            tx.reconcile(Some(enabled), 11).unwrap();
            let target = notification_target(enabled);
            assert_eq!(shared.borrow().notification_value, target);
            let (_, at_publish) = shared.borrow().notifications[0].clone();
            assert_eq!(
                at_publish.get("TouchKeyboardTapInvoke"),
                Some(&dword(target))
            );
            tx.reconcile(None, 11).unwrap();
            let data = shared.borrow();
            assert_eq!(data.notification_value, original_mode);
            assert_eq!(data.values, original);
            if target != original_mode {
                assert_eq!(
                    data.notifications.last().unwrap(),
                    &(original_mode, original)
                );
            }
            assert!(data.journal.is_none());
        }
    }
}

#[test]
fn registry_already_matching_does_not_skip_the_live_notification() {
    let shared = fixture();
    for definition in &KEYS {
        shared
            .borrow_mut()
            .values
            .insert(definition.name.into(), dword(definition.disabled));
    }
    let original = shared.borrow().values.clone();
    let mut tx = transaction(&shared);
    tx.reconcile(Some(false), 11).unwrap();
    assert_eq!(shared.borrow().writes, 0);
    assert_eq!(shared.borrow().flushes, 0);
    assert_eq!(shared.borrow().notification_value, 0);
    tx.reconcile(None, 11).unwrap();
    assert_eq!(shared.borrow().notification_value, 2);
    assert_eq!(shared.borrow().values, original);
    assert_eq!(shared.borrow().flushes, 0);
}

#[test]
fn unavailable_notification_api_leaves_registry_and_journal_untouched() {
    let shared = fixture();
    shared.borrow_mut().notification_read_failure = true;
    let mut tx = transaction(&shared);
    assert!(tx.reconcile(Some(false), 11).is_err());
    let data = shared.borrow();
    assert_eq!(data.writes, 0);
    assert!(data.journal.is_none());
    assert!(data.notifications.is_empty());
}

#[test]
fn failed_notification_intent_does_not_publish_and_rolls_back_registry() {
    let shared = fixture();
    let original = shared.borrow().values.clone();
    // The registry write is the first intent; notification is the second.
    shared.borrow_mut().fail_intent_sync_at = Some(1);
    let mut tx = transaction(&shared);
    assert!(tx.reconcile(Some(false), 11).is_err());
    let data = shared.borrow();
    assert_eq!(data.values, original);
    assert_eq!(data.notification_value, 2);
    assert!(data.notifications.is_empty());
    assert!(data.journal.is_none());
}

#[test]
fn failed_publication_or_readback_rolls_back_and_can_retry() {
    for fail_after_publish in [false, true] {
        let shared = fixture();
        let original = shared.borrow().values.clone();
        shared.borrow_mut().notification_fail_after_publish = fail_after_publish;
        shared.borrow_mut().notification_ignore_publish = !fail_after_publish;
        let mut tx = transaction(&shared);
        assert!(tx.reconcile(Some(false), 11).is_err());
        assert_eq!(shared.borrow().values, original);
        assert_eq!(shared.borrow().notification_value, 2);
        assert!(shared.borrow().journal.is_none());
        shared.borrow_mut().notification_ignore_publish = false;
        tx.reconcile(Some(false), 11).unwrap();
        tx.reconcile(None, 11).unwrap();
        assert_eq!(shared.borrow().notification_value, 2);
    }
}

#[test]
fn failed_notification_restore_keeps_journal_after_registry_is_restored() {
    let shared = fixture();
    let original = shared.borrow().values.clone();
    let mut tx = transaction(&shared);
    tx.reconcile(Some(false), 11).unwrap();
    shared.borrow_mut().notification_fail_value = Some(2);
    assert!(tx.reconcile(None, 11).is_err());
    assert_eq!(shared.borrow().values, original);
    assert_eq!(shared.borrow().notification_value, 0);
    assert!(shared.borrow().journal.is_some());
    shared.borrow_mut().notification_fail_value = None;
    tx.reconcile(None, 11).unwrap();
    assert_eq!(shared.borrow().notification_value, 2);
    assert!(shared.borrow().journal.is_none());
}

#[test]
fn crash_after_publication_recovers_original_live_value() {
    let shared = fixture();
    let original = shared.borrow().values.clone();
    {
        let mut tx = transaction(&shared);
        tx.reconcile(Some(false), 11).unwrap();
    }
    assert_eq!(shared.borrow().notification_value, 0);
    transaction(&shared).recover_on_start().unwrap();
    assert_eq!(shared.borrow().notification_value, 2);
    assert_eq!(shared.borrow().values, original);
    assert!(shared.borrow().journal.is_none());
}

#[test]
fn crash_recovery_respects_windows_logon_lifetime() {
    for (current_session, logon_time, still_live) in [
        (12, Some(100), true),
        (12, None, false),
        (11, Some(200), false),
    ] {
        let shared = fixture();
        let original = shared.borrow().values.clone();
        {
            let mut tx = transaction(&shared);
            tx.reconcile(Some(false), 11).unwrap();
        }
        {
            let mut data = shared.borrow_mut();
            data.session_id = current_session;
            data.logon_time = logon_time;
            data.notification_value = 1;
        }
        let result = transaction(&shared).recover_on_start();
        let data = shared.borrow();
        assert_eq!(result.is_err(), still_live);
        assert_eq!(data.journal.is_some(), still_live);
        assert_eq!(data.notification_value, 1);
        if still_live {
            assert_eq!(data.values.get(KEYS[0].name), Some(&dword(0)));
        } else {
            assert_eq!(data.values, original);
        }
    }
}

#[test]
fn session_query_failure_preserves_pending_recovery() {
    let shared = fixture();
    {
        let mut tx = transaction(&shared);
        tx.reconcile(Some(false), 11).unwrap();
    }
    shared.borrow_mut().session_query_failure = true;
    assert!(transaction(&shared).recover_on_start().is_err());
    let data = shared.borrow();
    assert!(data.journal.is_some());
    assert_eq!(data.values.get(KEYS[0].name), Some(&dword(0)));
    assert_eq!(data.notification_value, 0);
}

#[test]
fn switching_modes_preserves_the_original_live_value() {
    let shared = fixture();
    let original = shared.borrow().values.clone();
    shared.borrow_mut().notification_value = 1;
    let mut tx = transaction(&shared);
    tx.reconcile(Some(false), 11).unwrap();
    tx.reconcile(Some(true), 11).unwrap();
    tx.reconcile(None, 11).unwrap();
    let sequence: Vec<u32> = shared
        .borrow()
        .notifications
        .iter()
        .map(|(v, _)| *v)
        .collect();
    assert_eq!(sequence, [0, 1, 2, 1]);
    assert_eq!(shared.borrow().values, original);
}

#[test]
fn externally_changed_live_preference_is_preserved_on_restore() {
    let shared = fixture();
    let mut tx = transaction(&shared);
    tx.reconcile(Some(false), 11).unwrap();
    shared.borrow_mut().notification_value = 1;
    tx.reconcile(None, 11).unwrap();
    assert_eq!(shared.borrow().notification_value, 1);
    assert!(shared.borrow().journal.is_none());
}

#[test]
fn damaged_tail_does_not_prevent_restoring_a_completed_notification_intent() {
    let shared = fixture();
    let original = shared.borrow().values.clone();
    {
        let mut tx = transaction(&shared);
        tx.reconcile(Some(false), 11).unwrap();
        shared
            .borrow_mut()
            .journal
            .as_mut()
            .unwrap()
            .extend_from_slice(b"{\"index\":");
    }
    transaction(&shared).recover_on_start().unwrap();
    assert_eq!(shared.borrow().values, original);
    assert_eq!(shared.borrow().notification_value, 2);
    assert!(shared.borrow().journal.is_none());
}

#[test]
fn uninitialized_notification_is_initialized_and_restored_from_registry() {
    let shared = fixture();
    shared.borrow_mut().values.insert(KEYS[0].name.into(), dword(2));
    shared.borrow_mut().notification_unpublished = true;
    let original = shared.borrow().values.clone();
    let mut tx = transaction(&shared);
    tx.reconcile(Some(false), 11).unwrap();
    assert_eq!(shared.borrow().values.get(KEYS[0].name), Some(&dword(0)));
    assert_eq!(shared.borrow().notification_value, 0);
    tx.reconcile(None, 11).unwrap();
    let data = shared.borrow();
    assert_eq!(data.values, original);
    assert_eq!(data.notification_value, 2);
    assert!(data.journal.is_none());
    let sequence: Vec<u32> = data.notifications.iter().map(|(value, _)| *value).collect();
    assert_eq!(sequence, [0, 2]);
}

#[test]
fn uninitialized_notification_with_default_registry_restores_to_never() {
    let shared = fixture();
    shared.borrow_mut().values.remove(KEYS[0].name);
    shared.borrow_mut().notification_unpublished = true;
    let mut tx = transaction(&shared);
    tx.reconcile(Some(true), 11).unwrap();
    assert_eq!(shared.borrow().values.get(KEYS[0].name), Some(&dword(2)));
    assert_eq!(shared.borrow().notification_value, 2);
    tx.reconcile(None, 11).unwrap();
    let data = shared.borrow();
    assert!(!data.values.contains_key(KEYS[0].name));
    assert_eq!(data.notification_value, 0);
    assert!(data.journal.is_none());
}

#[test]
fn crash_recovery_restores_a_state_that_started_uninitialized() {
    let shared = fixture();
    shared.borrow_mut().values.insert(KEYS[0].name.into(), dword(2));
    shared.borrow_mut().notification_unpublished = true;
    let original = shared.borrow().values.clone();
    {
        let mut tx = transaction(&shared);
        tx.reconcile(Some(false), 11).unwrap();
    }
    assert_eq!(shared.borrow().notification_value, 0);
    transaction(&shared).recover_on_start().unwrap();
    let data = shared.borrow();
    assert_eq!(data.values, original);
    assert_eq!(data.notification_value, 2);
    assert!(data.journal.is_none());
}

#[test]
fn externally_published_value_is_preserved_when_original_was_uninitialized() {
    let shared = fixture();
    shared.borrow_mut().values.insert(KEYS[0].name.into(), dword(2));
    shared.borrow_mut().notification_unpublished = true;
    let original = shared.borrow().values.clone();
    let mut tx = transaction(&shared);
    tx.reconcile(Some(false), 11).unwrap();
    {
        let mut data = shared.borrow_mut();
        data.notification_unpublished = false;
        data.notification_value = 1;
    }
    tx.reconcile(None, 11).unwrap();
    let data = shared.borrow();
    assert_eq!(data.values, original);
    assert_eq!(data.notification_value, 1);
    let sequence: Vec<u32> = data.notifications.iter().map(|(value, _)| *value).collect();
    assert_eq!(sequence, [0]);
    assert!(data.journal.is_none());
}

#[test]
fn externally_changed_registry_value_drives_uninitialized_notification_restore() {
    let shared = fixture();
    shared.borrow_mut().values.insert(KEYS[0].name.into(), dword(1));
    shared.borrow_mut().notification_unpublished = true;
    let mut tx = transaction(&shared);
    tx.reconcile(Some(false), 11).unwrap();
    shared
        .borrow_mut()
        .values
        .insert(KEYS[0].name.into(), dword(2));

    tx.reconcile(None, 11).unwrap();

    let data = shared.borrow();
    assert_eq!(data.values.get(KEYS[0].name), Some(&dword(2)));
    assert_eq!(data.notification_value, 2);
    let sequence: Vec<u32> = data.notifications.iter().map(|(value, _)| *value).collect();
    assert_eq!(sequence, [0, 2]);
    assert!(data.journal.is_none());
}

#[test]
fn failed_publication_with_uninitialized_state_rolls_back_and_retries() {
    let shared = fixture();
    shared.borrow_mut().values.insert(KEYS[0].name.into(), dword(2));
    shared.borrow_mut().notification_unpublished = true;
    shared.borrow_mut().notification_fail_after_publish = true;
    let original = shared.borrow().values.clone();
    let mut tx = transaction(&shared);
    assert!(tx.reconcile(Some(false), 11).is_err());
    {
        let data = shared.borrow();
        assert_eq!(data.values, original);
        assert_eq!(data.notification_value, 2);
        assert!(data.journal.is_none());
    }
    shared.borrow_mut().notification_fail_after_publish = false;
    tx.reconcile(Some(false), 11).unwrap();
    tx.reconcile(None, 11).unwrap();
    assert_eq!(shared.borrow().notification_value, 2);
}

#[test]
fn journal_with_invalid_notification_value_is_rejected() {
    let snapshot = format!("{{\"version\":1,\"session_id\":11,\"logon_time\":100,\"enabled\":false,\"values\":[{{\"name\":\"{}\",\"original\":{{\"kind\":4,\"bytes\":[2,0,0,0]}}}}],\"notification_original\":3}}\n", KEYS[0].name);
    let shared = fixture();
    shared.borrow_mut().journal = Some(snapshot.into_bytes());
    let original = shared.borrow().values.clone();
    let mut tx = transaction(&shared);
    assert!(tx.recover_on_start().is_err());
    assert_eq!(shared.borrow().values, original);
    assert!(shared.borrow().journal.is_some());
}
