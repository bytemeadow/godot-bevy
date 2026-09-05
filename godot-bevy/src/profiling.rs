//! Profiling support with zero dependency leakage
//!
//! This module encapsulates all Tracy-specific code so that the proc macro
//! never needs to reference Tracy types directly. This prevents Tracy
//! dependencies from leaking into user code while maintaining full profiling support.

#[cfg(feature = "trace_tracy")]
use once_cell::sync::Lazy;

#[cfg(feature = "trace_tracy")]
static TRACY_CLIENT: Lazy<tracing_tracy::client::Client> =
    Lazy::new(tracing_tracy::client::Client::start);

/// Initialize the profiling system
/// Called by the #[bevy_app] macro during library initialization
pub fn init_profiler() {
    #[cfg(feature = "trace_tracy")]
    {
        use godot::obj::Singleton;
        let original_port = godot::classes::Os::singleton().get_environment("TRACY_PORT");
        let editor_port =
            godot::classes::Os::singleton().get_environment("GODOT_EDITOR_TRACY_PORT");
        let editor_port = if editor_port.is_empty() {
            godot::builtin::GString::from("7867")
        } else {
            editor_port
        };

        // Editor and game instances require distinct Tracy ports.
        if godot::classes::Engine::singleton().is_editor_hint() {
            godot::classes::Os::singleton().set_environment("TRACY_PORT", &editor_port);
        }

        let _ = &*TRACY_CLIENT;

        // Game instances must not inherit the editor's Tracy port.
        godot::classes::Os::singleton().set_environment("TRACY_PORT", &original_port);
    }
}

/// Shutdown the profiling system cleanly
/// Called by the #[bevy_app] macro during library deinitialization
pub fn shutdown_profiler() {
    #[cfg(feature = "trace_tracy")]
    {
        TRACY_CLIENT.frame_mark();

        // Give Tracy time to flush data
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// Mark the beginning of a frame
#[inline]
pub fn frame_mark() {
    #[cfg(feature = "trace_tracy")]
    {
        TRACY_CLIENT.frame_mark();
    }
}

/// Mark a secondary frame (e.g., physics)
#[inline]
pub fn secondary_frame_mark(name: &str) {
    #[cfg(feature = "trace_tracy")]
    {
        // Tracy's frame_name! macro accepts only literals; physics is our only secondary frame.
        match name {
            "physics" => {
                use tracing_tracy::client::frame_name;
                TRACY_CLIENT.secondary_frame_mark(frame_name!("physics"));
            }
            _ => {
                // Tracy secondary frame names must be compile-time literals.
                TRACY_CLIENT.frame_mark();
            }
        }
    }
    #[cfg(not(feature = "trace_tracy"))]
    {
        let _ = name;
    }
}

/// Check if profiler is running
#[inline]
pub fn is_profiler_running() -> bool {
    #[cfg(feature = "trace_tracy")]
    {
        tracing_tracy::client::Client::is_running()
    }
    #[cfg(not(feature = "trace_tracy"))]
    {
        false
    }
}

/// Create a profiling scope/span
///
/// Use this instead of direct tracing macros when you want
/// conditional profiling that doesn't leak dependencies
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        #[cfg(feature = "trace_tracy")]
        let _guard = tracing::span!(tracing::Level::INFO, $name).entered();
    };
}

/// Re-export for systems that want to use tracing instrumentation
/// This allows using #[godot_bevy::profile] without adding tracing as dependency
#[cfg(feature = "trace_tracy")]
pub use tracing::instrument as profile;

#[cfg(not(feature = "trace_tracy"))]
#[macro_export]
macro_rules! profile {
    ($($tt:tt)*) => {};
}
