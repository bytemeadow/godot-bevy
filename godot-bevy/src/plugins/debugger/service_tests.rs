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
    publish(&mut world, &mut subscription);
    assert_eq!(rx.try_iter().count(), 1);
    tracking.run(&mut world);
    subscription.as_mut().unwrap().sent_at = Instant::now() - Duration::from_secs(61);
    publish(&mut world, &mut subscription);
    assert!(rx.try_recv().is_err());
    assert!(world.resource::<SummaryChanges>().dirty.is_empty());
    world.clear_trackers();
    world.entity_mut(entity).insert(Name::new("first rename"));
    tracking.run(&mut world);
    publish(&mut world, &mut subscription);
    assert!(rx.try_recv().is_err());
    assert!(world.resource::<SummaryChanges>().dirty.contains(&entity));
    world.clear_trackers();
    world.entity_mut(entity).insert(Name::new("last rename"));
    tracking.run(&mut world);
    subscription.as_mut().unwrap().sent_at = Instant::now() - Duration::from_secs(61);
    publish(&mut world, &mut subscription);
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
    publish(&mut world, &mut subscription);
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
    publish(&mut world, &mut subscription);
    assert_eq!(rx.try_iter().count(), 1);
    tracking.run(&mut world);
    publish(&mut world, &mut subscription);
    assert!(rx.try_recv().is_err());
    world.clear_trackers();

    let added = world.spawn(Ordinary).id();
    tracking.run(&mut world);
    publish(&mut world, &mut subscription);
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
    tracking.run(&mut world);
    publish(&mut world, &mut subscription);
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
    assert_eq!(rows[0].get("parent"), Some(&Wire::Null));
    assert!(rx.try_recv().is_err());
    world.clear_trackers();

    world.entity_mut(entity).remove::<Name>();
    tracking.run(&mut world);
    publish(&mut world, &mut subscription);
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
    publish(&mut world, &mut subscription);
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
    publish(&mut world, &mut subscription);
    assert_eq!(rx.try_iter().count(), 1);
    world.entity_mut(entity).insert(Name::new("renamed"));
    tracking.run(&mut world);
    assert!(world.resource::<SummaryChanges>().dirty.contains(&entity));
    world.resource_mut::<DebuggerConfig>().enabled = false;
    publish(&mut world, &mut subscription);
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
    publish(&mut world, &mut subscription);
    assert!(rx.try_recv().is_err());
}
