/*
 * Attachable component integration tests
 */

use godot::prelude::*;
use godot_bevy::prelude::*;
use godot_bevy_test::prelude::*;

// A target Bevy component we want to attach
#[derive(bevy::prelude::Component, PartialEq, Debug, Default)]
pub struct TestMovement {
    pub max_speed: f32,
}

// A Godot class that implements AttachableComponent to map to TestMovement
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

/// A scene-spawned node of an attachable type gets its component attached to the parent.
#[itest(async)]
fn test_attachable_component_attaches_to_parent(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        // 1. Create a parent node first
        let mut parent_node = Node::new_alloc();
        parent_node.set_name("AttachableParent");
        ctx_clone.scene_tree.clone().add_child(&parent_node);

        // 2. Create the attachable child
        let mut child_node = TestMovementComponent::new_alloc();
        child_node.set_name("AttachableChild");

        // Capture the ID *before* it might be queued for free by the plugin!
        let child_instance_id = child_node.instance_id();

        // Mutate the export field to prove data mapping works
        child_node.bind_mut().max_speed = 42.0;

        parent_node.clone().add_child(&child_node);

        // Wait for entities to be created and the attachable logic to run
        app.updates(3).await;

        let parent_entity = app
            .entity_for_node(parent_node.instance_id())
            .expect("Parent entity should exist");

        // Check if the child entity was created (it shouldn't be, because it gets attached and freed)
        let child_entity_exists = app.has_entity_for_node(child_instance_id);

        // 3. Assertions
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
        // child_node is queue_freed by the plugin, but parent_node needs to be freed
        parent_node.free();
    })
}

/// A scene-spawned node of a non-attachable type does not get the component attached to the parent.
#[itest(async)]
fn test_attachable_component_skips_unregistered(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let mut parent_node = Node::new_alloc();
        parent_node.set_name("MissParent");
        ctx_clone.scene_tree.clone().add_child(&parent_node);

        let mut child_node = Node::new_alloc(); // Regular node, not attachable
        child_node.set_name("MissChild");

        // Capture ID before adding to tree
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
