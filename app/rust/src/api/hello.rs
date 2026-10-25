//! The M0 smoke test: the app calls into passingover-core.

/// The core's greeting, e.g. "Hello from passingover-core 0.1.0".
#[flutter_rust_bridge::frb(sync)]
pub fn hello() -> String {
    passingover_core::hello()
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
