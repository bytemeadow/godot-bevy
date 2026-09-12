mod model;
mod pixels;

pub use model::*;

use bevy::prelude::{App, Res, Resource, Startup, Time, Transform, World};
use bevy::time::{Fixed, Real, TimeUpdateStrategy, Virtual};
use godot::classes::{ClassDb, Control, Engine, INode, Node, Node2D, RenderingServer, SceneTree};
use godot::prelude::*;
use godot_bevy::prelude::{GodotAccess, TransformSyncMetadata};
use serde_json::{Value, json};
use std::cell::{Cell, RefCell};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub struct CaptureAdapter {
    pub name: &'static str,
    pub extension: fn(&mut World, &CaptureManifest, &Value) -> Result<(), String>,
    pub scenarios: &'static [&'static str],
    pub is_settled: fn(&mut World, &CaptureManifest) -> Result<bool, String>,
    pub reset: fn(&mut World, &CaptureManifest) -> Result<(), String>,
    pub before_frame: fn(&mut World, &CaptureManifest, u32) -> Result<(), String>,
}

fn protocol() -> &'static Value {
    static PROTOCOL: OnceLock<Value> = OnceLock::new();
    PROTOCOL.get_or_init(|| serde_json::from_str(include_str!("protocol.json")).unwrap())
}

fn text(key: &str) -> &'static str {
    protocol()[key].as_str().unwrap()
}

fn environment(key: &str) -> Result<String, String> {
    let name = protocol()["env"][key].as_str().unwrap();
    std::env::var(name).map_err(|error| format!("{name}: {error}"))
}

pub fn is_enabled() -> bool {
    std::env::var_os(protocol()["env"]["scenario"].as_str().unwrap()).is_some()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    WarmUp,
    Running,
}

static RUNNING: AtomicBool = AtomicBool::new(false);

thread_local! {
    static EVIDENCE_ROOT: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static ADAPTER_EVIDENCE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

pub fn phase() -> Phase {
    if RUNNING.load(Ordering::Acquire) {
        Phase::Running
    } else {
        Phase::WarmUp
    }
}

fn set_phase(phase: Phase) {
    RUNNING.store(phase == Phase::Running, Ordering::Release);
}

/// The current adapter's evidence subdirectory, available during its callbacks.
pub fn evidence_dir() -> Option<PathBuf> {
    ADAPTER_EVIDENCE.with(|path| path.borrow().clone())
}

#[unsafe(no_mangle)]
pub extern "C" fn godot_bevy_capture_version() -> u32 {
    protocol()["version"].as_u64().unwrap() as u32
}

#[derive(Default, Resource)]
struct CaptureAdapters(Vec<CaptureAdapter>);

pub fn install(app: &mut App, adapter: CaptureAdapter) {
    if !is_enabled() {
        return;
    }
    set_phase(Phase::WarmUp);
    if !app.world().contains_resource::<CaptureAdapters>() {
        app.init_resource::<CaptureAdapters>();
        app.add_systems(Startup, |adapters: Res<CaptureAdapters>, _: GodotAccess| {
            if let Err(error) = start(adapters.0.clone()) {
                fail(error);
            }
        });
    }
    app.world_mut()
        .resource_mut::<CaptureAdapters>()
        .0
        .push(adapter);
}

fn call_adapter<T>(
    adapter: &CaptureAdapter,
    run: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let path =
        EVIDENCE_ROOT.with(|root| root.borrow().as_ref().map(|root| root.join(adapter.name)));
    let previous = ADAPTER_EVIDENCE.with(|current| current.replace(path));
    let result = run();
    ADAPTER_EVIDENCE.with(|current| current.replace(previous));
    result
}

fn configure_extensions(
    world: &mut World,
    manifest: &CaptureManifest,
    adapters: &[CaptureAdapter],
) -> Result<(), String> {
    for (name, value) in &manifest.extensions {
        let adapter = adapters
            .iter()
            .find(|adapter| adapter.name == name.as_str())
            .ok_or_else(|| format!("no installed capture adapter for extensions.{name}"))?;
        call_adapter(adapter, || (adapter.extension)(world, manifest, value))?;
    }
    Ok(())
}

fn tree() -> Gd<SceneTree> {
    Engine::singleton()
        .get_main_loop()
        .expect("Godot main loop")
        .cast::<SceneTree>()
}

fn with_world<T>(run: impl FnOnce(&mut World) -> Result<T, String>) -> Result<T, String> {
    let mut singleton = godot_bevy::BevyApp::try_singleton().ok_or("missing BevyAppSingleton")?;
    let mut binding = singleton.bind_mut();
    let app = binding.get_app_mut().ok_or("Bevy app was torn down")?;
    run(app.world_mut())
}

fn line(message: String) -> Result<(), String> {
    println!("{message}");
    std::io::stdout().flush().map_err(|error| error.to_string())
}

fn fail(error: String) {
    set_phase(Phase::WarmUp);
    let _ = line(format!("{} {}", text("error"), json!(error)));
    tree()
        .quit_ex()
        .exit_code(protocol()["exit"]["fail"].as_i64().unwrap() as i32)
        .done();
}

fn parse_manifest(contents: &str) -> Result<CaptureManifest, String> {
    let value: Value = serde_json::from_str(contents).map_err(|error| error.to_string())?;
    if value["version"] == 1 {
        return Err(text("migration").into());
    }
    serde_json::from_value(value).map_err(|error| error.to_string())
}

fn start(adapters: Vec<CaptureAdapter>) -> Result<(), String> {
    let manifest = parse_manifest(
        &std::fs::read_to_string(environment("manifest")?).map_err(|error| error.to_string())?,
    )?;
    let mut names = std::collections::BTreeSet::new();
    for adapter in &adapters {
        if adapter.name.is_empty()
            || !adapter.name.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(&byte)
            })
            || !adapter.name.as_bytes()[0].is_ascii_alphanumeric()
            || adapter.name == "rendering_2d"
            || !names.insert(adapter.name)
        {
            return Err("invalid or duplicate capture adapter name".into());
        }
    }
    if manifest.version != godot_bevy_capture_version()
        || manifest.scenario != environment("scenario")?
        || adapters
            .iter()
            .any(|adapter| !adapter.scenarios.contains(&manifest.scenario.as_str()))
        || manifest.clocks.physics_hz != protocol()["fps"].as_u64().unwrap() as u32
        || manifest.clocks.bevy_step_ns != protocol()["step_ns"].as_u64().unwrap()
        || manifest.settle.stable_frames < 2
        || manifest.checkpoints.first().map(|c| c.frame) != Some(0)
        || manifest.checkpoints.last().map(|c| c.frame) != Some(manifest.frames)
        || manifest
            .checkpoints
            .windows(2)
            .any(|c| c[0].frame >= c[1].frame)
    {
        return Err("invalid capture configuration or unsupported scenario".into());
    }
    let version = Engine::singleton().get_version_info();
    for (key, expected) in [("major", 4), ("minor", 6), ("patch", 2)] {
        if version
            .get(key)
            .and_then(|value| value.try_to::<i64>().ok())
            != Some(expected)
        {
            return Err("native capture requires Godot 4.6.2".into());
        }
    }
    let timeout = environment("timeout")?
        .parse::<f64>()
        .map_err(|error| error.to_string())?;
    if !timeout.is_finite() || !(0.0..=3600.0).contains(&timeout) || timeout == 0.0 {
        return Err("invalid capture timeout".into());
    }
    let output = PathBuf::from(environment("output")?);
    if !output.is_absolute() || !output.is_dir() {
        return Err("capture output must be an existing absolute directory".into());
    }
    for adapter in &adapters {
        std::fs::create_dir_all(output.join(adapter.name)).map_err(|error| error.to_string())?;
    }
    EVIDENCE_ROOT.with(|root| root.replace(Some(output.clone())));
    let mut engine = Engine::singleton();
    engine.set_physics_ticks_per_second(manifest.clocks.physics_hz as i32);
    engine.set_time_scale(1.0);
    let mut tree = tree();
    tree.call("set_physics_interpolation_enabled", &[false.to_variant()]);
    let mut root = tree.get_root().ok_or("missing root viewport")?;
    let size = Vector2i::new(manifest.viewport[0] as i32, manifest.viewport[1] as i32);
    root.set_size(size);
    root.set_content_scale_size(size);
    root.set_content_scale_mode(godot::classes::window::ContentScaleMode::VIEWPORT);
    let mut hook = Gd::<CaptureHook>::from_init_fn(|base| CaptureHook {
        base,
        pending_draw: false,
        state: Some(CaptureState {
            manifest,
            adapters,
            output,
            deadline: Instant::now() + Duration::from_secs_f64(timeout),
            warm_up: WarmUp::default(),
            configured: false,
            scene_id: None,
            settle_scene: None,
            scene_path: String::new(),
            awaiting_instruction: false,
            frame: None,
            physics_ticks: 0,
            ready: false,
        }),
    });
    hook.set_process_priority(i32::MAX);
    let callback = hook.callable("before_physics");
    tree.connect("physics_frame", &callback);
    tree.connect("process_frame", &hook.callable("before_process"));
    RenderingServer::singleton().connect("frame_post_draw", &hook.callable("after_draw"));
    root.add_child(&hook);
    Ok(())
}

struct CaptureState {
    manifest: CaptureManifest,
    adapters: Vec<CaptureAdapter>,
    output: PathBuf,
    deadline: Instant,
    warm_up: WarmUp,
    configured: bool,
    scene_id: Option<InstanceId>,
    settle_scene: Option<InstanceId>,
    scene_path: String,
    awaiting_instruction: bool,
    frame: Option<u32>,
    physics_ticks: u32,
    ready: bool,
}

#[derive(GodotClass)]
#[class(base=Node, internal)]
struct CaptureHook {
    base: Base<Node>,
    state: Option<CaptureState>,
    pending_draw: bool,
}

#[godot_api]
impl INode for CaptureHook {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            state: None,
            pending_draw: false,
        }
    }

    fn process(&mut self, _delta: f64) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        if state.awaiting_instruction {
            match state.poll_instruction() {
                Ok(true) => self.state = None,
                Ok(false) => {}
                Err(error) => {
                    self.state = None;
                    fail(error);
                }
            }
            return;
        }
        if self.pending_draw {
            self.state = None;
            fail("previous frame did not complete its draw".into());
            return;
        }
        match state.advance() {
            Ok(pending) => self.pending_draw = pending,
            Err(error) => {
                self.state = None;
                fail(error);
            }
        }
    }
}

#[godot_api]
impl CaptureHook {
    #[func]
    fn after_draw(&mut self) {
        if !std::mem::take(&mut self.pending_draw) {
            return;
        }
        let Some(mut state) = self.state.take() else {
            return;
        };
        let callback = self.to_gd().callable("after_draw");
        let mut server = RenderingServer::singleton();
        server.disconnect("frame_post_draw", &callback);
        match state.finish_frame() {
            Ok(false) => {
                self.state = Some(state);
                server.connect("frame_post_draw", &callback);
            }
            Ok(true) => {}
            Err(error) => fail(error),
        }
    }

    #[func]
    fn before_physics(&mut self) {
        self.before_callback(true);
    }

    #[func]
    fn before_process(&mut self) {
        self.before_callback(false);
    }
}

impl CaptureHook {
    fn before_callback(&mut self, physics: bool) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        if let Some(frame) = state.upcoming_frame(physics)
            && let Err(error) = with_world(|world| {
                for adapter in &state.adapters {
                    call_adapter(adapter, || {
                        (adapter.before_frame)(world, &state.manifest, frame)
                    })?;
                }
                Ok(())
            }) {
                self.state = None;
                fail(error);
            }
    }
}

#[derive(Default)]
struct WarmUp {
    stable: u32,
    reset_done: bool,
}

fn adapters_settled(
    world: &mut World,
    manifest: &CaptureManifest,
    adapters: &[CaptureAdapter],
) -> Result<bool, String> {
    let mut settled = true;
    for adapter in adapters {
        settled &= call_adapter(adapter, || (adapter.is_settled)(world, manifest))?;
    }
    Ok(settled)
}

fn reseed_shadows(world: &mut World) {
    for (transform, mut metadata) in world
        .query::<(&Transform, &mut TransformSyncMetadata)>()
        .iter_mut(world)
    {
        metadata.shadow = *transform;
    }
}

fn reset_scenario(
    world: &mut World,
    manifest: &CaptureManifest,
    adapters: &[CaptureAdapter],
) -> Result<(), String> {
    reset_clocks(world, manifest.clocks.bevy_step_ns);
    for adapter in adapters {
        call_adapter(adapter, || (adapter.reset)(world, manifest))?;
        reseed_shadows(world);
    }
    Ok(())
}

impl WarmUp {
    fn poll(
        &mut self,
        world: &mut World,
        manifest: &CaptureManifest,
        adapters: &[CaptureAdapter],
        mut conditions: impl FnMut() -> bool,
        before_reset: impl FnOnce(),
    ) -> Result<bool, String> {
        let ready = adapters_settled(world, manifest, adapters)? && conditions();
        self.stable = if ready { self.stable + 1 } else { 0 };
        if self.stable < manifest.settle.stable_frames {
            return Ok(false);
        }
        if !self.reset_done {
            before_reset();
            reset_scenario(world, manifest, adapters)?;
            self.reset_done = true;
            if !adapters_settled(world, manifest, adapters)? || !conditions() {
                self.stable = 0;
                return Ok(false);
            }
        }
        reset_clocks(world, manifest.clocks.bevy_step_ns);
        reseed_shadows(world);
        Ok(true)
    }
}

fn scene_conditions(manifest: &CaptureManifest) -> bool {
    let Some(scene) = tree().get_current_scene() else {
        return false;
    };
    scene.is_node_ready()
        && manifest.settle.nodes.iter().all(|path| {
            scene
                .get_node_or_null(path.as_str())
                .is_some_and(|node| node.is_node_ready())
        })
        && manifest
            .settle
            .classes
            .iter()
            .all(|class| ClassDb::singleton().class_exists(class.as_str()))
}

fn check_scene(
    ready_scene: Option<InstanceId>,
    current_scene: Option<InstanceId>,
) -> Result<(), String> {
    if ready_scene.is_some() && ready_scene != current_scene {
        return Err("scene changed during capture after READY".into());
    }
    Ok(())
}

impl CaptureState {
    fn upcoming_frame(&mut self, physics: bool) -> Option<u32> {
        if self.awaiting_instruction || !self.ready {
            return None;
        }
        let frame = self.frame?;
        if physics {
            self.physics_ticks += 1;
        }
        (physics == (self.manifest.pacing == Pacing::Fixed)).then_some(frame + 1)
    }

    fn advance(&mut self) -> Result<bool, String> {
        if Instant::now() >= self.deadline {
            return Err("capture timed out before completion".into());
        }
        if let Some(frame) = self.frame {
            check_scene(
                self.scene_id,
                tree().get_current_scene().map(|scene| scene.instance_id()),
            )?;
            if self.manifest.pacing == Pacing::Fixed && self.physics_ticks != frame + 1 {
                return Err(format!(
                    "frame {}: expected one physics tick, got {}",
                    frame + 1,
                    self.physics_ticks - frame
                ));
            }
            self.frame = Some(frame + 1);
        } else {
            if !self.configured {
                with_world(|world| configure_extensions(world, &self.manifest, &self.adapters))?;
                self.configured = true;
            }
            let ready = with_world(|world| {
                self.warm_up.poll(
                    world,
                    &self.manifest,
                    &self.adapters,
                    || {
                        let current = tree().get_current_scene().map(|scene| scene.instance_id());
                        let changed = self.settle_scene.is_some() && self.settle_scene != current;
                        self.settle_scene = current;
                        !changed && scene_conditions(&self.manifest)
                    },
                    || godot::global::seed(self.manifest.seed as i64),
                )
            })?;
            if !ready {
                return Ok(false);
            }
            self.frame = Some(0);
            self.physics_ticks = 0;
        }
        Ok(true)
    }

    fn finish_frame(&mut self) -> Result<bool, String> {
        let scene = tree().get_current_scene();
        check_scene(
            self.scene_id,
            scene.as_ref().map(|scene| scene.instance_id()),
        )?;
        if !self.ready
            && (self.settle_scene != scene.as_ref().map(|scene| scene.instance_id())
                || !scene_conditions(&self.manifest))
        {
            self.frame = None;
            self.warm_up.stable = 0;
            return Ok(false);
        }
        let scene = scene.ok_or("missing drawn scene")?;
        let frame = self.frame.unwrap();
        let (elapsed, fixed_elapsed) = with_world(|world| {
            let fixed = world.resource::<Time<Fixed>>();
            if fixed.timestep() != fixed_step() {
                return Err("fixed clock timestep drifted".into());
            }
            Ok((
                world.resource::<Time<Virtual>>().elapsed().as_nanos() as u64,
                fixed.elapsed().as_nanos() as u64,
            ))
        })?;
        if elapsed != u64::from(frame) * self.manifest.clocks.bevy_step_ns {
            return Err(format!(
                "frame {frame}: virtual clock drifted to {elapsed} ns"
            ));
        }
        if fixed_elapsed != u64::from(self.physics_ticks) * fixed_step().as_nanos() as u64 {
            return Err(format!(
                "frame {frame}: fixed clock drifted to {fixed_elapsed} ns"
            ));
        }
        if !self.ready {
            self.scene_id = Some(scene.instance_id());
            self.scene_path = scene.get_scene_file_path().to_string();
            line(format!(
                "{} scenario={} scene={} frame=0",
                text("ready"),
                self.manifest.scenario,
                self.scene_path
            ))?;
            self.ready = true;
            set_phase(Phase::Running);
        }
        if let Some(checkpoint) = self
            .manifest
            .checkpoints
            .iter()
            .find(|point| point.frame == frame)
        {
            let request = read_request(
                &self.frame_file(frame, "request"),
                self.manifest.pacing,
                self.deadline,
            )?;
            if request != json!({"version": godot_bevy_capture_version(), "frame": frame}) {
                return Err(format!("frame {frame}: invalid capture request"));
            }
            let facts = self.capture(checkpoint, &scene, elapsed, fixed_elapsed)?;
            line(format!("{} frame={} {}", text("facts"), frame, facts))?;
        }
        if frame == self.manifest.frames {
            set_phase(Phase::WarmUp);
            self.awaiting_instruction = true;
            if self.manifest.pacing == Pacing::Fixed {
                let instruction = read_request(
                    &self.output.join(text("instruction_file")),
                    Pacing::Fixed,
                    self.deadline,
                )?;
                self.finish_instruction(instruction)?;
                return Ok(true);
            }
            if self.poll_instruction()? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn finish_instruction(&self, instruction: Value) -> Result<(), String> {
        let code = instruction_code(&instruction, &self.manifest.scenario)?;
        line(format!("{} exit={code}", text("ack")))?;
        tree().quit_ex().exit_code(code).done();
        Ok(())
    }

    fn poll_instruction(&self) -> Result<bool, String> {
        let path = self.output.join(text("instruction_file"));
        match std::fs::read_to_string(&path) {
            Ok(contents) => {
                self.finish_instruction(
                    serde_json::from_str(&contents).map_err(|error| error.to_string())?,
                )?;
                Ok(true)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if Instant::now() >= self.deadline {
                    Err("timed out waiting for capture instruction".into())
                } else {
                    Ok(false)
                }
            }
            Err(error) => Err(error.to_string()),
        }
    }

    fn frame_file(&self, frame: u32, kind: &str) -> PathBuf {
        self.output.join(format!(
            "{}{:0width$}{}",
            text("frame_prefix"),
            frame,
            text(&format!("{kind}_suffix")),
            width = protocol()["frame_digits"].as_u64().unwrap() as usize,
        ))
    }

    fn capture(
        &self,
        checkpoint: &Checkpoint,
        scene: &Gd<Node>,
        elapsed: u64,
        fixed_elapsed: u64,
    ) -> Result<Value, String> {
        let drawn = Rc::new(Cell::new(false));
        let observed = drawn.clone();
        let callback = Callable::from_fn("capture_post_draw", move |_| observed.set(true));
        let mut server = RenderingServer::singleton();
        let connected = server.connect("frame_post_draw", &callback);
        if connected != godot::global::Error::OK {
            return Err(format!("cannot observe frame_post_draw: {connected:?}"));
        }
        // The idle editor does not redraw without input.
        server.force_draw();
        server.disconnect("frame_post_draw", &callback);
        if !drawn.get() {
            return Err("force_draw did not complete frame_post_draw".into());
        }
        let root = tree().get_root().ok_or("missing root viewport")?;
        let image = root
            .get_texture()
            .and_then(|texture| texture.get_image())
            .ok_or("missing viewport image")?;
        if [image.get_width() as u32, image.get_height() as u32] != self.manifest.viewport {
            return Err("viewport image has the wrong dimensions".into());
        }
        let path = self.frame_file(checkpoint.frame, "png");
        let temporary = path.with_extension("png.tmp");
        let error = image.save_png(temporary.to_str().ok_or("non-UTF-8 PNG path")?);
        if error != godot::global::Error::OK {
            return Err(format!("PNG capture: {error:?}"));
        }
        std::fs::rename(temporary, path).map_err(|error| error.to_string())?;
        let nodes: Vec<Value> = checkpoint
            .nodes
            .iter()
            .map(|expected| {
                let node = scene.get_node_or_null(expected.path.as_str());
                let position = node.as_ref().and_then(|node| {
                    if let Ok(node) = node.clone().try_cast::<Node2D>() {
                        let position = node.get_position();
                        Some([position.x, position.y])
                    } else if let Ok(node) = node.clone().try_cast::<Control>() {
                        let position = node.get_position();
                        Some([position.x, position.y])
                    } else {
                        None
                    }
                });
                let visible = node
                    .and_then(|node| node.try_cast::<godot::classes::CanvasItem>().ok())
                    .map(|node| node.is_visible_in_tree());
                json!({"path": expected.path, "position": position, "visible": visible})
            })
            .collect();
        let mut regions = Vec::new();
        for region in &checkpoint.regions {
            let [x, y, width, height] = region.rect;
            if width == 0
                || height == 0
                || u64::from(x) + u64::from(width) > u64::from(self.manifest.viewport[0])
                || u64::from(y) + u64::from(height) > u64::from(self.manifest.viewport[1])
            {
                return Err(format!("region {} is outside the viewport", region.name));
            }
            let pixels = (y..y + height).flat_map(|row| {
                let image = &image;
                (x..x + width).map(move |column| {
                    let colour = image.get_pixel(column as i32, row as i32);
                    [colour.r, colour.g, colour.b]
                        .map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8)
                })
            });
            let (fraction, dominant) = pixels::summarize(
                pixels,
                region.non_blank.background,
                region.non_blank.tolerance,
            );
            regions.push(json!({
                "name": region.name, "rect": region.rect,
                "non_blank_fraction": fraction, "dominant_colour": dominant,
            }));
        }
        Ok(json!({
            "version": godot_bevy_capture_version(), "scenario": self.manifest.scenario, "scene": self.scene_path,
            "frame": checkpoint.frame, "viewport": self.manifest.viewport,
            "physics_ticks": self.physics_ticks, "virtual_elapsed_ns": elapsed,
            "fixed_elapsed_ns": fixed_elapsed, "fixed_step_ns": fixed_step().as_nanos() as u64,
            "nodes": nodes, "regions": regions,
            "configuration": {
                "godot": Engine::singleton().get_version_info().get("string").map(|v| v.to_string()),
                "os": godot::classes::Os::singleton().get_name().to_string(),
                "renderer": RenderingServer::singleton().call("get_current_rendering_method", &[]).to_string(),
                "gpu": RenderingServer::singleton().get_video_adapter_name().to_string(),
            },
        }))
    }
}

fn fixed_step() -> Duration {
    Duration::from_nanos(protocol()["fixed_step_ns"].as_u64().unwrap())
}

fn read_request(path: &Path, pacing: Pacing, deadline: Instant) -> Result<Value, String> {
    loop {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                return serde_json::from_str(&contents).map_err(|error| error.to_string());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if pacing == Pacing::Realtime {
                    return Err(format!("missing realtime request: {}", path.display()));
                }
            }
            Err(error) => return Err(error.to_string()),
        }
        if Instant::now() >= deadline {
            return Err(format!("timed out waiting for {}", path.display()));
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn instruction_code(instruction: &Value, scenario: &str) -> Result<i32, String> {
    let code = instruction["exit_code"]
        .as_i64()
        .ok_or("missing instruction exit code")?;
    if instruction["version"] != godot_bevy_capture_version()
        || instruction["scenario"] != scenario
        || !(0..=1).contains(&code)
        || instruction.as_object().map(|object| object.len()) != Some(3)
    {
        return Err("invalid capture instruction".into());
    }
    Ok(code as i32)
}

fn reset_clocks(world: &mut World, step_ns: u64) {
    let step = Duration::from_nanos(step_ns);
    let mut real = Time::<Real>::default();
    real.update_with_duration(Duration::ZERO);
    world.insert_resource(real);
    world.insert_resource(Time::<Virtual>::default());
    world.insert_resource(Time::<Fixed>::from_duration(fixed_step()));
    world.insert_resource(Time::<()>::default());
    world.insert_resource(TimeUpdateStrategy::ManualDuration(step));
}

#[cfg(test)]
mod tests;
