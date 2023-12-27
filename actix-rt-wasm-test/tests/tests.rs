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

// シングルスレッドでの動作なので panic すると panic する
// wasm_bindgen_futures::spawn_local は panic が起きるとリークするっぽい
// 以下同様
// see: https://rustwasm.github.io/wasm-bindgen/api/wasm_bindgen_futures/fn.future_to_promise.html#:~:text=or%20Err.-,Panics,-Note%20that%20in
#[test]
#[should_panic]
#[ignore]
async fn arbiter_drop_no_panic_fn() {
    // let _ = System::new();

    let arbiter = Arbiter::new();
    arbiter.spawn_fn(|| panic!("test"));

    arbiter.stop();
    arbiter.join_async().await.unwrap();
}

#[test]
#[should_panic]
#[ignore]
fn no_system_current_panic() {
    System::current();
}

#[test]
#[ignore]
fn no_system_arbiter_new_panic() {
    Arbiter::new();
}

#[test]
#[ignore]
fn try_current_no_system() {
    assert!(System::try_current().is_none())
}
