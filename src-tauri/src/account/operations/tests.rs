use super::*;
use std::{
    future::{poll_fn, Future},
    task::Poll,
};

#[tokio::test]
async fn same_account_waits_while_other_accounts_and_applications_can_proceed() {
    let operations = AccountOperations::default();
    let first = operations.acquire("a").await.unwrap();
    let mut waiting = Box::pin(operations.acquire("a"));
    assert!(poll_fn(|cx| Poll::Ready(waiting.as_mut().poll(cx).is_pending())).await);
    let _other = operations.acquire("b").await.unwrap();
    let independent = AccountOperations::default();
    let _independent = independent.acquire("a").await.unwrap();
    drop(first);
    let _next = waiting.await.unwrap();
}

#[tokio::test]
async fn cancelling_owner_releases_account_for_waiters() {
    let operations = Arc::new(AccountOperations::default());
    let owner_operations = Arc::clone(&operations);
    let (ready, started) = tokio::sync::oneshot::channel();
    let owner = tokio::spawn(async move {
        let _guard = owner_operations.acquire("a").await.unwrap();
        ready.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    started.await.unwrap();
    let mut waiting = Box::pin(operations.acquire("a"));
    assert!(poll_fn(|cx| Poll::Ready(waiting.as_mut().poll(cx).is_pending())).await);
    owner.abort();
    assert!(owner.await.unwrap_err().is_cancelled());
    let _next = waiting.await.unwrap();
}

#[tokio::test]
async fn cancelling_waiter_does_not_reserve_account_after_owner_leaves() {
    let operations = AccountOperations::default();
    let guard = operations.acquire("a").await.unwrap();
    let mut waiting = Box::pin(operations.acquire("a"));
    assert!(poll_fn(|cx| Poll::Ready(waiting.as_mut().poll(cx).is_pending())).await);
    drop(waiting);
    drop(guard);
    let _next = operations.acquire("a").await.unwrap();
}
