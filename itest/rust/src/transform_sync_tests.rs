/*
 * Transform synchronization tests
 *
 * Tests all transform sync modes using Bevy-style TestApp API:
 * - OneWay (Bevy → Godot only)
 * - TwoWay (bidirectional)
 * - Disabled (no sync)
 *
 * Uses explicit frame-by-frame control with app.update().await
 */

use bevy::prelude::*;
use godot::obj::NewAlloc;
use godot::prelude::*;
use godot_bevy::prelude::*;
use godot_bevy_test::prelude::*;

/// Test that position, rotation, and scale sync from Bevy to Godot (OneWay mode)
#[itest(async)]
fn test_bevy_to_godot_transform_sync(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("BevyMoverNode");
        node.set_position(Vector2::new(0.0, 0.0));
        ctx_clone.scene_tree.clone().add_child(&node);

        let node_id = node.instance_id();
        let target_angle = std::f32::consts::FRAC_PI_4;

        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.add_plugins(GodotTransformSyncPlugin::default());
            app.insert_resource(GodotTransformConfig::one_way());

            app.add_systems(
                Update,
                move |mut q: Query<(&GodotNodeHandle, &mut Transform)>| {
                    for (handle, mut transform) in q.iter_mut() {
                        if handle.instance_id() == node_id {
                            transform.translation.x = 10.0;
                            transform.translation.y = 5.0;
                            transform.rotation = Quat::from_rotation_z(target_angle);
                            transform.scale = Vec3::new(2.0, 0.5, 1.0);
                        }
                    }
                },
            );
        })
        .await;

        // Wait for Bevy transform to sync to Godot node.
        // The write runs in FixedLast (physics rate), so we need a physics tick.
        app.physics_update().await;

        let pos = node.get_position();
        let rot = node.get_rotation();
        let scale = node.get_scale();

        assert!(
            (pos.x - 10.0).abs() < 0.1 && (pos.y - 5.0).abs() < 0.1,
            "Position should sync, expected (10, 5), got ({:.1}, {:.1})",
            pos.x,
            pos.y
        );
        assert!(
            (rot - target_angle).abs() < 0.01,
            "Rotation should sync, expected {target_angle:.3}, got {rot:.3}"
        );
        assert!(
            (scale.x - 2.0).abs() < 0.01 && (scale.y - 0.5).abs() < 0.01,
            "Scale should sync, expected (2.0, 0.5), got ({:.3}, {:.3})",
            scale.x,
            scale.y
        );

        println!(
            "✓ Bevy→Godot transform sync: pos=({:.1},{:.1}), rot={rot:.3}, scale=({:.2},{:.2})",
            pos.x, pos.y, scale.x, scale.y
        );

        app.cleanup().await;
        node.free();
    })
}

/// Test that transforms sync from Godot to Bevy (TwoWay mode)
#[itest(async)]
fn test_godot_to_bevy_transform_sync(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("GodotMoverNode");
        node.set_position(Vector2::new(0.0, 0.0));
        ctx_clone.scene_tree.clone().add_child(&node);

        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.add_plugins(GodotTransformSyncPlugin::default());
            app.insert_resource(GodotTransformConfig::two_way());
        })
        .await;

        let entity = app.single_entity_with::<Transform>();
        let initial_x =
            app.with_world(|world| world.get::<Transform>(entity).unwrap().translation.x);

        // Move the Godot node (should sync to Bevy in TwoWay mode)
        node.set_position(Vector2::new(10.0, 0.0));

        // Wait for Godot position change to sync into Bevy
        app.update().await;

        let synced_x =
            app.with_world(|world| world.get::<Transform>(entity).unwrap().translation.x);

        assert!(
            (synced_x - 10.0).abs() < 0.1,
            "Bevy should detect Godot transform changes, expected ~10.0, got {synced_x:.1}"
        );

        println!("✓ Godot→Bevy transform sync: {initial_x:.1} → {synced_x:.1}");

        app.cleanup().await;
        node.free();
    })
}

/// Test bidirectional transform sync (TwoWay mode)
#[itest(async)]
fn test_bidirectional_transform_sync(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut bevy_node = godot::classes::Node2D::new_alloc();
        bevy_node.set_name("BevyControlled");
        bevy_node.set_position(Vector2::new(0.0, 0.0));
        ctx_clone.scene_tree.clone().add_child(&bevy_node);

        let mut godot_node = godot::classes::Node2D::new_alloc();
        godot_node.set_name("GodotControlled");
        godot_node.set_position(Vector2::new(0.0, 0.0));
        ctx_clone.scene_tree.clone().add_child(&godot_node);

        let bevy_id = bevy_node.instance_id();
        let godot_id = godot_node.instance_id();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.add_plugins(GodotTransformSyncPlugin::default());
            app.insert_resource(GodotTransformConfig::two_way());

            app.add_systems(
                Update,
                move |mut q: Query<(&GodotNodeHandle, &mut Transform)>| {
                    for (handle, mut transform) in q.iter_mut() {
                        if handle.instance_id() == bevy_id {
                            transform.translation.x += 1.0;
                        }
                    }
                },
            );
        })
        .await;

        let bevy_start = bevy_node.get_position().x;

        // Move Godot node (tests Godot→Bevy sync)
        godot_node.set_position(Vector2::new(20.0, 0.0));

        // Run several physics ticks so the Bevy-controlled node accumulates
        // enough movement for a meaningful assertion. The write runs in
        // FixedLast, so physics_update() is required to flush it.
        for _ in 0..4 {
            app.physics_update().await;
        }

        let bevy_end = bevy_node.get_position().x;

        // Check Bevy→Godot sync
        assert!(
            bevy_end > bevy_start,
            "Bevy-controlled node should move (Bevy→Godot), start={bevy_start:.1}, end={bevy_end:.1}"
        );

        // Check Godot→Bevy sync
        let godot_entity_x = app.with_world_mut(|world| {
            let mut query = world.query::<(&GodotNodeHandle, &Transform)>();
            for (handle, transform) in query.iter(world) {
                if handle.instance_id() == godot_id {
                    return transform.translation.x;
                }
            }
            0.0
        });

        assert!(
            (godot_entity_x - 20.0).abs() < 0.1,
            "Godot-controlled entity should sync to Bevy (Godot→Bevy), expected ~20.0, got {godot_entity_x:.1}"
        );

        println!(
            "✓ Bidirectional sync: Bevy {bevy_start:.1}→{bevy_end:.1}, Godot→Bevy {godot_entity_x:.1}"
        );

        app.cleanup().await;
        bevy_node.free();
        godot_node.free();
    })
}

/// Regression test for the TwoWay self-change guard across the `_process` /
/// `_physics_process` boundary.
///
/// The Godot→Bevy read-back runs every physics step in `FixedFirst` (before that
/// step's write; on a 0-step frame, in `PreUpdate`), while the Bevy→Godot write
/// runs in `FixedLast`. The guard is a value shadow
/// (`TransformSyncMetadata.shadow`): the write pushes only when the Bevy value
/// differs from the last-synced shadow, so a value that was read FROM Godot (and
/// stored as the shadow) is recognised as not-Bevy-authored and the echo write is
/// suppressed. Being value-based, the guard is order-independent -- no
/// read-after-write requirement.
///
/// Without the guard the write would echo the Godot-origin position back,
/// potentially resetting physics interpolation or overwriting a Godot-driven
/// position with a stale Bevy value.
///
/// We set the node purely from Godot (no Bevy system touches this Transform), run
/// physics ticks, and assert the Godot value is picked up by Bevy AND the node is
/// not disturbed by an echo. A two-substep variant runs `physics_update()` twice to
/// exercise multiple `FixedLast` passes against a single Godot move.
#[itest(async)]
fn test_twoway_no_echo_back_across_fixed_boundary(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("TwoWayEchoNode");
        node.set_position(Vector2::new(0.0, 0.0));
        ctx_clone.scene_tree.clone().add_child(&node);

        let node_id = node.instance_id();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.add_plugins(GodotTransformSyncPlugin::default());
            app.insert_resource(GodotTransformConfig::two_way());
        })
        .await;

        // Move the node from Godot. No Bevy system touches this Transform, so the
        // only path that could change it is the read-back (Godot->Bevy). The write
        // (Bevy->Godot) must suppress the echo and leave the node where Godot put it.
        node.set_position(Vector2::new(42.0, 17.0));

        // One physics tick: read-back (FixedFirst) already recorded the Godot value,
        // so the write (FixedLast) should suppress the echo.
        app.physics_update().await;

        let bevy_x = app.with_world_mut(|world| {
            let mut q = world.query::<(&GodotNodeHandle, &Transform)>();
            q.iter(world)
                .find(|(h, _)| h.instance_id() == node_id)
                .map(|(_, t)| t.translation.x)
                .unwrap_or(f32::NAN)
        });
        assert!(
            (bevy_x - 42.0).abs() < 0.1,
            "Bevy should pick up the Godot value, expected ~42.0, got {bevy_x:.3}"
        );

        let pos_after_one = node.get_position();
        assert!(
            (pos_after_one.x - 42.0).abs() < 0.1 && (pos_after_one.y - 17.0).abs() < 0.1,
            "Godot node must not be echoed/reset after 1 tick, expected (42, 17), got ({:.3}, {:.3})",
            pos_after_one.x,
            pos_after_one.y
        );

        // Second pass: another physics tick against the same (now settled) Godot
        // value. The guard must still suppress -- no echo, no stale overwrite.
        app.physics_update().await;

        let pos_after_two = node.get_position();
        assert!(
            (pos_after_two.x - 42.0).abs() < 0.1 && (pos_after_two.y - 17.0).abs() < 0.1,
            "Godot node must not be echoed/reset after 2 ticks, expected (42, 17), got ({:.3}, {:.3})",
            pos_after_two.x,
            pos_after_two.y
        );

        println!(
            "✓ TwoWay no echo-back: Bevy x={bevy_x:.1}, Godot stable at ({:.1}, {:.1})",
            pos_after_two.x, pos_after_two.y
        );

        app.cleanup().await;
        node.free();
    })
}

/// The canonical TwoWay coexistence pattern (see examples/two-way-sync-demo):
/// Godot drives the node's x every frame, Bevy drives the node's y every frame.
/// The self-change guard must let BOTH survive -- the read-back must not clobber
/// the Bevy y-edit, and the write must not echo the Godot x back as a stale value.
/// This is the case most sensitive to where the read-back runs relative to the
/// FixedLast write across the process/physics boundary.
#[itest(async)]
fn test_twoway_godot_and_bevy_coexist(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("CoexistNode");
        node.set_position(Vector2::new(0.0, 0.0));
        ctx_clone.scene_tree.clone().add_child(&node);

        let node_id = node.instance_id();

        // Bevy drives y each Update frame (like the example's update_quad_y_position).
        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.add_plugins(GodotTransformSyncPlugin::default());
            app.insert_resource(GodotTransformConfig::two_way());

            app.add_systems(
                Update,
                move |mut q: Query<(&GodotNodeHandle, &mut Transform)>| {
                    for (handle, mut transform) in q.iter_mut() {
                        if handle.instance_id() == node_id {
                            transform.translation.y += 1.0;
                        }
                    }
                },
            );
        })
        .await;

        // Godot drives x each frame (stand-in for the GDScript-side x update).
        let mut last_godot_x = 0.0_f32;
        for i in 0..6 {
            last_godot_x = (i as f32 + 1.0) * 5.0;
            let y = node.get_position().y;
            node.set_position(Vector2::new(last_godot_x, y));
            app.physics_update().await;
        }

        let pos = node.get_position();
        let bevy_y = app.with_world_mut(|world| {
            let mut q = world.query::<(&GodotNodeHandle, &Transform)>();
            q.iter(world)
                .find(|(h, _)| h.instance_id() == node_id)
                .map(|(_, t)| t.translation.y)
                .unwrap_or(f32::NAN)
        });

        // Godot's x edits must survive (not echoed over by a stale Bevy x=0).
        assert!(
            (pos.x - last_godot_x).abs() < 1.0,
            "Godot-driven x must survive the guard, expected ~{last_godot_x:.1}, got {:.1}",
            pos.x
        );
        // Bevy's y edits must survive (not clobbered by the read-back reading y=... from Godot).
        // The loop runs 6 physics_update() calls, each triggering at least one Update frame
        // (+1.0 y), so y must be >= 5.0 -- well clear of a partial-clobber regression.
        assert!(
            pos.y >= 5.0,
            "Bevy-driven y must accumulate and reach Godot, expected >=5, got {:.1}",
            pos.y
        );
        // Bevy must track the Godot-driven x.
        assert!(
            bevy_y >= 5.0,
            "Bevy entity y should reflect its own accumulating edits, expected >=5, got {bevy_y:.1}"
        );

        println!(
            "✓ TwoWay coexist: Godot x={:.1}, y={:.1}; Bevy y={bevy_y:.1}",
            pos.x, pos.y
        );

        app.cleanup().await;
        node.free();
    })
}

/// Test that spawning a synced node at a non-origin position does not produce
/// a one-tick interpolation slide from the origin when physics interpolation is
/// enabled. reset_physics_interpolation() must be called on the first Bevy→Godot
/// write after the node is registered so the engine treats the set position as
/// the canonical starting point.
#[itest(async)]
fn test_spawn_resets_physics_interpolation(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        // Physics interpolation is a global SceneTree flag -- the one cross-test
        // state the harness's Drop-based isolation doesn't reset. Restore it via a
        // guard so a failing assert below can't leak it into the next test.
        struct FtiGuard(Gd<godot::classes::SceneTree>);
        impl Drop for FtiGuard {
            fn drop(&mut self) {
                self.0.set_physics_interpolation_enabled(false);
            }
        }
        let mut tree = ctx_clone.scene_tree.get_tree();
        tree.set_physics_interpolation_enabled(true);
        let _fti = FtiGuard(tree);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotTransformSyncPlugin::default());
        })
        .await;

        let (node, entity) = app.add_node::<godot::classes::Node2D>("Spawned").await;
        app.with_world_mut(|w| {
            w.get_mut::<Transform>(entity).unwrap().translation =
                bevy::math::Vec3::new(500.0, 300.0, 0.0);
        });
        app.physics_update().await;

        // With reset on the first post-spawn write, the rendered/global transform
        // is the set value immediately -- no interpolation slide from origin.
        let pos = node.get_position();
        assert!(
            (pos.x - 500.0).abs() < 0.5 && (pos.y - 300.0).abs() < 0.5,
            "expected position ~(500, 300) after first write with FTI enabled, got {pos:?}"
        );

        app.cleanup().await;
        node.free();
    })
}

/// Test that sync can be disabled
#[itest(async)]
fn test_transform_sync_disabled(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("NoSyncNode");
        node.set_position(Vector2::new(0.0, 0.0));
        ctx_clone.scene_tree.clone().add_child(&node);

        let node_id = node.instance_id();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.add_plugins(GodotTransformSyncPlugin::default());
            app.insert_resource(GodotTransformConfig::disabled());

            app.add_systems(
                Update,
                move |mut q: Query<(&GodotNodeHandle, &mut Transform)>| {
                    for (handle, mut transform) in q.iter_mut() {
                        if handle.instance_id() == node_id {
                            transform.translation.x += 10.0;
                        }
                    }
                },
            );
        })
        .await;

        let start_pos = node.get_position().x;

        // Run a few frames so the Bevy entity accumulates multiple += 10.0
        // increments -- Godot node should stay at 0 since sync is disabled.
        app.updates(4).await;

        let end_pos = node.get_position().x;

        // Verify Bevy entity moved internally
        let entity = app.single_entity_with::<Transform>();
        let bevy_x = app.with_world(|world| world.get::<Transform>(entity).unwrap().translation.x);

        assert!(
            bevy_x > 0.0,
            "Bevy entity should move internally, got {bevy_x:.1}"
        );
        assert_eq!(
            end_pos, start_pos,
            "Godot node should NOT move when sync disabled, start={start_pos:.1}, end={end_pos:.1}"
        );

        println!(
            "✓ Transform sync disabled: Godot at {start_pos:.1}, Bevy at {bevy_x:.1} (no sync)"
        );

        app.cleanup().await;
        node.free();
    })
}

/// Brownfield guarantee: a Bevy system sees a Godot-authored transform in the SAME
/// frame it runs. The TwoWay read lands before the `Update` suffix -- the last
/// `FixedFirst` read on a tick frame, or the `PreUpdate` fallback on a 0-step frame --
/// so an `Update` system observes Godot's latest value this frame, not next frame. If
/// the read ran after the write, the system would observe the stale value and fail.
#[itest(async)]
fn test_twoway_read_before_logic(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct Observed(f32);

        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("ReadBeforeLogicNode");
        node.set_position(Vector2::new(0.0, 0.0));
        ctx_clone.scene_tree.clone().add_child(&node);
        let node_id = node.instance_id();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.add_plugins(GodotTransformSyncPlugin::default());
            app.insert_resource(GodotTransformConfig::two_way());
            app.init_resource::<Observed>();
            // Record what the Bevy value is when Update logic runs.
            app.add_systems(
                Update,
                move |q: Query<(&GodotNodeHandle, &Transform)>, mut obs: ResMut<Observed>| {
                    for (h, t) in q.iter() {
                        if h.instance_id() == node_id {
                            obs.0 = t.translation.x;
                        }
                    }
                },
            );
        })
        .await;

        // Author x purely from Godot, then advance one frame. The read (FixedFirst on
        // a tick frame, PreUpdate on a 0-step frame) must land it in Bevy before the
        // Update system runs that same frame.
        node.set_position(Vector2::new(99.0, 0.0));
        app.update().await;

        let observed = app.with_world(|w| w.resource::<Observed>().0);
        assert!(
            (observed - 99.0).abs() < 0.1,
            "Update logic must see the Godot-authored x the same frame (read-before-logic); got {observed:.1}"
        );

        println!("✓ TwoWay read-before-logic: Update saw Godot x={observed:.1} same frame");

        app.cleanup().await;
        node.free();
    })
}

/// Freeing a synced node from inside a Bevy system must not tear down the app. On
/// the free step the `FixedLast` write selects the just-mutated victim (`Changed`)
/// and hits its now-dead handle -- that must skip, not panic. Stays TwoWay and frees
/// from inside a system so the free lands after that step's `First` drain, reproducing
/// the dead-handle window; a second untouched node proves one dead node doesn't wedge
/// the write batch.
///
/// This only discriminates fixed-vs-unfixed in *release*, where the individual write's
/// `godot.get` on the dead handle panics pre-fix and tears the app down (the
/// `has_entity_for_node` call below then fails). Debug -- what CI runs -- never tears
/// down here (the bulk path's dead-id deref is a non-fatal GDScript error), so in
/// debug this is a liveness smoke test, not a regression guard. Run in release to
/// exercise the fix.
#[itest(async)]
fn test_freeing_synced_node_is_non_fatal(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct FreeVictim(bool);

        let mut victim = godot::classes::Node2D::new_alloc();
        victim.set_name("FreedVictim");
        victim.set_position(Vector2::new(0.0, 0.0));
        ctx_clone.scene_tree.clone().add_child(&victim);
        let victim_id = victim.instance_id();

        let mut survivor = godot::classes::Node2D::new_alloc();
        survivor.set_name("Survivor");
        survivor.set_position(Vector2::new(0.0, 0.0));
        ctx_clone.scene_tree.clone().add_child(&survivor);
        let survivor_id = survivor.instance_id();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.add_plugins(GodotTransformSyncPlugin::default());
            app.insert_resource(GodotTransformConfig::two_way());
            app.init_resource::<FreeVictim>();

            app.add_systems(
                FixedUpdate,
                move |mut flag: ResMut<FreeVictim>,
                      mut q: Query<(&GodotNodeHandle, &mut Transform)>,
                      mut godot: GodotAccess| {
                    let free_now = flag.0;
                    for (handle, mut transform) in q.iter_mut() {
                        let id = handle.instance_id();
                        if id == survivor_id {
                            // keep the survivor writing every step
                            transform.translation.x += 1.0;
                        } else if id == victim_id && free_now {
                            // trip Changed so this step's FixedLast write selects the
                            // victim, then free the node so that write hits a dead handle
                            transform.translation.x += 1.0;
                            if let Some(node) = godot.try_get::<godot::classes::Node2D>(*handle) {
                                node.free();
                            }
                        }
                    }
                    if free_now {
                        flag.0 = false;
                    }
                },
            );
        })
        .await;

        // Settle both nodes into steady sync, then record the survivor's baseline.
        app.physics_update().await;
        app.physics_update().await;
        let survivor_x_before = survivor.get_position().x;

        // Free the victim on the next physics step (in-system, mid-FixedMain).
        app.with_world_mut(|w| w.resource_mut::<FreeVictim>().0 = true);
        app.physics_update().await;

        // Release-only tripwire (see header): has_entity_for_node -> with_world ->
        // get_app().expect(...) panics on a torn-down app. Poll for the eventual despawn
        // -- never assert an exact frame (process/BevyApp slop).
        let mut despawned = false;
        for _ in 0..30 {
            if !app.has_entity_for_node(victim_id) {
                despawned = true;
                break;
            }
            app.physics_update().await;
        }
        assert!(
            despawned,
            "victim entity should eventually despawn after its node was freed"
        );

        // The survivor's Bevy->Godot writes must still land -- proves one freed node
        // doesn't abort the write batch. Assert on the write direction (Godot node
        // position), which stays granular even in debug's bulk path; the Godot->Bevy
        // read can stall one step in debug and is deliberately not asserted here.
        let survivor_x_after = survivor.get_position().x;
        assert!(
            survivor_x_after > survivor_x_before + 0.5,
            "survivor should keep syncing after the victim was freed, before={survivor_x_before:.1}, after={survivor_x_after:.1}"
        );

        println!(
            "✓ Freed synced node non-fatal: app survived, victim despawned, survivor {survivor_x_before:.1}->{survivor_x_after:.1}"
        );

        app.cleanup().await;
        // victim was already freed inside the system; only free the survivor.
        survivor.free();
    })
}
