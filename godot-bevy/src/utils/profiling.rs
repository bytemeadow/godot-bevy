#[cfg(feature = "trace_tracy")]
// Note: static items do not call [`Drop`] on program termination, so this won't be deallocated.
// this is fine, as the OS can deallocate the terminated program faster than we can free memory
// but tools like valgrind might report "memory leaks" as it isn't obvious this is intentional.
pub static TRACY_CLIENT: std::sync::LazyLock<tracing_tracy::client::Client> =
    std::sync::LazyLock::new(tracing_tracy::client::Client::start);
