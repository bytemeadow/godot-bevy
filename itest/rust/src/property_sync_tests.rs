// Remove when gdext includes godot-rust/gdext#1706; #[class(init)] emits base: base.
#![allow(clippy::redundant_field_names)]

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use godot::prelude::*;
use godot_bevy::interop::{GodotAccess, GodotMainThread};
#[cfg(feature = "test-frame-signal")]
use godot_bevy::plugins::property_sync::PropertySyncCounters;
use godot_bevy::plugins::scene_tree::ProtectedNodeEntity;
use godot_bevy::plugins::scene_tree::autosync::{
    AutoSyncBundleRegistry, register_all_autosync_bundles, register_all_required_components,
};
use godot_bevy::prelude::*;
use godot_bevy_test::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Component, Default, Reflect, GodotNode)]
#[gdbevy(class_name = PropertySyncNode)]
struct SyncedProperties {
    #[gdbevy(export, default = 10, sync = two_way)]
    health: i32,
    #[gdbevy(export, default = 2.0, sync = two_way)]
    speed: f32,
    #[gdbevy(export, default = 7)]
    creation_only: i32,
    runtime_only: i32,
}

fn test_app() -> App {
    let mut app = App::new();
    app.insert_non_send(GodotMainThread);
    app.init_schedule(PreUpdate);
    app.init_schedule(PostUpdate);
    register_all_required_components(&mut app);
    register_all_autosync_bundles(&mut app);
    app
}

fn decorate(app: &mut App, node: Gd<Node>, class_name: &'static str) -> Entity {
    let handle = GodotNodeHandle::new(node);
    let entity = app.world_mut().spawn(handle).id();
    let entry = godot_bevy::inventory::iter::<AutoSyncBundleRegistry>
        .into_iter()
        .find(|entry| (entry.godot_class_id_fn)().to_string() == class_name)
        .expect("registered property class");
    app.world_mut()
        .run_system_once(move |mut commands: Commands, mut godot: GodotAccess| {
            assert!((entry.create_bundle_fn)(
                &mut commands,
                entity,
                &mut godot,
                handle
            ));
        })
        .unwrap();
    entity
}

fn setup() -> (App, Gd<PropertySyncNode>, Entity) {
    let mut app = test_app();
    let node = PropertySyncNode::new_alloc();
    let entity = decorate(&mut app, node.clone().upcast(), "PropertySyncNode");
    (app, node, entity)
}

fn cycle(app: &mut App) {
    app.world_mut().run_schedule(PreUpdate);
    app.world_mut().run_schedule(PostUpdate);
    app.world_mut().clear_trackers();
}

#[derive(Resource, Default)]
struct Observed(Vec<(u32, i32)>);

#[derive(Resource)]
struct Cycle(u32);

#[itest]
fn property_sync_remote_edit_before_read() {
    let (mut app, mut node, entity) = setup();
    app.insert_resource(Cycle(42));
    app.init_resource::<Observed>();
    app.add_systems(
        PreUpdate,
        (|properties: Query<&SyncedProperties>, cycle: Res<Cycle>, mut seen: ResMut<Observed>| {
            seen.0.push((cycle.0, properties.single().unwrap().health));
        })
        .after(PropertySyncSet::Read),
    );
    app.add_systems(
        Update,
        |properties: Query<&SyncedProperties>, cycle: Res<Cycle>, mut seen: ResMut<Observed>| {
            seen.0.push((cycle.0, properties.single().unwrap().health));
        },
    );
    node.set("health", &37.to_variant());
    app.world_mut().run_schedule(PreUpdate);
    app.world_mut().run_schedule(Update);
    assert_eq!(app.world().resource::<Observed>().0, [(42, 37), (42, 37)]);
    assert_eq!(
        app.world().get::<SyncedProperties>(entity).unwrap().health,
        37
    );
    node.free();
}

#[itest]
fn property_sync_publish_after_postupdate() {
    let (mut app, node, _) = setup();
    app.init_resource::<Observed>();
    app.add_systems(Update, |mut properties: Query<&mut SyncedProperties>| {
        properties.single_mut().unwrap().health = 21;
    });
    app.add_systems(
        PostUpdate,
        (|mut properties: Query<&mut SyncedProperties>| {
            properties.single_mut().unwrap().health = 43;
        })
        .before(PropertySyncSet::Write),
    );
    app.add_systems(
        PostUpdate,
        (|handles: Query<&GodotNodeHandle>, mut godot: GodotAccess, mut seen: ResMut<Observed>| {
            let node = godot.get::<PropertySyncNode>(*handles.single().unwrap());
            seen.0.push((1, node.get("health").to::<i32>()));
        })
        .after(PropertySyncSet::Write),
    );
    app.world_mut().run_schedule(PreUpdate);
    app.world_mut().run_schedule(Update);
    app.world_mut().run_schedule(PostUpdate);
    assert_eq!(app.world().resource::<Observed>().0, [(1, 43)]);
    node.free();
}

#[itest]
fn property_sync_bevy_wins() {
    for stage in 0..4 {
        let (mut app, mut node, entity) = setup();
        node.set("health", &99.to_variant());
        let author = |mut properties: Query<&mut SyncedProperties>| {
            properties.single_mut().unwrap().health = 51;
        };
        match stage {
            0 => {
                app.add_systems(Startup, author);
                app.world_mut().run_schedule(Startup);
            }
            1 => {
                app.add_systems(First, author);
                app.world_mut().run_schedule(First);
            }
            _ => {}
        }
        app.world_mut().run_schedule(PreUpdate);
        if stage < 2 {
            assert_eq!(
                app.world().get::<SyncedProperties>(entity).unwrap().health,
                51
            );
        }
        match stage {
            2 => {
                app.add_systems(FixedUpdate, author);
                app.world_mut().run_schedule(FixedUpdate);
            }
            3 => {
                app.add_systems(Update, author);
                app.world_mut().run_schedule(Update);
            }
            _ => {}
        }
        app.world_mut().run_schedule(PostUpdate);
        assert_eq!(node.get("health").to::<i32>(), 51, "author stage {stage}");
        cycle(&mut app);
        assert_eq!(
            app.world().get::<SyncedProperties>(entity).unwrap().health,
            51
        );
        node.free();
    }
}

#[itest]
fn property_sync_field_independence() {
    let (mut app, mut node, entity) = setup();
    cycle(&mut app);
    {
        let mut properties = app.world_mut().get_mut::<SyncedProperties>(entity).unwrap();
        properties.health = 52;
        properties.runtime_only = 123;
        properties.creation_only = 456;
    }
    node.set("speed", &8.0f32.to_variant());
    node.set("creation_only", &999.to_variant());
    cycle(&mut app);
    let properties = app.world().get::<SyncedProperties>(entity).unwrap();
    assert_eq!(
        (
            properties.health,
            properties.speed,
            properties.runtime_only,
            properties.creation_only
        ),
        (52, 8.0, 123, 456)
    );
    assert_eq!(node.get("health").to::<i32>(), 52);
    assert_eq!(node.get("creation_only").to::<i32>(), 999);
    node.free();
}

static INBOUND: AtomicUsize = AtomicUsize::new(0);
static OUTBOUND: AtomicUsize = AtomicUsize::new(0);

fn from_percent(value: i32) -> f32 {
    INBOUND.fetch_add(1, Ordering::SeqCst);
    value.clamp(0, 100) as f32 / 100.0
}

fn to_percent(value: f32) -> i32 {
    OUTBOUND.fetch_add(1, Ordering::SeqCst);
    (value * 100.0).round() as i32
}

#[derive(Component, Default, GodotNode)]
#[gdbevy(class_name = PropertyLossyNode)]
struct LossyProperty {
    #[gdbevy(export, as = i32, default = 25, sync = two_way, with = from_percent, into = to_percent)]
    fraction: f32,
}

#[derive(Component, Default, GodotNode)]
#[gdbevy(class_name = PropertyOutboundNode)]
struct OutboundProperty {
    #[gdbevy(export, as = i32, default = 25, sync = to_godot, with = from_percent, into = to_percent)]
    fraction: f32,
}

#[itest]
fn property_sync_no_lossy_echo() {
    let mut app = test_app();
    let mut node = PropertyLossyNode::new_alloc();
    let entity = decorate(&mut app, node.clone().upcast(), "PropertyLossyNode");
    app.init_resource::<Observed>();
    app.add_systems(
        PreUpdate,
        (|changed: Query<&LossyProperty, Changed<LossyProperty>>, mut seen: ResMut<Observed>| {
            seen.0.push((0, changed.iter().count() as i32));
        })
        .after(PropertySyncSet::Read),
    );
    cycle(&mut app);
    app.world_mut()
        .get_mut::<LossyProperty>(entity)
        .unwrap()
        .fraction = 0.1234;
    cycle(&mut app);
    assert_eq!(node.get("fraction").to::<i32>(), 12);
    INBOUND.store(0, Ordering::SeqCst);
    OUTBOUND.store(0, Ordering::SeqCst);
    app.world_mut().resource_mut::<Observed>().0.clear();
    #[cfg(feature = "test-frame-signal")]
    {
        *app.world_mut().resource_mut::<PropertySyncCounters>() = default();
    }
    cycle(&mut app);
    assert_eq!(app.world().resource::<Observed>().0, [(0, 0)]);
    #[cfg(feature = "test-frame-signal")]
    assert_eq!(app.world().resource::<PropertySyncCounters>().writes, 0);
    assert_eq!(
        app.world().get::<LossyProperty>(entity).unwrap().fraction,
        0.1234
    );
    assert_eq!(INBOUND.load(Ordering::SeqCst), 0);
    assert_eq!(OUTBOUND.load(Ordering::SeqCst), 0);
    node.set("fraction", &57.to_variant());
    cycle(&mut app);
    assert_eq!(
        app.world().get::<LossyProperty>(entity).unwrap().fraction,
        0.57
    );
    assert_eq!(OUTBOUND.load(Ordering::SeqCst), 0);
    node.set("fraction", &150.to_variant());
    cycle(&mut app);
    assert_eq!(
        app.world().get::<LossyProperty>(entity).unwrap().fraction,
        1.0
    );
    assert_eq!(node.get("fraction").to::<i32>(), 150);
    assert_eq!(OUTBOUND.load(Ordering::SeqCst), 0);
    node.free();
}

#[itest]
fn property_sync_to_godot_no_read() {
    let mut app = test_app();
    let mut node = PropertyOutboundNode::new_alloc();
    let entity = decorate(&mut app, node.clone().upcast(), "PropertyOutboundNode");
    cycle(&mut app);
    INBOUND.store(0, Ordering::SeqCst);
    #[cfg(feature = "test-frame-signal")]
    {
        *app.world_mut().resource_mut::<PropertySyncCounters>() = default();
    }
    node.set("fraction", &99.to_variant());
    cycle(&mut app);
    assert_eq!(
        app.world()
            .get::<OutboundProperty>(entity)
            .unwrap()
            .fraction,
        0.25
    );
    app.world_mut()
        .get_mut::<OutboundProperty>(entity)
        .unwrap()
        .fraction = 0.75;
    cycle(&mut app);
    assert_eq!(node.get("fraction").to::<i32>(), 75);
    assert_eq!(INBOUND.load(Ordering::SeqCst), 0);
    #[cfg(feature = "test-frame-signal")]
    {
        let counts = app.world().resource::<PropertySyncCounters>();
        assert_eq!((counts.reads, counts.writes), (0, 1));
    }
    node.free();
}

#[itest]
fn property_sync_late_godot_edit() {
    let (mut app, mut node, entity) = setup();
    cycle(&mut app);
    app.world_mut().run_schedule(PreUpdate);
    node.set("health", &88.to_variant());
    app.world_mut().run_schedule(PostUpdate);
    assert_eq!(node.get("health").to::<i32>(), 88);
    assert_eq!(
        app.world().get::<SyncedProperties>(entity).unwrap().health,
        10
    );
    app.world_mut().run_schedule(PreUpdate);
    assert_eq!(
        app.world().get::<SyncedProperties>(entity).unwrap().health,
        88
    );
    node.set("health", &99.to_variant());
    app.world_mut()
        .get_mut::<SyncedProperties>(entity)
        .unwrap()
        .health = 20;
    app.world_mut().run_schedule(PostUpdate);
    assert_eq!(node.get("health").to::<i32>(), 20);
    node.free();
}

#[itest]
fn property_sync_late_edit_overwritten_when_conversion_matches_snapshot() {
    let mut app = test_app();
    let mut node = PropertyLossyNode::new_alloc();
    let entity = decorate(&mut app, node.clone().upcast(), "PropertyLossyNode");
    cycle(&mut app);
    app.world_mut().run_schedule(PreUpdate);
    node.set("fraction", &99.to_variant());
    app.world_mut()
        .get_mut::<LossyProperty>(entity)
        .unwrap()
        .fraction = 0.2501;
    app.world_mut().run_schedule(PostUpdate);
    assert_eq!(node.get("fraction").to::<i32>(), 25);
    node.free();
}

#[itest]
fn property_sync_readded_component_is_authoritative() {
    for between_passes in [false, true] {
        let (mut app, mut node, entity) = setup();
        cycle(&mut app);
        if between_passes {
            app.world_mut().run_schedule(PreUpdate);
        }
        app.world_mut()
            .entity_mut(entity)
            .remove::<SyncedProperties>();
        app.world_mut().entity_mut(entity).insert(SyncedProperties {
            health: 63,
            speed: 4.0,
            ..default()
        });
        node.set("health", &98.to_variant());
        if !between_passes {
            app.world_mut().run_schedule(PreUpdate);
        }
        app.world_mut().run_schedule(PostUpdate);
        assert_eq!(node.get("health").to::<i32>(), 63);
        node.set("health", &72.to_variant());
        app.world_mut().run_schedule(PreUpdate);
        assert_eq!(
            app.world().get::<SyncedProperties>(entity).unwrap().health,
            72,
            "between passes: {between_passes}"
        );
        node.free();
    }
}

#[itest]
fn property_sync_removed_component_stays_absent() {
    let (mut app, mut node, entity) = setup();
    cycle(&mut app);
    app.world_mut()
        .entity_mut(entity)
        .remove::<SyncedProperties>();
    node.set("health", &12.to_variant());
    cycle(&mut app);
    assert!(app.world().get::<SyncedProperties>(entity).is_none());
    assert_eq!(node.get("health").to::<i32>(), 12);
    node.free();
}

#[itest]
fn property_sync_handle_replacement_publishes_bevy() {
    let (mut app, old_node, entity) = setup();
    cycle(&mut app);
    let mut new_node = PropertySyncNode::new_alloc();
    new_node.set("health", &99.to_variant());
    app.world_mut()
        .entity_mut(entity)
        .insert(GodotNodeHandle::new(new_node.clone()));
    cycle(&mut app);
    assert_eq!(new_node.get("health").to::<i32>(), 10);
    new_node.set("health", &42.to_variant());
    cycle(&mut app);
    assert_eq!(
        app.world().get::<SyncedProperties>(entity).unwrap().health,
        42
    );
    old_node.free();
    new_node.free();
}

#[itest]
fn property_sync_freed_between_passes_and_despawn() {
    let (mut app, node, entity) = setup();
    app.world_mut().run_schedule(PreUpdate);
    app.world_mut()
        .get_mut::<SyncedProperties>(entity)
        .unwrap()
        .health = 20;
    node.free();
    app.world_mut().run_schedule(PostUpdate);
    cycle(&mut app);
    app.world_mut().despawn(entity);
    cycle(&mut app);
}

#[derive(Component, Default)]
struct NativeStats {
    health: i32,
    speed: f32,
    unmapped: i32,
}

#[derive(Component, Default)]
struct NativeLabel(String);

#[derive(Component, Default)]
struct NativeName(String);

fn from_string_name(value: StringName) -> String {
    value.to_string()
}
fn to_string_name(value: String) -> StringName {
    value.as_str().into()
}

fn from_gstring(value: GString) -> String {
    value.to_string()
}
fn to_gstring(value: String) -> GString {
    value.as_str().into()
}

type PropertyText = GString;
type PropertyName = StringName;

#[derive(GodotClass, BevyComponents)]
#[class(init, base = Node, rename = PropertyNativeRenamed)]
#[gdbevy(require(NativeStats { health: health, speed: speed }))]
struct PropertyNative {
    base: Base<Node>,
    #[export]
    #[init(val = 15)]
    #[gdbevy(sync = two_way)]
    health: i32,
    #[export]
    #[init(val = 3.0)]
    #[gdbevy(sync = to_godot)]
    speed: f32,
    #[export]
    #[init(val = GString::from("hello"))]
    #[gdbevy(component = NativeLabel, sync = two_way, with = from_gstring, into = to_gstring)]
    label: PropertyText,
    #[export]
    #[init(val = StringName::from("idle"))]
    #[gdbevy(component = NativeName, sync = two_way, with = from_string_name, into = to_string_name)]
    state: PropertyName,
}

#[itest]
fn property_sync_godot_first_renamed_class() {
    let mut app = test_app();
    let mut node = PropertyNative::new_alloc();
    let entity = decorate(&mut app, node.clone().upcast(), "PropertyNativeRenamed");
    assert_eq!(app.world().get::<NativeStats>(entity).unwrap().health, 15);
    assert_eq!(app.world().get::<NativeLabel>(entity).unwrap().0, "hello");
    cycle(&mut app);
    node.set("health", &27.to_variant());
    node.set("speed", &99.0f32.to_variant());
    node.set("label", &GString::from("remote").to_variant());
    node.set("state", &StringName::from("active").to_variant());
    app.world_mut()
        .get_mut::<NativeStats>(entity)
        .unwrap()
        .unmapped = 44;
    cycle(&mut app);
    let stats = app.world().get::<NativeStats>(entity).unwrap();
    assert_eq!((stats.health, stats.speed, stats.unmapped), (27, 3.0, 44));
    assert_eq!(app.world().get::<NativeLabel>(entity).unwrap().0, "remote");
    app.world_mut().get_mut::<NativeLabel>(entity).unwrap().0 = "local".into();
    app.world_mut()
        .get_mut::<NativeStats>(entity)
        .unwrap()
        .speed = 6.0;
    cycle(&mut app);
    assert_eq!(node.get("label").to::<GString>(), "local");
    assert_eq!(app.world().get::<NativeName>(entity).unwrap().0, "active");
    app.world_mut().get_mut::<NativeName>(entity).unwrap().0 = "finished".into();
    cycle(&mut app);
    assert_eq!(node.get("state").to::<StringName>(), "finished");
    assert_eq!(node.get("speed").to::<f32>(), 6.0);
    app.world_mut().entity_mut(entity).remove::<NativeStats>();
    node.set("label", &GString::from("still bound").to_variant());
    cycle(&mut app);
    assert!(app.world().get::<NativeStats>(entity).is_none());
    assert_eq!(
        app.world().get::<NativeLabel>(entity).unwrap().0,
        "still bound"
    );
    node.free();
}

#[derive(Component, Default)]
struct CompanionSpeed(f32);

#[derive(Component, Default)]
struct CompanionStats {
    mana: i32,
}

#[derive(Component, Default, GodotNode)]
#[gdbevy(class_name = PropertyCompanionNode)]
#[gdbevy(require(speed: CompanionSpeed, as = f32, default = 12.0, sync = two_way))]
#[gdbevy(require(stats: CompanionStats { mana(as = i32, default = 30, sync = to_godot) }))]
struct CompanionOwner;

#[derive(Component, Default, GodotNode)]
#[gdbevy(class_name = PropertyTupleNode)]
struct TupleProperty(#[gdbevy(export, default = 4, sync = two_way)] i32);

#[itest]
fn property_sync_companion_defaults_and_tuple() {
    let mut app = test_app();
    let mut node = PropertyCompanionNode::new_alloc();
    let entity = decorate(&mut app, node.clone().upcast(), "PropertyCompanionNode");
    assert_eq!(app.world().get::<CompanionSpeed>(entity).unwrap().0, 12.0);
    assert_eq!(app.world().get::<CompanionStats>(entity).unwrap().mana, 30);
    let pure_bevy = app.world_mut().spawn(CompanionOwner).id();
    assert_eq!(
        app.world().get::<CompanionSpeed>(pure_bevy).unwrap().0,
        12.0
    );
    node.set("speed", &19.0f32.to_variant());
    app.world_mut()
        .get_mut::<CompanionStats>(entity)
        .unwrap()
        .mana = 40;
    cycle(&mut app);
    assert_eq!(app.world().get::<CompanionSpeed>(entity).unwrap().0, 19.0);
    assert_eq!(
        app.world().get::<CompanionSpeed>(pure_bevy).unwrap().0,
        12.0
    );
    assert_eq!(node.get("mana").to::<i32>(), 40);
    app.world_mut()
        .entity_mut(entity)
        .remove::<CompanionSpeed>();
    node.set("speed", &22.0f32.to_variant());
    app.world_mut()
        .get_mut::<CompanionStats>(entity)
        .unwrap()
        .mana = 41;
    cycle(&mut app);
    assert!(app.world().get::<CompanionSpeed>(entity).is_none());
    assert_eq!(node.get("mana").to::<i32>(), 41);
    app.world_mut()
        .entity_mut(entity)
        .insert(CompanionSpeed(5.0));
    cycle(&mut app);
    assert_eq!(node.get("speed").to::<f32>(), 5.0);
    let mut tuple = PropertyTupleNode::new_alloc();
    let tuple_entity = decorate(&mut app, tuple.clone().upcast(), "PropertyTupleNode");
    tuple.set("value0", &13.to_variant());
    cycle(&mut app);
    assert_eq!(
        app.world().get::<TupleProperty>(tuple_entity).unwrap().0,
        13
    );
    node.free();
    tuple.free();
}

#[itest]
fn property_sync_two_apps_keep_independent_snapshots() {
    let (mut first, mut first_node, first_entity) = setup();
    let (mut second, mut second_node, second_entity) = setup();
    first_node.set("health", &11.to_variant());
    second_node.set("health", &22.to_variant());
    cycle(&mut first);
    cycle(&mut second);
    assert_eq!(
        first
            .world()
            .get::<SyncedProperties>(first_entity)
            .unwrap()
            .health,
        11
    );
    assert_eq!(
        second
            .world()
            .get::<SyncedProperties>(second_entity)
            .unwrap()
            .health,
        22
    );
    first_node.free();
    second_node.free();
}

#[itest]
fn property_sync_nan_is_stable() {
    let (mut app, mut node, entity) = setup();
    cycle(&mut app);
    node.set("speed", &f32::NAN.to_variant());
    cycle(&mut app);
    let bits = app
        .world()
        .get::<SyncedProperties>(entity)
        .unwrap()
        .speed
        .to_bits();
    assert!(
        app.world()
            .get::<SyncedProperties>(entity)
            .unwrap()
            .speed
            .is_nan()
    );
    app.init_resource::<Observed>();
    app.add_systems(
        PreUpdate,
        (|changed: Query<&SyncedProperties, Changed<SyncedProperties>>,
          mut seen: ResMut<Observed>| {
            seen.0.push((0, changed.iter().count() as i32));
        })
        .after(PropertySyncSet::Read),
    );
    cycle(&mut app);
    app.world_mut().resource_mut::<Observed>().0.clear();
    cycle(&mut app);
    assert_eq!(app.world().resource::<Observed>().0, [(0, 0)]);
    assert_eq!(
        app.world()
            .get::<SyncedProperties>(entity)
            .unwrap()
            .speed
            .to_bits(),
        bits
    );
    node.free();
}

#[itest]
fn property_sync_queue_free_skips_publication() {
    let (mut app, mut node, entity) = setup();
    cycle(&mut app);
    app.world_mut().run_schedule(PreUpdate);
    app.world_mut()
        .get_mut::<SyncedProperties>(entity)
        .unwrap()
        .health = 55;
    node.queue_free();
    app.world_mut().run_schedule(PostUpdate);
    assert_eq!(node.get("health").to::<i32>(), 10);
}

#[itest]
fn property_sync_class_mismatch_is_ignored() {
    let (mut app, node, entity) = setup();
    cycle(&mut app);
    let wrong_class = Node::new_alloc();
    app.world_mut()
        .entity_mut(entity)
        .insert(GodotNodeHandle::new(wrong_class.clone()));
    app.world_mut()
        .get_mut::<SyncedProperties>(entity)
        .unwrap()
        .health = 22;
    cycle(&mut app);
    assert_eq!(node.get("health").to::<i32>(), 10);
    node.free();
    wrong_class.free();
}

#[itest]
async fn property_sync_reparent_preserves_binding(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |_| {}).await;
    let first_parent = Node::new_alloc();
    let second_parent = Node::new_alloc();
    app.ctx().scene_tree.clone().add_child(&first_parent);
    app.ctx().scene_tree.clone().add_child(&second_parent);
    let mut node = PropertySyncNode::new_alloc();
    first_parent.clone().add_child(&node);
    app.updates(3).await;
    let entity = app.entity_for_node(node.instance_id()).unwrap();
    app.with_world_mut(|world| {
        let mut properties = world.get_mut::<SyncedProperties>(entity).unwrap();
        properties.health = 41;
        properties.runtime_only = 66;
    });
    node.reparent(&second_parent);
    app.updates(3).await;
    assert_eq!(app.entity_for_node(node.instance_id()), Some(entity));
    assert_eq!(node.get("health").to::<i32>(), 41);
    assert_eq!(
        app.with_world(|world| world.get::<SyncedProperties>(entity).unwrap().runtime_only),
        66
    );
    node.set("health", &52.to_variant());
    app.updates(2).await;
    assert_eq!(
        app.with_world(|world| world.get::<SyncedProperties>(entity).unwrap().health),
        52
    );
    app.cleanup().await;
    first_parent.free();
    second_parent.free();
}

#[itest]
fn property_sync_animation_before_read(ctx: &TestContext) {
    use godot::classes::animation::TrackType;
    use godot::classes::animation_mixer::AnimationCallbackModeProcess;
    use godot::classes::{Animation, AnimationLibrary, AnimationPlayer};

    let (mut app, mut node, entity) = setup();
    let mut player = AnimationPlayer::new_alloc();
    player.set_callback_mode_process(AnimationCallbackModeProcess::MANUAL);
    let mut animation = Animation::new_gd();
    animation.set_length(1.0);
    let track = animation.add_track(TrackType::VALUE);
    animation.track_set_path(track, ":speed");
    animation.track_insert_key(track, 0.0, &2.0f32.to_variant());
    animation.track_insert_key(track, 1.0, &10.0f32.to_variant());
    let mut library = AnimationLibrary::new_gd();
    assert_eq!(
        library.add_animation("speed", &animation),
        godot::global::Error::OK
    );
    assert_eq!(
        player.add_animation_library("", &library),
        godot::global::Error::OK
    );
    node.add_child(&player);
    ctx.scene_tree.clone().add_child(&node);
    player.play_ex().name("speed").done();
    player.advance(0.5);
    assert_eq!(node.get("speed").to::<f32>(), 6.0);
    app.world_mut().run_schedule(PreUpdate);
    assert_eq!(
        app.world().get::<SyncedProperties>(entity).unwrap().speed,
        6.0
    );
    node.free();
}

#[itest]
async fn property_sync_excluded_and_protected_detach(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |_| {}).await;
    let mut excluded = Node::new_alloc();
    excluded.set_meta("_bevy_exclude", &true.to_variant());
    let hidden = PropertySyncNode::new_alloc();
    excluded.add_child(&hidden);
    app.ctx().scene_tree.clone().add_child(&excluded);
    let mut visible = PropertySyncNode::new_alloc();
    app.ctx().scene_tree.clone().add_child(&visible);
    app.updates(3).await;
    assert!(app.entity_for_node(hidden.instance_id()).is_none());
    let entity = app.entity_for_node(visible.instance_id()).unwrap();
    app.with_world_mut(|world| {
        world.entity_mut(entity).insert(ProtectedNodeEntity);
    });
    app.ctx().scene_tree.clone().remove_child(&visible);
    app.updates(3).await;
    app.with_world_mut(|world| {
        assert!(world.get::<GodotNodeHandle>(entity).is_none());
        world.get_mut::<SyncedProperties>(entity).unwrap().health = 77;
    });
    visible.set("health", &88.to_variant());
    app.updates(2).await;
    assert_eq!(visible.get("health").to::<i32>(), 88);
    assert_eq!(
        app.with_world(|world| world.get::<SyncedProperties>(entity).unwrap().health),
        77
    );
    app.cleanup().await;
    visible.free();
    excluded.free();
}

#[derive(Event)]
struct PropertyAuthor;

#[itest]
fn property_sync_observer_and_reflection_win() {
    let (mut app, mut node, entity) = setup();
    app.world_mut().add_observer(
        |_: On<PropertyAuthor>, mut properties: Query<&mut SyncedProperties>| {
            properties.single_mut().unwrap().health = 71;
        },
    );
    node.set("health", &99.to_variant());
    app.world_mut().trigger(PropertyAuthor);
    cycle(&mut app);
    assert_eq!(node.get("health").to::<i32>(), 71);
    node.set("health", &98.to_variant());
    {
        let mut properties = app.world_mut().get_mut::<SyncedProperties>(entity).unwrap();
        let bevy::reflect::ReflectMut::Struct(value) = properties.reflect_mut() else {
            panic!("struct reflection");
        };
        value.field_mut("health").unwrap().apply(&83i32);
    }
    cycle(&mut app);
    assert_eq!(node.get("health").to::<i32>(), 83);
    cycle(&mut app);
    assert_eq!(
        app.world().get::<SyncedProperties>(entity).unwrap().health,
        83
    );
    node.free();
}

#[cfg(feature = "test-frame-signal")]
#[itest]
fn property_sync_split_frame_cadence() {
    use godot_bevy::plugins::fixed_schedule::{
        host_fixed_main_loop, run_main_suffix, run_physics_step, run_preamble,
    };
    use std::time::Duration;

    for paused in [false, true] {
        for fixed_steps in [0, 1, 3] {
            let mut app = App::new();
            app.insert_non_send(GodotMainThread);
            let entry = godot_bevy::inventory::iter::<AutoSyncBundleRegistry>
                .into_iter()
                .find(|entry| entry.godot_class_name == "PropertySyncNode")
                .unwrap();
            entry.register_property_sync.unwrap()(&mut app);
            let mut node = PropertySyncNode::new_alloc();
            let entity = decorate(&mut app, node.clone().upcast(), "PropertySyncNode");
            app.add_plugins(bevy::time::TimePlugin);
            host_fixed_main_loop(&mut app);
            if paused {
                app.world_mut().resource_mut::<Time<Virtual>>().pause();
            }
            app.init_resource::<Observed>();
            app.add_systems(
                FixedUpdate,
                |properties: Query<&SyncedProperties>, mut seen: ResMut<Observed>| {
                    seen.0.push((0, properties.single().unwrap().health));
                },
            );
            app.add_systems(Update, |mut properties: Query<&mut SyncedProperties>| {
                properties.single_mut().unwrap().health += 1;
            });
            node.set("health", &42.to_variant());
            for tick in 0..fixed_steps {
                run_physics_step(
                    app.world_mut(),
                    tick == 0,
                    tick == 0,
                    Duration::from_secs_f64(1.0 / 60.0),
                );
                assert_eq!(app.world().resource::<PropertySyncCounters>().reads, 2);
                assert_eq!(app.world().resource::<PropertySyncCounters>().writes, 0);
            }
            run_preamble(app.world_mut(), fixed_steps == 0, fixed_steps == 0);
            assert_eq!(
                app.world().get::<SyncedProperties>(entity).unwrap().health,
                42
            );
            run_main_suffix(app.world_mut());
            let counts = app.world().resource::<PropertySyncCounters>();
            assert_eq!((counts.reads, counts.writes), (2, 1));
            assert_eq!((counts.read_passes, counts.write_passes), (1, 1));
            assert_eq!(node.get("health").to::<i32>(), 43);
            let fixed_seen = &app.world().resource::<Observed>().0;
            assert_eq!(fixed_seen.len(), if paused { 0 } else { fixed_steps });
            assert!(fixed_seen.iter().all(|&(_, health)| health == 42));
            node.free();
        }
    }
}

#[cfg(feature = "test-frame-signal")]
#[itest]
fn property_sync_creation_and_idle_do_not_publish() {
    let (mut app, node, entity) = setup();
    assert_eq!(
        app.world().get::<SyncedProperties>(entity).unwrap().health,
        10
    );
    assert_eq!(
        app.world().get::<SyncedProperties>(entity).unwrap().speed,
        2.0
    );
    *app.world_mut().resource_mut::<PropertySyncCounters>() = default();
    let pure_bevy = app.world_mut().spawn(SyncedProperties::default()).id();
    assert_eq!(
        app.world().entity(pure_bevy).archetype().components().len(),
        1
    );
    cycle(&mut app);
    cycle(&mut app);
    let counts = app.world().resource::<PropertySyncCounters>();
    assert_eq!((counts.reads, counts.writes), (4, 0));
    node.free();
}

#[derive(Clone, Default, PartialEq)]
struct CustomValue(i32);

fn from_custom(value: i32) -> CustomValue {
    CustomValue(value)
}

fn to_custom(value: CustomValue) -> i32 {
    value.0
}

#[derive(Component, Default, GodotNode)]
#[component(immutable)]
#[gdbevy(class_name = PropertyImmutableNode)]
struct ImmutableProperty {
    #[gdbevy(export, as = i32, default = 5, sync = to_godot, with = from_custom, into = to_custom)]
    value: CustomValue,
}

#[itest]
fn property_sync_immutable_component_custom_value() {
    let mut app = test_app();
    let mut node = PropertyImmutableNode::new_alloc();
    let entity = decorate(&mut app, node.clone().upcast(), "PropertyImmutableNode");
    cycle(&mut app);
    node.set("value", &99.to_variant());
    cycle(&mut app);
    assert_eq!(
        app.world()
            .get::<ImmutableProperty>(entity)
            .unwrap()
            .value
            .0,
        5
    );
    app.world_mut()
        .entity_mut(entity)
        .insert(ImmutableProperty {
            value: CustomValue(9),
        });
    cycle(&mut app);
    assert_eq!(node.get("value").to::<i32>(), 9);
    node.free();
}

#[derive(Component, Default)]
struct AliasFields {
    a: i32,
    b: i32,
}

type Alias = AliasFields;

#[derive(GodotClass, BevyComponents)]
#[class(init, base = Node)]
#[gdbevy(require(AliasFields { a: left }), require(Alias { b: right }))]
struct PropertyAliasNode {
    base: Base<Node>,
    #[export]
    #[init(val = 11)]
    #[gdbevy(sync = two_way)]
    left: i32,
    #[export]
    #[init(val = 22)]
    right: i32,
}

#[itest]
fn property_sync_rejects_aliased_component_construction() {
    let mut app = App::new();
    app.insert_non_send(GodotMainThread);
    let node = PropertyAliasNode::new_alloc();
    let handle = GodotNodeHandle::new(node.clone());
    let entity = app.world_mut().spawn(handle).id();
    let entry = godot_bevy::inventory::iter::<AutoSyncBundleRegistry>
        .into_iter()
        .find(|entry| entry.godot_class_name == "PropertyAliasNode")
        .unwrap();
    entry.register_property_sync.unwrap()(&mut app);
    let created = app
        .world_mut()
        .run_system_once(move |mut commands: Commands, mut godot: GodotAccess| {
            (entry.create_bundle_fn)(&mut commands, entity, &mut godot, handle)
        })
        .unwrap();
    assert!(!created);
    assert_eq!(
        app.world()
            .get::<AliasFields>(entity)
            .map(|fields| (fields.a, fields.b)),
        None
    );
    assert_eq!(app.world().entity(entity).archetype().components().len(), 1);

    let valid_entry = godot_bevy::inventory::iter::<AutoSyncBundleRegistry>
        .into_iter()
        .find(|entry| entry.godot_class_name == "PropertySyncNode")
        .unwrap();
    valid_entry.register_property_sync.unwrap()(&mut app);
    let mut valid_node = PropertySyncNode::new_alloc();
    let valid_entity = decorate(&mut app, valid_node.clone().upcast(), "PropertySyncNode");
    valid_node.set("health", &37.to_variant());
    cycle(&mut app);
    assert_eq!(
        app.world()
            .get::<SyncedProperties>(valid_entity)
            .unwrap()
            .health,
        37
    );
    #[cfg(feature = "test-frame-signal")]
    {
        let counts = app.world().resource::<PropertySyncCounters>();
        assert_eq!((counts.read_passes, counts.write_passes), (1, 1));
    }
    node.free();
    valid_node.free();
}
