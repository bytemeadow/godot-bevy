use bevy::prelude::*;
use godot::classes::{Area2D, Node};
use godot::obj::Gd;
use godot_bevy::prelude::{
    GodotActionsPlugin, GodotCollisionsPlugin, GodotNodeHandle, GodotPackedScenePlugin,
    GodotResource, GodotScene,
};
use godot_bevy_test::prelude::*;

use crate::components::{Door, Gem, JumpBoost, Player};
use crate::gameplay::door::DoorPlugin;
use crate::gameplay::gem::{GemPlugin, GemsCollected};
use crate::gameplay::player::{PlayerInputMessage, PlayerMovementMessage, PlayerPlugin};

#[itest]
async fn gem_plugin_starts_empty(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |app| {
        app.add_plugins(GemPlugin);
    })
    .await;

    assert_eq!(
        app.with_world(|world| world.resource::<GemsCollected>().0),
        0
    );

    app.cleanup().await;
}

// ANCHOR: gem_collected
#[itest]
async fn gem_collected(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |app| {
        app.add_plugins(GemPlugin);
    })
    .await;

    let (_, gem_entity) = app.add_node::<Area2D>("gem").await;
    let (_, player_entity) = app.add_node::<Area2D>("player").await;
    app.with_world_mut(|world| {
        world.entity_mut(gem_entity).insert(Gem);
        world.entity_mut(player_entity).insert(Player);
        world.trigger(godot_bevy::prelude::CollisionStarted {
            entity1: gem_entity,
            entity2: player_entity,
        });
    });

    app.update().await;

    app.with_world(|world| {
        assert_eq!(world.resource::<GemsCollected>().0, 1);
        assert!(!world.entities().contains(gem_entity));
    });

    app.cleanup().await;
}
// ANCHOR_END: gem_collected

#[itest]
async fn jump_boost_attaches_to_scene_root(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |app| {
        app.add_plugins(GodotPackedScenePlugin);
        app.init_resource::<Assets<GodotResource>>();
    })
    .await;

    let entity_count = app.with_world(|world| world.entities().count_spawned());
    let entity = app.with_world_mut(|world| {
        world
            .spawn(
                GodotScene::from_path("res://scenes/attachable_probe.tscn")
                    .with_parent(GodotNodeHandle::new(ctx.scene_tree.clone())),
            )
            .id()
    });

    app.updates(5).await;

    let handle = app.with_world(|world| {
        assert_eq!(
            world.get::<JumpBoost>(entity),
            Some(&JumpBoost { multiplier: 2.5 })
        );
        assert_eq!(world.entities().count_spawned(), entity_count + 1);
        *world.get::<GodotNodeHandle>(entity).unwrap()
    });
    let root = Gd::<Node>::from_instance_id(handle.instance_id());
    assert_eq!(app.entity_for_node(root.instance_id()), Some(entity));
    assert!(!root.has_node("JumpBoost"));

    app.cleanup().await;
    root.free();
}

#[itest]
async fn player_plugin_registers_messages(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |app| {
        app.add_plugins((GodotActionsPlugin, PlayerPlugin));
    })
    .await;

    app.with_world(|world| {
        assert!(world.contains_resource::<Messages<PlayerInputMessage>>());
        assert!(world.contains_resource::<Messages<PlayerMovementMessage>>());
    });

    app.cleanup().await;
}

// ANCHOR: with_world_mut_query
#[itest]
async fn game_components_keep_their_defaults(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |app| {
        app.add_plugins((GodotCollisionsPlugin, DoorPlugin));
    })
    .await;

    app.with_world_mut(|world| {
        world.spawn((Gem, Door::default(), Player));

        let mut doors = world.query::<&Door>();
        assert_eq!(doors.iter(world).count(), 1);
    });

    app.update().await;
    app.cleanup().await;
}
// ANCHOR_END: with_world_mut_query

#[itest]
async fn whole_app_boots(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, crate::build_app).await;

    app.with_world(|world| {
        assert!(world.contains_resource::<State<crate::GameState>>());
        assert_eq!(world.resource::<GemsCollected>().0, 0);
    });

    app.cleanup().await;
}
