/// HostApi is the runtime capability set exposed to plugin guests.
#[derive(Debug, Default)]
pub struct HostRuntime {
    // Will hold tokio runtime handle, reqwest client, etc.
}

impl HostRuntime {
    pub fn new() -> Self {
        Self {}
    }
}
