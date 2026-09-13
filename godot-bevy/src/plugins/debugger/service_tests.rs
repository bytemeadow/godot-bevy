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
