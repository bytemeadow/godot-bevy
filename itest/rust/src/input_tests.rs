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
    ActionInput, BevyInputBridgePlugin, GodotInputEventPlugin, KeyboardInput, MouseMotion,
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
    mut keys: MessageReader<KeyboardInput>,
    mut actions: MessageReader<ActionInput>,
    mut motions: MessageReader<MouseMotion>,
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
