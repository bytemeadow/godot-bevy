/*
 * Ecosystem input integration tests
 *
 * Guards that godot-bevy's input bridge + split-Main loop don't break the two
 * major downstream input crates. In headless the bridge is the sole populator of
 * ButtonInput<KeyCode>, so a regression there yields zero reads in both crates:
 *
 * - leafwing-input-manager (pull-style ActionState): probed in both Update and
 *   FixedUpdate, so a fixed-clock reader also sees the input. leafwing keeps a
 *   separate fixed-input buffer via the Before/AfterFixedMainLoop anchors that
 *   godot-bevy hosts (host_fixed_main_loop) for ecosystem crates.
 * - bevy_enhanced_input (push-style observer): an On<Fire<Jump>> global observer,
 *   the path that previously exposed a godot-bevy bug.
 *
 * Both crates bind raw KeyCode::Space, so no Godot InputMap action is needed.
 */

use bevy::prelude::*;
use godot::classes::{Input, InputEventKey};
use godot::global::Key;
use godot::obj::{NewGd, Singleton};
use godot_bevy::plugins::input::BevyInputBridgePlugin;
use godot_bevy_test::prelude::*;

/// Inject a raw Godot key event through the real Input singleton. Key::SPACE maps
/// cleanly to KeyCode::Space in the bridge. Each test injects a matching release at
/// the end so ButtonInput state doesn't leak into the next itest.
fn parse_key(key: Key, pressed: bool) {
    let mut event = InputEventKey::new_gd();
    event.set_keycode(key);
    event.set_pressed(pressed);
    Input::singleton().parse_input_event(&event);
}

/// Windowed edge analysis shared by both clocks. `press` are samples taken while
/// the key is held; `release` after the release edge. Pre-edge settle frames are
/// excluded by the positional `pressed_while_held` check.
/// Returns (just_pressed_count, pressed_held_from_edge, just_released_count, eventually_released).
fn analyze_log(
    press: &[(bool, bool, bool)],
    release: &[(bool, bool, bool)],
) -> (usize, bool, usize, bool) {
    let jp = press.iter().filter(|&&(jp, _, _)| jp).count();
    let pressed_while_held = press
        .iter()
        .position(|&(jp, _, _)| jp)
        .is_some_and(|i| press[i..].iter().all(|&(_, p, _)| p));
    let jr = release.iter().filter(|&&(_, _, jr)| jr).count();
    let eventually_released = release.iter().any(|&(_, p, _)| !p);
    (jp, pressed_while_held, jr, eventually_released)
}

/// leafwing-input-manager reads `ButtonInput<KeyCode>` (populated by the bridge)
/// via its `CentralInputStore`. Verify `ActionState` sees the press/release edges
/// in both the Update clock and the FixedUpdate clock. The FixedUpdate read also
/// covers leafwing's fixed-input buffer (swap_to_fixed_update / swap_to_update),
/// which run in the RunFixedMainLoop Before/After anchors godot-bevy keeps live.
#[itest(async)]
fn test_leafwing_action_state_both_clocks(ctx: &TestContext) -> godot::task::TaskHandle {
    use leafwing_input_manager::prelude::*;

    #[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
    enum Action {
        Jump,
    }

    #[derive(Resource, Default)]
    struct UpdateLog(Vec<(bool, bool, bool)>); // (just_pressed, pressed, just_released)
    #[derive(Resource, Default)]
    struct FixedLog(Vec<(bool, bool, bool)>);

    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(BevyInputBridgePlugin);
            app.add_plugins(InputManagerPlugin::<Action>::default());
            app.init_resource::<UpdateLog>();
            app.init_resource::<FixedLog>();
            // InputMap requires ActionState; both are added to this entity.
            app.world_mut()
                .spawn(InputMap::new([(Action::Jump, KeyCode::Space)]));
            app.add_systems(
                Update,
                |q: Query<&ActionState<Action>>, mut log: ResMut<UpdateLog>| {
                    if let Some(s) = q.iter().next() {
                        log.0.push((
                            s.just_pressed(&Action::Jump),
                            s.pressed(&Action::Jump),
                            s.just_released(&Action::Jump),
                        ));
                    }
                },
            );
            app.add_systems(
                FixedUpdate,
                |q: Query<&ActionState<Action>>, mut log: ResMut<FixedLog>| {
                    if let Some(s) = q.iter().next() {
                        log.0.push((
                            s.just_pressed(&Action::Jump),
                            s.pressed(&Action::Jump),
                            s.just_released(&Action::Jump),
                        ));
                    }
                },
            );
        })
        .await;

        // Drop settle-frame samples so the press window starts clean.
        app.with_world_mut(|w| {
            w.resource_mut::<UpdateLog>().0.clear();
            w.resource_mut::<FixedLog>().0.clear();
        });

        parse_key(Key::SPACE, true);
        app.updates(8).await;

        let upd_press = app.with_world(|w| w.resource::<UpdateLog>().0.clone());
        let fix_press = app.with_world(|w| w.resource::<FixedLog>().0.clone());
        app.with_world_mut(|w| {
            w.resource_mut::<UpdateLog>().0.clear();
            w.resource_mut::<FixedLog>().0.clear();
        });

        parse_key(Key::SPACE, false);
        app.updates(8).await;

        let upd_release = app.with_world(|w| w.resource::<UpdateLog>().0.clone());
        let fix_release = app.with_world(|w| w.resource::<FixedLog>().0.clone());

        app.cleanup().await;

        let (u_jp, u_held, u_jr, u_rel) = analyze_log(&upd_press, &upd_release);
        let (f_jp, f_held, f_jr, f_rel) = analyze_log(&fix_press, &fix_release);

        println!(
            "leafwing Update: press_frames={}, jp={u_jp}, jr={u_jr}",
            upd_press.len()
        );
        println!(
            "leafwing Fixed: press_ticks={}, jp={f_jp}, jr={f_jr}",
            fix_press.len()
        );

        // Update clock
        assert_eq!(
            u_jp, 1,
            "leafwing Update just_pressed must fire exactly once; frames: {upd_press:?}"
        );
        assert!(
            u_held,
            "leafwing Update pressed must stay true from the jp frame onward; frames: {upd_press:?}"
        );
        assert_eq!(
            u_jr, 1,
            "leafwing Update just_released must fire exactly once; frames: {upd_release:?}"
        );
        assert!(
            u_rel,
            "leafwing Update pressed must become false after release; frames: {upd_release:?}"
        );

        // FixedUpdate clock -- guards host_fixed_main_loop's Before/AfterFixedMainLoop anchors
        assert!(
            !fix_press.is_empty(),
            "leafwing FixedUpdate press log empty -- no physics ticks fired in 8 frames"
        );
        assert_eq!(
            f_jp, 1,
            "leafwing FixedUpdate just_pressed must fire exactly once; ticks: {fix_press:?}"
        );
        assert!(
            f_held,
            "leafwing FixedUpdate pressed must stay true from the jp tick onward; ticks: {fix_press:?}"
        );
        assert!(
            !fix_release.is_empty(),
            "leafwing FixedUpdate release log empty -- no physics ticks fired in 8 frames"
        );
        assert_eq!(
            f_jr, 1,
            "leafwing FixedUpdate just_released must fire exactly once; ticks: {fix_release:?}"
        );
        assert!(
            f_rel,
            "leafwing FixedUpdate pressed must become false after release; ticks: {fix_release:?}"
        );

        println!(
            "✓ leafwing ActionState: Update jp={u_jp}/jr={u_jr}, Fixed jp={f_jp}/jr={f_jr} -- both clocks bridged"
        );
    })
}

/// bevy_enhanced_input's observer path. A bare binding (no condition) behaves as a
/// `Down` condition, so `Fire<Jump>` triggers every frame the key is held. The
/// global `On<Fire<Jump>>` observer counts those. Counter must be 0 before any
/// input (no spurious fire) and >= 1 while held -- a broken bridge leaves
/// ButtonInput<KeyCode> empty and yields 0.
#[itest(async)]
fn test_enhanced_input_observer_fires(ctx: &TestContext) -> godot::task::TaskHandle {
    use bevy_enhanced_input::prelude::*;

    #[derive(Component)]
    struct Player;

    #[derive(InputAction)]
    #[action_output(bool)]
    struct Jump;

    #[derive(Resource, Default)]
    struct JumpFireCount(u32);

    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(BevyInputBridgePlugin);
            app.add_plugins(EnhancedInputPlugin);
            app.add_input_context::<Player>();
            app.init_resource::<JumpFireCount>();
            app.add_observer(|_: On<Fire<Jump>>, mut c: ResMut<JumpFireCount>| {
                c.0 += 1;
            });
        })
        .await;

        // Spawn the context entity after do_initialize -> app.finish(), which
        // installs enhanced_input's per-context systems and the
        // ContextInstances<PreUpdate> resource that the registration observer
        // (On<Insert, ContextPriority<Player>>) reads. Spawning inside the setup
        // closure would fire that observer before finish() creates the resource.
        app.with_world_mut(|w| {
            w.spawn((
                Player,
                actions!(Player[(Action::<Jump>::new(), bindings![KeyCode::Space])]),
            ));
        });
        // Let the context register and evaluate once with no input held.
        app.updates(2).await;

        let before = app.with_world(|w| w.resource::<JumpFireCount>().0);
        assert_eq!(
            before, 0,
            "Fire<Jump> must not fire before any input is injected; got {before}"
        );

        parse_key(Key::SPACE, true);
        app.updates(8).await;
        let held = app.with_world(|w| w.resource::<JumpFireCount>().0);

        // Release so ButtonInput<KeyCode> doesn't leak into the next itest.
        parse_key(Key::SPACE, false);
        app.updates(2).await;

        app.cleanup().await;

        assert!(
            held >= 1,
            "Fire<Jump> must fire while Space is held (bare binding == Down condition); got {held}. \
             A broken bridge leaves ButtonInput<KeyCode> empty -> 0."
        );

        println!("✓ bevy_enhanced_input observer: Fire<Jump> fired {held}x while Space held");
    })
}
