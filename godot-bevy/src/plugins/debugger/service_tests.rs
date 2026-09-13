use super::*;
use std::{sync::mpsc::channel, time::Duration};

#[test]
fn intervals_coalesce_changes_and_unsubscribe_clears_state() {
    let (tx, rx) = channel();
    let mut world = World::new();
    world.insert_resource(DebuggerConfig::default());
    world.insert_resource(DebuggerTransport::new(move |frame| {
        tx.send(frame).unwrap();
    }));
    world.init_resource::<SummaryChanges>();
    let mut tracking = Schedule::default();
    tracking.add_systems(track_summary_changes);
    let entity = world.spawn(Name::new("original")).id();
    tracking.run(&mut world);
    world.clear_trackers();
    let mut subscription = None;
    dispatch(
        &mut world,
        &mut subscription,
        &Wire::object([
            ("jsonrpc", Wire::String("2.0".into())),
            ("id", Wire::Integer(1)),
            ("method", Wire::String("godot.subscribe".into())),
            ("params", Wire::object([("interval_s", Wire::Float(60.0))])),
        ]),
    )
    .unwrap();
    publish(&mut world, &mut subscription, &mut true);
    assert_eq!(rx.try_iter().count(), 1);
    tracking.run(&mut world);
    subscription.as_mut().unwrap().sent_at = Instant::now() - Duration::from_secs(61);
    publish(&mut world, &mut subscription, &mut true);
    assert!(rx.try_recv().is_err());
    assert!(world.resource::<SummaryChanges>().dirty.is_empty());
    world.clear_trackers();
    world.entity_mut(entity).insert(Name::new("first rename"));
    tracking.run(&mut world);
    publish(&mut world, &mut subscription, &mut true);
    assert!(rx.try_recv().is_err());
    assert!(world.resource::<SummaryChanges>().dirty.contains(&entity));
    world.clear_trackers();
    world.entity_mut(entity).insert(Name::new("last rename"));
    tracking.run(&mut world);
    subscription.as_mut().unwrap().sent_at = Instant::now() - Duration::from_secs(61);
    publish(&mut world, &mut subscription, &mut true);
    let frames = rx.try_iter().collect::<Vec<_>>();
    assert_eq!(frames.len(), 1);
    assert!(world.resource::<SummaryChanges>().dirty.is_empty());
    assert_eq!(
        frames[0]
            .get("params")
            .unwrap()
            .get("updated")
            .unwrap()
            .as_array()
            .unwrap()[0]
            .get("name"),
        Some(&Wire::String("last rename".into()))
    );
    publish(&mut world, &mut subscription, &mut true);
    assert!(rx.try_recv().is_err());
    dispatch(
        &mut world,
        &mut subscription,
        &Wire::object([
            ("jsonrpc", Wire::String("2.0".into())),
            ("id", Wire::Integer(2)),
            ("method", Wire::String("godot.unsubscribe".into())),
        ]),
    )
    .unwrap();
    assert!(subscription.is_none());
    assert!(!world.resource::<SummaryChanges>().active);
    assert!(world.resource::<SummaryChanges>().dirty.is_empty());
}

#[derive(Component)]
struct Ordinary;

#[derive(Component, bevy_reflect::Reflect)]
struct Registered;

#[test]
fn membership_changes_update_summaries_without_name_or_parent_changes() {
    let (tx, rx) = channel();
    let mut world = World::new();
    world.insert_resource(DebuggerConfig::default());
    world.insert_resource(DebuggerTransport::new(move |frame| {
        tx.send(frame).unwrap();
    }));
    world.init_resource::<SummaryChanges>();
    world.init_resource::<AppTypeRegistry>();
    world
        .resource::<AppTypeRegistry>()
        .write()
        .register::<Registered>();
    let mut tracking = Schedule::default();
    tracking.add_systems(track_summary_changes);
    let entity = world.spawn_empty().id();
    world.entity_mut(entity).insert((Registered, Ordinary));
    world.entity_mut(entity).remove::<(Registered, Ordinary)>();
    // Running the schedule creates the world's Schedules resource entity; do it before the
    // baseline so it is part of the snapshot instead of arriving as an addition later.
    tracking.run(&mut world);
    world.clear_trackers();
    let mut subscription = None;
    subscribe(&mut world, &mut subscription, 60.0);
    publish(&mut world, &mut subscription, &mut true);
    let initial = rx.try_recv().unwrap();
    let original = initial
        .get("params")
        .unwrap()
        .get("added")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row.get("entity") == Some(&reference(entity)))
        .unwrap()
        .clone();
    let generation = world.archetypes().generation();
    assert!(rx.try_recv().is_err());
    for present in [true, false, true, false] {
        world.clear_trackers();
        if present {
            world.entity_mut(entity).insert((Registered, Ordinary));
        } else {
            world.entity_mut(entity).remove::<(Registered, Ordinary)>();
        }
        assert_eq!(
            world.archetypes().generation(),
            generation,
            "fixture moves between existing archetypes"
        );
        tracking.run(&mut world);
        publish(&mut world, &mut subscription, &mut true);
        assert!(
            rx.try_recv().is_err(),
            "membership updates respect the interval"
        );
        subscription.as_mut().unwrap().sent_at = Instant::now() - Duration::from_secs(61);
        publish(&mut world, &mut subscription, &mut true);
        let frames = rx.try_iter().collect::<Vec<_>>();
        assert_eq!(frames.len(), 1, "one membership notification per interval");
        let params = frames[0].get("params").unwrap();
        assert_eq!(params.get("snapshot"), Some(&Wire::Bool(false)));
        assert_eq!(params.get("added"), Some(&Wire::Array(vec![])));
        assert_eq!(params.get("removed"), Some(&Wire::Array(vec![])));
        let rows = params.get("updated").unwrap().as_array().unwrap();
        assert_eq!(rows.len(), 1, "only the changed entity is updated");
        assert_eq!(rows[0].get("entity"), Some(&reference(entity)));
        assert_eq!(rows[0].get("name"), original.get("name"));
        assert_eq!(rows[0].get("parent"), original.get("parent"));
        let mut components = if present {
            vec![
                std::any::type_name::<Ordinary>(),
                std::any::type_name::<Registered>(),
            ]
        } else {
            vec![]
        };
        components.sort();
        assert_eq!(
            rows[0].get("components"),
            Some(&Wire::Array(
                components
                    .into_iter()
                    .map(|name| Wire::String(name.into()))
                    .collect()
            ))
        );
        let unsupported = if present {
            Wire::object([(
                std::any::type_name::<Ordinary>(),
                Wire::String("component not registered".into()),
            )])
        } else {
            Wire::object([])
        };
        assert_eq!(rows[0].get("unsupported_components"), Some(&unsupported));
        subscription.as_mut().unwrap().sent_at = Instant::now() - Duration::from_secs(61);
        tracking.run(&mut world);
        publish(&mut world, &mut subscription, &mut true);
        assert!(rx.try_recv().is_err(), "unchanged membership sends nothing");
    }
    world.entity_mut(entity).insert(Ordinary);
    world.entity_mut(entity).remove::<Ordinary>();
    subscription.as_mut().unwrap().sent_at = Instant::now() - Duration::from_secs(61);
    tracking.run(&mut world);
    publish(&mut world, &mut subscription, &mut true);
    assert!(
        rx.try_recv().is_err(),
        "membership changes coalesce to the last sent archetype"
    );
}

#[derive(Component, bevy_reflect::Reflect)]
#[type_path = "game"]
struct LateRegistered;

#[test]
fn dirty_summary_sends_registration_changes_without_an_archetype_change() {
    let (tx, rx) = channel();
    let mut world = World::new();
    world.insert_resource(DebuggerConfig::default());
    world.insert_resource(DebuggerTransport::new(move |frame| {
        tx.send(frame).unwrap();
    }));
    world.init_resource::<SummaryChanges>();
    world.init_resource::<AppTypeRegistry>();
    let mut tracking = Schedule::default();
    tracking.add_systems(track_summary_changes);
    let entity = world.spawn((LateRegistered, Name::new("same name"))).id();
    tracking.run(&mut world);
    world.clear_trackers();
    let mut subscription = None;
    subscribe(&mut world, &mut subscription, 0.0);
    publish(&mut world, &mut subscription, &mut true);
    assert_eq!(rx.try_iter().count(), 1);
    let original = subscription.as_ref().unwrap().summaries[&entity].clone();
    assert!(
        original
            .unsupported_components
            .contains_key(std::any::type_name::<LateRegistered>())
    );
    world
        .resource::<AppTypeRegistry>()
        .write()
        .register::<LateRegistered>();
    world.entity_mut(entity).insert(Name::new("same name"));
    tracking.run(&mut world);
    publish(&mut world, &mut subscription, &mut true);
    let frames = rx.try_iter().collect::<Vec<_>>();
    assert_eq!(
        frames.len(),
        1,
        "dirty registration metadata must reach the editor"
    );
    let params = frames[0].get("params").unwrap();
    assert_eq!(params.get("added"), Some(&Wire::Array(vec![])));
    assert_eq!(params.get("removed"), Some(&Wire::Array(vec![])));
    let rows = params.get("updated").unwrap().as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("entity"), Some(&reference(entity)));
    assert_eq!(rows[0].get("name"), original.to_wire().get("name"));
    assert_eq!(
        rows[0].get("components"),
        Some(&Wire::Array(vec![
            Wire::String(std::any::type_name::<Name>().into()),
            Wire::String("game::LateRegistered".into()),
        ])),
        "registration replaces the component name with its reflected type path"
    );
    assert!(
        rows[0]
            .get("unsupported_components")
            .unwrap()
            .get(std::any::type_name::<LateRegistered>())
            .is_none(),
        "newly registered component loses its unsupported reason"
    );
    assert_eq!(
        subscription.as_ref().unwrap().summaries[&entity].archetype,
        original.archetype
    );
    publish(&mut world, &mut subscription, &mut true);
    assert!(rx.try_recv().is_err(), "sent metadata is the next baseline");
}

fn subscribe(world: &mut World, subscription: &mut Option<Subscription>, interval_s: f64) {
    dispatch(
        world,
        subscription,
        &Wire::object([
            ("jsonrpc", Wire::String("2.0".into())),
            ("method", Wire::String("godot.subscribe".into())),
            (
                "params",
                Wire::object([("interval_s", Wire::Float(interval_s))]),
            ),
        ]),
    )
    .unwrap();
}

#[test]
fn snapshots_are_chunked_with_an_explicit_end_and_unique_entities() {
    assert_eq!(DebuggerConfig::default().snapshot_chunk_size, 256);
    for (extra_entities, chunk_size) in [(0, usize::MAX), (0, 2), (1, 2), (5, 2), (6, 2), (3, 0)] {
        let (tx, rx) = channel();
        let mut world = World::new();
        world.insert_resource(DebuggerConfig {
            snapshot_chunk_size: chunk_size,
            ..Default::default()
        });
        world.insert_resource(DebuggerTransport::new(move |frame| {
            tx.send(frame).unwrap();
        }));
        world.init_resource::<SummaryChanges>();
        for _ in 0..extra_entities {
            world.spawn_empty();
        }
        let mut entities = world.query::<Entity>().iter(&world).collect::<Vec<_>>();
        entities.sort();
        let count = entities.len();
        if chunk_size == usize::MAX {
            assert!(
                count > 0 && count < chunk_size,
                "fixture fits a partial single chunk"
            );
        }
        let mut subscription = None;
        subscribe(&mut world, &mut subscription, 0.0);
        let expected_chunks = count.div_ceil(chunk_size.max(1)).max(1);
        let mut received = Vec::new();
        for index in 0..expected_chunks {
            let mut budget = true;
            publish(&mut world, &mut subscription, &mut budget);
            publish(&mut world, &mut subscription, &mut budget);
            let frames = rx.try_iter().collect::<Vec<_>>();
            assert_eq!(frames.len(), 1, "one snapshot chunk per drain budget");
            let params = frames[0].get("params").unwrap();
            assert_eq!(params.get("snapshot"), Some(&Wire::Bool(true)));
            assert_eq!(
                params.get("snapshot_index"),
                Some(&Wire::Integer(index as i64))
            );
            assert_eq!(
                params.get("snapshot_complete"),
                Some(&Wire::Bool(index + 1 == expected_chunks))
            );
            assert_eq!(params.get("removed"), Some(&Wire::Array(vec![])));
            assert_eq!(params.get("updated"), Some(&Wire::Array(vec![])));
            let rows = params.get("added").unwrap().as_array().unwrap();
            assert_eq!(
                rows.len(),
                (count - index * chunk_size.max(1)).min(chunk_size.max(1))
            );
            received.extend(rows.iter().map(|row| row.get("entity").unwrap().clone()));
        }
        assert_eq!(
            received,
            entities.into_iter().map(reference).collect::<Vec<_>>(),
            "every entity appears exactly once across the snapshot"
        );
        publish(&mut world, &mut subscription, &mut true);
        assert!(
            rx.try_recv().is_err(),
            "snapshot completion returns to silent unchanged intervals"
        );
    }
}

#[test]
fn disabling_notifies_once_and_discards_the_subscription() {
    for chunk_size in [1, usize::MAX] {
        let (tx, rx) = channel();
        let mut world = World::new();
        world.insert_resource(DebuggerConfig {
            snapshot_chunk_size: chunk_size,
            ..Default::default()
        });
        world.insert_resource(DebuggerTransport::new(move |frame| {
            tx.send(frame).unwrap();
        }));
        world.init_resource::<SummaryChanges>();
        let mut subscription = None;
        subscribe(&mut world, &mut subscription, 60.0);
        publish(&mut world, &mut subscription, &mut true);
        let snapshot = rx.try_recv().unwrap();
        assert_eq!(
            snapshot.get("params").unwrap().get("snapshot_complete"),
            Some(&Wire::Bool(chunk_size != 1))
        );
        world.resource_mut::<DebuggerConfig>().enabled = false;
        publish(&mut world, &mut subscription, &mut false);
        let frames = rx.try_iter().collect::<Vec<_>>();
        assert_eq!(
            frames.len(),
            1,
            "disable sends a terminal notification even with no chunk budget"
        );
        assert_eq!(
            frames[0].get("method"),
            Some(&Wire::String("godot.summary".into()))
        );
        assert!(frames[0].get("id").is_none());
        assert_eq!(
            frames[0].get("params"),
            Some(&Wire::object([
                ("subscription_ended", Wire::Bool(true)),
                ("reason", Wire::String("debugger disabled".into())),
            ]))
        );
        assert!(subscription.is_none());
        assert!(!world.resource::<SummaryChanges>().active);
        assert!(world.resource::<SummaryChanges>().dirty.is_empty());
        publish(&mut world, &mut subscription, &mut true);
        assert!(
            rx.try_recv().is_err(),
            "terminal notification is sent only once"
        );
        world.resource_mut::<DebuggerConfig>().enabled = true;
        world.resource_mut::<DebuggerConfig>().snapshot_chunk_size = usize::MAX;
        subscribe(&mut world, &mut subscription, 0.0);
        publish(&mut world, &mut subscription, &mut true);
        let fresh = rx.try_recv().unwrap();
        assert_eq!(
            fresh.get("params").unwrap().get("snapshot_index"),
            Some(&Wire::Integer(0))
        );
        assert_eq!(
            fresh.get("params").unwrap().get("snapshot_complete"),
            Some(&Wire::Bool(true))
        );
    }
}

#[test]
fn unsubscribe_discards_an_unfinished_snapshot() {
    let (tx, rx) = channel();
    let mut world = World::new();
    world.insert_resource(DebuggerConfig {
        snapshot_chunk_size: 1,
        ..Default::default()
    });
    world.insert_resource(DebuggerTransport::new(move |frame| {
        tx.send(frame).unwrap();
    }));
    world.init_resource::<SummaryChanges>();
    let first = world.spawn_empty().id();
    let second = world.spawn_empty().id();
    let mut subscription = None;
    subscribe(&mut world, &mut subscription, 0.0);
    publish(&mut world, &mut subscription, &mut true);
    assert_eq!(
        rx.try_recv()
            .unwrap()
            .get("params")
            .unwrap()
            .get("snapshot_complete"),
        Some(&Wire::Bool(false))
    );
    dispatch(
        &mut world,
        &mut subscription,
        &Wire::object([
            ("jsonrpc", Wire::String("2.0".into())),
            ("method", Wire::String("godot.unsubscribe".into())),
        ]),
    )
    .unwrap();
    assert!(subscription.is_none());
    world.despawn(first);
    world.despawn(second);
    let fresh = world.spawn_empty().id();
    world.resource_mut::<DebuggerConfig>().snapshot_chunk_size = 256;
    subscribe(&mut world, &mut subscription, 0.0);
    publish(&mut world, &mut subscription, &mut true);
    let frame = rx.try_recv().unwrap();
    let params = frame.get("params").unwrap();
    assert_eq!(params.get("snapshot_index"), Some(&Wire::Integer(0)));
    assert_eq!(params.get("snapshot_complete"), Some(&Wire::Bool(true)));
    let rows = params.get("added").unwrap().as_array().unwrap();
    assert!(
        rows.iter()
            .any(|row| row.get("entity") == Some(&reference(fresh)))
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.get("entity") == Some(&reference(first))
                || row.get("entity") == Some(&reference(second)))
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn tracking_reconciles_live_entities_and_removed_summary_fields() {
    let (tx, rx) = channel();
    let mut world = World::new();
    world.insert_resource(DebuggerConfig::default());
    world.insert_resource(DebuggerTransport::new(move |frame| {
        tx.send(frame).unwrap();
    }));
    world.init_resource::<SummaryChanges>();
    let mut tracking = Schedule::default();
    tracking.add_systems(track_summary_changes);
    let parent = world.spawn(Ordinary).id();
    let entity = world
        .spawn((Ordinary, Name::new("named"), GodotChildOf(parent)))
        .id();
    tracking.run(&mut world);
    world.clear_trackers();
    let mut subscription = None;
    dispatch(
        &mut world,
        &mut subscription,
        &Wire::object([
            ("jsonrpc", Wire::String("2.0".into())),
            ("id", Wire::Integer(1)),
            ("method", Wire::String("godot.subscribe".into())),
            ("params", Wire::object([("interval_s", Wire::Float(0.0))])),
        ]),
    )
    .unwrap();
    publish(&mut world, &mut subscription, &mut true);
    assert_eq!(rx.try_iter().count(), 1);
    tracking.run(&mut world);
    publish(&mut world, &mut subscription, &mut true);
    assert!(rx.try_recv().is_err());
    world.clear_trackers();

    let added = world.spawn(Ordinary).id();
    tracking.run(&mut world);
    publish(&mut world, &mut subscription, &mut true);
    let frame = rx.try_recv().unwrap();
    let rows = frame
        .get("params")
        .unwrap()
        .get("added")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("entity"), Some(&reference(added)));
    assert!(rx.try_recv().is_err());
    world.clear_trackers();

    world.entity_mut(entity).remove::<GodotChildOf>();
    world.flush();
    tracking.run(&mut world);
    publish(&mut world, &mut subscription, &mut true);
    let frame = rx.try_recv().unwrap();
    let rows = frame
        .get("params")
        .unwrap()
        .get("updated")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(
        rows.len(),
        2,
        "unlink updates the child and the parent's membership"
    );
    let child = rows
        .iter()
        .find(|row| row.get("entity") == Some(&reference(entity)))
        .unwrap();
    assert_eq!(child.get("parent"), Some(&Wire::Null));
    let parent = rows
        .iter()
        .find(|row| row.get("entity") == Some(&reference(parent)))
        .unwrap();
    assert_eq!(
        parent.get("components"),
        Some(&Wire::Array(vec![Wire::String(
            std::any::type_name::<Ordinary>().into()
        )]))
    );
    assert!(rx.try_recv().is_err());
    world.clear_trackers();

    world.entity_mut(entity).remove::<Name>();
    tracking.run(&mut world);
    publish(&mut world, &mut subscription, &mut true);
    let frame = rx.try_recv().unwrap();
    let rows = frame
        .get("params")
        .unwrap()
        .get("updated")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("entity"), Some(&reference(entity)));
    assert_eq!(rows[0].get("name"), Some(&Wire::String(String::new())));
    assert!(rx.try_recv().is_err());
    world.clear_trackers();

    world.despawn(added);
    tracking.run(&mut world);
    publish(&mut world, &mut subscription, &mut true);
    let frame = rx.try_recv().unwrap();
    assert_eq!(
        frame.get("params").unwrap().get("removed"),
        Some(&Wire::Array(vec![reference(added)]))
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn disabling_clears_subscription_and_dirty_state_without_a_request() {
    let (tx, rx) = channel();
    let mut world = World::new();
    world.insert_resource(DebuggerConfig::default());
    world.insert_resource(DebuggerTransport::new(move |frame| {
        tx.send(frame).unwrap();
    }));
    world.init_resource::<SummaryChanges>();
    let mut tracking = Schedule::default();
    tracking.add_systems(track_summary_changes);
    let entity = world.spawn(Name::new("original")).id();
    tracking.run(&mut world);
    world.clear_trackers();
    let mut subscription = None;
    dispatch(
        &mut world,
        &mut subscription,
        &Wire::object([
            ("jsonrpc", Wire::String("2.0".into())),
            ("id", Wire::Integer(1)),
            ("method", Wire::String("godot.subscribe".into())),
            ("params", Wire::object([("interval_s", Wire::Float(60.0))])),
        ]),
    )
    .unwrap();
    publish(&mut world, &mut subscription, &mut true);
    assert_eq!(rx.try_iter().count(), 1);
    world.entity_mut(entity).insert(Name::new("renamed"));
    tracking.run(&mut world);
    assert!(world.resource::<SummaryChanges>().dirty.contains(&entity));
    world.resource_mut::<DebuggerConfig>().enabled = false;
    publish(&mut world, &mut subscription, &mut true);
    assert_eq!(
        rx.try_recv().unwrap().get("params"),
        Some(&Wire::object([
            ("subscription_ended", Wire::Bool(true)),
            ("reason", Wire::String("debugger disabled".into())),
        ]))
    );
    assert!(subscription.is_none());
    assert!(!world.resource::<SummaryChanges>().active);
    assert!(world.resource::<SummaryChanges>().dirty.is_empty());
    for _ in 0..10 {
        world.clear_trackers();
        world.entity_mut(entity).insert(Name::new("disabled"));
        tracking.run(&mut world);
        assert!(world.resource::<SummaryChanges>().dirty.is_empty());
    }
    let result = dispatch(
        &mut world,
        &mut subscription,
        &Wire::object([
            ("jsonrpc", Wire::String("2.0".into())),
            ("method", Wire::String("godot.unsubscribe".into())),
        ]),
    )
    .unwrap();
    assert_eq!(result.get("subscribed"), Some(&Wire::Bool(false)));
    world.resource_mut::<DebuggerConfig>().enabled = true;
    publish(&mut world, &mut subscription, &mut true);
    assert!(rx.try_recv().is_err());
}

#[derive(Resource, bevy_reflect::Reflect)]
#[type_path = "game::resources"]
struct Inventory;

#[derive(Resource)]
struct UnregisteredInventory;

#[test]
fn resource_descriptors_use_the_marker_and_keep_absent_identity_across_chunks() {
    let (tx, rx) = channel();
    let mut world = World::new();
    world.insert_resource(DebuggerConfig {
        snapshot_chunk_size: 1,
        ..Default::default()
    });
    world.insert_resource(DebuggerTransport::new(move |frame| {
        tx.send(frame).unwrap();
    }));
    world.init_resource::<SummaryChanges>();
    world.init_resource::<AppTypeRegistry>();
    world
        .resource::<AppTypeRegistry>()
        .write()
        .register::<Inventory>();
    world.insert_resource(Inventory);
    let entity = world
        .resource_entities()
        .get(world.component_id::<Inventory>().unwrap())
        .unwrap();
    world
        .entity_mut(entity)
        .insert((Ordinary, Registered, Name::new("misleading name")));
    let descriptor = |present| {
        Wire::object([
            (
                "type_path",
                Wire::String("game::resources::Inventory".into()),
            ),
            ("present", Wire::Bool(present)),
        ])
    };
    assert_eq!(
        summary(&world, entity).unwrap().to_wire().get("resource"),
        Some(&descriptor(true))
    );
    let ordinary = world.spawn(Ordinary).id();
    assert_eq!(
        summary(&world, ordinary).unwrap().to_wire().get("resource"),
        Some(&Wire::Null)
    );
    world.insert_resource(UnregisteredInventory);
    let unregistered = world
        .resource_entities()
        .get(world.component_id::<UnregisteredInventory>().unwrap())
        .unwrap();
    let fallback = summary(&world, unregistered).unwrap().to_wire();
    assert_eq!(
        fallback.get("resource"),
        Some(&Wire::object([
            (
                "type_path",
                Wire::String(std::any::type_name::<UnregisteredInventory>().into())
            ),
            ("present", Wire::Bool(true)),
        ]))
    );
    assert_eq!(
        fallback
            .get("unsupported_components")
            .unwrap()
            .get(std::any::type_name::<UnregisteredInventory>()),
        Some(&Wire::String("component not registered".into()))
    );
    world.remove_resource::<UnregisteredInventory>();
    assert_eq!(
        summary(&world, unregistered)
            .unwrap()
            .to_wire()
            .get("resource"),
        Some(&Wire::object([
            (
                "type_path",
                Wire::String(std::any::type_name::<UnregisteredInventory>().into())
            ),
            ("present", Wire::Bool(false)),
        ]))
    );
    world.insert_resource(UnregisteredInventory);
    let mut subscription = None;
    subscribe(&mut world, &mut subscription, 0.0);
    publish(&mut world, &mut subscription, &mut true);
    world.remove_resource::<Inventory>();
    assert!(world.get_entity(entity).is_ok());
    assert_eq!(
        world
            .get::<bevy_ecs::resource::IsResource>(entity)
            .unwrap()
            .resource_component_id(),
        world.component_id::<Inventory>().unwrap()
    );
    while subscription.as_ref().unwrap().snapshot.is_some() {
        publish(&mut world, &mut subscription, &mut true);
    }
    let chunks = rx.try_iter().collect::<Vec<_>>();
    let mut seen = Vec::new();
    for (index, chunk) in chunks.iter().enumerate() {
        let params = chunk.get("params").unwrap();
        assert_eq!(
            params.get("snapshot_index"),
            Some(&Wire::Integer(index as i64))
        );
        assert_eq!(
            params.get("snapshot_complete"),
            Some(&Wire::Bool(index + 1 == chunks.len()))
        );
        let rows = params.get("added").unwrap().as_array().unwrap();
        assert_eq!(rows.len(), 1);
        seen.push(rows[0].get("entity").unwrap().clone());
        if rows[0].get("entity") == Some(&reference(entity)) {
            assert_eq!(
                rows[0].get("resource"),
                Some(&descriptor(true)),
                "snapshot retains original presence"
            );
        }
    }
    assert_eq!(
        seen.iter().filter(|id| **id == reference(entity)).count(),
        1
    );
    for present in [false, true, false] {
        if present {
            world.insert_resource(Inventory);
        } else {
            world.remove_resource::<Inventory>();
        }
        publish(&mut world, &mut subscription, &mut true);
        let frames = rx.try_iter().collect::<Vec<_>>();
        assert_eq!(frames.len(), 1);
        let params = frames[0].get("params").unwrap();
        assert_eq!(params.get("added"), Some(&Wire::Array(vec![])));
        assert_eq!(params.get("removed"), Some(&Wire::Array(vec![])));
        let rows = params.get("updated").unwrap().as_array().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("entity"), Some(&reference(entity)));
        assert_eq!(rows[0].get("resource"), Some(&descriptor(present)));
        publish(&mut world, &mut subscription, &mut true);
        assert!(rx.try_recv().is_err());
    }
    // A resource's backing entity belongs to the world: despawning it by hand leaves
    // `resource_entities` pointing at a dead id and the next resource command panics, so the
    // ordinary-removal path is exercised with a plain entity instead.
    let plain = world.spawn(Ordinary).id();
    subscription.as_mut().unwrap().sent_at = Instant::now() - Duration::from_secs(61);
    publish(&mut world, &mut subscription, &mut true);
    rx.try_iter().for_each(drop);
    world.despawn(plain);
    subscription.as_mut().unwrap().sent_at = Instant::now() - Duration::from_secs(61);
    publish(&mut world, &mut subscription, &mut true);
    assert_eq!(
        rx.try_recv().unwrap().get("params").unwrap().get("removed"),
        Some(&Wire::Array(vec![reference(plain)]))
    );
}

#[test]
fn resource_query_filters_inventory_including_absent_rows_and_full_type_paths() {
    let mut world = World::new();
    world.init_resource::<AppTypeRegistry>();
    world
        .resource::<AppTypeRegistry>()
        .write()
        .register::<Inventory>();
    world.insert_resource(Inventory);
    let entity = world
        .resource_entities()
        .get(world.component_id::<Inventory>().unwrap())
        .unwrap();
    world.entity_mut(entity).insert(Ordinary);
    let ordinary = world.spawn(Ordinary).id();
    for present in [true, false] {
        if !present {
            world.remove_resource::<Inventory>();
        }
        let params = Wire::object([
            ("resource", Wire::Bool(true)),
            (
                "name_contains",
                Wire::String("game::resources::Inventory".into()),
            ),
            ("page", Wire::Integer(0)),
            ("page_size", Wire::Integer(1)),
        ]);
        let result = query(&mut world, &params).unwrap();
        assert_eq!(result.get("total"), Some(&Wire::Integer(1)));
        let rows = result.get("entities").unwrap().as_array().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("entity"), Some(&reference(entity)));
        assert_eq!(
            rows[0].get("resource").unwrap().get("present"),
            Some(&Wire::Bool(present))
        );
        let mut next_page = params.clone();
        let Wire::Object(fields) = &mut next_page else {
            unreachable!()
        };
        fields.insert("page".into(), Wire::Integer(1));
        let result = query(&mut world, &next_page).unwrap();
        assert_eq!(result.get("total"), Some(&Wire::Integer(1)));
        assert_eq!(result.get("entities"), Some(&Wire::Array(vec![])));
    }
    let result = query(
        &mut world,
        &Wire::object([
            ("resource", Wire::Bool(false)),
            ("page", Wire::Integer(0)),
            ("page_size", Wire::Integer(256)),
        ]),
    )
    .unwrap();
    assert_eq!(result.get("total"), Some(&Wire::Integer(1)));
    assert_eq!(
        result.get("entities").unwrap().as_array().unwrap()[0].get("entity"),
        Some(&reference(ordinary))
    );
    assert_eq!(
        query(
            &mut world,
            &Wire::object([
                ("resource", Wire::String("true".into())),
                ("page", Wire::Integer(0)),
                ("page_size", Wire::Integer(1)),
            ])
        )
        .unwrap_err()
        .code,
        -32602
    );
    let discovery = discover();
    let methods = discovery.get("methods").unwrap().as_array().unwrap();
    assert!(methods.iter().all(|method| {
        !method
            .get("name")
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with("world.")
    }));
    let query = methods
        .iter()
        .find(|method| method.get("name").unwrap().as_str() == Some("godot.query"))
        .unwrap();
    assert!(
        query
            .get("params")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .any(
                |param| param.get("name").unwrap().as_str() == Some("resource")
                    && param.get("required") == Some(&Wire::Bool(false))
                    && param.get("schema").unwrap().get("type").unwrap().as_str()
                        == Some("boolean")
            )
    );
}
