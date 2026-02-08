/*
 * Signal integration tests
 *
 * Tests typed signal connections and observer triggering:
 * - Signal connections made with connect are applied same-frame
 * - Signals fire and trigger observers on the next frame
 * - Multiple signal connections work correctly
 */

use bevy::prelude::*;
use godot::obj::NewAlloc;
use godot::prelude::*;
use godot_bevy::prelude::*;
use godot_bevy_test::prelude::*;

/// Test event type for signal testing
#[derive(Event, Debug, Clone)]
struct TestSignalFired {
    source_name: String,
}

/// Test that connect connections are applied same-frame and signals work next frame
#[itest(async)]
fn test_signal_connection_same_frame(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct SignalReceived(bool);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotSignalsPlugin::<TestSignalFired>::default());
            app.init_resource::<SignalReceived>();
            app.add_observer(
                |trigger: On<TestSignalFired>, mut received: ResMut<SignalReceived>| {
                    println!("Signal received from: {}", trigger.event().source_name);
                    received.0 = true;
                },
            );
        })
        .await;

        let (mut button, _entity) = app.add_node::<godot::classes::Button>("TestButton").await;

        let button_id = button.instance_id();

        app.with_world_mut(|world| {
            let handle = world
                .get::<GodotNodeHandle>(
                    world
                        .resource::<NodeEntityIndex>()
                        .get(button_id)
                        .expect("Button entity should exist in index"),
                )
                .copied()
                .expect("Entity should have GodotNodeHandle");

            let mut system_state: bevy::ecs::system::SystemState<GodotSignals<TestSignalFired>> =
                bevy::ecs::system::SystemState::new(world);
            let signals = system_state.get(world);

            signals.connect(handle, "pressed", None, |_args, _handle, _entity| {
                Some(TestSignalFired {
                    source_name: "TestButton".to_string(),
                })
            });

            system_state.apply(world);
        });

        // One frame for pending connections to be processed (Last schedule)
        app.update().await;

        button.emit_signal("pressed", &[]);

        // One frame for drain_and_trigger_signals (First schedule)
        app.update().await;

        let was_received = app.with_world(|world| world.resource::<SignalReceived>().0);

        assert!(
            was_received,
            "Signal should be received after connect (connection applied same-frame)"
        );

        app.cleanup().await;
        button.queue_free();
    })
}

/// Event type for connect_object testing
#[derive(Event, Debug, Clone)]
struct NodeAdded;

/// Test that connect_object connects signals from non-entity Godot objects.
#[itest(async)]
fn test_connect_object_signal(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct SignalReceived(bool);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotSignalsPlugin::<NodeAdded>::default());
            app.init_resource::<SignalReceived>();
            app.add_observer(
                |_trigger: On<NodeAdded>, mut received: ResMut<SignalReceived>| {
                    received.0 = true;
                },
            );
        })
        .await;

        let scene_tree: Gd<godot::classes::SceneTree> = ctx_clone.scene_tree.get_tree().unwrap();

        app.with_world_mut(|world| {
            let mut system_state: bevy::ecs::system::SystemState<GodotSignals<NodeAdded>> =
                bevy::ecs::system::SystemState::new(world);
            let signals = system_state.get(world);

            signals.connect_object(scene_tree, "node_added", |_args| Some(NodeAdded));

            system_state.apply(world);
        });

        // One frame for pending connections to be processed
        app.update().await;

        let mut trigger_node = godot::classes::Node::new_alloc();
        trigger_node.set_name("ConnectObjectTrigger");
        ctx_clone.scene_tree.clone().add_child(&trigger_node);

        // One frame for signal to be drained and triggered
        app.update().await;

        let was_received = app.with_world(|world| world.resource::<SignalReceived>().0);

        assert!(
            was_received,
            "connect_object should receive signals from non-entity Godot objects"
        );

        app.cleanup().await;
        trigger_node.queue_free();
    })
}

/// Test that multiple signal connections work correctly
#[itest(async)]
fn test_multiple_signal_connections(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct SignalCounts {
            button1: i32,
            button2: i32,
        }

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotSignalsPlugin::<TestSignalFired>::default());
            app.init_resource::<SignalCounts>();
            app.add_observer(
                |trigger: On<TestSignalFired>, mut counts: ResMut<SignalCounts>| match trigger
                    .event()
                    .source_name
                    .as_str()
                {
                    "Button1" => counts.button1 += 1,
                    "Button2" => counts.button2 += 1,
                    _ => {}
                },
            );
        })
        .await;

        let (mut button1, _) = app.add_node::<godot::classes::Button>("Button1").await;
        let (mut button2, _) = app.add_node::<godot::classes::Button>("Button2").await;

        let button1_id = button1.instance_id();
        let button2_id = button2.instance_id();

        app.with_world_mut(|world| {
            let index = world.resource::<NodeEntityIndex>();
            let button1_handle = world
                .get::<GodotNodeHandle>(index.get(button1_id).expect("Button1 entity should exist"))
                .copied();
            let button2_handle = world
                .get::<GodotNodeHandle>(index.get(button2_id).expect("Button2 entity should exist"))
                .copied();

            let mut system_state: bevy::ecs::system::SystemState<GodotSignals<TestSignalFired>> =
                bevy::ecs::system::SystemState::new(world);
            let signals = system_state.get(world);

            if let Some(handle) = button1_handle {
                signals.connect(handle, "pressed", None, |_args, _handle, _entity| {
                    Some(TestSignalFired {
                        source_name: "Button1".to_string(),
                    })
                });
            }

            if let Some(handle) = button2_handle {
                signals.connect(handle, "pressed", None, |_args, _handle, _entity| {
                    Some(TestSignalFired {
                        source_name: "Button2".to_string(),
                    })
                });
            }

            system_state.apply(world);
        });

        // One frame for pending connections to be processed
        app.update().await;

        button1.emit_signal("pressed", &[]);
        button2.emit_signal("pressed", &[]);
        button1.emit_signal("pressed", &[]);

        // One frame for signals to be drained and triggered
        app.update().await;

        let counts = app.with_world(|world| {
            let c = world.resource::<SignalCounts>();
            (c.button1, c.button2)
        });

        assert_eq!(counts.0, 2, "Button1 should have received 2 signals");
        assert_eq!(counts.1, 1, "Button2 should have received 1 signal");

        app.cleanup().await;
        button1.queue_free();
        button2.queue_free();
    })
}

/// Test that signal connections made via system work correctly
#[itest(async)]
fn test_signal_connection_via_system(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct SignalReceived(bool);

        #[derive(Resource, Default)]
        struct ConnectionMade(bool);

        let mut button = godot::classes::Button::new_alloc();
        button.set_name("SystemTestButton");
        ctx_clone.scene_tree.clone().add_child(&button);
        let button_id = button.instance_id();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.add_plugins(GodotSignalsPlugin::<TestSignalFired>::default());
            app.init_resource::<SignalReceived>();
            app.init_resource::<ConnectionMade>();

            app.add_systems(
                Update,
                move |mut connection_made: ResMut<ConnectionMade>,
                      query: Query<&GodotNodeHandle>,
                      signals: GodotSignals<TestSignalFired>| {
                    if !connection_made.0
                        && let Some(handle) =
                            query.iter().find(|h| h.instance_id() == button_id).copied()
                    {
                        signals.connect(handle, "pressed", None, |_args, _handle, _entity| {
                            Some(TestSignalFired {
                                source_name: "SystemTestButton".to_string(),
                            })
                        });
                        connection_made.0 = true;
                    }
                },
            );

            app.add_observer(
                |_trigger: On<TestSignalFired>, mut received: ResMut<SignalReceived>| {
                    received.0 = true;
                },
            );
        })
        .await;

        // Two frames: one for the system to find the handle and queue the
        // connection, one for process_pending_signal_connections (Last) to apply it
        app.updates(2).await;

        let connection_made = app.with_world(|world| world.resource::<ConnectionMade>().0);
        assert!(connection_made, "Signal connection should have been made");

        button.emit_signal("pressed", &[]);

        // One frame for drain_and_trigger_signals
        app.update().await;

        let was_received = app.with_world(|world| world.resource::<SignalReceived>().0);

        assert!(
            was_received,
            "Signal should work when connection is made via system"
        );

        app.cleanup().await;
        button.queue_free();
    })
}
