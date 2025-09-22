//! Signal integration tests
//!
//! Simple test to verify if signals work in the test environment

use bevy::prelude::*;
use godot::prelude::*;
use godot_bevy::prelude::*;
use godot_bevy_testability::*;

// Define a typed event for our test signal
#[derive(Event, Debug, Clone)]
struct TestSignalReceived;

#[derive(Resource, Default)]
struct SignalState {
    received: bool,
}

fn test_signals_work(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    let mut env = ctx.setup_full_integration();

    // Add the typed signals plugin for our event
    ctx.app
        .add_plugins(GodotTypedSignalsPlugin::<TestSignalReceived>::default());
    ctx.app.init_resource::<SignalState>();

    // Create a script that emits a signal
    let mut script = godot::classes::GDScript::new_gd();
    script.set_source_code(
        r#"
extends Node
signal test_signal()
func emit_test():
    print("GDScript: Emitting test_signal")
    emit_signal("test_signal")
"#,
    );
    script.reload();

    // Create node with script
    let mut node = godot::classes::Node::new_alloc();
    node.set_name("SignalTestNode");
    node.set_script(&script.to_variant());
    env.add_node_to_scene(node.clone());

    // Process to create entities
    ctx.app.update();

    // Find the entity for our node
    let node_id = node.instance_id();
    let mut entity_found = None;
    {
        let world = ctx.app.world_mut();
        let mut query = world.query::<(Entity, &GodotNodeHandle)>();
        for (entity, handle) in query.iter(world) {
            if handle.instance_id() == node_id {
                entity_found = Some(entity);
                println!("Found entity {:?} for node {}", entity, node_id);
                break;
            }
        }
    }

    if entity_found.is_none() {
        println!("ERROR: No entity created for node");
        node.queue_free();
        return Err(TestError::assertion("Entity was not created for node"));
    }

    // Connect the signal using a one-shot system
    let entity = entity_found.unwrap();
    let connected = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let connected_clone = connected.clone();

    ctx.app.add_systems(
        Update,
        move |mut query: Query<&mut GodotNodeHandle>,
              typed: TypedGodotSignals<TestSignalReceived>| {
            if !connected_clone.load(std::sync::atomic::Ordering::Relaxed) {
                if let Ok(mut handle) = query.get_mut(entity) {
                    typed.connect_map(
                        &mut handle,
                        "test_signal",
                        Some(entity),
                        |_args, _node, _ent| TestSignalReceived,
                    );
                    println!("Connected signal for entity {:?}", entity);
                    connected_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        },
    );

    // Run update to connect the signal
    ctx.app.update();

    // Listen for the typed event
    ctx.app.add_systems(
        Update,
        |mut events: EventReader<TestSignalReceived>, mut state: ResMut<SignalState>| {
            for _ in events.read() {
                println!("Bevy: Received TestSignalReceived event");
                state.received = true;
            }
        },
    );

    // Emit the signal
    println!("Test: Calling emit_test()");
    node.call("emit_test", &[]);

    // Check if received
    for i in 0..10 {
        ctx.app.update();
        if ctx.app.world().resource::<SignalState>().received {
            println!("SUCCESS: Typed signals work in test environment");
            node.queue_free();
            return Ok(());
        }
        println!("Test: Update {}, signal not yet received", i);
    }

    println!("FAIL: Typed signals do NOT work in test environment");
    node.queue_free();
    Err(TestError::assertion(
        "Typed signals are not working in test environment",
    ))
}

bevy_godot_test_main! {
    test_signals_work,
}
