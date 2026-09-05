#![cfg(feature = "test-frame-signal")]

use bevy::prelude::*;
use godot::prelude::*;
use godot_bevy::{BevyApp, interop::GodotAccess};
use godot_bevy_test::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[derive(Resource, Clone)]
struct ExitCount(Arc<AtomicUsize>);

#[derive(Default)]
struct Counts {
    inits: AtomicUsize,
    startups: AtomicUsize,
    terminal: AtomicUsize,
    last: AtomicUsize,
    exits: Arc<AtomicUsize>,
    drops: AtomicUsize,
}

#[derive(Resource)]
struct DropSentinel(Arc<Counts>);

impl Drop for DropSentinel {
    fn drop(&mut self) {
        self.0.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Resource)]
struct StartupOnly;

#[derive(Component)]
struct Unrelated(u32);

fn count_app(app: &mut App, counts: &Arc<Counts>) {
    counts.inits.fetch_add(1, Ordering::SeqCst);
    app.insert_resource(DropSentinel(counts.clone()));
    app.insert_resource(ExitCount(counts.exits.clone()));
    let started = counts.clone();
    app.add_systems(Startup, move |mut commands: Commands| {
        started.startups.fetch_add(1, Ordering::SeqCst);
        commands.insert_resource(StartupOnly);
    });
    let last = counts.clone();
    app.add_systems(
        Last,
        move |mut exits: MessageReader<AppExit>, count: Res<ExitCount>, _: Res<StartupOnly>| {
            last.last.fetch_add(1, Ordering::SeqCst);
            for exit in exits.read() {
                count.0.fetch_add(1, Ordering::SeqCst);
                if exit.is_success() {
                    last.terminal.fetch_add(1, Ordering::SeqCst);
                }
            }
        },
    );
}

struct LocalApp {
    node: Gd<BevyApp>,
    counts: Arc<Counts>,
}

impl LocalApp {
    fn new(parent: &mut Gd<Node>, setup: impl Fn(&mut App) + Send + Sync + 'static) -> Self {
        let mut node = BevyApp::new_alloc();
        node.set_meta("_bevy_exclude", &true.to_variant());
        let counts = Arc::new(Counts::default());
        let observed = counts.clone();
        node.bind_mut().set_instance_init_func(Box::new(move |app| {
            count_app(app, &observed);
            setup(app);
        }));
        parent.add_child(&node);
        assert_eq!(counts.inits.load(Ordering::SeqCst), 1);
        Self { node, counts }
    }

    fn assert_counts(&self, inits: usize, startups: usize, terminal: usize, drops: usize) {
        assert_eq!(
            self.counts.inits.load(Ordering::SeqCst),
            inits,
            "initializations"
        );
        assert_eq!(
            self.counts.startups.load(Ordering::SeqCst),
            startups,
            "Startup runs"
        );
        assert_eq!(
            self.counts.terminal.load(Ordering::SeqCst),
            terminal,
            "terminal passes"
        );
        assert_eq!(
            self.counts.drops.load(Ordering::SeqCst),
            drops,
            "resource drops"
        );
    }
}

impl Drop for LocalApp {
    fn drop(&mut self) {
        if self.node.is_instance_valid() {
            self.node.clone().free();
        }
    }
}

#[itest]
async fn shutdown_on_free_after_detach(ctx: TestContext) {
    let mut clock = TestApp::new(&ctx, |_| {}).await;
    for detached in [false, true] {
        let probe = LocalApp::new(&mut ctx.scene_tree.clone(), |_| {});
        clock.updates(3).await;
        let last = probe.counts.last.load(Ordering::SeqCst);
        if detached {
            ctx.scene_tree.clone().remove_child(&probe.node);
            clock.updates(3).await;
            probe.assert_counts(1, 1, 0, 0);
            assert_eq!(probe.counts.last.load(Ordering::SeqCst), last);
        }
        probe.node.clone().free();
        probe.assert_counts(1, 1, 1, 1);
        assert_eq!(probe.counts.exits.load(Ordering::SeqCst), 1);
        assert_eq!(probe.counts.last.load(Ordering::SeqCst), last + 1);
    }
    clock.cleanup().await;
}

#[itest]
async fn shutdown_on_queue_free_after_detach(ctx: TestContext) {
    let mut clock = TestApp::new(&ctx, |_| {}).await;
    for detached in [false, true] {
        let mut probe = LocalApp::new(&mut ctx.scene_tree.clone(), |_| {});
        clock.updates(3).await;
        if detached {
            ctx.scene_tree.clone().remove_child(&probe.node);
            clock.updates(3).await;
            probe.assert_counts(1, 1, 0, 0);
        }
        probe.node.queue_free();
        clock.updates(3).await;
        assert!(!probe.node.is_instance_valid());
        probe.assert_counts(1, 1, 1, 1);
        assert_eq!(probe.counts.exits.load(Ordering::SeqCst), 1);
    }
    clock.cleanup().await;
}

async fn preserve_world(ctx: TestContext, reparent: bool, request_ready: bool) {
    let mut clock = TestApp::new(&ctx, |_| {}).await;
    let mut probe = LocalApp::new(&mut ctx.scene_tree.clone(), |_| {});
    clock.updates(3).await;
    let entity = probe
        .node
        .bind_mut()
        .get_app_mut()
        .unwrap()
        .world_mut()
        .spawn(Unrelated(42))
        .id();
    if request_ready {
        probe.node.request_ready();
    }
    let mut other = Node::new_alloc();
    ctx.scene_tree.clone().add_child(&other);
    if reparent {
        probe.node.reparent(&other);
    } else {
        ctx.scene_tree.clone().remove_child(&probe.node);
        clock.updates(3).await;
        probe.assert_counts(1, 1, 0, 0);
        other.add_child(&probe.node);
    }
    clock.updates(3).await;
    probe.assert_counts(1, 1, 0, 0);
    {
        let binding = probe.node.bind();
        let world = binding.get_app().expect("world survives move").world();
        assert!(Arc::ptr_eq(
            &world.resource::<DropSentinel>().0,
            &probe.counts
        ));
        assert_eq!(world.get::<Unrelated>(entity).unwrap().0, 42);
    }
    other.free();
    probe.assert_counts(1, 1, 1, 1);
    clock.cleanup().await;
}

#[itest]
async fn shutdown_remove_readd_preserves_world(ctx: TestContext) {
    preserve_world(ctx, false, false).await;
}

#[itest]
async fn shutdown_reparent_preserves_world(ctx: TestContext) {
    preserve_world(ctx, true, false).await;
}

#[itest]
async fn shutdown_request_ready_preserves_world(ctx: TestContext) {
    preserve_world(ctx, false, true).await;
}

async fn system_shutdown(ctx: TestContext, fixed: bool) {
    let mut clock = TestApp::new(&ctx, |_| {}).await;
    for ancestor in [false, true] {
        let mut parent = Node::new_alloc();
        ctx.scene_tree.clone().add_child(&parent);
        let mut probe = LocalApp::new(&mut parent, |_| {});
        clock.updates(3).await;
        let id = probe.node.instance_id();
        let actions = Arc::new(AtomicUsize::new(0));
        let observed = actions.clone();
        let counts = probe.counts.clone();
        let system = move |_: GodotAccess, mut done: Local<bool>| {
            if *done {
                return;
            }
            *done = true;
            let mut node = Gd::<Node>::from_instance_id(id);
            let mut parent = node.get_parent().unwrap();
            let panics = Arc::new(AtomicUsize::new(0));
            let captured = panics.clone();
            let hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |_| {
                captured.fetch_add(1, Ordering::SeqCst);
            }));
            parent.remove_child(&node);
            parent.add_child(&node);
            std::panic::set_hook(hook);
            assert_eq!(
                panics.load(Ordering::SeqCst),
                0,
                "tree callbacks must not reborrow the host"
            );
            assert_eq!(
                counts.terminal.load(Ordering::SeqCst),
                0,
                "removal must not end the world"
            );
            observed.fetch_add(1, Ordering::SeqCst);
            if ancestor {
                parent.queue_free();
            } else {
                node.queue_free();
            }
        };
        {
            let mut binding = probe.node.bind_mut();
            let app = binding.get_app_mut().unwrap();
            if fixed {
                app.add_systems(FixedUpdate, system);
            } else {
                app.add_systems(Update, system);
            }
        }
        clock.updates(4).await;
        assert!(godot_bevy::app::drain_test_frame_panics().is_empty());
        assert_eq!(actions.load(Ordering::SeqCst), 1);
        assert!(!probe.node.is_instance_valid());
        probe.assert_counts(1, 1, 1, 1);
        if parent.is_instance_valid() {
            parent.free();
        }
    }
    clock.cleanup().await;
}

#[itest]
async fn shutdown_from_update(ctx: TestContext) {
    system_shutdown(ctx, false).await;
}

#[itest]
async fn shutdown_from_fixed_update(ctx: TestContext) {
    system_shutdown(ctx, true).await;
}

#[itest]
async fn shutdown_cleanup_drop_reinitialize(ctx: TestContext) {
    let counts = Arc::new(Counts::default());
    let observed = counts.clone();
    let mut clock = TestApp::new(&ctx, move |app| count_app(app, &observed)).await;
    clock.with_world_mut(|world| {
        world.write_message(AppExit::error());
    });
    clock.updates(3).await;
    assert_eq!(counts.exits.load(Ordering::SeqCst), 1);
    assert_eq!(counts.terminal.load(Ordering::SeqCst), 0);
    let last = counts.last.load(Ordering::SeqCst);
    clock.cleanup().await;
    clock.cleanup().await;
    drop(clock);
    assert_eq!(counts.inits.load(Ordering::SeqCst), 1);
    assert_eq!(counts.startups.load(Ordering::SeqCst), 1);
    assert_eq!(counts.terminal.load(Ordering::SeqCst), 1);
    assert_eq!(counts.exits.load(Ordering::SeqCst), 2);
    assert_eq!(counts.last.load(Ordering::SeqCst), last + 1);
    assert_eq!(counts.drops.load(Ordering::SeqCst), 1);

    let observed = counts.clone();
    let clock = TestApp::new(&ctx, move |app| count_app(app, &observed)).await;
    drop(clock);
    assert_eq!(counts.inits.load(Ordering::SeqCst), 2);
    assert_eq!(counts.startups.load(Ordering::SeqCst), 2);
    assert_eq!(counts.terminal.load(Ordering::SeqCst), 2);
    assert_eq!(counts.drops.load(Ordering::SeqCst), 2);

    let mut clock = TestApp::new(&ctx, |_| {}).await;
    let mut probe = LocalApp::new(&mut ctx.scene_tree.clone(), |_| {});
    clock.updates(3).await;
    probe.node.bind_mut().initialize();
    probe.assert_counts(2, 1, 1, 1);
    clock.updates(3).await;
    probe.node.bind_mut().teardown();
    probe.node.bind_mut().teardown();
    probe.node.clone().free();
    probe.assert_counts(2, 2, 2, 2);
    assert_eq!(probe.counts.exits.load(Ordering::SeqCst), 2);
    clock.cleanup().await;
}

fn watcher_ids(node: &Gd<BevyApp>) -> Vec<godot::obj::InstanceId> {
    [
        "SceneTreeWatcher",
        "OptimizedSceneTreeWatcher",
        "CollisionWatcher",
        "InputEventWatcher",
    ]
    .into_iter()
    .map(|name| node.get_node_as::<Node>(name).instance_id())
    .collect()
}

fn add_watchers(app: &mut App) {
    app.add_plugins((
        godot_bevy::prelude::GodotCollisionsPlugin,
        godot_bevy::prelude::GodotInputEventPlugin,
    ));
}

#[itest]
async fn shutdown_last_panic_disposes(ctx: TestContext) {
    let mut clock = TestApp::new(&ctx, |_| {}).await;
    for automatic in [false, true] {
        let mut probe = LocalApp::new(&mut ctx.scene_tree.clone(), |app| {
            add_watchers(app);
            app.add_systems(Last, |mut exits: MessageReader<AppExit>| {
                if exits.read().next().is_some() {
                    panic!("shutdown Last sentinel");
                }
            });
        });
        clock.updates(3).await;
        let watchers = watcher_ids(&probe.node);
        if automatic {
            probe.node.clone().free();
        } else {
            probe.node.bind_mut().teardown();
            assert!(probe.node.bind().get_app().is_none());
        }
        clock.updates(3).await;
        let panics = godot_bevy::app::drain_test_frame_panics();
        assert_eq!(
            panics,
            vec![("shutdown Last", "shutdown Last sentinel".to_string())]
        );
        assert_eq!(probe.counts.drops.load(Ordering::SeqCst), 1);
        assert!(
            watchers
                .into_iter()
                .all(|id| Gd::<Node>::try_from_instance_id(id).is_err())
        );
    }
    clock.cleanup().await;
}

#[itest]
async fn shutdown_frame_panic_disposes(ctx: TestContext) {
    let mut clock = TestApp::new(&ctx, |_| {}).await;
    for fixed in [false, true] {
        let mut probe = LocalApp::new(&mut ctx.scene_tree.clone(), add_watchers);
        clock.updates(3).await;
        let watchers = watcher_ids(&probe.node);
        {
            let mut binding = probe.node.bind_mut();
            let app = binding.get_app_mut().unwrap();
            let system = || panic!("shutdown frame sentinel");
            if fixed {
                app.add_systems(FixedUpdate, system);
            } else {
                app.add_systems(Update, system);
            }
        }
        clock.updates(3).await;
        let panics = godot_bevy::app::drain_test_frame_panics();
        let callback = if fixed {
            "_physics_process"
        } else {
            "_process"
        };
        assert_eq!(
            panics,
            vec![(callback, "shutdown frame sentinel".to_string())]
        );
        assert!(probe.node.bind().get_app().is_none());
        probe.assert_counts(1, 1, 0, 1);
        assert!(
            watchers
                .into_iter()
                .all(|id| Gd::<Node>::try_from_instance_id(id).is_err())
        );
    }
    clock.cleanup().await;
}

#[derive(Resource)]
struct PanickingDrop(Arc<AtomicBool>);

impl Drop for PanickingDrop {
    fn drop(&mut self) {
        if self.0.swap(false, Ordering::SeqCst) {
            panic!("shutdown resource Drop sentinel");
        }
    }
}

#[itest]
async fn shutdown_resource_drop_panic_is_caught(ctx: TestContext) {
    let mut clock = TestApp::new(&ctx, |_| {}).await;
    let armed = Arc::new(AtomicBool::new(true));
    let mut probe = LocalApp::new(&mut ctx.scene_tree.clone(), move |app| {
        app.insert_resource(PanickingDrop(armed.clone()));
    });
    clock.updates(3).await;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        probe.node.bind_mut().teardown()
    }));
    let panics = godot_bevy::app::drain_test_frame_panics();
    assert!(result.is_ok(), "resource Drop panic escaped teardown");
    assert_eq!(
        panics,
        vec![(
            "shutdown Drop",
            "shutdown resource Drop sentinel".to_string()
        )]
    );
    assert!(probe.node.bind().get_app().is_none());
    clock.cleanup().await;
}

#[itest]
async fn shutdown_before_startup(ctx: TestContext) {
    let mut clock = TestApp::new(&ctx, |_| {}).await;
    for automatic in [true, false] {
        let mut probe = LocalApp::new(&mut ctx.scene_tree.clone(), |_| {});
        let panics = Arc::new(AtomicUsize::new(0));
        let observed = panics.clone();
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |_| {
            observed.fetch_add(1, Ordering::SeqCst);
        }));
        if automatic {
            probe.node.clone().free();
        } else {
            probe.node.bind_mut().teardown();
        }
        std::panic::set_hook(hook);
        assert_eq!(
            panics.load(Ordering::SeqCst),
            0,
            "unstarted Last must not run"
        );
        probe.assert_counts(1, 0, 0, 1);
        assert_eq!(probe.counts.last.load(Ordering::SeqCst), 0);
        assert!(godot_bevy::app::drain_test_frame_panics().is_empty());
    }
    clock.cleanup().await;
}
