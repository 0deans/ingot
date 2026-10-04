use super::*;
use std::sync::Arc;

fn code() -> MicrosoftDeviceCode {
    MicrosoftDeviceCode {
        user_code: "user".into(),
        verification_uri: "https://fixture.invalid/link".into(),
        device_code: "secret".into(),
        interval: 5,
        expires_in: 900,
    }
}

#[test]
fn cancellation_is_scoped_to_the_application() {
    let first = LoginAttempts::default();
    let second = LoginAttempts::default();
    let attempt = first.begin().unwrap();
    second.invalidate();
    assert!(!first.is_cancelled(attempt.id));
    first.invalidate();
    assert!(first.is_cancelled(attempt.id));
}

#[test]
fn stale_start_response_cannot_replace_a_newer_code() {
    let login = LoginAttempts::default();
    let old = login.begin().unwrap();
    let current = login.begin().unwrap();
    let mut new_code = code();
    new_code.device_code = "new-secret".into();
    login.issued(current, new_code.clone()).unwrap();
    assert!(matches!(login.issued(old, code()), Err(LoginError::Stale)));
    assert!(login.claim(&new_code).is_ok());
}

#[test]
fn issuance_and_completion_share_the_original_monotonic_deadline() {
    let time = Arc::new(AtomicU64::new(0));
    let clock = time.clone();
    let login =
        LoginAttempts::with_clock(move || Duration::from_millis(clock.load(Ordering::SeqCst)));
    let attempt = login.begin().unwrap();
    time.store(120_000, Ordering::SeqCst);
    let issued = login.issued(attempt, code()).unwrap();
    time.store(300_500, Ordering::SeqCst);
    assert_eq!(login.claim(&issued).unwrap().code.expires_in, 599);
}

#[test]
fn expired_responses_and_exhausted_generations_cannot_issue_codes() {
    let time = Arc::new(AtomicU64::new(0));
    let clock = time.clone();
    let login =
        LoginAttempts::with_clock(move || Duration::from_secs(clock.load(Ordering::SeqCst)));
    let attempt = login.begin().unwrap();
    time.store(900, Ordering::SeqCst);
    assert!(matches!(
        login.issued(attempt, code()),
        Err(LoginError::Expired)
    ));
    assert!(matches!(login.claim(&code()), Err(LoginError::UnknownCode)));
    login.generation.store(u64::MAX - 1, Ordering::SeqCst);
    assert!(matches!(login.begin(), Err(LoginError::Exhausted)));
    login.invalidate();
    assert_eq!(login.current(), AttemptId(u64::MAX));
    assert!(matches!(login.begin(), Err(LoginError::Exhausted)));
}

#[test]
fn diagnostics_do_not_include_device_code_secrets() {
    let login = LoginAttempts::default();
    let issued = login.issued(login.begin().unwrap(), code()).unwrap();
    let mut changed = issued.clone();
    changed.device_code = "private-device-code".into();
    let error = login.claim(&changed).err().unwrap();
    assert!(matches!(error, LoginError::UnknownCode));
    assert!(!format!("{error:?}: {error}").contains("private-device-code"));
}

#[test]
fn concurrent_completions_claim_the_code_once() {
    let login = LoginAttempts::default();
    let issued = login.issued(login.begin().unwrap(), code()).unwrap();
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            barrier.wait();
            login.claim(&issued)
        });
        let second = scope.spawn(|| {
            barrier.wait();
            login.claim(&issued)
        });
        let outcomes = [first.join().unwrap(), second.join().unwrap()];
        assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| matches!(outcome, Err(LoginError::AlreadyClaimed)))
                .count(),
            1
        );
    });
}

#[test]
fn poisoned_state_blocks_sign_in_but_still_allows_cancellation() {
    let login = Arc::new(LoginAttempts::default());
    let attempt = login.begin().unwrap();
    let other = login.clone();
    assert!(std::thread::spawn(move || {
        let _guard = other.issued.lock().unwrap();
        panic!("injected state failure");
    })
    .join()
    .is_err());
    assert!(matches!(login.begin(), Err(LoginError::Unavailable)));
    assert!(matches!(login.claim(&code()), Err(LoginError::Unavailable)));
    assert!(matches!(
        login.publication(attempt.id),
        Err(LoginError::Unavailable)
    ));
    login.invalidate();
    assert!(login.is_cancelled(attempt.id));
    assert!(matches!(login.begin(), Err(LoginError::Unavailable)));
}
