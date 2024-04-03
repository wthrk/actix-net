use std::{
    cell::RefCell,
    collections::HashMap,
    future::Future,
    io,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll},
};

use futures_core::ready;
use tokio::sync::{mpsc, oneshot};

use crate::{arbiter::ArbiterHandle, Arbiter};

static SYSTEM_COUNT: AtomicUsize = AtomicUsize::new(0);

thread_local!(
    static CURRENT: RefCell<Option<System>> = RefCell::new(None);
);

/// A manager for a per-thread distributed async runtime.
#[derive(Clone, Debug)]
pub struct System {
    id: usize,

    /// Handle to the first [Arbiter] that is created with the System.
    arbiter_handle: ArbiterHandle,
}

impl System {
    pub fn new() -> SystemRunner {
        SystemRunner::new()
    }

    /// Constructs new system and registers it on the current thread.
    pub(crate) fn construct(arbiter_handle: ArbiterHandle) {
        let sys = System {
            arbiter_handle,
            id: SYSTEM_COUNT.fetch_add(1, Ordering::SeqCst),
        };

        System::set_current(sys.clone());
    }

    /// Get current running system.
    ///
    /// # Panics
    /// Panics if no system is registered on the current thread.
    pub fn current() -> System {
        CURRENT.with(|cell| match *cell.borrow() {
            Some(ref sys) => sys.clone(),
            None => panic!("System is not running"),
        })
    }

    /// Try to get current running system.
    ///
    /// Returns `None` if no System has been started.
    ///
    /// Unlike [`current`](Self::current), this never panics.
    pub fn try_current() -> Option<System> {
        CURRENT.with(|cell| cell.borrow().clone())
    }

    /// Get handle to a the System's initial [Arbiter].
    pub fn arbiter(&self) -> &ArbiterHandle {
        &self.arbiter_handle
    }

    /// Check if there is a System registered on the current thread.
    pub fn is_registered() -> bool {
        CURRENT.with(|sys| sys.borrow().is_some())
    }

    /// Register given system on current thread.
    #[doc(hidden)]
    pub fn set_current(sys: System) {
        CURRENT.with(|cell| {
            *cell.borrow_mut() = Some(sys);
        })
    }

    /// Numeric system identifier.
    ///
    /// Useful when using multiple Systems.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Stop the system (with code 0).
    pub fn stop(&self) {
        self.stop_with_code(0)
    }

    /// Stop the system with a given exit code.
    pub fn stop_with_code(&self, _code: i32) {
        self.arbiter().stop();
    }

    //pub(crate) fn tx(&self) -> &mpsc::UnboundedSender<SystemCommand> {
    //    &self.sys_tx
    //}
}

pub struct SystemRunner(RefCell<Option<Arbiter>>);
pub struct SystemCommand;

impl SystemRunner {
    pub fn new() -> Self {
        SystemRunner(RefCell::new(None))
    }

    /// なんとなく他のと似た API にしてみる
    pub async fn block_on<F: Future>(&self, fut: F) -> F::Output {
        self.run();
        let result = fut.await;
        // self.stop();
        result
    }

    /// Run the system.
    pub fn run(&self) {
        if Arbiter::try_current().is_none() {
            let arbiter = Arbiter::new();
            self.0.borrow_mut().replace(arbiter);
        }
    }

    fn stop(&self) {
        if let Some(arbiter) = self.0.borrow_mut().take() {
            arbiter.stop();
        }
    }
}
