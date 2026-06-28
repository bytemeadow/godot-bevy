/*
 * Input system integration tests
 *
 * Tests the full input pipeline through real Godot frames:
 * - Input.parse_input_event() feeds Godot's input system
 * - GodotInputWatcher receives _input/_unhandled_input callbacks → channel
 * - write_input_messages (First) drains the channel into typed Bevy messages
 * - BevyInputBridgePlugin further bridges into Bevy's ButtonInput resources
 */

use bevy::input::{ButtonInput, keyboard::KeyCode};
use bevy::prelude::*;
use godot::classes::{Input, InputEventKey, InputEventMouseMotion, InputMap};
use godot::global::Key;
use godot::obj::{NewGd, Singleton};
use godot::prelude::*;
use godot_bevy::plugins::input::{
    Action, ActionInput, BevyInputBridgePlugin, GodotActions, GodotActionsPlugin,
    GodotInputEventPlugin, GodotInputSet, GodotKeyboardInput, GodotMouseMotion,
};
use godot_bevy_test::prelude::*;

/// Collects input messages each frame so tests can assert on them without
/// racing the two-frame message buffer.
#[derive(Resource, Default)]
struct CollectedInput {
    keys: Vec<(Key, bool)>,
    actions: Vec<(String, bool, f32)>,
    motions: Vec<Vec2>,
}

fn collect_input_messages(
    mut store: ResMut<CollectedInput>,
    mut keys: MessageReader<GodotKeyboardInput>,
    mut actions: MessageReader<ActionInput>,
    mut motions: MessageReader<GodotMouseMotion>,
) {
    for msg in keys.read() {
        store.keys.push((msg.keycode, msg.pressed));
    }
    for msg in actions.read() {
        store
            .actions
            .push((msg.action.clone(), msg.pressed, msg.strength));
    }
    for msg in motions.read() {
        store.motions.push(msg.delta);
    }
}

fn setup_input_collector(app: &mut App) {
    app.add_plugins(GodotInputEventPlugin)
        .init_resource::<CollectedInput>()
        .add_systems(Update, collect_input_messages);
}

fn parse_key_event(key: Key, pressed: bool) {
    let mut event = InputEventKey::new_gd();
    event.set_keycode(key);
    event.set_pressed(pressed);
    Input::singleton().parse_input_event(&event);
}

/// Test that a Godot keyboard event arrives as a KeyboardInput message.
#[itest(async)]
fn test_keyboard_input_reaches_bevy(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, setup_input_collector).await;

        parse_key_event(Key::W, true);
        app.updates(2).await;
        parse_key_event(Key::W, false);
        app.updates(2).await;

        let keys = app.with_world(|world| world.resource::<CollectedInput>().keys.clone());
        assert_eq!(
            keys,
            vec![(Key::W, true), (Key::W, false)],
            "Expected one press and one release for W, got {keys:?}"
        );

        println!("✓ Keyboard events reach Bevy as KeyboardInput messages");

        app.cleanup().await;
    })
}

/// Test that a key bound to an InputMap action produces an ActionInput message.
#[itest(async)]
fn test_action_input_via_input_map(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let action = StringName::from("godot_bevy_test_action");
        let mut input_map = InputMap::singleton();
        input_map.add_action(&action);
        let mut trigger = InputEventKey::new_gd();
        trigger.set_keycode(Key::T);
        input_map.action_add_event(&action, &trigger);

        let mut app = TestApp::new(&ctx_clone, setup_input_collector).await;

        parse_key_event(Key::T, true);
        app.updates(2).await;

        let actions = app.with_world(|world| world.resource::<CollectedInput>().actions.clone());
        input_map.erase_action(&action);

        assert_eq!(
            actions,
            vec![("godot_bevy_test_action".to_string(), true, 1.0)],
            "Expected one pressed ActionInput, got {actions:?}"
        );

        println!("✓ InputMap actions reach Bevy as ActionInput messages");

        app.cleanup().await;
    })
}

/// Test that mouse motion arrives as a MouseMotion message with the right delta.
#[itest(async)]
fn test_mouse_motion_reaches_bevy(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, setup_input_collector).await;

        let mut event = InputEventMouseMotion::new_gd();
        event.set_relative(Vector2::new(3.0, -2.0));
        event.set_position(Vector2::new(100.0, 50.0));
        Input::singleton().parse_input_event(&event);
        app.updates(2).await;

        let motions = app.with_world(|world| world.resource::<CollectedInput>().motions.clone());
        assert_eq!(
            motions,
            vec![Vec2::new(3.0, -2.0)],
            "Expected one MouseMotion with delta (3, -2), got {motions:?}"
        );

        println!("✓ Mouse motion reaches Bevy as MouseMotion messages");

        app.cleanup().await;
    })
}

/// Test that BevyInputBridgePlugin maintains Bevy's ButtonInput<KeyCode> state.
#[itest(async)]
fn test_input_bridge_button_input(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(BevyInputBridgePlugin);
        })
        .await;

        parse_key_event(Key::W, true);
        app.updates(2).await;
        let pressed = app.with_world(|world| {
            world
                .resource::<ButtonInput<KeyCode>>()
                .pressed(KeyCode::KeyW)
        });
        assert!(pressed, "KeyW should be pressed after press event");

        parse_key_event(Key::W, false);
        app.updates(2).await;
        let pressed = app.with_world(|world| {
            world
                .resource::<ButtonInput<KeyCode>>()
                .pressed(KeyCode::KeyW)
        });
        assert!(!pressed, "KeyW should be released after release event");

        println!("✓ BevyInputBridgePlugin maintains ButtonInput<KeyCode> state");

        app.cleanup().await;
    })
}

/// The bridge emits Bevy `KeyboardInput` events so Bevy's `keyboard_input_system`
/// owns edge lifecycle. Verifies just_pressed and just_released each fire exactly once.
#[itest(async)]
fn test_keyboard_just_pressed_fires_once(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct KeyEdgeLog(Vec<(bool, bool, bool)>); // (just_pressed, pressed, just_released)

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(BevyInputBridgePlugin);
            app.init_resource::<KeyEdgeLog>();
            app.add_systems(
                Update,
                |input: Res<ButtonInput<KeyCode>>, mut log: ResMut<KeyEdgeLog>| {
                    log.0.push((
                        input.just_pressed(KeyCode::KeyW),
                        input.pressed(KeyCode::KeyW),
                        input.just_released(KeyCode::KeyW),
                    ));
                },
            );
        })
        .await;

        parse_key_event(Key::W, true);
        app.updates(5).await;

        let press_log = app.with_world(|w| w.resource::<KeyEdgeLog>().0.clone());
        app.with_world_mut(|w| w.resource_mut::<KeyEdgeLog>().0.clear());

        parse_key_event(Key::W, false);
        app.updates(5).await;

        let release_log = app.with_world(|w| w.resource::<KeyEdgeLog>().0.clone());
        app.cleanup().await;

        let jp_count = press_log.iter().filter(|&&(jp, _, _)| jp).count();
        // Once the press is first seen (jp frame), pressed must stay true in all
        // subsequent frames. Pre-jp frames (settling + 1-frame propagation delay)
        // naturally have pressed=false and are excluded by the positional check.
        let pressed_while_held = press_log
            .iter()
            .position(|&(jp, _, _)| jp)
            .is_some_and(|i| press_log[i..].iter().all(|&(_, p, _)| p));
        let no_release_during_press = press_log.iter().all(|&(_, _, jr)| !jr);
        let jr_count = release_log.iter().filter(|&&(_, _, jr)| jr).count();
        let eventually_released = release_log.iter().any(|&(_, p, _)| !p);

        println!(
            "keyboard edges: press_frames={}, jp={jp_count}, jr={jr_count}",
            press_log.len()
        );

        assert_eq!(
            jp_count, 1,
            "just_pressed must fire exactly once; frames: {press_log:?}"
        );
        assert!(
            pressed_while_held,
            "pressed must stay true from the jp frame onward; frames: {press_log:?}"
        );
        assert!(
            no_release_during_press,
            "just_released must not fire during press phase; frames: {press_log:?}"
        );
        assert_eq!(
            jr_count, 1,
            "just_released must fire exactly once; frames: {release_log:?}"
        );
        assert!(
            eventually_released,
            "pressed must become false after release; frames: {release_log:?}"
        );

        println!("✓ Keyboard: jp fires {jp_count}x, jr fires {jr_count}x");
    })
}

/// `ButtonInput<KeyCode>::just_pressed` reaches `FixedUpdate` because `First →
/// PreUpdate` run in the physics-process prefix (before `FixedMain`), so
/// `keyboard_input_system` populates `ButtonInput` before `FixedUpdate` runs.
///
/// Under `--fixed-fps 60` a single press edge reaches `FixedUpdate` deterministically.
/// `GodotActions` is the right per-clock API for game code that needs per-clock
/// edge semantics.
#[itest(async)]
fn test_button_input_just_pressed_in_fixed_update(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct FixedJpProbe {
            just_pressed_count: u32,
            pressed_count: u32,
        }

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(BevyInputBridgePlugin);
            app.init_resource::<FixedJpProbe>();
            app.add_systems(
                FixedUpdate,
                |input: Res<ButtonInput<KeyCode>>, mut probe: ResMut<FixedJpProbe>| {
                    if input.just_pressed(KeyCode::Space) {
                        probe.just_pressed_count += 1;
                    }
                    if input.pressed(KeyCode::Space) {
                        probe.pressed_count += 1;
                    }
                },
            );
        })
        .await;

        parse_key_event(Key::SPACE, true);
        app.updates(2).await;
        parse_key_event(Key::SPACE, false);
        app.updates(2).await;

        let (jp, pressed) = app.with_world(|w| {
            let p = w.resource::<FixedJpProbe>();
            (p.just_pressed_count, p.pressed_count)
        });

        assert!(
            jp >= 1,
            "ButtonInput::just_pressed must reach FixedUpdate (PreUpdate runs before FixedMain); got {jp}"
        );
        assert!(
            pressed >= 1,
            "ButtonInput::pressed must be seen in FixedUpdate; got {pressed}"
        );

        println!(
            "✓ ButtonInput in FixedUpdate: just_pressed reached it {jp}x, pressed seen {pressed}x"
        );

        app.cleanup().await;
    })
}

/// GodotActions just_pressed and just_released each fire exactly once in the
/// process clock (Update) and exactly once in the physics clock (FixedUpdate).
///
/// The physics edge may lag by one tick (Godot's +1 physics stamp) -- we count,
/// never pin the exact tick index.
#[itest(async)]
fn test_godot_actions_edges_in_both_clocks(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        const ACTION: &str = "godot_bevy_test_bc_action";

        let action_sn = StringName::from(ACTION);
        let mut input_map = InputMap::singleton();
        input_map.add_action(&action_sn);
        let mut trigger = InputEventKey::new_gd();
        trigger.set_keycode(Key::Q);
        input_map.action_add_event(&action_sn, &trigger);

        #[derive(Resource, Default)]
        struct ProcessLog(Vec<(bool, bool, bool)>); // (just_pressed, pressed, just_released)
        #[derive(Resource, Default)]
        struct PhysicsLog(Vec<(bool, bool, bool)>);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotActionsPlugin);
            app.init_resource::<ProcessLog>();
            app.init_resource::<PhysicsLog>();
            app.add_systems(
                Update,
                (|actions: Res<GodotActions>, mut log: ResMut<ProcessLog>| {
                    log.0.push((
                        actions.just_pressed(ACTION),
                        actions.pressed(ACTION),
                        actions.just_released(ACTION),
                    ));
                })
                .after(GodotInputSet),
            );
            app.add_systems(
                FixedUpdate,
                |actions: Res<GodotActions>, mut log: ResMut<PhysicsLog>| {
                    log.0.push((
                        actions.just_pressed(ACTION),
                        actions.pressed(ACTION),
                        actions.just_released(ACTION),
                    ));
                },
            );
        })
        .await;

        parse_key_event(Key::Q, true);
        app.updates(8).await;

        let proc_press = app.with_world(|w| w.resource::<ProcessLog>().0.clone());
        let phys_press = app.with_world(|w| w.resource::<PhysicsLog>().0.clone());
        app.with_world_mut(|w| {
            w.resource_mut::<ProcessLog>().0.clear();
            w.resource_mut::<PhysicsLog>().0.clear();
        });

        parse_key_event(Key::Q, false);
        app.updates(8).await;

        let proc_release = app.with_world(|w| w.resource::<ProcessLog>().0.clone());
        let phys_release = app.with_world(|w| w.resource::<PhysicsLog>().0.clone());

        input_map.erase_action(&action_sn);
        app.cleanup().await;

        // "pressed while held": from the jp frame onward every entry has pressed=true;
        // pre-jp settling frames are excluded by the positional check.
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

        let (proc_jp, proc_pressed_while_held, proc_jr, proc_eventually_released) =
            analyze_log(&proc_press, &proc_release);
        let (phys_jp, phys_pressed_while_held, phys_jr, phys_eventually_released) =
            analyze_log(&phys_press, &phys_release);

        println!(
            "process clock: press_frames={}, jp={proc_jp}, jr={proc_jr}",
            proc_press.len()
        );
        println!(
            "physics clock: press_ticks={}, jp={phys_jp}, jr={phys_jr}",
            phys_press.len()
        );

        // Process clock
        assert_eq!(
            proc_jp, 1,
            "process just_pressed must fire exactly once; frames: {proc_press:?}"
        );
        assert!(
            proc_pressed_while_held,
            "process pressed must stay true from jp frame onward; frames: {proc_press:?}"
        );
        assert_eq!(
            proc_jr, 1,
            "process just_released must fire exactly once; frames: {proc_release:?}"
        );
        assert!(
            proc_eventually_released,
            "process pressed must become false after release; frames: {proc_release:?}"
        );

        // Physics clock -- +1 lag is acceptable, count must still be 1
        assert!(
            !phys_press.is_empty(),
            "physics log empty -- no physics ticks fired in 8 frames (increase updates count?)"
        );
        assert_eq!(
            phys_jp, 1,
            "physics just_pressed must fire exactly once (may lag one tick); ticks: {phys_press:?}"
        );
        assert!(
            phys_pressed_while_held,
            "physics pressed must stay true from jp tick onward; ticks: {phys_press:?}"
        );
        assert!(
            !phys_release.is_empty(),
            "physics release log empty -- no physics ticks fired in 8 frames"
        );
        assert_eq!(
            phys_jr, 1,
            "physics just_released must fire exactly once; ticks: {phys_release:?}"
        );
        assert!(
            phys_eventually_released,
            "physics pressed must become false after release; ticks: {phys_release:?}"
        );

        println!(
            "✓ GodotActions: process jp={proc_jp}/jr={proc_jr}, physics jp={phys_jp}/jr={phys_jr} -- per-clock edges independent"
        );
    })
}

/// `Action::new("name")` (typed, StringName-keyed) and `"name"` (&str) must return
/// identical results from GodotActions every frame.
#[itest(async)]
fn test_godot_actions_typed_handle_matches_str(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        const ACTION: &str = "godot_bevy_test_c_action";

        let action_sn = StringName::from(ACTION);
        let mut input_map = InputMap::singleton();
        input_map.add_action(&action_sn);
        let mut trigger = InputEventKey::new_gd();
        trigger.set_keycode(Key::E);
        input_map.action_add_event(&action_sn, &trigger);

        #[derive(Resource, Default)]
        struct TypedStrLog(Vec<(bool, bool)>); // (typed_just_pressed, str_just_pressed)

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotActionsPlugin);
            app.init_resource::<TypedStrLog>();
            app.add_systems(
                Update,
                (|actions: Res<GodotActions>, mut log: ResMut<TypedStrLog>| {
                    let typed = Action::new(ACTION);
                    log.0
                        .push((actions.just_pressed(&typed), actions.just_pressed(ACTION)));
                })
                .after(GodotInputSet),
            );
        })
        .await;

        parse_key_event(Key::E, true);
        app.updates(5).await;

        let log = app.with_world(|w| w.resource::<TypedStrLog>().0.clone());

        input_map.erase_action(&action_sn);
        app.cleanup().await;

        assert!(
            log.iter().all(|&(typed, s)| typed == s),
            "typed Action and &str must return the same result each frame; log: {log:?}"
        );
        assert!(
            log.iter().any(|&(typed, s)| typed && s),
            "neither path saw just_pressed -- check InputMap setup or synthetic input; log: {log:?}"
        );

        println!("✓ Typed Action and &str agree across {} frames", log.len());
    })
}
