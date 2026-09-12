use super::*;

#[test]
fn reset_primes_the_first_manual_tick_and_clears_elapsed_time() {
    let mut world = World::new();
    reset_clocks(&mut world, 16_666_667);
    let step = Duration::from_nanos(16_666_667);
    world
        .resource_mut::<Time<Real>>()
        .update_with_duration(step);
    assert_eq!(world.resource::<Time<Real>>().delta(), step);
    world.resource_mut::<Time<Virtual>>().advance_by(step);
    reset_clocks(&mut world, 16_666_667);
    assert_eq!(world.resource::<Time<Virtual>>().elapsed(), Duration::ZERO);
    assert_eq!(world.resource::<Time<Fixed>>().elapsed(), Duration::ZERO);
    assert_eq!(world.resource::<Time>().elapsed(), Duration::ZERO);
}

#[test]
fn an_empty_region_is_not_non_blank() {
    let (fraction, dominant) = pixels::summarize([[0, 0, 0]; 4], [0, 0, 0], 2);
    assert_eq!(fraction, 0.0);
    assert_eq!(dominant, [0, 0, 0]);
}

#[test]
fn non_blank_uses_the_declared_background_tolerance() {
    let (fraction, dominant) =
        pixels::summarize([[2, 0, 0], [3, 0, 0], [3, 0, 0], [0, 0, 0]], [0, 0, 0], 2);
    assert_eq!(fraction, 0.5);
    assert_eq!(dominant, [3, 0, 0]);
}

fn manifest() -> CaptureManifest {
    parse_manifest(include_str!(
        "../../../examples/simple-node2d-movement/capture/orbit.json"
    ))
    .unwrap()
}

fn adapter() -> CaptureAdapter {
    CaptureAdapter {
        name: "sample",
        scenarios: &["orbit"],
        extension: |_, _, _| Ok(()),
        is_settled: |_, _| Ok(true),
        reset: |_, _| Ok(()),
        before_frame: |_, _, _| Ok(()),
    }
}

#[test]
fn reset_reseeds_every_transform_shadow_before_the_first_write() {
    let mut world = World::new();
    let stale = Transform::from_xyz(99.98611, 1.66659, 0.0);
    for x in [100.0, 600.0] {
        world.spawn((
            Transform::from_xyz(x, 0.0, 0.0),
            TransformSyncMetadata {
                shadow: stale,
                written_once: true,
            },
        ));
    }
    let mut adapter = adapter();
    adapter.reset = |world, _| {
        for mut transform in world.query::<&mut Transform>().iter_mut(world) {
            transform.translation.y = 1.0;
        }
        Ok(())
    };
    reset_scenario(&mut world, &manifest(), &[adapter]).unwrap();
    for (transform, metadata) in world
        .query::<(&Transform, &TransformSyncMetadata)>()
        .iter(&world)
    {
        assert_eq!(metadata.shadow, *transform);
        assert_ne!(metadata.shadow, stale);
        assert!(metadata.written_once);
    }
}

#[test]
fn scenario_systems_stay_idle_until_warm_up_completes() {
    use bevy::prelude::{IntoScheduleConfigs, Resource, Update};
    #[derive(Resource, Default)]
    struct Ticks(u32);
    let mut app = App::new();
    app.init_resource::<Ticks>();
    app.add_systems(
        Update,
        (|mut ticks: bevy::prelude::ResMut<Ticks>| ticks.0 += 1)
            .run_if(|| phase() == Phase::Running),
    );
    set_phase(Phase::WarmUp);
    for _ in 0..61 {
        app.update();
    }
    assert_eq!(app.world().resource::<Ticks>().0, 0);
    set_phase(Phase::Running);
    app.update();
    assert_eq!(app.world().resource::<Ticks>().0, 1);
    set_phase(Phase::WarmUp);
}

#[test]
fn settlement_calls_adapter_before_scene_conditions_even_on_first_frame() {
    use bevy::prelude::Resource;
    #[derive(Resource, Default)]
    struct Calls(u32);
    let mut world = World::new();
    world.init_resource::<Calls>();
    let mut adapter = adapter();
    adapter.is_settled = |world, _| {
        world.resource_mut::<Calls>().0 += 1;
        Ok(false)
    };
    let mut warm_up = WarmUp::default();
    assert!(
        !warm_up
            .poll(
                &mut world,
                &manifest(),
                &[adapter],
                || panic!("scene checked before adapter was ready"),
                || {}
            )
            .unwrap()
    );
    assert_eq!(world.resource::<Calls>().0, 1);
    adapter.is_settled = |world, _| {
        world.resource_mut::<Calls>().0 += 1;
        Ok(true)
    };
    assert!(
        !warm_up
            .poll(&mut world, &manifest(), &[adapter], || false, || {})
            .unwrap()
    );
    assert_eq!(world.resource::<Calls>().0, 2);
}

#[test]
fn queued_audio_reset_drains_before_frame_zero_without_resetting_twice() {
    use bevy::prelude::Resource;
    #[derive(Resource, Default)]
    struct Audio {
        queued: bool,
        resets: u32,
    }
    let mut world = World::new();
    world.init_resource::<Audio>();
    let mut adapter = adapter();
    adapter.reset = |world, _| {
        let mut audio = world.resource_mut::<Audio>();
        audio.queued = true;
        audio.resets += 1;
        Ok(())
    };
    adapter.is_settled = |world, _| Ok(!world.resource::<Audio>().queued);
    let manifest = manifest();
    let mut warm_up = WarmUp::default();
    for _ in 0..manifest.settle.stable_frames + 2 {
        assert!(
            !warm_up
                .poll(&mut world, &manifest, &[adapter], || true, || {})
                .unwrap()
        );
    }
    assert_eq!(world.resource::<Audio>().resets, 1);
    world.resource_mut::<Audio>().queued = false;
    for _ in 1..manifest.settle.stable_frames {
        assert!(
            !warm_up
                .poll(&mut world, &manifest, &[adapter], || true, || {})
                .unwrap()
        );
    }
    assert!(
        warm_up
            .poll(&mut world, &manifest, &[adapter], || true, || {})
            .unwrap()
    );
    assert_eq!(world.resource::<Audio>().resets, 1);
}

#[test]
fn extensions_reach_only_the_named_adapter_without_interpretation() {
    let mut world = World::new();
    let mut manifest = manifest();
    manifest
        .extensions
        .insert("sample".into(), json!([null, {"unknown": [1, true]}]));
    let mut adapter = adapter();
    adapter.extension = |_, _, value| {
        assert_eq!(value, &json!([null, {"unknown": [1, true]}]));
        Ok(())
    };
    configure_extensions(&mut world, &manifest, &[adapter]).unwrap();
    manifest.extensions.insert("absent".into(), json!({}));
    assert!(configure_extensions(&mut world, &manifest, &[adapter]).is_err());
}

#[test]
fn manifest_v1_rejection_names_extensions_and_pacing() {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../examples/simple-node2d-movement/capture/orbit.json"
    ))
    .unwrap();
    value["version"] = json!(1);
    let error = parse_manifest(&value.to_string()).err().unwrap();
    assert!(error.contains("extensions"));
    assert!(error.contains("pacing"));
}

#[test]
fn fixed_and_virtual_clocks_are_distinct() {
    let mut world = World::new();
    reset_clocks(&mut world, 16_666_667);
    assert_eq!(
        world.resource::<Time<Fixed>>().timestep(),
        Duration::from_nanos(16_666_668)
    );
    assert_eq!(
        fixed_step(),
        Duration::from_secs_f64(f64::from(1.0_f32 / 60.0))
    );
}

#[test]
fn realtime_missing_request_fails_without_waiting() {
    let path = std::env::temp_dir().join(format!("missing-capture-request-{}", std::process::id()));
    let error = read_request(
        &path,
        Pacing::Realtime,
        Instant::now() + Duration::from_secs(60),
    )
    .unwrap_err();
    assert!(error.contains("missing realtime request"));
}

#[test]
fn instruction_requires_v2_and_only_run_exit_codes() {
    for code in [0, 1] {
        assert_eq!(
            instruction_code(
                &json!({"version": 2, "scenario": "orbit", "exit_code": code}),
                "orbit"
            )
            .unwrap(),
            code
        );
    }
    for value in [
        json!({"version": 1, "scenario": "orbit", "exit_code": 0}),
        json!({"version": 2, "scenario": "orbit", "exit_code": 2}),
        json!({"version": 2, "scenario": "wrong", "exit_code": 0}),
    ] {
        assert!(instruction_code(&value, "orbit").is_err());
    }
}

#[test]
fn capture_feature_exports_the_protocol_version() {
    assert_eq!(godot_bevy_capture_version(), 2);
}

#[test]
fn scene_traversal_is_allowed_until_ready_then_identity_is_locked() {
    let menu = InstanceId::from_i64(1);
    let level = InstanceId::from_i64(2);
    assert!(check_scene(None, None).is_ok());
    assert!(check_scene(None, Some(menu)).is_ok());
    assert!(check_scene(None, Some(level)).is_ok());
    assert!(check_scene(Some(level), Some(level)).is_ok());
    assert!(check_scene(Some(level), Some(menu)).is_err());
    assert!(check_scene(Some(level), None).is_err());
}

#[test]
fn evidence_path_is_scoped_to_each_adapter_callback() {
    let root = std::env::temp_dir().join("capture-evidence");
    EVIDENCE_ROOT.with(|path| path.replace(Some(root.clone())));
    assert_eq!(evidence_dir(), None);
    call_adapter(&adapter(), || {
        assert_eq!(evidence_dir(), Some(root.join("sample")));
        Ok(())
    })
    .unwrap();
    assert_eq!(evidence_dir(), None);
    EVIDENCE_ROOT.with(|path| path.replace(None));
}

#[test]
fn realtime_cues_run_once_before_process_even_without_a_physics_tick() {
    let mut state = CaptureState {
        manifest: manifest(),
        adapters: vec![adapter()],
        output: PathBuf::new(),
        deadline: Instant::now() + Duration::from_secs(1),
        warm_up: WarmUp::default(),
        configured: true,
        scene_id: None,
        settle_scene: None,
        scene_path: "res://main.tscn".into(),
        awaiting_instruction: false,
        frame: Some(0),
        physics_ticks: 0,
        ready: true,
    };
    state.manifest.pacing = Pacing::Realtime;
    assert_eq!(state.upcoming_frame(false), Some(1));
    assert_eq!(state.physics_ticks, 0);
    state.frame = Some(1);
    assert_eq!(state.upcoming_frame(true), None);
    assert_eq!(state.upcoming_frame(true), None);
    assert_eq!(state.upcoming_frame(false), Some(2));
    assert_eq!(state.physics_ticks, 2);
    state.awaiting_instruction = true;
    assert_eq!(state.upcoming_frame(false), None);
}
