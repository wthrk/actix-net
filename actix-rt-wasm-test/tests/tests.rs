/// 現状 actix_rt::System は wasm では使わない
use std::future::Future;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::wasm_bindgen_test as test;

use actix_rt::{
    time::{sleep, Instant},
    Arbiter, System,
};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

async fn spawn_and_wait<F>(f: F)
where
    F: Future<Output = ()> + 'static,
{
    let (tx, rx) = oneshot::channel::<()>();
    wasm_bindgen_futures::spawn_local(async move {
        f.await;
        tx.send(()).unwrap();
    });
    rx.await.unwrap();
}

#[test]
async fn join_another_arbiter() {
    let time = Duration::from_secs(1);
    let instant = Instant::now();
    spawn_and_wait(async move {
        let arbiter = Arbiter::new();
        let current = arbiter.handle();
        arbiter.spawn(Box::pin(async move {
            sleep(time).await;
            current.stop();
        }));
        arbiter.join_async().await.unwrap();
    })
    .await;
    assert!(
        instant.elapsed() >= time,
        "Join on another arbiter should complete only when it calls stop"
    );

    let instant = Instant::now();
    spawn_and_wait(async move {
        let arbiter = Arbiter::new();
        let current = arbiter.handle();
        arbiter.spawn_fn(move || {
            wasm_bindgen_futures::spawn_local(async move {
                sleep(time).await;
                current.stop();
            });
        });
        arbiter.join_async().await.unwrap();
    })
    .await;
    assert!(
        instant.elapsed() >= time,
        "Join on an arbiter that has used actix_rt::spawn should wait for said future"
    );

    let instant = Instant::now();
    spawn_and_wait(async move {
        let arbiter = Arbiter::new();
        let current = arbiter.handle();
        arbiter.spawn(Box::pin(async move {
            sleep(time).await;
            current.stop();
        }));
        arbiter.stop();
        arbiter.join_async().await.unwrap();
    })
    .await;
    assert!(
        instant.elapsed() < time,
        "Premature stop of arbiter should conclude regardless of it's current state"
    );
}

#[test]
async fn arbiter_spawn_fn_runs() {
    let (tx, mut rx) = mpsc::channel::<u32>(1);

    let arbiter = Arbiter::new();
    arbiter.spawn(async move { tx.send(42).await.unwrap() });

    let num = rx.recv().await.unwrap();

    assert_eq!(num, 42);

    arbiter.stop();
    arbiter.join_async().await.unwrap();
}

#[test]
async fn system_block_on_arbiter_is_registered() {
    let system = System::new();
    system
        .block_on(async {
            assert!(Arbiter::try_current().is_some());
        })
        .await;
}

#[test]
async fn system_block_on_arbiter_is_registered_with_spawn() {
    let (tx, mut rx) = mpsc::channel::<u32>(1);
    let system = System::new();
    system
        .block_on(async {
            actix_rt::spawn(async move {
                if let Some(arbiter) = Arbiter::try_current() {
                    tx.send(42).await.unwrap();
                } else {
                    tx.send(0).await.unwrap();
                }
            });
        })
        .await;

    let num = rx.recv().await.unwrap();

    assert_eq!(num, 42);
}
