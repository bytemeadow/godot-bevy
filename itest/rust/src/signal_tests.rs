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
        await_frames(1).await;

        // Track if signal was received
        #[derive(Resource, Default)]
        struct SignalReceived(bool);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotSignalsPlugin::<TestSignalFired>::default());
            app.init_resource::<SignalReceived>();
            // Use observer instead of system with MessageReader
            app.add_observer(
                |trigger: On<TestSignalFired>, mut received: ResMut<SignalReceived>| {
                    println!("Signal received from: {}", trigger.event().source_name);
                    received.0 = true;
                },
            );
        })
        .await;

        // Frame 1: Initial sync
        app.update().await;

        // Create a button node that has a "pressed" signal
        let mut button = godot::classes::Button::new_alloc();
        button.set_name("TestButton");
        ctx_clone.scene_tree.clone().add_child(&button);

        let button_id = button.instance_id();

        // Frame 2: Entity created for button
        app.update().await;

        // Frame 3: Extra frame to ensure entity is fully ready
        app.update().await;

        // Find the button's entity and connect the signal
        let entity_found = app.with_world_mut(|world| {
            let handle = world
                .query::<&GodotNodeHandle>()
                .iter(world)
                .find(|h| h.instance_id() == button_id)
                .copied();

            if let Some(handle) = handle {
                // Get GodotSignals and connect
                let mut system_state: bevy::ecs::system::SystemState<
                    GodotSignals<TestSignalFired>,
                > = bevy::ecs::system::SystemState::new(world);
                let signals = system_state.get(world);

                signals.connect(handle, "pressed", None, |_args, _handle, _entity| {
                    Some(TestSignalFired {
                        source_name: "TestButton".to_string(),
                    })
                });

                system_state.apply(world);
                true
            } else {
                false
            }
        });

        assert!(entity_found, "Button entity should exist");

        // Frame 4: Connection is applied at end of this frame (in Last schedule)
        app.update().await;

        // Emit the signal (simulating a button press)
        button.emit_signal("pressed", &[]);

        // Frame 5: Signal should be received
        app.update().await;

        let was_received = app.with_world(|world| world.resource::<SignalReceived>().0);

        assert!(
            was_received,
            "Signal should be received after connect (connection applied same-frame)"
        );

        println!("✓ Signal connection works same-frame: connect → emit → receive");

        // Cleanup
        app.cleanup();
        button.queue_free();
        await_frames(1).await;
    })
}

/// Test that multiple signal connections work correctly
#[itest(async)]
fn test_multiple_signal_connections(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        #[derive(Resource, Default)]
        struct SignalCounts {
            button1: i32,
            button2: i32,
        }

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotSignalsPlugin::<TestSignalFired>::default());
            app.init_resource::<SignalCounts>();
            // Use observer instead of system with MessageReader
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

        app.update().await;

        // Create two buttons
        let mut button1 = godot::classes::Button::new_alloc();
        button1.set_name("Button1");
        ctx_clone.scene_tree.clone().add_child(&button1);

        let mut button2 = godot::classes::Button::new_alloc();
        button2.set_name("Button2");
        ctx_clone.scene_tree.clone().add_child(&button2);

        let button1_id = button1.instance_id();
        let button2_id = button2.instance_id();

        // Wait for entities to be created
        app.update().await;
        app.update().await;

        // Connect signals for both buttons
        app.with_world_mut(|world| {
            // Find button handles
            let button1_handle = world
                .query::<&GodotNodeHandle>()
                .iter(world)
                .find(|h| h.instance_id() == button1_id)
                .copied();

            let button2_handle = world
                .query::<&GodotNodeHandle>()
                .iter(world)
                .find(|h| h.instance_id() == button2_id)
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

        // Apply connections
        app.update().await;

        // Emit signals
        button1.emit_signal("pressed", &[]);
        button2.emit_signal("pressed", &[]);
        button1.emit_signal("pressed", &[]); // Button1 pressed twice

        app.update().await;

        let counts = app.with_world(|world| {
            let c = world.resource::<SignalCounts>();
            (c.button1, c.button2)
        });

        assert_eq!(counts.0, 2, "Button1 should have received 2 signals");
        assert_eq!(counts.1, 1, "Button2 should have received 1 signal");

        println!(
            "✓ Multiple signal connections work correctly: Button1={}, Button2={}",
            counts.0, counts.1
        );

        // Cleanup
        app.cleanup();
        button1.queue_free();
        button2.queue_free();
        await_frames(1).await;
    })
}

/// Test that signal connections made via system work correctly
#[itest(async)]
fn test_signal_connection_via_system(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        #[derive(Resource, Default)]
        struct SignalReceived(bool);

        #[derive(Resource, Default)]
        struct ConnectionMade(bool);

        // Create button before TestApp so we can capture its ID
        let mut button = godot::classes::Button::new_alloc();
        button.set_name("SystemTestButton");
        ctx_clone.scene_tree.clone().add_child(&button);
        let button_id = button.instance_id();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.add_plugins(GodotSignalsPlugin::<TestSignalFired>::default());
            app.init_resource::<SignalReceived>();
            app.init_resource::<ConnectionMade>();

            // System that connects signal when it finds the button entity
            app.add_systems(
                Update,
                move |mut connection_made: ResMut<ConnectionMade>,
                      query: Query<&GodotNodeHandle>,
                      signals: GodotSignals<TestSignalFired>| {
                    if !connection_made.0 {
                        // Look for our test button by instance ID
                        if let Some(handle) =
                            query.iter().find(|h| h.instance_id() == button_id).copied()
                        {
                            signals.connect(handle, "pressed", None, |_args, _handle, _entity| {
                                Some(TestSignalFired {
                                    source_name: "SystemTestButton".to_string(),
                                })
                            });
                            connection_made.0 = true;
                        }
                    }
                },
            );

            // Use observer instead of system with MessageReader
            app.add_observer(
                |_trigger: On<TestSignalFired>, mut received: ResMut<SignalReceived>| {
                    received.0 = true;
                },
            );
        })
        .await;

        // Frame 1: Entity created, signal connection made via system
        app.update().await;

        // Frame 2: Connection applied (in Last schedule)
        app.update().await;

        // Verify connection was made
        let connection_made = app.with_world(|world| world.resource::<ConnectionMade>().0);
        assert!(connection_made, "Signal connection should have been made");

        // Emit signal
        button.emit_signal("pressed", &[]);

        // Frame 3: Signal received
        app.update().await;

        let was_received = app.with_world(|world| world.resource::<SignalReceived>().0);

        assert!(
            was_received,
            "Signal should work when connection is made via system"
        );

        println!("✓ Signal connection via system works correctly");

        // Cleanup
        app.cleanup();
        button.queue_free();
        await_frames(1).await;
    })
}
