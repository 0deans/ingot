use super::*;

#[test]
fn microsoft_token_refresh_margin_includes_the_boundary_and_missing_expiry() {
    assert!(!microsoft_token_is_fresh(None, 1000));
    assert!(!microsoft_token_is_fresh(Some(999), 1000));
    assert!(!microsoft_token_is_fresh(Some(1300), 1000));
    assert!(microsoft_token_is_fresh(Some(1301), 1000));
    assert!(!microsoft_token_is_fresh(Some(u64::MAX), u64::MAX));
}

fn profile(id: &str, active: bool) -> AccountProfile {
    AccountProfile {
        id: id.to_owned(),
        account_type: "offline".to_owned(),
        username: id.to_owned(),
        uuid: id.to_owned(),
        skin_url: None,
        is_active: active,
        created_at: 42,
    }
}

#[test]
fn selection_has_one_active_account_and_missing_selection_does_not_mutate() {
    let mut accounts = vec![profile("a", true), profile("b", false)];
    select(&mut accounts, "b").unwrap();
    assert!(!accounts[0].is_active);
    assert!(accounts[1].is_active);
    assert!(matches!(
        select(&mut accounts, "missing"),
        Err(AccountPolicyError::NotFound)
    ));
    assert!(!accounts[0].is_active);
    assert!(accounts[1].is_active);
}

#[test]
fn removal_promotes_first_only_when_the_active_account_was_removed() {
    let mut accounts = vec![profile("a", false), profile("b", true), profile("c", false)];
    remove(&mut accounts, "c").unwrap();
    assert!(accounts[1].is_active);
    assert!(!accounts[0].is_active);
    assert!(remove(&mut accounts, "b").unwrap().is_active);
    assert!(accounts[0].is_active);
    assert!(matches!(
        remove(&mut accounts, "missing"),
        Err(AccountPolicyError::NotFound)
    ));
    remove(&mut accounts, "a").unwrap();
    assert!(accounts.is_empty());
}

#[test]
fn partial_reorder_preserves_omitted_accounts_metadata_and_selection() {
    let accounts = vec![profile("a", false), profile("b", true), profile("c", false)];
    let reordered = reorder(&accounts, &["c".to_owned(), "unknown".to_owned()]);
    assert_eq!(
        reordered.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
        ["c", "a", "b"]
    );
    assert!(reordered[2].is_active);
    assert!(reordered
        .iter()
        .all(|a| a.created_at == 42 && a.skin_url.is_none()));
    assert_eq!(accounts[0].id, "a");
}
