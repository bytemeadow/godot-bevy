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

use crate::plugins::fixed_schedule::{neutralize_in_process_fixed_loop, run_godot_fixed_main};

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

/// Build an app wired exactly like the redesign: stock message handshake, the
/// in-process fixed loop neutralized, one Ping written per fixed tick, and
/// readers in FixedUpdate and Update recording every id they see.
fn wired_app() -> App {
    let mut app = App::new();
    app.add_plugins(TimePlugin);
    neutralize_in_process_fixed_loop(&mut app);
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

/// 1 substep: the message is delivered once to the FixedUpdate reader (same pass)
/// and once to the Update reader (process path), with no loss or duplication.
#[test]
fn one_substep_delivers_once_to_both_readers() {
    let mut app = wired_app();
    run_godot_fixed_main(app.world_mut(), DT);
    app.update();
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
    run_godot_fixed_main(app.world_mut(), DT);
    run_godot_fixed_main(app.world_mut(), DT);
    app.update();
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
    run_godot_fixed_main(app.world_mut(), DT); // writes Ping(0)
    app.update(); // process frame for the tick above
    app.update(); // 0-substep frame
    app.update(); // 0-substep frame
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

/// A message written in `Update` must survive idle (0-substep) frames and still be
/// read by a LATER physics tick's `FixedUpdate`. This is the case the rejected
/// `ShouldUpdateMessages::Always` override fails: under `Always`, the intervening
/// idle `app.update()` calls age the buffer twice and drop the message before the
/// physics tick reads it. The stock `Waiting` handshake (no fixed step ran → no
/// aging) preserves it. This test fails under `Always` and passes under `Waiting`.
#[test]
fn update_message_survives_idle_until_next_physics_tick() {
    let mut app = App::new();
    app.add_plugins(TimePlugin);
    neutralize_in_process_fixed_loop(&mut app);
    app.add_message::<Ping>();
    app.init_resource::<Recv>();
    app.init_resource::<Wrote>();
    // Write exactly one Ping, in Update, on the first frame only.
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

    app.update(); // frame 1 (process): Update writes Ping(0); no physics tick
    app.update(); // frame 2: idle (no fixed tick -> Waiting -> no aging)
    app.update(); // frame 3: idle
    run_godot_fixed_main(app.world_mut(), DT); // frame 4: physics tick -> FixedUpdate reads

    let recv = app.world().resource::<Recv>();
    assert_eq!(
        recv.in_fixed,
        vec![0],
        "FixedUpdate must read the Update-written message after idle frames (Waiting preserves it; Always drops it)"
    );
}
