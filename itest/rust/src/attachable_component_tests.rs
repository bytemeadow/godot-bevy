use bevy::ecs::system::SystemState;
use bevy::prelude::{Assets, Component, First, Messages};
use godot::classes::node::InternalMode;
use godot::prelude::*;
use godot_bevy::plugins::scene_tree::{
    SceneTreeMessage, SceneTreeMessageReader, SceneTreeMessageType,
};
use godot_bevy::prelude::*;
use godot_bevy_test::prelude::*;
use std::cell::RefCell;

#[derive(bevy::prelude::Component, PartialEq, Debug, Default)]
pub struct TestMovement {
    pub max_speed: f32,
}

#[derive(AttachableComponent, GodotClass)]
#[class(init, base=Node)]
#[gdbevy(target = TestMovement)]
pub struct TestMovementComponent {
    #[export]
    pub max_speed: f32,
}

impl From<&TestMovementComponent> for TestMovement {
    fn from(value: &TestMovementComponent) -> TestMovement {
        TestMovement {
            max_speed: value.max_speed,
        }
    }
}

#[itest(async)]
fn test_attachable_component_attaches_to_parent(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let mut parent_node = Node::new_alloc();
        parent_node.set_name("AttachableParent");
        ctx_clone.scene_tree.clone().add_child(&parent_node);

        let mut child_node = TestMovementComponent::new_alloc();
        child_node.set_name("AttachableChild");

        let child_instance_id = child_node.instance_id();

        child_node.bind_mut().max_speed = 42.0;

        parent_node.clone().add_child(&child_node);

        app.updates(3).await;

        let parent_entity = app
            .entity_for_node(parent_node.instance_id())
            .expect("Parent entity should exist");

        let child_entity_exists = app.has_entity_for_node(child_instance_id);

        app.with_world(|world| {
            let movement = world.get::<TestMovement>(parent_entity);
            assert!(
                movement.is_some(),
                "attachable component should be attached to parent entity"
            );
            assert_eq!(
                movement.unwrap().max_speed,
                42.0,
                "component data should be mapped correctly from the Godot node"
            );
        });

        assert!(
            !child_entity_exists,
            "attachable child node should NOT get its own Bevy entity (it should be freed)"
        );

        app.cleanup().await;
        parent_node.free();
    })
}

#[itest(async)]
fn test_attachable_component_skips_unregistered(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let mut parent_node = Node::new_alloc();
        parent_node.set_name("MissParent");
        ctx_clone.scene_tree.clone().add_child(&parent_node);

        let mut child_node = Node::new_alloc();
        child_node.set_name("MissChild");

        let child_instance_id = child_node.instance_id();

        parent_node.clone().add_child(&child_node);

        app.updates(3).await;

        let parent_entity = app
            .entity_for_node(parent_node.instance_id())
            .expect("Parent entity should exist");

        app.with_world(|world| {
            assert!(
                world.get::<TestMovement>(parent_entity).is_none(),
                "unregistered type must not receive attachable component"
            );
        });

        let child_entity_exists = app.has_entity_for_node(child_instance_id);
        assert!(
            child_entity_exists,
            "non-attachable child node SHOULD get its own Bevy entity"
        );

        app.cleanup().await;
        child_node.free();
        parent_node.free();
    })
}

thread_local! {
    static CONVERSIONS: RefCell<Vec<i64>> = const { RefCell::new(Vec::new()) };
}

#[derive(Component)]
struct AttachValue {
    value: i64,
    captured: Option<GodotNodeHandle>,
}

#[derive(AttachableComponent, GodotClass)]
#[class(init, base=Node)]
#[gdbevy(target = AttachValue)]
struct AttachCarrier {
    #[export]
    value: i64,
    #[export]
    captured: Option<Gd<Node>>,
}

impl From<&AttachCarrier> for AttachValue {
    fn from(carrier: &AttachCarrier) -> Self {
        CONVERSIONS.with_borrow_mut(|calls| calls.push(carrier.value));
        Self {
            value: carrier.value,
            captured: carrier.captured.clone().map(GodotNodeHandle::new),
        }
    }
}

fn carrier(name: &str, value: i64) -> Gd<AttachCarrier> {
    CONVERSIONS.with_borrow_mut(|calls| calls.retain(|call| *call != value));
    let mut node = AttachCarrier::new_alloc();
    node.set_name(name);
    node.bind_mut().value = value;
    node
}

fn conversions(value: i64) -> usize {
    CONVERSIONS.with_borrow(|calls| calls.iter().filter(|call| **call == value).count())
}

fn node(name: &str) -> Gd<Node> {
    let mut node = Node::new_alloc();
    node.set_name(name);
    node
}

fn take_messages(app: &mut TestApp) -> Vec<SceneTreeMessage> {
    app.with_world(|world| {
        world
            .resource::<SceneTreeMessageReader>()
            .0
            .lock()
            .try_iter()
            .collect()
    })
}

fn drain(app: &mut TestApp, messages: Vec<SceneTreeMessage>) {
    app.with_world_mut(|world| {
        world
            .resource_mut::<Messages<SceneTreeMessage>>()
            .write_batch(messages);
        world.run_schedule(First);
        world.run_schedule(First);
    });
}

fn pump(app: &mut TestApp) {
    let messages = take_messages(app);
    drain(app, messages);
}

fn value(app: &TestApp, node_id: InstanceId) -> Option<i64> {
    let entity = app.entity_for_node(node_id)?;
    app.with_world(|world| world.get::<AttachValue>(entity).map(|value| value.value))
}

fn assert_unmirrored(app: &mut TestApp, node_id: InstanceId) {
    assert!(
        !app.has_entity_for_node(node_id),
        "carrier must not enter NodeEntityIndex"
    );
    app.with_world_mut(|world| {
        assert!(
            world
                .query::<&GodotNodeHandle>()
                .iter(world)
                .all(|handle| handle.instance_id() != node_id),
            "carrier must not have an entity"
        );
    });
}

#[itest]
async fn test_attach_consumes_leaf(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |_| {}).await;
    let (mut parent, _) = app.add_node::<Node>("LeafParent").await;
    let child = carrier("LeafCarrier", 101);
    let child_id = child.instance_id();
    parent.add_child(&child);
    pump(&mut app);
    assert_eq!(value(&app, parent.instance_id()), Some(101));
    assert_eq!(conversions(101), 1);
    assert_unmirrored(&mut app, child_id);
    app.updates(3).await;
    assert!(!child_id.lookup_validity());
    assert!(!parent.has_node("LeafCarrier"));
    app.cleanup().await;
}

async fn rejects_children(ctx: TestContext, internal: bool, capture: bool) {
    let mut app = TestApp::new(&ctx, |_| {}).await;
    let mut parent = node("ChildrenParent");
    let mut child = carrier("NonleafCarrier", 201);
    let leaf = node("AuthoredChild");
    if internal {
        child
            .add_child_ex(&leaf)
            .internal(InternalMode::FRONT)
            .done();
        assert_eq!(child.get_child_count(), 0);
    } else {
        child.add_child(&leaf);
    }
    if capture {
        child.bind_mut().captured = Some(leaf.clone());
    }
    let (child_id, leaf_id) = (child.instance_id(), leaf.instance_id());
    parent.add_child(&child);
    ctx.scene_tree.clone().add_child(&parent);
    pump(&mut app);
    assert_eq!(
        conversions(201),
        0,
        "nonleaf carriers must be rejected before From"
    );
    assert_eq!(value(&app, parent.instance_id()), None);
    assert_unmirrored(&mut app, child_id);
    app.updates(3).await;
    assert!(child_id.lookup_validity() && leaf_id.lookup_validity());
    assert_eq!(leaf.get_parent().unwrap().instance_id(), child_id);
    assert!(app.has_entity_for_node(leaf_id));
    app.cleanup().await;
}

#[itest]
async fn test_attach_rejects_children(ctx: TestContext) {
    rejects_children(ctx, false, false).await;
}

#[itest]
async fn test_attach_rejects_children_internal(ctx: TestContext) {
    rejects_children(ctx, true, false).await;
}

#[itest]
async fn test_attach_rejects_descendant_capture(ctx: TestContext) {
    rejects_children(ctx, false, true).await;
}

#[itest]
async fn test_attach_rejects_nested_carriers(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |_| {}).await;
    let mut parent = node("NestedParent");
    let mut outer = carrier("OuterCarrier", 301);
    let inner = carrier("InnerCarrier", 302);
    let (outer_id, inner_id) = (outer.instance_id(), inner.instance_id());
    outer.add_child(&inner);
    parent.add_child(&outer);
    ctx.scene_tree.clone().add_child(&parent);
    pump(&mut app);
    assert_eq!(conversions(301), 0, "outer carrier must preserve its child");
    assert_eq!(conversions(302), 0, "carrier parent is not a destination");
    assert_eq!(value(&app, parent.instance_id()), None);
    assert_unmirrored(&mut app, outer_id);
    assert_unmirrored(&mut app, inner_id);
    app.updates(3).await;
    assert!(outer_id.lookup_validity() && inner_id.lookup_validity());
    assert_eq!(inner.get_parent().unwrap().instance_id(), outer_id);
    app.cleanup().await;
}

async fn reparent_once(ctx: TestContext, metadata: bool) {
    let mut app = TestApp::new(&ctx, |_| {}).await;
    let (mut first, _) = app.add_node::<Node>("FirstParent").await;
    let (second, _) = app.add_node::<Node>("LiveParent").await;
    let mut child = carrier("ReparentCarrier", 401);
    let child_id = child.instance_id();
    first.add_child(&child);
    child.reparent(&second);
    let mut messages = take_messages(&mut app);
    assert_eq!(
        messages
            .iter()
            .filter(|m| m.node_id.instance_id() == child_id
                && matches!(m.message_type, SceneTreeMessageType::NodeAdded))
            .count(),
        2
    );
    if !metadata {
        for message in &mut messages {
            message.node_type = None;
            message.node_name = None;
            message.parent_id = None;
            message.collision_mask = None;
            message.groups = None;
        }
    }
    let duplicate = messages
        .iter()
        .find(|m| {
            m.node_id.instance_id() == child_id
                && matches!(m.message_type, SceneTreeMessageType::NodeAdded)
        })
        .unwrap()
        .clone();
    messages.push(duplicate.clone());
    drain(&mut app, messages);
    assert_eq!(conversions(401), 1, "duplicate adds must convert once");
    assert_eq!(value(&app, first.instance_id()), None);
    assert_eq!(value(&app, second.instance_id()), Some(401));
    assert!(child.is_queued_for_deletion());
    assert_unmirrored(&mut app, child_id);
    drain(&mut app, vec![duplicate]);
    assert_eq!(
        conversions(401),
        1,
        "queued carriers must not convert in another drain"
    );
    app.updates(3).await;
    assert!(!child_id.lookup_validity());
    app.cleanup().await;
}

#[itest]
async fn test_attach_reparent_once(ctx: TestContext) {
    reparent_once(ctx, true).await;
}

#[itest]
async fn test_attach_reparent_once_fallback(ctx: TestContext) {
    reparent_once(ctx, false).await;
}

#[itest]
async fn test_attach_same_batch_parent(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |_| {}).await;
    let mut parent = node("SameBatchParent");
    let child = carrier("ChildFirstCarrier", 501);
    let (parent_id, child_id) = (parent.instance_id(), child.instance_id());
    parent.add_child(&child);
    ctx.scene_tree.clone().add_child(&parent);
    let mut messages = take_messages(&mut app);
    messages.sort_by_key(|message| message.node_id.instance_id() != child_id);
    drain(&mut app, messages);
    assert_eq!(
        value(&app, parent_id),
        Some(501),
        "same-batch parent must be indexed before attachment"
    );
    assert_eq!(conversions(501), 1);
    assert_unmirrored(&mut app, child_id);
    app.updates(3).await;
    assert!(!child_id.lookup_validity());
    app.cleanup().await;
}

#[itest]
async fn test_attach_rejects_unindexed_parent(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |_| {}).await;
    let mut parent = node("UnindexedParent");
    let child = carrier("UnindexedCarrier", 502);
    let (parent_id, child_id) = (parent.instance_id(), child.instance_id());
    parent.add_child(&child);
    ctx.scene_tree.clone().add_child(&parent);
    let messages = take_messages(&mut app)
        .into_iter()
        .filter(|message| message.node_id.instance_id() != parent_id)
        .collect();
    drain(&mut app, messages);
    assert_eq!(conversions(502), 0);
    assert_unmirrored(&mut app, child_id);
    app.updates(3).await;
    assert!(child_id.lookup_validity());
    assert!(!child.is_queued_for_deletion());
    app.cleanup().await;
}

#[itest]
async fn test_attach_rejects_viewport_parent(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |_| {}).await;
    let child = carrier("ViewportCarrier", 601);
    let child_id = child.instance_id();
    let mut root = ctx.scene_tree.get_tree().get_root().unwrap();
    root.add_child(&child);
    pump(&mut app);
    let indexed = app.has_entity_for_node(child_id);
    let alive = child_id.lookup_validity() && !child.is_queued_for_deletion();
    root.remove_child(&child);
    ctx.scene_tree.clone().add_child(&child);
    assert_eq!(conversions(601), 0);
    assert!(!indexed, "viewport carrier must remain unmirrored");
    assert!(alive);
    app.cleanup().await;
}

#[itest]
async fn test_attach_reparent_into_excluded(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |_| {}).await;
    let (mut first, _) = app.add_node::<Node>("OldParent").await;
    let mut excluded = node("ExcludedDestination");
    excluded.set_meta("_bevy_exclude", &true.to_variant());
    ctx.scene_tree.clone().add_child(&excluded);
    let mut child = carrier("ExcludedReparentCarrier", 701);
    let child_id = child.instance_id();
    first.add_child(&child);
    child.reparent(&excluded);
    pump(&mut app);
    assert_eq!(
        conversions(701),
        0,
        "stale adds must respect live exclusion"
    );
    assert_eq!(value(&app, first.instance_id()), None);
    assert_unmirrored(&mut app, child_id);
    app.updates(3).await;
    assert!(child_id.lookup_validity());
    assert!(!app.has_entity_for_node(excluded.instance_id()));
    app.cleanup().await;
}

async fn cancels_pending(ctx: TestContext, mode: &str) {
    let mut app = TestApp::new(&ctx, |_| {}).await;
    let (mut parent, _) = app.add_node::<Node>("CancelledParent").await;
    let mut child = carrier("CancelledCarrier", 801);
    let child_id = child.instance_id();
    parent.add_child(&child);
    match mode {
        "detach" => parent.remove_child(&child),
        "queued" => child.queue_free(),
        "freed" => child.clone().free(),
        "parent_queued" => parent.queue_free(),
        _ => unreachable!(),
    }
    pump(&mut app);
    let calls = conversions(801);
    let attached = value(&app, parent.instance_id());
    let indexed = app.has_entity_for_node(child_id);
    if mode == "detach" {
        assert!(child_id.lookup_validity());
        child.free();
    }
    assert_eq!(calls, 0, "detached or dying carriers must not convert");
    assert_eq!(attached, None);
    assert!(!indexed);
    app.updates(3).await;
    assert!(!child_id.lookup_validity());
    assert_unmirrored(&mut app, child_id);
    app.cleanup().await;
}

#[itest]
async fn test_attach_cancels_pending(ctx: TestContext) {
    cancels_pending(ctx, "detach").await;
}

#[itest]
async fn test_attach_cancels_pending_queued(ctx: TestContext) {
    cancels_pending(ctx, "queued").await;
}

#[itest]
async fn test_attach_cancels_pending_freed(ctx: TestContext) {
    cancels_pending(ctx, "freed").await;
}

#[itest]
async fn test_attach_cancels_pending_parent_queued(ctx: TestContext) {
    cancels_pending(ctx, "parent_queued").await;
}

#[itest]
async fn test_attach_excluded_parent(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |_| {}).await;
    let mut parent = node("ExcludedParent");
    parent.set_meta("_bevy_exclude", &true.to_variant());
    let child = carrier("ExcludedCarrier", 901);
    let child_id = child.instance_id();
    parent.add_child(&child);
    ctx.scene_tree.clone().add_child(&parent);
    app.updates(3).await;
    assert_eq!(conversions(901), 0);
    assert!(child_id.lookup_validity());
    assert!(!app.has_entity_for_node(parent.instance_id()));
    assert_unmirrored(&mut app, child_id);
    app.cleanup().await;
}

#[itest]
async fn test_attach_current_scene_parent(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |_| {}).await;
    let scene = ctx.scene_tree.get_tree().get_current_scene().unwrap();
    assert_eq!(scene.instance_id(), ctx.scene_tree.instance_id());
    let child = carrier("CurrentSceneCarrier", 902);
    let child_id = child.instance_id();
    ctx.scene_tree.clone().add_child(&child);
    app.updates(3).await;
    assert_eq!(value(&app, scene.instance_id()), Some(902));
    assert_eq!(conversions(902), 1);
    assert!(!child_id.lookup_validity());
    assert_unmirrored(&mut app, child_id);
    app.cleanup().await;
}

#[itest]
async fn test_attach_initial_scan(ctx: TestContext) {
    let mut parent = node("StartupParent");
    let child = carrier("StartupCarrier", 1001);
    let (parent_id, child_id) = (parent.instance_id(), child.instance_id());
    parent.add_child(&child);
    ctx.scene_tree.clone().add_child(&parent);
    let mut app = TestApp::new(&ctx, |_| {}).await;
    app.updates(3).await;
    assert_eq!(value(&app, parent_id), Some(1001));
    assert_eq!(conversions(1001), 1);
    assert!(!child_id.lookup_validity());
    assert_unmirrored(&mut app, child_id);
    app.cleanup().await;
}

#[itest]
async fn test_attach_packed_scene(ctx: TestContext) {
    CONVERSIONS.with_borrow_mut(|calls| calls.retain(|call| *call != 1101));
    let mut app = TestApp::new(&ctx, |app| {
        app.add_plugins(GodotPackedScenePlugin);
        app.init_resource::<Assets<GodotResource>>();
    })
    .await;
    for count in 1..=2 {
        let entity = app.with_world_mut(|world| {
            world
                .spawn(
                    GodotScene::from_path("res://test_attach_scene.tscn")
                        .with_parent(GodotNodeHandle::new(ctx.scene_tree.clone())),
                )
                .id()
        });
        app.updates(5).await;
        let handle = app.with_world(|world| *world.get::<GodotNodeHandle>(entity).unwrap());
        let root = Gd::<Node>::from_instance_id(handle.instance_id());
        assert_eq!(app.entity_for_node(handle.instance_id()), Some(entity));
        assert_eq!(value(&app, handle.instance_id()), Some(1101));
        assert_eq!(conversions(1101), count);
        assert!(!root.has_node("PackedCarrier"));
        assert_eq!(root.get_child_count(), 0);
        root.free();
        app.updates(3).await;
        assert!(!app.has_entity_for_node(handle.instance_id()));
    }
    app.cleanup().await;
}

#[itest]
async fn test_attach_scene_reentry(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |_| {}).await;
    let mut parent = node("ReusableScene");
    let child = carrier("ConsumedCarrier", 1201);
    let (parent_id, child_id) = (parent.instance_id(), child.instance_id());
    parent.add_child(&child);
    ctx.scene_tree.clone().add_child(&parent);
    app.updates(3).await;
    let original_entity = app.entity_for_node(parent_id).unwrap();
    assert_eq!(value(&app, parent_id), Some(1201));
    assert_eq!(conversions(1201), 1);
    assert!(!child_id.lookup_validity());
    ctx.scene_tree.clone().remove_child(&parent);
    app.updates(3).await;
    let detached_entity = app.entity_for_node(parent_id);
    ctx.scene_tree.clone().add_child(&parent);
    app.updates(3).await;
    assert_eq!(detached_entity, None);
    assert_ne!(app.entity_for_node(parent_id).unwrap(), original_entity);
    assert_eq!(value(&app, parent_id), None);
    assert_eq!(conversions(1201), 1);
    assert!(!parent.has_node("ConsumedCarrier"));
    app.cleanup().await;
}

#[itest]
async fn test_attach_external_handle(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |_| {}).await;
    let mut parent = node("ExternalParent");
    let sibling = node("SurvivingSibling");
    let mut child = carrier("ExternalCarrier", 1301);
    child.bind_mut().captured = Some(sibling.clone());
    let (parent_id, sibling_id, child_id) = (
        parent.instance_id(),
        sibling.instance_id(),
        child.instance_id(),
    );
    parent.add_child(&sibling);
    parent.add_child(&child);
    ctx.scene_tree.clone().add_child(&parent);
    app.updates(3).await;
    let entity = app.entity_for_node(parent_id).unwrap();
    let captured =
        app.with_world(|world| world.get::<AttachValue>(entity).unwrap().captured.unwrap());
    assert_eq!(captured.instance_id(), sibling_id);
    assert_eq!(conversions(1301), 1);
    assert!(!child_id.lookup_validity());
    let resolves = app.with_world_mut(|world| {
        SystemState::<GodotAccess>::new(world)
            .get_mut(world)
            .unwrap()
            .try_get::<Node>(captured)
            .is_some()
    });
    assert!(resolves);
    sibling.free();
    let resolves = app.with_world_mut(|world| {
        SystemState::<GodotAccess>::new(world)
            .get_mut(world)
            .unwrap()
            .try_get::<Node>(captured)
            .is_some()
    });
    assert!(!resolves);
    app.cleanup().await;
}
