/// A Tokio-based runtime proxy.
///
/// All spawned futures will be executed on the current thread. Therefore, there is no `Send` bound
/// on submitted futures.
#[derive(Debug)]
pub struct Runtime();

impl Runtime {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> std::io::Result<Self> {
        Ok(Runtime())
    }
}
