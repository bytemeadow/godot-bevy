//! Deterministic validation that messages written in the Godot-driven FixedMain
//! pass are delivered exactly once to both FixedUpdate and Update readers, across
//! 0/1/2-substep render frames. Reproduces the real schedule wiring without Godot.

use bevy_app::{App, FixedFirst, FixedUpdate, Update};
use bevy_ecs::{
    message::{Message, MessageReader, MessageWriter},
    prelude::*,
};
use bevy_time::TimePlugin;
use std::time::Duration;

use crate::plugins::fixed_schedule::{
    host_fixed_main_loop, run_godot_fixed_main, run_main_prefix, run_main_suffix, run_startup,
};

#[derive(Message, Clone)]
struct Ping(u32);

#[derive(Resource, Default)]
struct NextId(u32);

#[derive(Resource, Default)]
struct Recv {
    in_fixed: Vec<u32>,
    in_update: Vec<u32>,
}

#[derive(Resource, Default)]
struct Wrote(bool);

const DT: Duration = Duration::from_nanos(16_666_667); // ~1/60 s

/// Build an app wired like the production split driver: stock message handshake, the
/// fixed loop hosted and driven from Godot's physics step, one Ping written per
/// fixed tick, and readers in FixedUpdate and Update recording every id they see.
fn wired_app() -> App {
    let mut app = App::new();
    app.add_plugins(TimePlugin);
    host_fixed_main_loop(&mut app);
    app.add_message::<Ping>();
    app.init_resource::<NextId>();
    app.init_resource::<Recv>();
    app.add_systems(
        FixedFirst,
        |mut w: MessageWriter<Ping>, mut n: ResMut<NextId>| {
            w.write(Ping(n.0));
            n.0 += 1;
        },
    );
    app.add_systems(
        FixedUpdate,
        |mut r: MessageReader<Ping>, mut recv: ResMut<Recv>| {
            for p in r.read() {
                recv.in_fixed.push(p.0);
            }
        },
    );
    app.add_systems(
        Update,
        |mut r: MessageReader<Ping>, mut recv: ResMut<Recv>| {
            for p in r.read() {
                recv.in_update.push(p.0);
            }
        },
    );
    app
}

/// Simulate one production render frame, mirroring the split driver in `app.rs`:
/// - startup schedules run once (gated by a marker, like app.rs's `started` flag)
/// - idle frame (n_steps == 0): prefix + suffix + clear
/// - physics frame (n_steps >= 1): prefix + N fixed steps + suffix + clear
fn frame(app: &mut App, n_steps: u32) {
    #[derive(Resource)]
    struct StartupRan;

    let world = app.world_mut();
    // run_startup itself is not idempotent; gate it here the way app.rs does.
    if !world.contains_resource::<StartupRan>() {
        run_startup(world);
        world.insert_resource(StartupRan);
    }
    run_main_prefix(world);
    for _ in 0..n_steps {
        run_godot_fixed_main(world, DT);
    }
    run_main_suffix(world);
    world.clear_trackers();
}

/// 1 substep: the message is delivered once to the FixedUpdate reader (same pass)
/// and once to the Update reader (suffix), with no loss or duplication.
#[test]
fn one_substep_delivers_once_to_both_readers() {
    let mut app = wired_app();
    frame(&mut app, 1);
    let recv = app.world().resource::<Recv>();
    assert_eq!(
        recv.in_fixed,
        vec![0],
        "FixedUpdate should see exactly Ping(0)"
    );
    assert_eq!(recv.in_update, vec![0], "Update should see exactly Ping(0)");
}

/// 2 substeps: each substep's FixedUpdate reads its own message; the Update reader
/// sees the union once each. Catches mid-loop over-aging dropping substep-1.
#[test]
fn two_substeps_deliver_union_once() {
    let mut app = wired_app();
    frame(&mut app, 2);
    let recv = app.world().resource::<Recv>();
    assert_eq!(
        recv.in_fixed,
        vec![0, 1],
        "each substep's FixedUpdate reads its own message"
    );
    assert_eq!(
        recv.in_update,
        vec![0, 1],
        "Update sees both substeps' messages, once each"
    );
}

/// Idle (0-substep) frames after a write must not drop or re-deliver the message:
/// the Update reader sees it exactly once and never again. (This sequence passes
/// under both Waiting and Always; the discriminating case is the next test.)
#[test]
fn idle_frames_preserve_exactly_once() {
    let mut app = wired_app();
    frame(&mut app, 1); // physics frame: FixedFirst writes Ping(0), both readers consume it
    frame(&mut app, 0); // idle
    frame(&mut app, 0); // idle
    let recv = app.world().resource::<Recv>();
    assert_eq!(
        recv.in_update,
        vec![0],
        "Update sees Ping(0) exactly once across idle frames"
    );
    assert_eq!(
        recv.in_fixed,
        vec![0],
        "FixedUpdate saw Ping(0) once, no idle re-read"
    );
}

/// A message written in `Update` (suffix) must survive idle (0-substep) frames
/// and still be read by a LATER physics tick's `FixedUpdate`. This validates the
/// `Waiting` handshake under the production split path: `First` runs in the prefix
/// of every frame, but under `Waiting` it does NOT age messages when no fixed step
/// ran since the last aging -- so idle prefix runs leave the buffer untouched.
/// Under `ShouldUpdateMessages::Always`, the idle prefix-First calls would age the
/// buffer twice and drop the message before the physics tick reads it. This test
/// fails under `Always` and passes under `Waiting`.
#[test]
fn update_message_survives_idle_until_next_physics_tick() {
    let mut app = App::new();
    app.add_plugins(TimePlugin);
    host_fixed_main_loop(&mut app);
    app.add_message::<Ping>();
    app.init_resource::<Recv>();
    app.init_resource::<Wrote>();
    // Write exactly one Ping, in Update (suffix), on the first frame only.
    app.add_systems(
        Update,
        |mut w: MessageWriter<Ping>, mut wrote: ResMut<Wrote>| {
            if !wrote.0 {
                w.write(Ping(0));
                wrote.0 = true;
            }
        },
    );
    app.add_systems(
        FixedUpdate,
        |mut r: MessageReader<Ping>, mut recv: ResMut<Recv>| {
            for p in r.read() {
                recv.in_fixed.push(p.0);
            }
        },
    );

    frame(&mut app, 0); // frame 1: prefix(First no-ages) + suffix(Update writes Ping(0))
    frame(&mut app, 0); // frame 2: idle — Waiting → First does not age
    frame(&mut app, 0); // frame 3: idle
    frame(&mut app, 1); // frame 4: prefix(First no-ages) + fixed(FixedUpdate reads Ping(0))

    let recv = app.world().resource::<Recv>();
    assert_eq!(
        recv.in_fixed,
        vec![0],
        "FixedUpdate must read the Update-written message after idle frames (Waiting preserves it; Always drops it)"
    );
}
