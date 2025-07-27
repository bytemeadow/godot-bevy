#[cfg(feature = "profiling")]
// Single global handle; will be initialised exactly once.
pub static TRACY_CLIENT: std::sync::OnceLock<tracing_tracy::client::Client> =
    std::sync::OnceLock::new();
