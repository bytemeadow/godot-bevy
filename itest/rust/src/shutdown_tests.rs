use bevy::prelude::*;
use godot::obj::NewAlloc;
use godot::prelude::*;
use godot_bevy::BevyApp;
use godot_bevy_test::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Clone)]
struct ExitCount(Arc<AtomicUsize>);

/// A live BevyApp dispatches AppExit and runs Last exactly once when Godot
/// removes it from the scene tree.
#[itest(async)]
fn test_bevy_app_exit_tree_runs_last_once(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let count = Arc::new(AtomicUsize::new(0));
        let count_for_app = Arc::clone(&count);

        let mut app = TestApp::new(&ctx_clone, |_| {}).await;

        let mut node = BevyApp::new_alloc();
        node.set_name("BevyAppShutdownProbe");
        node.bind_mut()
            .set_instance_init_func(Box::new(move |bevy_app: &mut App| {
                bevy_app.insert_resource(ExitCount(Arc::clone(&count_for_app)));
                bevy_app.add_systems(
                    Last,
                    |mut exits: MessageReader<AppExit>, count: Res<ExitCount>| {
                        for _ in exits.read() {
                            count.0.fetch_add(1, Ordering::SeqCst);
                        }
                    },
                );
            }));

        ctx_clone
            .scene_tree
            .clone()
            .add_child(&node.clone().upcast::<Node>());
        node.bind_mut().initialize();

        // Let initialization settle before removing the node.
        app.update().await;
        node.clone().upcast::<Node>().free();

        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "exit_tree should dispatch AppExit and run Last exactly once"
        );

        app.cleanup().await;
    })
}
