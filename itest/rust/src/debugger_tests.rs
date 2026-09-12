use std::sync::mpsc::{Receiver, channel};

use bevy::prelude::*;
use godot::classes::Node;
use godot::prelude::{VarDictionary as Dictionary, *};
use godot_bevy::plugins::debugger::{
    DebuggerEndpoint, DebuggerSet, DebuggerTransport, GodotDebuggerPlugin, Wire,
};
use godot_bevy::plugins::scene_tree::plugin::SceneTreeSet;
use godot_bevy::prelude::GodotNodeHandle;
use godot_bevy_test::prelude::*;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct Speed(f32);

#[derive(Component, Reflect)]
#[reflect(Component)]
struct Count(i32);

#[derive(Component, Default)]
struct Position(f32);

#[derive(Resource, Default)]
struct Seen {
    changed: usize,
    displacement: f32,
}

async fn setup(ctx: &TestContext) -> (TestApp, DebuggerEndpoint, Receiver<Wire>, Entity) {
    let (tx, rx) = channel();
    let mut app = TestApp::new(ctx, move |app| {
        app.add_plugins(GodotDebuggerPlugin)
            .register_type::<Speed>()
            .insert_resource(DebuggerTransport::new(move |frame| {
                tx.send(frame).unwrap();
            }))
            .init_resource::<Seen>()
            .add_systems(
                Update,
                |query: Query<(), Changed<Speed>>, mut seen: ResMut<Seen>| {
                    seen.changed += query.iter().count();
                },
            )
            .add_systems(
                FixedUpdate,
                |mut query: Query<(&Speed, &mut Position)>, mut seen: ResMut<Seen>| {
                    for (speed, mut position) in &mut query {
                        let before = position.0;
                        position.0 += speed.0;
                        seen.displacement = position.0 - before;
                    }
                },
            );
        app.world_mut().spawn((Speed(1.0), Position::default()));
    })
    .await;
    let endpoint = app.with_world(|world| world.non_send::<DebuggerEndpoint>().clone());
    let entity = app.single_entity_with::<Speed>();
    (app, endpoint, rx, entity)
}

fn request(id: i64, method: &str, params: Dictionary) -> Dictionary {
    let mut frame = Dictionary::new();
    frame.set("jsonrpc", "2.0");
    frame.set("id", id);
    frame.set("method", method);
    frame.set("params", &params);
    frame
}

fn entity_param(entity: Entity) -> Dictionary {
    let mut reference = Dictionary::new();
    reference.set("bits", entity.to_bits().to_string().as_str());
    reference.set("generation", i64::from(entity.generation().to_bits()));
    let mut params = Dictionary::new();
    params.set("entity", &reference);
    params
}

fn edit(id: i64, entity: Entity, speed: f64) -> Dictionary {
    let mut params = entity_param(entity);
    params.set("component", Speed::type_path());
    let mut segment = Dictionary::new();
    segment.set("index", 0);
    let mut path = VarArray::new();
    path.push(&segment.to_variant());
    params.set("path", &path);
    params.set("value", speed);
    request(id, "godot.mutate_leaf", params)
}

fn response(rx: &Receiver<Wire>, id: i64) -> Wire {
    let frames = rx.try_iter().collect::<Vec<_>>();
    assert_eq!(
        frames.len(),
        1,
        "one response and no unsolicited notifications"
    );
    assert_eq!(frames[0].get("jsonrpc"), Some(&Wire::String("2.0".into())));
    assert_eq!(frames[0].get("id"), Some(&Wire::Integer(id)));
    frames.into_iter().next().unwrap()
}

#[itest(async)]
fn debugger_no_mutation_before_drain(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, entity) = setup(&ctx).await;
        endpoint.submit(edit(1, entity, 4.0));
        assert_eq!(
            app.with_world(|world| world.get::<Speed>(entity).unwrap().0),
            1.0
        );
        assert!(rx.try_recv().is_err());
        app.update().await;
        assert!(response(&rx, 1).get("result").is_some());
        assert_eq!(
            app.with_world(|world| world.get::<Speed>(entity).unwrap().0),
            4.0
        );
        app.cleanup().await;
    })
}

#[itest(async)]
fn debugger_one_correlated_response(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, entity) = setup(&ctx).await;
        endpoint.submit(edit(11, entity, 4.0));
        endpoint.submit(request(12, "unknown", Dictionary::new()));
        endpoint.submit(request(13, "godot.mutate_leaf", Dictionary::new()));
        app.update().await;
        let frames = rx.try_iter().collect::<Vec<_>>();
        assert_eq!(frames.len(), 3);
        for id in [11, 12, 13] {
            assert_eq!(
                frames
                    .iter()
                    .filter(|frame| frame.get("id") == Some(&Wire::Integer(id)))
                    .count(),
                1
            );
        }
        assert!(frames[0].get("result").is_some());
        assert!(frames[1].get("error").is_some());
        assert!(frames[2].get("error").is_some());
        app.update().await;
        assert!(rx.try_recv().is_err());
        app.cleanup().await;
    })
}

#[itest(async)]
fn debugger_typed_containers_return_errors_and_drain_recovers(
    ctx: &TestContext,
) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, entity) = setup(&ctx).await;
        let mut segment = Dictionary::new();
        segment.set("index", 0);
        let mut path = Array::<Dictionary>::new();
        path.push(&segment);
        let mut params = entity_param(entity);
        params.set("component", Speed::type_path());
        params.set("path", &path);
        params.set("value", 9.0);
        let mut typed_dict = godot::builtin::Dictionary::<GString, i64>::new();
        typed_dict.set("index", 0);
        for (id, malformed, reason) in [
            (
                201,
                request(201, "godot.mutate_leaf", params.duplicate_shallow()),
                "typed arrays are not supported in requests",
            ),
            (
                203,
                {
                    let mut nested = VarArray::new();
                    nested.push(&typed_dict.to_variant());
                    params.set("path", &nested);
                    request(203, "godot.mutate_leaf", params)
                },
                "typed dictionaries are not supported in requests",
            ),
            (
                205,
                {
                    let mut frame = request(205, "rpc.discover", Dictionary::new());
                    frame.set("id", &path);
                    frame
                },
                "typed arrays are not supported in requests",
            ),
        ] {
            endpoint.submit(malformed);
            endpoint.submit(request(id + 1, "rpc.discover", Dictionary::new()));
            app.update().await;
            let frames = rx.try_iter().collect::<Vec<_>>();
            assert_eq!(frames.len(), 2);
            assert_eq!(
                frames[0].get("id"),
                Some(&if id == 205 {
                    Wire::Null
                } else {
                    Wire::Integer(id)
                })
            );
            assert_eq!(
                frames[0].get("error").unwrap().get("code"),
                Some(&Wire::Integer(-32600))
            );
            assert_eq!(
                frames[0].get("error").unwrap().get("message"),
                Some(&Wire::String(reason.into()))
            );
            assert_eq!(frames[1].get("id"), Some(&Wire::Integer(id + 1)));
            assert!(frames[1].get("result").is_some());
            assert_eq!(
                app.with_world(|world| world.get::<Speed>(entity).unwrap().0),
                1.0
            );
        }
        endpoint.submit(edit(207, entity, 4.0));
        app.update().await;
        assert!(response(&rx, 207).get("result").is_some());
        assert_eq!(
            app.with_world(|world| world.get::<Speed>(entity).unwrap().0),
            4.0
        );
        app.cleanup().await;
    })
}

#[itest(async)]
fn debugger_notifications_dispatch_without_responses(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, entity) = setup(&ctx).await;
        let mut notification = edit(211, entity, 4.0);
        let _ = notification.remove("id");
        endpoint.submit(notification);
        for method in ["unknown", "godot.mutate_leaf"] {
            let mut frame = request(212, method, Dictionary::new());
            let _ = frame.remove("id");
            endpoint.submit(frame);
        }
        app.update().await;
        assert!(rx.try_recv().is_err());
        assert_eq!(
            app.with_world(|world| world.get::<Speed>(entity).unwrap().0),
            4.0
        );
        let mut params = Dictionary::new();
        params.set("interval_s", 0.0);
        endpoint.submit(request(213, "godot.subscribe", params));
        app.update().await;
        assert_eq!(rx.try_iter().count(), 2);
        let mut unsubscribe = request(214, "godot.unsubscribe", Dictionary::new());
        let _ = unsubscribe.remove("id");
        endpoint.submit(unsubscribe);
        app.update().await;
        app.with_world_mut(|world| {
            world.entity_mut(entity).insert(Name::new("unsubscribed"));
        });
        app.updates(2).await;
        assert!(rx.try_recv().is_err());
        let mut malformed = Dictionary::new();
        malformed.set("jsonrpc", "2.0");
        endpoint.submit(malformed);
        let mut explicit_null = request(215, "rpc.discover", Dictionary::new());
        explicit_null.set("id", &Variant::nil());
        endpoint.submit(explicit_null);
        app.update().await;
        let frames = rx.try_iter().collect::<Vec<_>>();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].get("id"), Some(&Wire::Null));
        assert_eq!(
            frames[0].get("error").unwrap().get("code"),
            Some(&Wire::Integer(-32600))
        );
        assert_eq!(frames[1].get("id"), Some(&Wire::Null));
        assert!(frames[1].get("result").is_some());
        app.cleanup().await;
    })
}

#[itest(async)]
fn debugger_numeric_variants_coerce(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, entity) = setup(&ctx).await;
        app.with_world_mut(|world| {
            world
                .resource::<bevy::ecs::reflect::AppTypeRegistry>()
                .write()
                .register::<Count>();
            world.entity_mut(entity).insert(Count(0));
        });
        for (id, component, input, expected) in [
            (
                231,
                Speed::type_path(),
                2_i64.to_variant(),
                Wire::Float(2.0),
            ),
            (
                232,
                Count::type_path(),
                3.0_f64.to_variant(),
                Wire::Integer(3),
            ),
        ] {
            let mut frame = edit(id, entity, 0.0);
            let mut params = frame.get("params").unwrap().to::<Dictionary>();
            params.set("component", component);
            params.set("value", &input);
            frame.set("params", &params);
            endpoint.submit(frame);
            app.update().await;
            assert_eq!(
                response(&rx, id).get("result").unwrap().get("value"),
                Some(&expected)
            );
        }
        assert_eq!(
            app.with_world(|world| world.get::<Speed>(entity).unwrap().0),
            2.0
        );
        assert_eq!(
            app.with_world(|world| world.get::<Count>(entity).unwrap().0),
            3
        );
        app.cleanup().await;
    })
}

#[itest(async)]
fn debugger_disabling_ends_subscription(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, entity) = setup(&ctx).await;
        let mut params = Dictionary::new();
        params.set("interval_s", 0.0);
        endpoint.submit(request(221, "godot.subscribe", params.clone()));
        app.update().await;
        assert_eq!(rx.try_iter().count(), 2);
        app.with_world_mut(|world| {
            world
                .resource_mut::<godot_bevy::prelude::DebuggerConfig>()
                .enabled = false;
        });
        app.update().await;
        app.with_world_mut(|world| {
            world.entity_mut(entity).insert(Name::new("disabled"));
            world
                .resource_mut::<godot_bevy::prelude::DebuggerConfig>()
                .enabled = true;
        });
        app.updates(2).await;
        assert!(rx.try_recv().is_err());
        endpoint.submit(request(222, "godot.subscribe", params));
        app.update().await;
        assert_eq!(rx.try_iter().count(), 2);
        app.with_world_mut(|world| {
            world
                .resource_mut::<godot_bevy::prelude::DebuggerConfig>()
                .enabled = false;
        });
        endpoint.submit(request(223, "godot.unsubscribe", Dictionary::new()));
        app.update().await;
        assert_eq!(
            response(&rx, 223).get("result").unwrap().get("subscribed"),
            Some(&Wire::Bool(false))
        );
        app.cleanup().await;
    })
}

#[itest(async)]
fn debugger_edit_observed_by_changed(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, entity) = setup(&ctx).await;
        app.with_world_mut(|world| world.resource_mut::<Seen>().changed = 0);
        endpoint.submit(edit(21, entity, 6.0));
        app.update().await;
        assert!(response(&rx, 21).get("result").is_some());
        assert_eq!(app.with_world(|world| world.resource::<Seen>().changed), 1);
        app.cleanup().await;
    })
}

#[derive(Resource, Default)]
struct Fire(bool);

#[derive(Resource, Default)]
struct MidFrame {
    submitted_tick: u32,
    answered_tick: u32,
    tick: u32,
    speed_after_submit: f32,
}

#[itest(async)]
fn debugger_submit_during_frame(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (tx, rx) = channel();
        let mut app = TestApp::new(&ctx, move |app| {
            app.add_plugins(GodotDebuggerPlugin)
                .register_type::<Speed>()
                .insert_resource(DebuggerTransport::new(move |frame| {
                    tx.send(frame).unwrap();
                }))
                .init_resource::<Fire>()
                .init_resource::<MidFrame>()
                .add_systems(Update, |world: &mut World| {
                    world.resource_mut::<MidFrame>().tick += 1;
                    let entity = world
                        .query_filtered::<Entity, With<Speed>>()
                        .single(world)
                        .unwrap();
                    if world.resource::<Fire>().0 {
                        world
                            .non_send::<DebuggerEndpoint>()
                            .submit(edit(31, entity, 8.0));
                        let speed = world.get::<Speed>(entity).unwrap().0;
                        let mut seen = world.resource_mut::<MidFrame>();
                        seen.submitted_tick = seen.tick;
                        seen.speed_after_submit = speed;
                        world.resource_mut::<Fire>().0 = false;
                    } else if world.get::<Speed>(entity).unwrap().0 == 8.0
                        && world.resource::<MidFrame>().answered_tick == 0
                    {
                        let mut seen = world.resource_mut::<MidFrame>();
                        seen.answered_tick = seen.tick;
                    }
                });
            app.world_mut().spawn(Speed(1.0));
        })
        .await;
        app.with_world_mut(|world| world.resource_mut::<Fire>().0 = true);
        app.updates(3).await;
        app.with_world(|world| {
            let seen = world.resource::<MidFrame>();
            assert_eq!(seen.speed_after_submit, 1.0);
            assert_eq!(seen.answered_tick, seen.submitted_tick + 1);
        });
        assert!(response(&rx, 31).get("result").is_some());
        app.cleanup().await;
    })
}

#[itest(async)]
fn debugger_stale_edit_rejected(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, entity) = setup(&ctx).await;
        let replacement = app.with_world_mut(|world| {
            world.despawn(entity);
            world.spawn(Speed(17.0)).id()
        });
        endpoint.submit(edit(41, entity, 99.0));
        app.update().await;
        assert_eq!(
            response(&rx, 41).get("error").unwrap().get("message"),
            Some(&Wire::String("stale entity".into()))
        );
        assert_eq!(
            app.with_world(|world| world.get::<Speed>(replacement).unwrap().0),
            17.0
        );
        app.cleanup().await;
    })
}

#[derive(Resource, Default)]
struct PendingEvidence(Option<f32>);

#[itest(async)]
fn debugger_pending_despawn_rejected(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (tx, rx) = channel();
        let mut app = TestApp::new(&ctx, move |app| {
            app.add_plugins(GodotDebuggerPlugin)
                .register_type::<Speed>()
                .insert_resource(DebuggerTransport::new(move |frame| {
                    tx.send(frame).unwrap();
                }))
                .init_resource::<Fire>()
                .init_resource::<PendingEvidence>()
                .add_systems(
                    First,
                    (|world: &mut World| {
                        if !world.resource::<Fire>().0 {
                            return;
                        }
                        let (entity, handle) = world
                            .query::<(Entity, &GodotNodeHandle, &Speed)>()
                            .iter(world)
                            .map(|(entity, handle, _)| (entity, *handle))
                            .next()
                            .unwrap();
                        Gd::<Node>::from_instance_id(handle.instance_id()).queue_free();
                        world
                            .non_send::<DebuggerEndpoint>()
                            .submit(edit(51, entity, 99.0));
                    })
                    .after(SceneTreeSet::Apply)
                    .before(DebuggerSet::Drain),
                )
                .add_systems(
                    First,
                    (|world: &mut World| {
                        if !world.resource::<Fire>().0 {
                            return;
                        }
                        let speed = world.query::<&Speed>().single(world).unwrap().0;
                        world.resource_mut::<PendingEvidence>().0 = Some(speed);
                        world.resource_mut::<Fire>().0 = false;
                    })
                    .after(DebuggerSet::Drain),
                );
        })
        .await;
        let (_, entity) = app.add_node::<Node>("DebuggerPending").await;
        app.with_world_mut(|world| {
            world.entity_mut(entity).insert(Speed(2.0));
            world.resource_mut::<Fire>().0 = true;
        });
        app.update().await;
        assert_eq!(
            response(&rx, 51).get("error").unwrap().get("message"),
            Some(&Wire::String("pending despawn".into()))
        );
        assert_eq!(
            app.with_world(|world| world.resource::<PendingEvidence>().0),
            Some(2.0)
        );
        app.cleanup().await;
    })
}

fn resolve(id: i64) -> Dictionary {
    let mut params = Dictionary::new();
    params.set("scene_path", "res://debugger_resolve.tscn");
    params.set("node_path", ".");
    request(id, "godot.resolve_node", params)
}

fn candidates(frame: &Wire) -> &[Wire] {
    frame
        .get("result")
        .unwrap()
        .get("candidates")
        .unwrap()
        .as_array()
        .unwrap()
}

#[itest(async)]
fn debugger_resolve_node_replacement_and_removal(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, _) = setup(&ctx).await;
        let (mut old, old_entity) = app.add_node::<Node>("DebuggerResolve").await;
        old.set_scene_file_path("res://debugger_resolve.tscn");
        endpoint.submit(resolve(61));
        app.update().await;
        assert_eq!(
            candidates(&response(&rx, 61))[0].get("bits"),
            Some(&Wire::String(old_entity.to_bits().to_string()))
        );
        old.get_parent().unwrap().remove_child(&old);
        old.queue_free();
        app.updates(2).await;
        let (mut new, new_entity) = app.add_node::<Node>("DebuggerResolve").await;
        new.set_scene_file_path("res://debugger_resolve.tscn");
        endpoint.submit(resolve(62));
        app.update().await;
        let frame = response(&rx, 62);
        assert_eq!(candidates(&frame).len(), 1);
        assert_eq!(
            candidates(&frame)[0].get("bits"),
            Some(&Wire::String(new_entity.to_bits().to_string()))
        );
        new.get_parent().unwrap().remove_child(&new);
        new.queue_free();
        app.updates(2).await;
        endpoint.submit(resolve(63));
        app.update().await;
        assert!(candidates(&response(&rx, 63)).is_empty());
        app.cleanup().await;
    })
}

#[itest(async)]
fn debugger_unchanged_intervals_send_nothing(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, _) = setup(&ctx).await;
        let mut params = Dictionary::new();
        params.set("interval_s", 0.0);
        endpoint.submit(request(71, "godot.subscribe", params));
        app.update().await;
        let frames = rx.try_iter().collect::<Vec<_>>();
        assert_eq!(
            frames
                .iter()
                .filter(|frame| frame.get("id") == Some(&Wire::Integer(71)))
                .count(),
            1
        );
        assert_eq!(
            frames
                .iter()
                .filter(
                    |frame| frame.get("method") == Some(&Wire::String("godot.summary".into()))
                        && frame.get("id").is_none()
                )
                .count(),
            1
        );
        app.updates(3).await;
        assert!(rx.try_recv().is_err());
        app.cleanup().await;
    })
}

#[itest(async)]
fn debugger_unsubscribed_app_sends_nothing(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, _) = setup(&ctx).await;
        app.updates(3).await;
        assert!(rx.try_recv().is_err());
        let mut params = Dictionary::new();
        params.set("interval_s", 0.0);
        endpoint.submit(request(81, "godot.subscribe", params));
        app.update().await;
        rx.try_iter().for_each(drop);
        endpoint.submit(request(82, "godot.unsubscribe", Dictionary::new()));
        app.update().await;
        assert!(response(&rx, 82).get("result").is_some());
        app.with_world_mut(|world| {
            world.spawn(Name::new("after unsubscribe"));
        });
        app.updates(3).await;
        assert!(rx.try_recv().is_err());
        app.cleanup().await;
    })
}

#[itest(async)]
fn debugger_speed_edit_changes_next_tick_position(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, entity) = setup(&ctx).await;
        app.physics_update().await;
        assert_eq!(
            app.with_world(|world| world.resource::<Seen>().displacement),
            1.0
        );
        let before = app.with_world(|world| world.get::<Position>(entity).unwrap().0);
        endpoint.submit(edit(91, entity, 5.0));
        app.physics_update().await;
        assert!(response(&rx, 91).get("result").is_some());
        assert_eq!(
            app.with_world(|world| world.resource::<Seen>().displacement),
            5.0
        );
        assert!(app.with_world(|world| world.get::<Position>(entity).unwrap().0) >= before + 5.0);
        app.cleanup().await;
    })
}

#[derive(Component)]
struct Unreflected;

#[itest(async)]
fn debugger_query_and_selected_components(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, entity) = setup(&ctx).await;
        app.with_world_mut(|world| {
            world
                .entity_mut(entity)
                .insert((Name::new("debugger needle"), Unreflected));
        });
        let mut params = Dictionary::new();
        params.set("name_contains", "needle");
        params.set("component", Speed::type_path());
        params.set("page", 0);
        params.set("page_size", 1);
        endpoint.submit(request(101, "godot.query", params));
        app.update().await;
        let frame = response(&rx, 101);
        let result = frame.get("result").unwrap();
        assert_eq!(result.get("total"), Some(&Wire::Integer(1)));
        let summaries = result.get("entities").unwrap().as_array().unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(
            summaries[0].get("entity").unwrap().get("bits"),
            Some(&Wire::String(entity.to_bits().to_string()))
        );
        assert!(
            summaries[0]
                .get("components")
                .unwrap()
                .as_array()
                .unwrap()
                .contains(&Wire::String(std::any::type_name::<Unreflected>().into()))
        );
        assert_eq!(
            summaries[0]
                .get("unsupported_components")
                .unwrap()
                .get(std::any::type_name::<Unreflected>()),
            Some(&Wire::String("component not registered".into()))
        );
        endpoint.submit(request(102, "godot.get_components", entity_param(entity)));
        app.update().await;
        assert_eq!(
            response(&rx, 102)
                .get("result")
                .unwrap()
                .get(std::any::type_name::<Unreflected>())
                .unwrap()
                .get("reason"),
            Some(&Wire::String("component not registered".into()))
        );
        let mut params = entity_param(entity);
        let mut components = VarArray::new();
        components.push(Speed::type_path());
        params.set("components", &components);
        endpoint.submit(request(103, "godot.get_components", params.clone()));
        app.update().await;
        let before = response(&rx, 103).get("result").unwrap().clone();
        app.with_world_mut(|world| {
            for _ in 0..20 {
                world.spawn((Name::new("unrelated"), Speed(100.0)));
            }
        });
        endpoint.submit(request(104, "godot.get_components", params));
        app.update().await;
        assert_eq!(response(&rx, 104).get("result"), Some(&before));
        app.cleanup().await;
    })
}

#[itest(async)]
fn debugger_discovery_and_malformed_frame(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, _) = setup(&ctx).await;
        endpoint.submit(request(111, "rpc.discover", Dictionary::new()));
        app.update().await;
        let frame = response(&rx, 111);
        let result = frame.get("result").unwrap();
        assert!(result.get("openrpc").is_some());
        let names = result
            .get("methods")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|method| method.get("name").unwrap().as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "rpc.discover",
                "godot.subscribe",
                "godot.unsubscribe",
                "godot.query",
                "godot.get_components",
                "godot.mutate_leaf",
                "godot.resolve_node"
            ]
        );
        let mut malformed = request(112, "rpc.discover", Dictionary::new());
        malformed.set("jsonrpc", "1.0");
        endpoint.submit(malformed);
        app.update().await;
        let frame = response(&rx, 112);
        assert_eq!(
            frame.get("error").unwrap().get("code"),
            Some(&Wire::Integer(-32600))
        );
        assert!(frame.get("error").unwrap().get("data").is_some());
        assert!(frame.get("session").is_none());
        app.cleanup().await;
        assert!(!endpoint.submit(request(113, "rpc.discover", Dictionary::new())));
    })
}

fn summary_delta(rx: &Receiver<Wire>, kind: &str, entity: Entity) -> Wire {
    let frames = rx.try_iter().collect::<Vec<_>>();
    assert_eq!(frames.len(), 1);
    assert!(frames[0].get("id").is_none());
    assert_eq!(
        frames[0].get("method"),
        Some(&Wire::String("godot.summary".into()))
    );
    let params = frames[0].get("params").unwrap();
    assert_eq!(params.get("snapshot"), Some(&Wire::Bool(false)));
    for key in ["added", "removed", "updated"] {
        assert_eq!(
            params.get(key).unwrap().as_array().unwrap().len(),
            usize::from(key == kind)
        );
    }
    let row = &params.get(kind).unwrap().as_array().unwrap()[0];
    let reference = if kind == "removed" {
        row
    } else {
        row.get("entity").unwrap()
    };
    assert_eq!(
        reference.get("bits"),
        Some(&Wire::String(entity.to_bits().to_string()))
    );
    row.clone()
}

#[itest(async)]
fn debugger_summary_deltas(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, entity) = setup(&ctx).await;
        let mut params = Dictionary::new();
        params.set("interval_s", 0.0);
        endpoint.submit(request(121, "godot.subscribe", params));
        app.update().await;
        assert_eq!(rx.try_iter().count(), 2);
        let ordinary =
            app.with_world_mut(|world| world.spawn((Speed(1.0), Position::default())).id());
        app.update().await;
        let added = summary_delta(&rx, "added", ordinary);
        assert!(
            added
                .get("components")
                .unwrap()
                .as_array()
                .unwrap()
                .contains(&Wire::String(Speed::type_path().into()))
        );
        app.with_world_mut(|world| {
            world.entity_mut(entity).insert(Name::new("renamed"));
        });
        app.update().await;
        assert_eq!(
            summary_delta(&rx, "updated", entity).get("name"),
            Some(&Wire::String("renamed".into()))
        );
        app.with_world_mut(|world| {
            world
                .entity_mut(entity)
                .insert(godot_bevy::plugins::scene_tree::GodotChildOf(ordinary));
        });
        app.update().await;
        assert_eq!(
            summary_delta(&rx, "updated", entity)
                .get("parent")
                .unwrap()
                .get("bits"),
            Some(&Wire::String(ordinary.to_bits().to_string()))
        );
        app.with_world_mut(|world| {
            world
                .entity_mut(entity)
                .remove::<godot_bevy::plugins::scene_tree::GodotChildOf>();
        });
        app.update().await;
        assert_eq!(
            summary_delta(&rx, "updated", entity).get("parent"),
            Some(&Wire::Null)
        );
        app.with_world_mut(|world| {
            world.entity_mut(entity).remove::<Name>();
        });
        app.update().await;
        assert_eq!(
            summary_delta(&rx, "updated", entity).get("name"),
            Some(&Wire::String(String::new()))
        );
        for removed in [ordinary, entity] {
            app.with_world_mut(|world| {
                world.despawn(removed);
            });
            app.update().await;
            summary_delta(&rx, "removed", removed);
        }
        let empty = app.with_world_mut(|world| world.spawn_empty().id());
        app.update().await;
        summary_delta(&rx, "added", empty);
        app.with_world_mut(|world| {
            world.despawn(empty);
        });
        app.update().await;
        summary_delta(&rx, "removed", empty);
        app.update().await;
        assert!(rx.try_recv().is_err());
        app.cleanup().await;
    })
}

#[itest(async)]
fn debugger_resolver_keeps_multiple_candidates(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, _) = setup(&ctx).await;
        let (mut first, first_entity) = app.add_node::<Node>("DebuggerFirst").await;
        let (mut second, second_entity) = app.add_node::<Node>("DebuggerSecond").await;
        first.set_scene_file_path("res://debugger_resolve.tscn");
        second.set_scene_file_path("res://debugger_resolve.tscn");
        endpoint.submit(resolve(131));
        app.update().await;
        let frame = response(&rx, 131);
        assert_eq!(candidates(&frame).len(), 2);
        for entity in [first_entity, second_entity] {
            assert!(
                candidates(&frame)
                    .iter()
                    .any(|candidate| candidate.get("bits")
                        == Some(&Wire::String(entity.to_bits().to_string())))
            );
        }
        endpoint.submit(request(
            132,
            "godot.get_components",
            entity_param(first_entity),
        ));
        app.update().await;
        let frame = response(&rx, 132);
        let handle = frame
            .get("result")
            .unwrap()
            .get(std::any::type_name::<GodotNodeHandle>())
            .unwrap();
        assert_eq!(handle.get("kind"), Some(&Wire::String("node".into())));
        assert_eq!(handle.get("valid"), Some(&Wire::Bool(true)));
        assert_eq!(
            handle.get("writable").unwrap().get("allowed"),
            Some(&Wire::Bool(false))
        );
        app.cleanup().await;
        first.queue_free();
        second.queue_free();
    })
}

#[itest(async)]
fn debugger_legacy_dock_stream_until_subscription(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (tx, rx) = channel();
        let mut app = TestApp::new(&ctx, move |app| {
            app.add_plugins(GodotDebuggerPlugin)
                .register_type::<Speed>();
            app.world_mut()
                .resource_mut::<godot_bevy::plugins::debugger::DebuggerConfig>()
                .update_interval = 0.0;
            app.insert_resource(DebuggerTransport::new(|_| {}).with_legacy_sink(
                || true,
                move |data| {
                    for value in data.iter_shared() {
                        let row = value.to::<VarArray>();
                        if row.get(1).unwrap().to::<GString>() != "Legacy" {
                            continue;
                        }
                        let components = row.get(4).unwrap().to::<VarArray>();
                        let speed = components
                            .iter_shared()
                            .map(|value| value.to::<Dictionary>())
                            .find(|component| {
                                component.get("short_name").unwrap().to::<GString>() == "Speed"
                            })
                            .unwrap();
                        let fields = speed
                            .get("value")
                            .unwrap()
                            .to::<Dictionary>()
                            .get("fields")
                            .unwrap()
                            .to::<VarArray>();
                        tx.send((row.len(), fields.get(0).unwrap().to::<f64>()))
                            .unwrap();
                    }
                },
            ));
            app.world_mut().spawn((Name::new("Legacy"), Speed(1.0)));
        })
        .await;
        let endpoint = app.with_world(|world| world.non_send::<DebuggerEndpoint>().clone());
        let frames = rx.try_iter().collect::<Vec<_>>();
        assert!(!frames.is_empty());
        assert!(frames.iter().all(|frame| *frame == (5, 1.0)));
        let mut params = Dictionary::new();
        params.set("interval_s", 0.0);
        endpoint.submit(request(141, "godot.subscribe", params));
        app.updates(2).await;
        assert!(rx.try_recv().is_err());
        endpoint.submit(request(142, "godot.unsubscribe", Dictionary::new()));
        app.update().await;
        assert!(rx.try_recv().is_ok());
        app.cleanup().await;
    })
}

#[derive(Resource, Default)]
struct LegacyStep(std::time::Duration);

#[itest(async)]
fn debugger_legacy_cadence_uses_bevy_time(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (tx, rx) = channel();
        let mut app = TestApp::new(&ctx, move |app| {
            app.insert_resource(godot_bevy::prelude::DebuggerConfig {
                update_interval: 1800.0,
                ..default()
            })
            .add_plugins(GodotDebuggerPlugin)
            .init_resource::<LegacyStep>()
            .add_systems(
                PreUpdate,
                |step: Res<LegacyStep>,
                 mut virtual_time: ResMut<Time<bevy::time::Virtual>>,
                 mut time: ResMut<Time>| {
                    virtual_time.advance_by(step.0);
                    *time = virtual_time.as_generic();
                },
            )
            .insert_resource(DebuggerTransport::new(|_| {}).with_legacy_sink(
                || true,
                move |_| {
                    tx.send(()).unwrap();
                },
            ));
        })
        .await;
        assert!(rx.try_recv().is_err());
        app.with_world_mut(|world| {
            world.resource_mut::<LegacyStep>().0 = std::time::Duration::from_secs(1800);
        });
        app.updates(2).await;
        assert!(rx.try_iter().count() >= 1);
        app.with_world_mut(|world| {
            world.resource_mut::<LegacyStep>().0 = std::time::Duration::ZERO;
        });
        app.updates(3).await;
        assert!(rx.try_recv().is_err());
        app.cleanup().await;
    })
}

#[itest(async)]
fn debugger_resolves_relative_child_and_queries_runtime_path(
    ctx: &TestContext,
) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, _) = setup(&ctx).await;
        let (mut root, root_entity) = app.add_node::<Node>("DebuggerAuthoredRoot").await;
        root.set_scene_file_path("res://debugger_resolve.tscn");
        let mut child = Node::new_alloc();
        child.set_name("Child");
        root.add_child(&child);
        let mut child_entity = None;
        for _ in 0..3 {
            app.update().await;
            child_entity = app.entity_for_node(child.instance_id());
            if child_entity.is_some() {
                break;
            }
        }
        let child_entity = child_entity.expect("child entity should appear within three frames");
        let mut params = Dictionary::new();
        params.set("scene_path", "res://debugger_resolve.tscn");
        params.set("node_path", "Child");
        endpoint.submit(request(151, "godot.resolve_node", params));
        app.update().await;
        let frame = response(&rx, 151);
        assert_eq!(candidates(&frame).len(), 1);
        assert_eq!(
            candidates(&frame)[0].get("bits"),
            Some(&Wire::String(child_entity.to_bits().to_string()))
        );
        assert_ne!(child_entity, root_entity);
        let mut params = Dictionary::new();
        params.set("node_path", child.get_path().to_string().as_str());
        params.set("page", 0);
        params.set("page_size", 1);
        endpoint.submit(request(152, "godot.query", params));
        app.update().await;
        let frame = response(&rx, 152);
        assert_eq!(
            frame.get("result").unwrap().get("total"),
            Some(&Wire::Integer(1))
        );
        assert_eq!(
            frame
                .get("result")
                .unwrap()
                .get("entities")
                .unwrap()
                .as_array()
                .unwrap()[0]
                .get("entity")
                .unwrap()
                .get("bits"),
            Some(&Wire::String(child_entity.to_bits().to_string()))
        );
        app.cleanup().await;
        root.queue_free();
    })
}

#[itest(async)]
fn debugger_invalid_node_handle_is_reported(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, endpoint, rx, _) = setup(&ctx).await;
        let node = Node::new_alloc();
        let handle = GodotNodeHandle::new(node.clone());
        node.free();
        let entity = app.with_world_mut(|world| world.spawn(handle).id());
        endpoint.submit(request(161, "godot.get_components", entity_param(entity)));
        app.update().await;
        let frame = response(&rx, 161);
        assert_eq!(
            frame
                .get("result")
                .unwrap()
                .get(std::any::type_name::<GodotNodeHandle>())
                .unwrap()
                .get("valid"),
            Some(&Wire::Bool(false))
        );
        app.cleanup().await;
    })
}
