use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_reflect::{Reflect, TypePath};
use bevy_state::{
    app::{AppExtStates, StatesPlugin},
    prelude::*,
};

use super::super::{DebuggerConfig, Wire, service};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, States, Reflect)]
enum Mode {
    #[default]
    Menu,
    Playing,
    Paused,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Reflect)]
struct Playing;

impl ComputedStates for Playing {
    type SourceStates = Mode;

    fn compute(state: Mode) -> Option<Self> {
        (state == Mode::Playing).then_some(Self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, States, Reflect)]
enum Payload {
    Count(u32),
}

fn app() -> App {
    let mut app = App::new();
    app.insert_resource(DebuggerConfig::default());
    app
}

fn mutable_app() -> App {
    let mut app = app();
    app.register_type_mutable_state::<Mode>()
        .insert_resource(State::new(Mode::Menu))
        .init_resource::<NextState<Mode>>();
    app
}

fn call(app: &mut App, method: &str, params: Wire) -> Result<Wire, service::RpcError> {
    service::dispatch(
        app.world_mut(),
        &mut None,
        &Wire::object([
            ("jsonrpc", Wire::String("2.0".into())),
            ("method", Wire::String(method.into())),
            ("params", params),
        ]),
    )
}

fn catalogue(app: &mut App) -> Vec<Wire> {
    call(app, "godot.list_states", Wire::object([]))
        .unwrap()
        .get("states")
        .unwrap()
        .as_array()
        .unwrap()
        .to_vec()
}

fn params(row: &Wire) -> Wire {
    Wire::object([
        ("type_path", row.get("type_path").unwrap().clone()),
        ("state_entity", row.get("state_entity").unwrap().clone()),
        (
            "next_state_entity",
            row.get("next_state_entity").unwrap().clone(),
        ),
    ])
}

fn request(row: &Wire, variant: &str) -> Wire {
    let mut params = params(row);
    let Wire::Object(fields) = &mut params else {
        unreachable!()
    };
    fields.insert(
        "value".into(),
        Wire::object([("variant", Wire::String(variant.into()))]),
    );
    params
}

fn receipt(result: &Wire) -> Option<&str> {
    result.get("receipt").and_then(Wire::as_str)
}

fn reference<R: Resource>(app: &App) -> Wire {
    let world = app.world();
    let entity = world
        .resource_entities()
        .get(world.component_id::<R>().unwrap())
        .unwrap();
    super::super::wire::entity(entity.to_bits(), entity.generation().to_bits())
}

#[test]
fn discovery_requires_bevys_state_type_data() {
    let mut app = app();
    app.register_type::<Mode>()
        .register_type::<State<Mode>>()
        .register_type::<NextState<Mode>>()
        .insert_resource(State::new(Mode::Menu))
        .init_resource::<NextState<Mode>>();
    let result = call(&mut app, "godot.list_states", Wire::object([])).unwrap();
    assert_eq!(result.get("states"), Some(&Wire::Array(vec![])));
    assert_eq!(
        result.get("reason").and_then(Wire::as_str),
        Some(
            "Register states with App::register_type_mutable_state::<S>() or App::register_type_state::<S>()"
        )
    );
    app.register_type_mutable_state::<Mode>();
    let rows = catalogue(&mut app);
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(
        row.get("type_path").and_then(Wire::as_str),
        Some(Mode::type_path())
    );
    assert_eq!(
        row.get("state_entity"),
        Some(&reference::<State<Mode>>(&app))
    );
    assert_eq!(
        row.get("next_state_entity"),
        Some(&reference::<NextState<Mode>>(&app))
    );
    assert_eq!(row.get("present"), Some(&Wire::Bool(true)));
    assert_eq!(row.get("next_present"), Some(&Wire::Bool(true)));
    assert_eq!(row.get("mutable"), Some(&Wire::Bool(true)));
    assert_eq!(row.get("can_request"), Some(&Wire::Bool(true)));
    assert_eq!(row.get("reason"), Some(&Wire::Null));
    assert!(row.get("current").is_none());
    assert!(row.get("queued").is_none());
    assert!(!app.is_plugin_added::<StatesPlugin>());
}

#[test]
fn registration_does_not_initialize_states_or_schedules() {
    let mut app = app();
    app.register_type_mutable_state::<Mode>();
    let row = catalogue(&mut app).remove(0);
    assert_eq!(row.get("state_entity"), Some(&Wire::Null));
    assert_eq!(row.get("next_state_entity"), Some(&Wire::Null));
    assert_eq!(row.get("present"), Some(&Wire::Bool(false)));
    let result = call(&mut app, "godot.get_state", params(&row)).unwrap();
    assert_eq!(result.get("current"), Some(&Wire::Null));
    assert_eq!(result.get("queued"), Some(&Wire::Null));
    assert_eq!(result.get("receipt"), Some(&Wire::Null));
    assert!(!app.world().contains_resource::<State<Mode>>());
    assert!(!app.world().contains_resource::<NextState<Mode>>());
    assert!(app.get_schedule(StateTransition).is_none());
}

#[test]
fn computed_states_are_read_only_and_need_no_next_state() {
    let mut app = app();
    app.register_type_state::<Playing>()
        .insert_resource(State::new(Playing));
    let row = catalogue(&mut app).remove(0);
    assert_eq!(row.get("mutable"), Some(&Wire::Bool(false)));
    assert_eq!(row.get("can_request"), Some(&Wire::Bool(false)));
    assert_eq!(
        row.get("reason").and_then(Wire::as_str),
        Some("state is registered read-only")
    );
    assert_eq!(row.get("next_state_entity"), Some(&Wire::Null));
    let result = call(&mut app, "godot.get_state", params(&row)).unwrap();
    assert_eq!(
        result
            .get("current")
            .unwrap()
            .get("type_path")
            .and_then(Wire::as_str),
        Some(Playing::type_path())
    );
    assert_eq!(
        result
            .get("current")
            .unwrap()
            .get("writable")
            .unwrap()
            .get("allowed"),
        Some(&Wire::Bool(false))
    );
}

#[test]
fn read_only_registration_does_not_report_an_unreadable_queue_as_empty() {
    let mut app = app();
    app.register_type_state::<Mode>()
        .insert_resource(State::new(Mode::Menu))
        .insert_resource(NextState::Pending(Mode::Playing));
    let row = catalogue(&mut app).remove(0);
    assert_eq!(row.get("next_present"), Some(&Wire::Null));
    assert_eq!(row.get("queued_available"), Some(&Wire::Bool(false)));
    let result = call(&mut app, "godot.get_state", params(&row)).unwrap();
    assert_eq!(
        result
            .get("current")
            .unwrap()
            .get("variant")
            .and_then(Wire::as_str),
        Some("Menu")
    );
    assert_eq!(result.get("queued_available"), Some(&Wire::Bool(false)));
    assert_eq!(
        result.get("queued_reason").and_then(Wire::as_str),
        Some("NextState reflection not registered")
    );
    assert!(matches!(
        app.world().resource::<NextState<Mode>>(),
        NextState::Pending(Mode::Playing)
    ));
}

#[test]
fn current_and_both_pending_forms_are_read_separately_without_a_schedule() {
    let mut app = mutable_app();
    let row = catalogue(&mut app).remove(0);
    for next in [
        NextState::Pending(Mode::Playing),
        NextState::PendingIfNeq(Mode::Playing),
    ] {
        app.insert_resource(next);
        for _ in 0..3 {
            let result = call(&mut app, "godot.get_state", params(&row)).unwrap();
            assert_eq!(
                result
                    .get("current")
                    .unwrap()
                    .get("variant")
                    .and_then(Wire::as_str),
                Some("Menu")
            );
            assert_eq!(
                result
                    .get("queued")
                    .unwrap()
                    .get("variant")
                    .and_then(Wire::as_str),
                Some("Playing")
            );
            for key in ["current", "queued"] {
                assert_eq!(
                    result
                        .get(key)
                        .unwrap()
                        .get("writable")
                        .unwrap()
                        .get("allowed"),
                    Some(&Wire::Bool(false))
                );
            }
            assert_eq!(result.get("receipt"), Some(&Wire::Null));
        }
    }
    app.world_mut().resource_mut::<NextState<Mode>>().reset();
    let result = call(&mut app, "godot.get_state", params(&row)).unwrap();
    assert_eq!(result.get("queued"), Some(&Wire::Null));
    assert_eq!(app.world().resource::<State<Mode>>().get(), &Mode::Menu);
}

#[test]
fn removal_preserves_backing_identity_and_reinsertion_reads_fresh_values() {
    let mut app = mutable_app();
    let row = catalogue(&mut app).remove(0);
    app.world_mut().remove_resource::<State<Mode>>();
    app.world_mut().remove_resource::<NextState<Mode>>();
    let absent = catalogue(&mut app).remove(0);
    assert_eq!(absent.get("state_entity"), row.get("state_entity"));
    assert_eq!(
        absent.get("next_state_entity"),
        row.get("next_state_entity")
    );
    assert_eq!(absent.get("present"), Some(&Wire::Bool(false)));
    assert_eq!(absent.get("next_present"), Some(&Wire::Bool(false)));
    let result = call(&mut app, "godot.get_state", params(&row)).unwrap();
    assert_eq!(result.get("current"), Some(&Wire::Null));
    assert_eq!(result.get("queued"), Some(&Wire::Null));
    app.insert_resource(State::new(Mode::Paused));
    let result = call(&mut app, "godot.get_state", params(&row)).unwrap();
    assert_eq!(
        result
            .get("current")
            .unwrap()
            .get("variant")
            .and_then(Wire::as_str),
        Some("Paused")
    );
    assert_eq!(
        result.get("reason").and_then(Wire::as_str),
        Some("NextState resource absent")
    );
}

#[test]
fn reads_reject_unknown_types_and_mismatched_backing_identities() {
    let mut app = mutable_app();
    let row = catalogue(&mut app).remove(0);
    for key in ["state_entity", "next_state_entity"] {
        let mut stale = params(&row);
        let Wire::Object(fields) = &mut stale else {
            unreachable!()
        };
        fields.insert(
            key.into(),
            Wire::object([
                ("bits", Wire::String("1".into())),
                ("generation", Wire::Integer(999)),
            ]),
        );
        let error = call(&mut app, "godot.get_state", stale)
            .unwrap_err()
            .to_wire();
        assert_eq!(
            error.get("message").and_then(Wire::as_str),
            Some("stale state backing identity")
        );
    }
    let error = call(
        &mut app,
        "godot.get_state",
        Wire::object([("type_path", Wire::String("game::NotRegistered".into()))]),
    )
    .unwrap_err()
    .to_wire();
    assert_eq!(
        error.get("message").and_then(Wire::as_str),
        Some("state not registered with Bevy state reflection")
    );
    let error = call(
        &mut app,
        "godot.get_state",
        Wire::object([("type_path", Wire::String(Mode::type_path().into()))]),
    )
    .unwrap_err()
    .to_wire();
    assert_eq!(error.get("code"), Some(&Wire::Integer(-32602)));
}

#[test]
fn payload_states_remain_readable_without_constructible_variants() {
    let mut app = app();
    app.register_type_mutable_state::<Payload>()
        .insert_resource(State::new(Payload::Count(7)))
        .init_resource::<NextState<Payload>>();
    let row = catalogue(&mut app).remove(0);
    assert_eq!(row.get("unit_variants"), Some(&Wire::Array(vec![])));
    assert_eq!(
        row.get("reason").and_then(Wire::as_str),
        Some("state has no fieldless enum variants")
    );
    let result = call(&mut app, "godot.get_state", params(&row)).unwrap();
    assert_eq!(
        result
            .get("current")
            .unwrap()
            .get("variant")
            .and_then(Wire::as_str),
        Some("Count")
    );
}

#[test]
fn requests_refuse_to_overwrite_the_game_queue() {
    let mut app = mutable_app();
    let row = catalogue(&mut app).remove(0);
    for next in [
        NextState::Pending(Mode::Paused),
        NextState::PendingIfNeq(Mode::Paused),
    ] {
        app.insert_resource(next.clone());
        for variant in ["Playing", "Paused", "Menu"] {
            let error = call(
                &mut app,
                "godot.request_state_transition",
                request(&row, variant),
            )
            .unwrap_err()
            .to_wire();
            assert_eq!(
                error.get("message").and_then(Wire::as_str),
                Some(if variant == "Menu" {
                    "Already current"
                } else {
                    "a state transition is already queued"
                })
            );
            assert_eq!(app.world().resource::<State<Mode>>().get(), &Mode::Menu);
            assert_eq!(
                format!("{:?}", app.world().resource::<NextState<Mode>>()),
                format!("{next:?}")
            );
        }
    }
}

#[test]
fn requests_remain_queued_when_the_transition_schedule_never_runs() {
    let mut app = mutable_app();
    let row = catalogue(&mut app).remove(0);
    let result = call(
        &mut app,
        "godot.request_state_transition",
        request(&row, "Playing"),
    )
    .unwrap();
    assert_eq!(receipt(&result), Some("Queued: Playing; current: Menu"));
    assert!(matches!(
        app.world().resource::<NextState<Mode>>(),
        NextState::Pending(Mode::Playing)
    ));
    for _ in 0..3 {
        let result = call(&mut app, "godot.get_state", params(&row)).unwrap();
        assert_eq!(receipt(&result), Some("Queued: Playing; current: Menu"));
        assert_eq!(app.world().resource::<State<Mode>>().get(), &Mode::Menu);
    }
    assert!(app.get_schedule(StateTransition).is_none());
}

#[test]
fn sampled_transition_receipt_survives_departure_and_is_replaced_by_the_next_request() {
    let mut app = app();
    app.add_plugins(StatesPlugin)
        .insert_state(Mode::Menu)
        .register_type_mutable_state::<Mode>();
    let row = catalogue(&mut app).remove(0);
    call(
        &mut app,
        "godot.request_state_transition",
        request(&row, "Playing"),
    )
    .unwrap();
    assert_eq!(app.world().resource::<State<Mode>>().get(), &Mode::Menu);
    app.world_mut().run_schedule(StateTransition);
    let result = call(&mut app, "godot.get_state", params(&row)).unwrap();
    assert_eq!(receipt(&result), Some("Transition to Playing observed"));
    assert_eq!(result.get("queued"), Some(&Wire::Null));
    app.world_mut()
        .resource_mut::<NextState<Mode>>()
        .set(Mode::Paused);
    app.world_mut().run_schedule(StateTransition);
    let result = call(&mut app, "godot.get_state", params(&row)).unwrap();
    assert_eq!(receipt(&result), Some("Transition to Playing observed"));
    let result = call(
        &mut app,
        "godot.request_state_transition",
        request(&row, "Menu"),
    )
    .unwrap();
    assert_eq!(receipt(&result), Some("Queued: Menu; current: Paused"));
}

#[test]
fn samples_distinguish_replacement_consumption_and_unseen_round_trip() {
    for outcome in ["replacement", "consumption", "round trip"] {
        let mut app = app();
        app.add_plugins(StatesPlugin)
            .insert_state(Mode::Menu)
            .register_type_mutable_state::<Mode>();
        let row = catalogue(&mut app).remove(0);
        call(
            &mut app,
            "godot.request_state_transition",
            request(&row, "Playing"),
        )
        .unwrap();
        match outcome {
            "replacement" => app
                .world_mut()
                .resource_mut::<NextState<Mode>>()
                .set(Mode::Paused),
            "consumption" => app.world_mut().resource_mut::<NextState<Mode>>().reset(),
            _ => {
                app.world_mut().run_schedule(StateTransition);
                app.world_mut()
                    .resource_mut::<NextState<Mode>>()
                    .set(Mode::Menu);
                app.world_mut().run_schedule(StateTransition);
            }
        }
        let result = call(&mut app, "godot.get_state", params(&row)).unwrap();
        assert_eq!(
            receipt(&result),
            Some(if outcome == "replacement" {
                "Request replaced by Paused"
            } else {
                "No longer pending; transition to Playing not observed"
            })
        );
    }
}

#[test]
fn already_current_refuses_the_request_without_touching_next_state() {
    let mut app = mutable_app();
    let row = catalogue(&mut app).remove(0);
    let changed = app
        .world()
        .get_resource_ref::<NextState<Mode>>()
        .unwrap()
        .last_changed();
    app.world_mut().increment_change_tick();
    let error = call(
        &mut app,
        "godot.request_state_transition",
        request(&row, "Menu"),
    )
    .unwrap_err()
    .to_wire();
    assert_eq!(
        error.get("message").and_then(Wire::as_str),
        Some("Already current")
    );
    assert!(matches!(
        app.world().resource::<NextState<Mode>>(),
        NextState::Unchanged
    ));
    assert_eq!(
        app.world()
            .get_resource_ref::<NextState<Mode>>()
            .unwrap()
            .last_changed(),
        changed
    );
    let result = call(&mut app, "godot.get_state", params(&row)).unwrap();
    assert_eq!(receipt(&result), Some("Already current"));
}

#[test]
fn requests_validate_capability_presence_and_fieldless_value() {
    let mut app = mutable_app();
    let row = catalogue(&mut app).remove(0);
    for value in [
        Wire::Null,
        Wire::String("Playing".into()),
        Wire::object([]),
        Wire::object([("variant", Wire::String("Missing".into()))]),
        Wire::object([
            ("variant", Wire::String("Playing".into())),
            ("fields", Wire::Array(vec![])),
        ]),
    ] {
        let mut params = params(&row);
        let Wire::Object(fields) = &mut params else {
            unreachable!()
        };
        fields.insert("value".into(), value);
        assert!(call(&mut app, "godot.request_state_transition", params).is_err());
        assert!(matches!(
            app.world().resource::<NextState<Mode>>(),
            NextState::Unchanged
        ));
    }
    app.world_mut().remove_resource::<State<Mode>>();
    let error = call(
        &mut app,
        "godot.request_state_transition",
        request(&row, "Playing"),
    )
    .unwrap_err()
    .to_wire();
    assert_eq!(
        error.get("message").and_then(Wire::as_str),
        Some("State resource absent")
    );

    let mut app = self::app();
    app.register_type_state::<Mode>()
        .insert_resource(State::new(Mode::Menu));
    let row = catalogue(&mut app).remove(0);
    let error = call(
        &mut app,
        "godot.request_state_transition",
        request(&row, "Playing"),
    )
    .unwrap_err()
    .to_wire();
    assert_eq!(
        error.get("message").and_then(Wire::as_str),
        Some("state is registered read-only")
    );

    let mut app = self::app();
    app.register_type_mutable_state::<Payload>()
        .insert_resource(State::new(Payload::Count(7)))
        .init_resource::<NextState<Payload>>();
    let row = catalogue(&mut app).remove(0);
    assert!(
        call(
            &mut app,
            "godot.request_state_transition",
            request(&row, "Count")
        )
        .is_err()
    );
    assert!(matches!(
        app.world().resource::<NextState<Payload>>(),
        NextState::Unchanged
    ));
}

#[test]
fn real_state_wrappers_remain_protected_from_generic_mutation() {
    use super::super::{
        edit::{Edit, mutate_component},
        value::ValueLimits,
    };

    let mut app = app();
    app.add_plugins(StatesPlugin)
        .insert_state(Mode::Menu)
        .register_type_mutable_state::<Mode>();
    app.world_mut()
        .resource_mut::<NextState<Mode>>()
        .set(Mode::Playing);
    app.world_mut().run_schedule(StateTransition);
    for (id, path) in [
        (
            app.world().component_id::<State<Mode>>().unwrap(),
            State::<Mode>::type_path(),
        ),
        (
            app.world().component_id::<NextState<Mode>>().unwrap(),
            NextState::<Mode>::type_path(),
        ),
        (
            app.world().component_id::<PreviousState<Mode>>().unwrap(),
            PreviousState::<Mode>::type_path(),
        ),
    ] {
        let entity = app.world().resource_entities().get(id).unwrap();
        let error = mutate_component(
            app.world_mut(),
            entity,
            path,
            &[],
            &Edit::Variant("Menu".into()),
            &ValueLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.0, "state wrapper requires a transition request");
    }
    assert_eq!(app.world().resource::<State<Mode>>().get(), &Mode::Playing);
    assert_eq!(
        app.world().resource::<PreviousState<Mode>>().get(),
        &Mode::Menu
    );
    assert!(matches!(
        app.world().resource::<NextState<Mode>>(),
        NextState::Unchanged
    ));
}
