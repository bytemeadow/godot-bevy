use bevy::prelude::*;
use godot_bevy_test::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static FLAKY_ATTEMPT: AtomicUsize = AtomicUsize::new(0);

#[itest(async)]
fn __harness_probe_flaky(_: &TestContext) -> godot::task::TaskHandle {
    let attempt = FLAKY_ATTEMPT.fetch_add(1, Ordering::Relaxed);
    godot::task::spawn(async move {
        if attempt == 1 {
            panic!("repeat probe sentinel");
        }
    })
}

#[itest(async)]
fn __harness_probe_timeout(_: &TestContext) -> godot::task::TaskHandle {
    godot::task::spawn(async move {
        std::future::pending::<()>().await;
    })
}

#[itest]
fn __harness_probe_sync_panic(_: &TestContext) {
    panic!("sync panic sentinel");
}

#[itest(async)]
fn __harness_probe_async_startup_panic(_: &TestContext) -> godot::task::TaskHandle {
    panic!("async startup panic sentinel");
}

#[itest(async)]
fn __harness_probe_async_task_panic(_: &TestContext) -> godot::task::TaskHandle {
    godot::task::spawn(async move {
        panic!("async task panic sentinel");
    })
}

fn bevy_frame_panic_system() {
    panic!("Bevy frame panic sentinel");
}

#[itest(async)]
fn __harness_probe_bevy_frame_panic(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx, |app| {
            app.add_systems(Update, bevy_frame_panic_system);
        })
        .await;
        app.cleanup().await;
    })
}

#[cfg(feature = "harness-focus-probe")]
#[itest(async, focus)]
fn __harness_probe_focus(_: &TestContext) -> godot::task::TaskHandle {
    godot::task::spawn(async {})
}

#[cfg(not(feature = "harness-focus-probe"))]
#[itest(async)]
fn __harness_probe_focus(_: &TestContext) -> godot::task::TaskHandle {
    godot::task::spawn(async {})
}
