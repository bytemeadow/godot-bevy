use super::*;
use crate::plugins::debugger::edit::{Edit, PathSegment, mutate_component, read_component};
use bevy_ecs::prelude::*;
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_reflect::{Reflect, TypePath};

#[derive(Component, Reflect, Debug, PartialEq)]
#[reflect(Component)]
struct Sample {
    nested: (Vec<i128>, [u64; 2]),
    mode: Mode,
    #[reflect(ignore)]
    hidden: i32,
}

#[derive(Reflect, Debug, PartialEq)]
enum Mode {
    Idle,
    Walking { speed: f32 },
    Pair(i32),
    Stopped,
}

fn sample_world() -> (World, Entity) {
    let mut world = World::new();
    let registry = AppTypeRegistry::default();
    registry.write().register::<Sample>();
    world.insert_resource(registry);
    let entity = world
        .spawn(Sample {
            nested: (vec![i128::MAX], [u64::MAX, 2]),
            mode: Mode::Walking { speed: 1.0 },
            hidden: 9,
        })
        .id();
    (world, entity)
}

fn view(value: &dyn PartialReflect) -> Value {
    inspect(
        value,
        &TypeRegistry::default(),
        Writable::Yes,
        &ValueLimits::default(),
    )
}

fn value_count(value: &Value) -> usize {
    1 + match &value.kind {
        Kind::Struct(fields) | Kind::Enum { fields, .. } => fields
            .iter()
            .map(|field| value_count(&field.value))
            .sum::<usize>(),
        Kind::TupleStruct(items)
        | Kind::Tuple(items)
        | Kind::List { items, .. }
        | Kind::Array { items, .. }
        | Kind::Set { members: items, .. } => items.iter().map(value_count).sum::<usize>(),
        Kind::Map { entries, .. } => entries
            .iter()
            .map(|(key, value)| value_count(key) + value_count(value))
            .sum::<usize>(),
        _ => 0,
    }
}

#[test]
fn nested_collections_share_a_total_value_budget() {
    let source = vec![vec![(0..100_i32).collect::<Vec<_>>(); 100]; 100];
    let value = view(&source);
    assert!(value_count(&value) <= 16_384);
    let Kind::List { items, truncated } = &value.kind else {
        panic!("outer list expected")
    };
    assert_eq!(items.len() + truncated, 100);
    let mut omitted = *truncated;
    let mut leaves = 0;
    for middle in items {
        let Kind::List { items, truncated } = &middle.kind else {
            panic!("middle list expected")
        };
        assert_eq!(items.len() + truncated, 100);
        omitted += truncated;
        for inner in items {
            let Kind::List { items, truncated } = &inner.kind else {
                panic!("inner list expected")
            };
            assert_eq!(items.len() + truncated, 100);
            omitted += truncated;
            for (index, leaf) in items.iter().enumerate() {
                assert_eq!(leaf.kind, Kind::Scalar(Scalar::Integer(index as i64)));
                assert_eq!(leaf.writable, Writable::Yes);
                leaves += 1;
            }
        }
    }
    assert!(leaves > 0);
    assert!(omitted > 0);
}

#[test]
fn collection_budget_counts_containers_keys_and_values() {
    let limits = ValueLimits {
        max_values: 4,
        ..Default::default()
    };
    let list = vec![10_i32, 20, 30, 40, 50];
    let array = [10_i32, 20, 30, 40, 50];
    let registry = TypeRegistry::default();
    for source in [&list as &dyn PartialReflect, &array] {
        let value = inspect(source, &registry, Writable::Yes, &limits);
        assert_eq!(value_count(&value), 4);
        let (Kind::List { items, truncated } | Kind::Array { items, truncated }) = value.kind
        else {
            panic!("list or array expected")
        };
        assert_eq!(truncated, 2);
        assert_eq!(items.len(), 3);
        for (item, number) in items.iter().zip([10, 20, 30]) {
            assert_eq!(item.kind, Kind::Scalar(Scalar::Integer(number)));
        }
    }
    let map =
        bevy_platform::collections::HashMap::<i32, i32>::from_iter([(1, 10), (2, 20), (3, 30)]);
    let value = inspect(&map, &registry, Writable::Yes, &limits);
    assert_eq!(value_count(&value), 3);
    let Kind::Map { entries, truncated } = value.kind else {
        panic!("map expected")
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(truncated, 2);
    assert!(matches!((&entries[0].0.kind, &entries[0].1.kind),
        (Kind::Scalar(Scalar::Integer(key)), Kind::Scalar(Scalar::Integer(value)))
        if *value == *key * 10));

    let set = bevy_platform::collections::HashSet::<i32>::from_iter([1, 2, 3, 4, 5]);
    let value = inspect(&set, &registry, Writable::Yes, &limits);
    assert_eq!(value_count(&value), 4);
    let Kind::Set { members, truncated } = value.kind else {
        panic!("set expected")
    };
    assert_eq!(members.len(), 3);
    assert_eq!(truncated, 2);
    for member in members {
        assert!(matches!(member.kind, Kind::Scalar(Scalar::Integer(1..=5))));
        assert_eq!(
            member.writable,
            Writable::No(ReadOnlyReason::KindNotEditable)
        );
    }
}

#[test]
fn nested_map_entries_reserve_both_key_and_value() {
    let source = bevy_platform::collections::HashMap::<Vec<i32>, Vec<i32>>::from_iter([
        (vec![1, 2, 3], vec![10, 20, 30, 40]),
        (vec![4, 5, 6], vec![50, 60, 70, 80]),
    ]);
    let value = inspect(
        &source,
        &TypeRegistry::default(),
        Writable::Yes,
        &ValueLimits {
            max_values: 6,
            ..Default::default()
        },
    );
    assert_eq!(value_count(&value), 6);
    let Kind::Map { entries, truncated } = value.kind else {
        panic!("map expected")
    };
    assert_eq!(entries.len(), 2);
    assert_eq!(truncated, 0);
    for (key, value) in entries {
        for (value, len) in [(key, 3), (value, 4)] {
            let Kind::List { items, truncated } = value.kind else {
                panic!("list expected")
            };
            assert_eq!(items.len() + truncated, len);
            assert!(truncated > 0);
        }
    }
}

#[test]
fn non_collection_elision_has_an_honest_read_only_notice() {
    use crate::plugins::debugger::wire::Wire;

    let mut source = bevy_reflect::structs::DynamicStruct::default();
    source.insert("items", vec![7_i32; 100]);
    source.insert("mode", Mode::Walking { speed: 2.0 });
    let value = inspect(
        &source,
        &TypeRegistry::default(),
        Writable::Yes,
        &ValueLimits {
            max_values: 5,
            ..Default::default()
        },
    );
    assert_eq!(value_count(&value), 5);
    let Kind::Struct(fields) = value.kind else {
        panic!("struct expected")
    };
    assert_eq!(fields[0].name, "items");
    assert!(
        matches!(&fields[0].value.kind, Kind::List { items, truncated: 98 }
        if items.len() == 2 && items.iter().all(|item| item.kind == Kind::Scalar(Scalar::Integer(7))))
    );
    assert_eq!(fields[1].name, "mode");
    let notice = &fields[1].value;
    assert_eq!(notice.type_path, Mode::type_path());
    assert_eq!(notice.kind, Kind::ValueLimit);
    assert_eq!(notice.writable, Writable::No(ReadOnlyReason::ValueLimit));
    let Wire::Object(wire) = Wire::from(notice) else {
        panic!("wire object expected")
    };
    assert_eq!(wire["kind"], Wire::String("value_limit".into()));
    assert_eq!(
        wire["reason"],
        Wire::String("maximum value count reached".into())
    );
}

#[test]
fn fixed_fields_cannot_emit_notices_beyond_the_budget() {
    let mut wide = bevy_reflect::structs::DynamicStruct::default();
    for index in 0..1000 {
        wide.insert(format!("field_{index}"), index);
    }
    let pair = (1_i32, 2_i32);
    let wrapped = Readable(3);
    let mode = Mode::Walking { speed: 4.0 };
    for source in [&wide as &dyn PartialReflect, &pair, &wrapped, &mode] {
        let value = inspect(
            source,
            &TypeRegistry::default(),
            Writable::Yes,
            &ValueLimits {
                max_values: 1,
                ..Default::default()
            },
        );
        assert_eq!(value_count(&value), 1);
        assert_eq!(value.kind, Kind::ValueLimit);
        assert_eq!(value.writable, Writable::No(ReadOnlyReason::ValueLimit));
    }
}

#[test]
fn ordinary_components_and_leaf_edits_fit_the_budget() {
    let (mut world, entity) = sample_world();
    let default =
        read_component(&world, entity, Sample::type_path(), &ValueLimits::default()).unwrap();
    assert_eq!(value_count(&default), 9);
    let exact = read_component(
        &world,
        entity,
        Sample::type_path(),
        &ValueLimits {
            max_values: 9,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(exact, default);
    for max_values in [0, 1] {
        let limits = ValueLimits {
            max_values,
            ..Default::default()
        };
        assert_eq!(
            inspect(&42_i32, &TypeRegistry::default(), Writable::Yes, &limits),
            view(&42_i32)
        );
        let accepted = mutate_component(
            &mut world,
            entity,
            Sample::type_path(),
            &[
                PathSegment::Field("mode".into()),
                PathSegment::Variant("Walking".into()),
                PathSegment::Field("speed".into()),
            ],
            &Edit::Scalar(Scalar::Float(3.0)),
            &limits,
        )
        .unwrap();
        assert_eq!(accepted.kind, Kind::Scalar(Scalar::Float(3.0)));
        assert_eq!(accepted.writable, Writable::Yes);
    }
    assert_eq!(
        world.get::<Sample>(entity).unwrap().mode,
        Mode::Walking { speed: 3.0 }
    );
}

#[test]
fn integer_identity() {
    assert_eq!(
        view(&u64::MAX).kind,
        Kind::Scalar(Scalar::Decimal(u64::MAX.to_string()))
    );
    assert_eq!(
        view(&i128::MAX).kind,
        Kind::Scalar(Scalar::Decimal(i128::MAX.to_string()))
    );
    assert_eq!(
        view(&i128::MIN).kind,
        Kind::Scalar(Scalar::Decimal(i128::MIN.to_string()))
    );
    assert_eq!(view(&u64::MAX).type_path, "u64");
    assert_eq!(view(&i128::MAX).type_path, "i128");
    assert_eq!(view(&42_u64).kind, Kind::Scalar(Scalar::Integer(42)));
}

#[test]
fn nested_paths_and_existing_indices() {
    let (mut world, entity) = sample_world();
    let path = [
        PathSegment::Field("nested".into()),
        PathSegment::Index(0),
        PathSegment::Index(0),
    ];
    let accepted = mutate_component(
        &mut world,
        entity,
        Sample::type_path(),
        &path,
        &Edit::Scalar(Scalar::Decimal(i128::MIN.to_string())),
        &ValueLimits::default(),
    )
    .unwrap();
    assert_eq!(
        accepted.kind,
        Kind::Scalar(Scalar::Decimal(i128::MIN.to_string()))
    );
    assert_eq!(world.get::<Sample>(entity).unwrap().nested.0[0], i128::MIN);
    let array_path = [
        PathSegment::Field("nested".into()),
        PathSegment::Index(1),
        PathSegment::Index(1),
    ];
    mutate_component(
        &mut world,
        entity,
        Sample::type_path(),
        &array_path,
        &Edit::Scalar(Scalar::Integer(7)),
        &ValueLimits::default(),
    )
    .unwrap();
    assert_eq!(world.get::<Sample>(entity).unwrap().nested.1[1], 7);
}

#[test]
fn enum_payload_and_unit_switch() {
    let (mut world, entity) = sample_world();
    let path = [
        PathSegment::Field("mode".into()),
        PathSegment::Variant("Walking".into()),
        PathSegment::Field("speed".into()),
    ];
    mutate_component(
        &mut world,
        entity,
        Sample::type_path(),
        &path,
        &Edit::Scalar(Scalar::Float(5.0)),
        &ValueLimits::default(),
    )
    .unwrap();
    assert_eq!(
        world.get::<Sample>(entity).unwrap().mode,
        Mode::Walking { speed: 5.0 }
    );
    let mode = [PathSegment::Field("mode".into())];
    mutate_component(
        &mut world,
        entity,
        Sample::type_path(),
        &mode,
        &Edit::Variant("Idle".into()),
        &ValueLimits::default(),
    )
    .unwrap();
    assert_eq!(world.get::<Sample>(entity).unwrap().mode, Mode::Idle);
    mutate_component(
        &mut world,
        entity,
        Sample::type_path(),
        &mode,
        &Edit::Variant("Stopped".into()),
        &ValueLimits::default(),
    )
    .unwrap();
    assert_eq!(world.get::<Sample>(entity).unwrap().mode, Mode::Stopped);
    assert_eq!(
        mutate_component(
            &mut world,
            entity,
            Sample::type_path(),
            &mode,
            &Edit::Variant("Walking".into()),
            &ValueLimits::default()
        )
        .unwrap_err()
        .0,
        "payload-bearing variant construction is not editable"
    );
    assert_eq!(world.get::<Sample>(entity).unwrap().mode, Mode::Stopped);
    assert_eq!(
        mutate_component(
            &mut world,
            entity,
            Sample::type_path(),
            &path,
            &Edit::Scalar(Scalar::Float(9.0)),
            &ValueLimits::default()
        )
        .unwrap_err()
        .0,
        "path shape changed"
    );
    assert_eq!(world.get::<Sample>(entity).unwrap().mode, Mode::Stopped);
    assert!(
        matches!(view(&Mode::Pair(3)).kind, Kind::Enum { fields, .. } if fields.len() == 1 && fields[0].name == "0")
    );
    world.get_mut::<Sample>(entity).unwrap().mode = Mode::Pair(3);
    mutate_component(
        &mut world,
        entity,
        Sample::type_path(),
        &[
            PathSegment::Field("mode".into()),
            PathSegment::Variant("Pair".into()),
            PathSegment::Index(0),
        ],
        &Edit::Scalar(Scalar::Integer(4)),
        &ValueLimits::default(),
    )
    .unwrap();
    assert_eq!(world.get::<Sample>(entity).unwrap().mode, Mode::Pair(4));
    assert!(
        matches!(view(&Mode::Idle).kind, Kind::Enum { variant, fields, .. } if variant == "Idle" && fields.is_empty())
    );
}

#[test]
fn ignored_fields_are_absent() {
    let (mut world, entity) = sample_world();
    let value =
        read_component(&world, entity, Sample::type_path(), &ValueLimits::default()).unwrap();
    let Kind::Struct(fields) = value.kind else {
        panic!("struct expected")
    };
    assert_eq!(
        fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["nested", "mode"]
    );
    assert!(
        mutate_component(
            &mut world,
            entity,
            Sample::type_path(),
            &[PathSegment::Field("hidden".into())],
            &Edit::Scalar(Scalar::Integer(1)),
            &ValueLimits::default()
        )
        .is_err()
    );
    assert_eq!(world.get::<Sample>(entity).unwrap().hidden, 9);
}

#[test]
fn maps_sets_and_truncation() {
    let map = bevy_platform::collections::HashMap::<i32, i32>::from_iter([
        (1_i32, 10_i32),
        (2, 20),
        (3, 30),
    ]);
    let value = inspect(
        &map,
        &TypeRegistry::default(),
        Writable::Yes,
        &ValueLimits {
            max_depth: 8,
            max_elements: 2,
            ..Default::default()
        },
    );
    let Kind::Map { entries, truncated } = value.kind else {
        panic!("map expected")
    };
    assert_eq!(entries.len(), 2);
    assert_eq!(truncated, 1);
    assert!(entries.iter().all(|(key, value)| matches!((&key.kind, &value.kind), (Kind::Scalar(Scalar::Integer(k)), Kind::Scalar(Scalar::Integer(v))) if *v == *k * 10)));
    assert!(
        entries
            .iter()
            .all(|(_, value)| value.writable == Writable::No(ReadOnlyReason::KindNotEditable))
    );
    let set = bevy_platform::collections::HashSet::<i32>::from_iter([1_i32, 2, 3]);
    assert!(
        matches!(inspect(&set, &TypeRegistry::default(), Writable::Yes, &ValueLimits { max_depth: 8, max_elements: 2, ..Default::default() }).kind, Kind::Set { members, truncated: 1 } if members.len() == 2)
    );
    assert!(
        matches!(inspect(&vec![1, 2, 3], &TypeRegistry::default(), Writable::Yes, &ValueLimits { max_depth: 8, max_elements: 1, ..Default::default() }).kind, Kind::List { items, truncated: 2 } if items.len() == 1)
    );
    assert!(matches!(
        inspect(
            &vec![vec![1]],
            &TypeRegistry::default(),
            Writable::Yes,
            &ValueLimits {
                max_depth: 0,
                max_elements: 8,
                ..Default::default()
            }
        )
        .kind,
        Kind::DepthLimit
    ));
}

#[derive(Component)]
struct Unregistered;

#[derive(Component, Reflect)]
struct Readable(i32);

#[derive(Component, Reflect)]
#[component(immutable)]
#[reflect(Component)]
struct Immutable(i32);

#[test]
fn registration_and_writability_reasons() {
    let (mut world, entity) = sample_world();
    world
        .entity_mut(entity)
        .insert((Unregistered, Readable(3), Immutable(4)));
    world
        .resource::<AppTypeRegistry>()
        .write()
        .register::<Readable>();
    world
        .resource::<AppTypeRegistry>()
        .write()
        .register::<Immutable>();
    let missing = read_component(
        &world,
        entity,
        std::any::type_name::<Unregistered>(),
        &ValueLimits::default(),
    )
    .unwrap();
    assert_eq!(
        missing.writable,
        Writable::No(ReadOnlyReason::ComponentNotRegistered)
    );
    assert_eq!(
        missing.kind,
        Kind::Unsupported("component not registered".into())
    );
    let readable = read_component(
        &world,
        entity,
        Readable::type_path(),
        &ValueLimits::default(),
    )
    .unwrap();
    assert_eq!(
        readable.writable,
        Writable::No(ReadOnlyReason::NoReflectComponent)
    );
    assert!(
        matches!(readable.kind, Kind::TupleStruct(fields) if fields[0].writable == Writable::No(ReadOnlyReason::NoReflectComponent))
    );
    assert_eq!(
        read_component(
            &world,
            entity,
            Immutable::type_path(),
            &ValueLimits::default()
        )
        .unwrap()
        .writable,
        Writable::No(ReadOnlyReason::ComponentImmutable)
    );
    assert_eq!(
        view(&entity).writable,
        Writable::No(ReadOnlyReason::Reference)
    );
    assert!(
        matches!(view(&entity).kind, Kind::Entity { bits, generation } if bits == entity.to_bits() && generation == entity.generation().to_bits())
    );
    world
        .resource::<AppTypeRegistry>()
        .write()
        .register::<Guarded>();
    world.entity_mut(entity).insert(Guarded {
        secret: Secret(7),
        values: Default::default(),
    });
    let value = read_component(
        &world,
        entity,
        Guarded::type_path(),
        &ValueLimits::default(),
    )
    .unwrap();
    let Kind::Struct(fields) = value.kind else {
        panic!("struct expected")
    };
    assert_eq!(fields[0].name, "secret");
    assert!(matches!(fields[0].value.kind, Kind::Opaque(_)));
    assert_eq!(
        fields[0].value.writable,
        Writable::No(ReadOnlyReason::OpaqueAncestor)
    );
}

#[test]
fn rejected_edit_is_unchanged_and_not_changed() {
    let (mut world, entity) = sample_world();
    world.clear_trackers();
    let path = [
        PathSegment::Field("nested".into()),
        PathSegment::Index(1),
        PathSegment::Index(0),
    ];
    assert!(
        mutate_component(
            &mut world,
            entity,
            Sample::type_path(),
            &path,
            &Edit::Scalar(Scalar::Decimal("18446744073709551616".into())),
            &ValueLimits::default()
        )
        .is_err()
    );
    assert_eq!(world.get::<Sample>(entity).unwrap().nested.1[0], u64::MAX);
    assert_eq!(
        world
            .query_filtered::<Entity, Changed<Sample>>()
            .iter(&world)
            .count(),
        0
    );
}

#[test]
fn accepted_edit_marks_changed() {
    let (mut world, entity) = sample_world();
    world.clear_trackers();
    let path = [
        PathSegment::Field("nested".into()),
        PathSegment::Index(1),
        PathSegment::Index(0),
    ];
    mutate_component(
        &mut world,
        entity,
        Sample::type_path(),
        &path,
        &Edit::Scalar(Scalar::Integer(12)),
        &ValueLimits::default(),
    )
    .unwrap();
    assert_eq!(
        world
            .query_filtered::<Entity, Changed<Sample>>()
            .iter(&world)
            .collect::<Vec<_>>(),
        vec![entity]
    );
}

#[derive(Reflect, Clone, Debug)]
#[reflect(opaque)]
struct Secret(i32);

#[derive(Component, Reflect)]
#[reflect(Component)]
struct Guarded {
    secret: Secret,
    values: bevy_platform::collections::HashMap<i32, i32>,
}

#[test]
fn opaque_and_map_ancestors_reject_edits() {
    let (mut world, _) = sample_world();
    world
        .resource::<AppTypeRegistry>()
        .write()
        .register::<Guarded>();
    let entity = world
        .spawn(Guarded {
            secret: Secret(7),
            values: bevy_platform::collections::HashMap::<i32, i32>::from_iter([(1, 9)]),
        })
        .id();
    let error = mutate_component(
        &mut world,
        entity,
        Guarded::type_path(),
        &[PathSegment::Field("secret".into()), PathSegment::Index(0)],
        &Edit::Scalar(Scalar::Integer(10)),
        &ValueLimits::default(),
    )
    .unwrap_err();
    assert_eq!(error.0, ReadOnlyReason::OpaqueAncestor.as_str());
    assert_eq!(world.get::<Guarded>(entity).unwrap().secret.0, 7);
    let error = mutate_component(
        &mut world,
        entity,
        Guarded::type_path(),
        &[PathSegment::Field("values".into()), PathSegment::Index(1)],
        &Edit::Scalar(Scalar::Integer(10)),
        &ValueLimits::default(),
    )
    .unwrap_err();
    assert_eq!(error.0, ReadOnlyReason::KindNotEditable.as_str());
    assert_eq!(world.get::<Guarded>(entity).unwrap().values[&1], 9);
}

#[derive(bevy_asset::Asset, bevy_reflect::TypePath)]
struct TestAsset;

#[test]
fn asset_handles_are_typed_readonly_ids() {
    let mut registry = TypeRegistry::default();
    registry.register::<bevy_asset::Handle<TestAsset>>();
    let handle = bevy_asset::Handle::<TestAsset>::default();
    let value = inspect(&handle, &registry, Writable::Yes, &ValueLimits::default());
    assert_eq!(
        value.type_path,
        bevy_asset::Handle::<TestAsset>::type_path()
    );
    assert_eq!(
        value.kind,
        Kind::Asset {
            id: format!("{:?}", handle.clone().untyped().id())
        }
    );
    assert_eq!(value.writable, Writable::No(ReadOnlyReason::Reference));
}

#[test]
fn immutable_and_unregistered_edits_are_rejected() {
    let (mut world, entity) = sample_world();
    world
        .entity_mut(entity)
        .insert((Unregistered, Immutable(4)));
    world
        .resource::<AppTypeRegistry>()
        .write()
        .register::<Immutable>();
    let edit = Edit::Scalar(Scalar::Integer(100));
    assert_eq!(
        mutate_component(
            &mut world,
            entity,
            Immutable::type_path(),
            &[PathSegment::Index(0)],
            &edit,
            &ValueLimits::default()
        )
        .unwrap_err()
        .0,
        "component immutable"
    );
    assert_eq!(world.get::<Immutable>(entity).unwrap().0, 4);
    assert_eq!(
        mutate_component(
            &mut world,
            entity,
            std::any::type_name::<Unregistered>(),
            &[],
            &edit,
            &ValueLimits::default()
        )
        .unwrap_err()
        .0,
        "component not registered"
    );
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct TextAndNumbers {
    flag: bool,
    text: String,
    letter: char,
    float: f64,
}

#[test]
fn scalar_kinds_round_trip() {
    let (mut world, _) = sample_world();
    world
        .resource::<AppTypeRegistry>()
        .write()
        .register::<TextAndNumbers>();
    let entity = world
        .spawn(TextAndNumbers {
            flag: false,
            text: String::new(),
            letter: 'a',
            float: 0.0,
        })
        .id();
    for (name, scalar) in [
        ("flag", Scalar::Bool(true)),
        ("text", Scalar::String("hello".into())),
        ("letter", Scalar::Char('界')),
        ("float", Scalar::Float(1.25)),
    ] {
        let accepted = mutate_component(
            &mut world,
            entity,
            TextAndNumbers::type_path(),
            &[PathSegment::Field(name.into())],
            &Edit::Scalar(scalar.clone()),
            &ValueLimits::default(),
        )
        .unwrap();
        assert_eq!(accepted.kind, Kind::Scalar(scalar));
    }
    let value = world.get::<TextAndNumbers>(entity).unwrap();
    assert!(value.flag);
    assert_eq!(value.text, "hello");
    assert_eq!(value.letter, '界');
    assert_eq!(value.float, 1.25);
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct Options {
    #[reflect(@InspectorReadOnly)]
    locked: (i32,),
    #[reflect(@InspectorRange::new(0.0, 10.0))]
    speed: f32,
}

#[test]
fn metadata_permissions_inherit_and_ranges_reject() {
    let (mut world, _) = sample_world();
    world
        .resource::<AppTypeRegistry>()
        .write()
        .register::<Options>();
    let entity = world
        .spawn(Options {
            locked: (3,),
            speed: 1.0,
        })
        .id();
    let value = read_component(
        &world,
        entity,
        Options::type_path(),
        &ValueLimits::default(),
    )
    .unwrap();
    let Kind::Struct(fields) = value.kind else {
        panic!("struct expected")
    };
    let Kind::Tuple(locked) = &fields[0].value.kind else {
        panic!("tuple expected")
    };
    assert_eq!(
        locked[0].writable,
        Writable::No(ReadOnlyReason::InspectorReadOnly)
    );
    assert_eq!(fields[1].value.range, Some(InspectorRange::new(0.0, 10.0)));
    let error = mutate_component(
        &mut world,
        entity,
        Options::type_path(),
        &[PathSegment::Field("locked".into()), PathSegment::Index(0)],
        &Edit::Scalar(Scalar::Integer(8)),
        &ValueLimits::default(),
    )
    .unwrap_err();
    assert_eq!(error.0, ReadOnlyReason::InspectorReadOnly.as_str());
    world.clear_trackers();
    let error = mutate_component(
        &mut world,
        entity,
        Options::type_path(),
        &[PathSegment::Field("speed".into())],
        &Edit::Scalar(Scalar::Float(11.0)),
        &ValueLimits::default(),
    )
    .unwrap_err();
    assert_eq!(error.0, "value outside InspectorRange");
    assert_eq!(world.get::<Options>(entity).unwrap().locked.0, 3);
    assert_eq!(world.get::<Options>(entity).unwrap().speed, 1.0);
    assert_eq!(
        world
            .query_filtered::<Entity, Changed<Options>>()
            .iter(&world)
            .count(),
        0
    );
}

#[test]
fn sync_mode_does_not_remove_writability() {
    let (mut world, entity) = sample_world();
    world.insert_resource(crate::plugins::transforms::GodotTransformConfig::one_way());
    let path = [
        PathSegment::Field("nested".into()),
        PathSegment::Index(1),
        PathSegment::Index(0),
    ];
    let accepted = mutate_component(
        &mut world,
        entity,
        Sample::type_path(),
        &path,
        &Edit::Scalar(Scalar::Integer(15)),
        &ValueLimits::default(),
    )
    .unwrap();
    assert_eq!(accepted.writable, Writable::Yes);
    assert_eq!(world.get::<Sample>(entity).unwrap().nested.1[0], 15);
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct WideRange(#[reflect(@InspectorRange::new(0.0, 18446744073709551616.0))] u128);

#[derive(Component, Reflect, Debug, PartialEq)]
#[reflect(Component)]
enum EnumOptions {
    Active {
        #[reflect(@InspectorReadOnly)]
        locked: (i32,),
        #[reflect(@InspectorRange::new(0.0, 10.0))]
        speed: f32,
    },
}

#[test]
fn struct_variant_indices_enforce_field_metadata() {
    let (mut world, _) = sample_world();
    world
        .resource::<AppTypeRegistry>()
        .write()
        .register::<EnumOptions>();
    let entity = world
        .spawn(EnumOptions::Active {
            locked: (3,),
            speed: 1.0,
        })
        .id();
    let value = read_component(
        &world,
        entity,
        EnumOptions::type_path(),
        &ValueLimits::default(),
    )
    .unwrap();
    let Kind::Enum { fields, .. } = value.kind else {
        panic!("enum expected")
    };
    let Kind::Tuple(locked) = &fields[0].value.kind else {
        panic!("tuple expected")
    };
    assert_eq!(
        locked[0].writable,
        Writable::No(ReadOnlyReason::InspectorReadOnly)
    );
    assert_eq!(fields[1].value.range, Some(InspectorRange::new(0.0, 10.0)));
    world.clear_trackers();
    for field in [PathSegment::Field("locked".into()), PathSegment::Index(0)] {
        let error = mutate_component(
            &mut world,
            entity,
            EnumOptions::type_path(),
            &[
                PathSegment::Variant("Active".into()),
                field,
                PathSegment::Index(0),
            ],
            &Edit::Scalar(Scalar::Integer(8)),
            &ValueLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.0, "field is marked InspectorReadOnly");
    }
    for field in [PathSegment::Field("speed".into()), PathSegment::Index(1)] {
        let error = mutate_component(
            &mut world,
            entity,
            EnumOptions::type_path(),
            &[PathSegment::Variant("Active".into()), field],
            &Edit::Scalar(Scalar::Float(11.0)),
            &ValueLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.0, "value outside InspectorRange");
    }
    assert_eq!(
        world.get::<EnumOptions>(entity),
        Some(&EnumOptions::Active {
            locked: (3,),
            speed: 1.0
        })
    );
    assert_eq!(
        world
            .query_filtered::<Entity, Changed<EnumOptions>>()
            .iter(&world)
            .count(),
        0
    );
    let accepted = mutate_component(
        &mut world,
        entity,
        EnumOptions::type_path(),
        &[PathSegment::Variant("Active".into()), PathSegment::Index(1)],
        &Edit::Scalar(Scalar::Float(10.0)),
        &ValueLimits::default(),
    )
    .unwrap();
    assert_eq!(accepted.kind, Kind::Scalar(Scalar::Float(10.0)));
    assert_eq!(accepted.range, Some(InspectorRange::new(0.0, 10.0)));
    assert_eq!(
        world.get::<EnumOptions>(entity),
        Some(&EnumOptions::Active {
            locked: (3,),
            speed: 10.0
        })
    );
}

#[derive(Component, Reflect, Debug, PartialEq)]
#[reflect(Component)]
struct Numeric(f32, f64, i32, u128);

#[test]
fn numeric_edits_coerce_without_truncating_or_saturating() {
    let (mut world, _) = sample_world();
    world
        .resource::<AppTypeRegistry>()
        .write()
        .register::<Numeric>();
    let entity = world.spawn(Numeric(0.0, 0.0, 0, 0)).id();
    for (index, input, expected) in [
        (0, Scalar::Integer(2), Scalar::Float(2.0)),
        (1, Scalar::Integer(2), Scalar::Float(2.0)),
        (2, Scalar::Float(3.0), Scalar::Integer(3)),
        (
            3,
            Scalar::Float(2.0_f64.powi(127)),
            Scalar::Decimal((1_u128 << 127).to_string()),
        ),
        (3, Scalar::Float(-0.0), Scalar::Integer(0)),
    ] {
        let accepted = mutate_component(
            &mut world,
            entity,
            Numeric::type_path(),
            &[PathSegment::Index(index)],
            &Edit::Scalar(input),
            &ValueLimits::default(),
        )
        .unwrap();
        assert_eq!(accepted.kind, Kind::Scalar(expected));
    }
    assert_eq!(world.get::<Numeric>(entity), Some(&Numeric(2.0, 2.0, 3, 0)));
    world.clear_trackers();
    for (index, input) in [
        (2, 3.5),
        (2, f64::NAN),
        (2, f64::INFINITY),
        (2, 2147483648.0),
        (2, -2147483649.0),
        (3, -1.0),
        (3, 2.0_f64.powi(128)),
        (0, f64::MAX),
    ] {
        let error = mutate_component(
            &mut world,
            entity,
            Numeric::type_path(),
            &[PathSegment::Index(index)],
            &Edit::Scalar(Scalar::Float(input)),
            &ValueLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.0, "decode failure");
        assert_eq!(world.get::<Numeric>(entity), Some(&Numeric(2.0, 2.0, 3, 0)));
    }
    assert_eq!(
        world
            .query_filtered::<Entity, Changed<Numeric>>()
            .iter(&world)
            .count(),
        0
    );
}

#[test]
fn wide_integer_range_does_not_round_the_value() {
    let (mut world, _) = sample_world();
    world
        .resource::<AppTypeRegistry>()
        .write()
        .register::<WideRange>();
    let entity = world.spawn(WideRange(0)).id();
    let boundary = 1_u128 << 64;
    mutate_component(
        &mut world,
        entity,
        WideRange::type_path(),
        &[PathSegment::Index(0)],
        &Edit::Scalar(Scalar::Decimal(boundary.to_string())),
        &ValueLimits::default(),
    )
    .unwrap();
    assert!(
        mutate_component(
            &mut world,
            entity,
            WideRange::type_path(),
            &[PathSegment::Index(0)],
            &Edit::Scalar(Scalar::Decimal((boundary + 1).to_string())),
            &ValueLimits::default()
        )
        .is_err()
    );
    assert_eq!(world.get::<WideRange>(entity).unwrap().0, boundary);
}

#[test]
fn set_members_and_nested_depth_marker_are_explicit() {
    let set = bevy_platform::collections::HashSet::<i32>::from_iter([1_i32, 2, 3]);
    let value = inspect(
        &set,
        &TypeRegistry::default(),
        Writable::Yes,
        &ValueLimits {
            max_depth: 8,
            max_elements: 2,
            ..Default::default()
        },
    );
    let Kind::Set { members, truncated } = value.kind else {
        panic!("set expected")
    };
    assert_eq!(truncated, 1);
    let members = members
        .into_iter()
        .map(|member| {
            let Kind::Scalar(Scalar::Integer(number)) = member.kind else {
                panic!("integer member expected")
            };
            number
        })
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(members.len(), 2);
    assert!(members.is_subset(&std::collections::HashSet::from([1, 2, 3])));
    let value = inspect(
        &vec![vec![1]],
        &TypeRegistry::default(),
        Writable::Yes,
        &ValueLimits {
            max_depth: 1,
            max_elements: 8,
            ..Default::default()
        },
    );
    assert!(
        matches!(value.kind, Kind::List { items, .. } if items[0].kind == Kind::DepthLimit && items[0].writable == Writable::No(ReadOnlyReason::DepthLimit))
    );
}

#[derive(Resource, Reflect, Debug, PartialEq)]
#[reflect(Resource)]
struct ResourceSample {
    exact: u128,
    signed: i128,
    #[reflect(@InspectorReadOnly)]
    locked: i32,
    #[reflect(@InspectorRange::new(0.0, 10.0))]
    bounded: i32,
    values: Vec<i32>,
}

fn resource_entity<R: Resource>(world: &World) -> Entity {
    world
        .resource_entities()
        .get(world.component_id::<R>().unwrap())
        .unwrap()
}

#[test]
fn reflected_resources_reuse_component_reads_edits_and_exact_integers() {
    let mut world = World::new();
    let registry = AppTypeRegistry::default();
    registry.write().register::<ResourceSample>();
    assert!(
        registry
            .read()
            .get_type_data::<bevy_ecs::reflect::ReflectComponent>(std::any::TypeId::of::<
                ResourceSample,
            >())
            .is_some(),
        "reflect(Resource) must supply ReflectComponent"
    );
    world.insert_resource(registry);
    world.insert_resource(ResourceSample {
        exact: u128::MAX,
        signed: i128::MIN,
        locked: 3,
        bounded: 5,
        values: vec![1, 2],
    });
    let entity = resource_entity::<ResourceSample>(&world);
    let limits = ValueLimits::default();
    let value = read_component(&world, entity, ResourceSample::type_path(), &limits).unwrap();
    let Kind::Struct(fields) = value.kind else {
        panic!("resource struct expected")
    };
    assert_eq!(
        fields[0].value.kind,
        Kind::Scalar(Scalar::Decimal(u128::MAX.to_string()))
    );
    assert_eq!(
        fields[1].value.kind,
        Kind::Scalar(Scalar::Decimal(i128::MIN.to_string()))
    );
    assert_eq!(fields[0].value.writable, Writable::Yes);
    assert_eq!(
        fields[2].value.writable,
        Writable::No(ReadOnlyReason::InspectorReadOnly)
    );
    assert_eq!(fields[3].value.range, Some(InspectorRange::new(0.0, 10.0)));
    world.clear_trackers();
    for (field, input, reason) in [
        (
            "exact",
            Scalar::Decimal("340282366920938463463374607431768211456".into()),
            "decode failure",
        ),
        (
            "locked",
            Scalar::Integer(9),
            "field is marked InspectorReadOnly",
        ),
        (
            "bounded",
            Scalar::Integer(11),
            "value outside InspectorRange",
        ),
    ] {
        assert_eq!(
            mutate_component(
                &mut world,
                entity,
                ResourceSample::type_path(),
                &[PathSegment::Field(field.into())],
                &Edit::Scalar(input),
                &limits
            )
            .unwrap_err()
            .0,
            reason
        );
        assert!(!world.is_resource_changed::<ResourceSample>());
    }
    for (field, text) in [
        ("exact", (u128::MAX - 1).to_string()),
        ("signed", i128::MAX.to_string()),
    ] {
        let accepted = mutate_component(
            &mut world,
            entity,
            ResourceSample::type_path(),
            &[PathSegment::Field(field.into())],
            &Edit::Scalar(Scalar::Decimal(text.clone())),
            &limits,
        )
        .unwrap();
        assert_eq!(accepted.kind, Kind::Scalar(Scalar::Decimal(text)));
    }
    assert!(world.is_resource_changed::<ResourceSample>());
    assert_eq!(world.resource::<ResourceSample>().exact, u128::MAX - 1);
    assert_eq!(world.resource::<ResourceSample>().signed, i128::MAX);
    assert_eq!(world.resource::<ResourceSample>().locked, 3);
    assert_eq!(world.resource::<ResourceSample>().bounded, 5);
    let limited = read_component(
        &world,
        entity,
        ResourceSample::type_path(),
        &ValueLimits {
            max_depth: 2,
            max_elements: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let Kind::Struct(fields) = limited.kind else {
        panic!("resource struct expected")
    };
    assert!(
        matches!(&fields[4].value.kind, Kind::List { items, truncated: 1 } if items.len() == 1)
    );
    assert_eq!(
        mutate_component(
            &mut world,
            entity,
            ResourceSample::type_path(),
            &[PathSegment::Field("values".into()), PathSegment::Index(0)],
            &Edit::Scalar(Scalar::Integer(9)),
            &ValueLimits {
                max_depth: 1,
                max_elements: 1,
                ..Default::default()
            }
        )
        .unwrap_err()
        .0,
        "maximum depth reached"
    );
    assert_eq!(world.resource::<ResourceSample>().values, [1, 2]);
}

#[derive(Resource, Reflect)]
struct UnadaptedResource(i32);

#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[component(immutable)]
struct ImmutableResource(i32);

#[test]
fn resources_keep_registration_adapter_and_storage_permissions() {
    let mut world = World::new();
    world.init_resource::<AppTypeRegistry>();
    world.insert_resource(UnadaptedResource(3));
    let entity = resource_entity::<UnadaptedResource>(&world);
    for registered in [false, true] {
        if registered {
            world
                .resource::<AppTypeRegistry>()
                .write()
                .register::<UnadaptedResource>();
        }
        let reason = if registered {
            ReadOnlyReason::NoReflectComponent
        } else {
            ReadOnlyReason::ComponentNotRegistered
        };
        let value = read_component(
            &world,
            entity,
            UnadaptedResource::type_path(),
            &ValueLimits::default(),
        )
        .unwrap();
        assert_eq!(value.writable, Writable::No(reason.clone()));
        world.clear_trackers();
        assert_eq!(
            mutate_component(
                &mut world,
                entity,
                UnadaptedResource::type_path(),
                &[PathSegment::Index(0)],
                &Edit::Scalar(Scalar::Integer(9)),
                &ValueLimits::default()
            )
            .unwrap_err()
            .0,
            reason.as_str()
        );
        assert_eq!(world.resource::<UnadaptedResource>().0, 3);
        assert!(!world.is_resource_changed::<UnadaptedResource>());
    }
    world
        .resource::<AppTypeRegistry>()
        .write()
        .register::<ImmutableResource>();
    world.insert_resource(ImmutableResource(4));
    let entity = resource_entity::<ImmutableResource>(&world);
    let value = read_component(
        &world,
        entity,
        ImmutableResource::type_path(),
        &ValueLimits::default(),
    )
    .unwrap();
    assert!(
        matches!(value.kind, Kind::TupleStruct(fields) if fields[0].writable == Writable::No(ReadOnlyReason::ComponentImmutable))
    );
    assert_eq!(
        mutate_component(
            &mut world,
            entity,
            ImmutableResource::type_path(),
            &[PathSegment::Index(0)],
            &Edit::Scalar(Scalar::Integer(9)),
            &ValueLimits::default()
        )
        .unwrap_err()
        .0,
        "component immutable"
    );
    assert_eq!(world.resource::<ImmutableResource>().0, 4);
}

mod state_wrapper_paths {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Reflect, bevy_state::prelude::States)]
    enum Mode {
        Idle,
        Active,
    }

    #[derive(Resource, Reflect)]
    #[reflect(Resource)]
    #[type_path = "bevy_state::state::resources"]
    struct State<T: Send + Sync + 'static>(T);

    #[derive(Resource, Reflect)]
    #[reflect(Resource)]
    #[type_path = "bevy_state::state::resources"]
    struct NextState<T: Send + Sync + 'static>(T);

    #[derive(Resource, Reflect)]
    #[reflect(Resource)]
    #[type_path = "bevy_state::state::resources"]
    struct PreviousState<T: Send + Sync + 'static>(T);

    #[test]
    fn state_wrapper_paths_are_read_only_without_state_adapters() {
        let mut world = World::new();
        let registry = AppTypeRegistry::default();
        registry.write().register::<State<Mode>>();
        registry.write().register::<NextState<Mode>>();
        registry.write().register::<PreviousState<Mode>>();
        world.insert_resource(registry);
        let fixtures = [
            (
                world.spawn(State(Mode::Idle)).id(),
                State::<Mode>::type_path(),
                std::any::type_name::<bevy_state::prelude::State<Mode>>(),
            ),
            (
                world.spawn(NextState(Mode::Idle)).id(),
                NextState::<Mode>::type_path(),
                std::any::type_name::<bevy_state::prelude::NextState<Mode>>(),
            ),
            (
                world.spawn(PreviousState(Mode::Idle)).id(),
                PreviousState::<Mode>::type_path(),
                std::any::type_name::<bevy_state::prelude::PreviousState<Mode>>(),
            ),
        ];
        for (entity, path, actual) in fixtures {
            assert_eq!(path, actual);
            let value = read_component(&world, entity, path, &ValueLimits::default()).unwrap();
            assert!(
                matches!(value.kind, Kind::TupleStruct(fields) if matches!(&fields[0].writable,
                    Writable::No(reason) if reason.as_str() == "state wrapper requires a transition request"))
            );
            assert_eq!(
                mutate_component(
                    &mut world,
                    entity,
                    path,
                    &[PathSegment::Index(0)],
                    &Edit::Variant("Active".into()),
                    &ValueLimits::default()
                )
                .unwrap_err()
                .0,
                "state wrapper requires a transition request"
            );
        }
        assert_eq!(world.resource::<State<Mode>>().0, Mode::Idle);
        assert_eq!(world.resource::<NextState<Mode>>().0, Mode::Idle);
        assert_eq!(world.resource::<PreviousState<Mode>>().0, Mode::Idle);
    }
}
