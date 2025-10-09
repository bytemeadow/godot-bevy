/*
 * Ergonomic helpers for writing async integration tests
 */

use std::sync::Arc;
use std::sync::atomic::AtomicU32;

/// Helper to create a shared counter that can be used in systems
/// Clone is cheap (just an Arc clone), so it works in Fn closures
#[derive(Clone, Default)]
pub struct Counter(Arc<AtomicU32>);

impl Counter {
    pub fn new() -> Self {
        Self(Arc::new(AtomicU32::new(0)))
    }

    pub fn get(&self) -> u32 {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn increment(&self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Macro to reduce boilerplate in async BevyApp tests
///
/// For tests, we use a minimal plugin setup - just the core schedules without any watchers.
/// This avoids the watcher initialization issues that would require full scene setup.
///
/// Each test gets its own isolated BevyApp instance with its own configuration.
///
/// Usage:
/// ```
/// bevy_app_test!(ctx, counter, |app| {
///     // Setup bevy app
///     app.add_systems(Update, my_system);
/// }, async {
///     // Async test body
///     await_frames(5).await;
///     assert_eq!(counter.get(), 5);
/// })
/// ```
#[macro_export]
macro_rules! bevy_app_test {
    ($ctx:expr, $counter:ident, |$app:ident| $setup:block, async $test:block) => {{
        use godot::obj::NewAlloc;

        let $counter = $crate::framework::Counter::new();
        let ctx_clone = $ctx.clone();

        godot::task::spawn(async move {
            // Wait one frame to ensure previous test's BevyApp is cleaned up
            $crate::framework::await_frame().await;

            // Create BevyApp and set its instance init function
            let mut bevy_app = godot_bevy::BevyApp::new_alloc();

            // Set the per-instance init function for THIS test
            let c = $counter.clone();
            bevy_app.bind_mut().set_instance_init_func(Box::new(move |$app: &mut bevy::prelude::App| {
                let $counter = c.clone();
                // Add core plugins for schedules, but not the optional plugins (no watchers)
                $app.add_plugins(godot_bevy::plugins::GodotBaseCorePlugin);
                // Now add test-specific setup
                $setup
            }));

            // Add to scene tree (this will trigger ready())
            ctx_clone.scene_tree.clone().add_child(&bevy_app);

            // Wait for ready() to complete
            $crate::framework::await_frame().await;

            // Run the test
            $test

            // Cleanup
            bevy_app.queue_free();

            // Wait for cleanup to complete
            $crate::framework::await_frame().await;
        })
    }};
}
