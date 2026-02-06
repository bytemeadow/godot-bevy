/*
 * Scene tree watcher initialization tests
 *
 * Tests that verify correct initialization of OptimizedSceneTreeWatcher
 * and SceneTreeWatcher during BevyApp::ready():
 * - Only one OptimizedSceneTreeWatcher should exist (no duplicates)
 * - OptimizedSceneTreeWatcher.rust_watcher should be connected (not null)
 */

use godot::prelude::*;
use godot_bevy_test::prelude::*;

/// Find the BevyApp node that TestApp created as a child of ctx.scene_tree.
/// Returns the first child that is a BevyApp.
fn find_bevy_app_node(
    scene_tree_node: &Gd<godot::classes::Node>,
) -> Option<Gd<godot::classes::Node>> {
    for i in 0..scene_tree_node.get_child_count() {
        if let Some(child) = scene_tree_node.get_child(i)
            && child.get_class() == GString::from("BevyApp")
        {
            return Some(child.clone());
        }
    }
    None
}

/// Count children whose name starts with the given prefix.
fn count_children_with_prefix(parent: &Gd<godot::classes::Node>, prefix: &str) -> usize {
    let mut count = 0;
    for i in 0..parent.get_child_count() {
        if let Some(child) = parent.get_child(i)
            && child.get_name().to_string().starts_with(prefix)
        {
            count += 1;
        }
    }
    count
}

/// Test that only one OptimizedSceneTreeWatcher exists after initialization.
#[itest(async)]
fn test_single_optimized_scene_tree_watcher(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let bevy_app_node = find_bevy_app_node(&ctx_clone.scene_tree)
            .expect("BevyApp node should exist as child of scene_tree");

        let watcher_count = count_children_with_prefix(&bevy_app_node, "OptimizedSceneTreeWatcher");

        assert_eq!(
            watcher_count, 1,
            "Expected exactly 1 OptimizedSceneTreeWatcher, found {watcher_count}. \
             Duplicate registration detected."
        );

        println!("✓ Single OptimizedSceneTreeWatcher: no duplicates");

        app.cleanup();
        await_frames(1).await;
    })
}

/// Test that OptimizedSceneTreeWatcher's rust_watcher variable is connected
/// to the SceneTreeWatcher after initialization.
#[itest(async)]
fn test_optimized_watcher_rust_watcher_connected(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let bevy_app_node = find_bevy_app_node(&ctx_clone.scene_tree)
            .expect("BevyApp node should exist as child of scene_tree");

        let watcher = bevy_app_node
            .get_node_or_null("OptimizedSceneTreeWatcher")
            .expect("OptimizedSceneTreeWatcher should exist");

        let rust_watcher_value = watcher.get("rust_watcher");
        let is_connected = rust_watcher_value.get_type() != VariantType::NIL;

        assert!(
            is_connected,
            "OptimizedSceneTreeWatcher.rust_watcher should be connected to SceneTreeWatcher, \
             but it is null. This means the optimized watcher was registered before \
             SceneTreeWatcher existed."
        );

        println!("✓ OptimizedSceneTreeWatcher.rust_watcher is connected");

        app.cleanup();
        await_frames(1).await;
    })
}
