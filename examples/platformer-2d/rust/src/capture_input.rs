use crate::components::Player;
use crate::gameplay::player::{PlayerPlugin, PlayerSystemSet};
use bevy::prelude::*;
use godot::classes::{
    AnimatedSprite2D, Button, Camera2D, CanvasItem, CanvasLayer, CharacterBody2D, ColorRect,
    Engine, INode, Input, InputEvent, InputEventJoypadButton, InputEventJoypadMotion,
    InputEventKey, InputEventMouseButton, InputEventMouseMotion, InputMap, Label, Node, Node2D,
    PackedScene, RenderingServer, ResourceLoader, SceneTree, control::MouseFilter,
    node::ProcessMode,
};
use godot::global::Key;
use godot::prelude::*;
use godot_bevy::prelude::{GodotAccess, GodotActions, GodotActionsPlugin, GodotDefaultPlugins};
use godot_bevy_test::capture::{self, CaptureAdapter, CaptureManifest, Pacing};
use serde::Deserialize;
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::rc::Rc;
use std::time::Instant;

#[derive(Clone, Deserialize)]
struct Step {
    device: String,
    prompt: String,
    expected: Value,
    deadline_seconds: f64,
}

#[derive(Clone, Deserialize)]
struct Cue {
    frame: u32,
    action: String,
    binding: String,
    code: i32,
    pressed: bool,
}

#[derive(Deserialize)]
struct PhysicalConfig {
    actions: Vec<String>,
    steps: Vec<Step>,
}

#[derive(Deserialize)]
struct ReplayConfig {
    actions: Vec<String>,
    trace: Vec<Cue>,
    max_latency_frames: u32,
}

struct Prompt {
    index: usize,
    step: Step,
    shown_ns: Option<u64>,
    candidate: Option<Value>,
    previous_device: Option<i64>,
}

struct Recording {
    origin: Instant,
    actions: Vec<String>,
    prompt: Option<Prompt>,
    prompts: Vec<Value>,
    events: Vec<Value>,
    seen: BTreeSet<String>,
    held: BTreeMap<String, i64>,
    start_rect: Rect2,
    start_clicked: bool,
    draw_connected: bool,
}

impl Recording {
    fn now(&self) -> u64 {
        self.origin.elapsed().as_nanos() as u64
    }
}

struct Session {
    name: &'static str,
    recording: Rc<RefCell<Recording>>,
    steps: Vec<Step>,
    trace: Vec<Cue>,
    next_step: usize,
    frame: u32,
    frames: u32,
    label: Option<Gd<Label>>,
    injections: Vec<Value>,
}

#[derive(Resource, Default)]
struct InputGameplay(bool);

pub fn is_selected() -> bool {
    matches!(
        std::env::var("GODOT_BEVY_CAPTURE").as_deref(),
        Ok("devices" | "input-replay")
    )
}

pub fn build(app: &mut App) {
    app.add_plugins((GodotDefaultPlugins, GodotActionsPlugin, PlayerPlugin))
        .init_resource::<InputGameplay>()
        .configure_sets(
            FixedUpdate,
            (
                PlayerSystemSet::InputDetection,
                PlayerSystemSet::Movement,
                PlayerSystemSet::Animation,
            )
                .run_if(|game: Res<InputGameplay>| {
                    game.0 && capture::phase() == capture::Phase::Running
                }),
        )
        .add_systems(
            PostUpdate,
            record_frame.run_if(|| capture::phase() == capture::Phase::Running),
        );
    let physical = std::env::var("GODOT_BEVY_CAPTURE").as_deref() == Ok("devices");
    capture::install(
        app,
        CaptureAdapter {
            name: if physical {
                "physical_input"
            } else {
                "synthetic_input"
            },
            extension: if physical {
                configure_physical
            } else {
                configure_replay
            },
            scenarios: if physical {
                &["devices"]
            } else {
                &["input-replay"]
            },
            is_settled,
            reset,
            before_frame,
        },
    );
}

fn configure_physical(
    world: &mut World,
    manifest: &CaptureManifest,
    value: &Value,
) -> Result<(), String> {
    let config: PhysicalConfig =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    let seconds: f64 = config.steps.iter().map(|step| step.deadline_seconds).sum();
    if manifest.pacing != Pacing::Realtime || f64::from(manifest.frames) < 60.0 * (seconds + 10.0) {
        return Err(
            "devices requires realtime pacing and N >= 60 * (sum of deadlines + 10)".into(),
        );
    }
    configure(
        world,
        manifest,
        "physical_input",
        config.actions,
        config.steps,
        Vec::new(),
    )
}

fn configure_replay(
    world: &mut World,
    manifest: &CaptureManifest,
    value: &Value,
) -> Result<(), String> {
    let config: ReplayConfig =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    if manifest.pacing != Pacing::Fixed
        || config
            .trace
            .iter()
            .any(|cue| cue.frame + config.max_latency_frames > manifest.frames)
    {
        return Err(
            "input-replay requires fixed pacing and room for every observation window".into(),
        );
    }
    let mut input_map = InputMap::singleton();
    for cue in &config.trace {
        let bound = input_map
            .action_get_events(cue.action.as_str())
            .iter_shared()
            .any(|event| {
                event
                    .try_cast::<InputEventKey>()
                    .is_ok_and(|key| match cue.binding.as_str() {
                        "keycode" => key.get_keycode().ord() == cue.code,
                        "physical_keycode" => key.get_physical_keycode().ord() == cue.code,
                        _ => false,
                    })
            });
        if !bound {
            return Err(format!(
                "{} has no {} binding for {}",
                cue.action, cue.binding, cue.code
            ));
        }
    }
    configure(
        world,
        manifest,
        "synthetic_input",
        config.actions,
        Vec::new(),
        config.trace,
    )
}

fn configure(
    world: &mut World,
    manifest: &CaptureManifest,
    name: &'static str,
    actions: Vec<String>,
    steps: Vec<Step>,
    trace: Vec<Cue>,
) -> Result<(), String> {
    if manifest.extensions.len() != 1 || !manifest.extensions.contains_key(name) {
        return Err(
            "input scenarios require exactly their own extension; devices cannot inject input"
                .into(),
        );
    }
    if actions
        .iter()
        .any(|action| !InputMap::singleton().has_action(action.as_str()))
    {
        return Err("input capture names an action absent from the platformer InputMap".into());
    }
    world.insert_non_send(Session {
        name,
        recording: Rc::new(RefCell::new(Recording {
            origin: Instant::now(),
            actions,
            prompt: None,
            prompts: Vec::new(),
            events: Vec::new(),
            seen: BTreeSet::new(),
            held: BTreeMap::new(),
            start_rect: Rect2::default(),
            start_clicked: false,
            draw_connected: false,
        })),
        steps,
        trace,
        next_step: 0,
        frame: 0,
        frames: manifest.frames,
        label: None,
        injections: Vec::new(),
    });
    Ok(())
}

fn tree() -> Gd<SceneTree> {
    Engine::singleton()
        .get_main_loop()
        .expect("Godot main loop")
        .cast::<SceneTree>()
}

fn is_settled(world: &mut World, _: &CaptureManifest) -> Result<bool, String> {
    let Some(mut scene) = tree().get_current_scene() else {
        return Ok(false);
    };
    let Some(mut session) = world.get_non_send_mut::<Session>() else {
        return Err("input scenario requires its extension".into());
    };
    if session.label.is_none() {
        let Some(button) = scene.get_node_or_null("Options/StartButton") else {
            return Ok(false);
        };
        let mut button = button.cast::<Button>();
        let packed = ResourceLoader::singleton()
            .load("res://scenes/levels/level_1.tscn")
            .ok_or("missing platformer first level")?
            .cast::<PackedScene>();
        let mut level = packed
            .instantiate()
            .ok_or("cannot instantiate platformer first level")?
            .cast::<Node2D>();
        level
            .get_node_as::<Camera2D>("Player/Camera2D")
            .set_enabled(false);
        level.set_visible(false);
        level.set_process_mode(ProcessMode::DISABLED);
        scene.add_child(&level);

        let mut overlay = CanvasLayer::new_alloc();
        overlay.set_name("InputOverlay");
        overlay.set_layer(100);
        let mut panel = ColorRect::new_alloc();
        panel.set_name("Panel");
        panel.set_color(Color::BLACK);
        panel.set_size(Vector2::new(1152.0, 112.0));
        panel.set_mouse_filter(MouseFilter::IGNORE);
        overlay.add_child(&panel);
        let mut label = Label::new_alloc();
        label.set_name("Prompt");
        label.set_position(Vector2::new(32.0, 32.0));
        label.set_size(Vector2::new(1088.0, 64.0));
        label.set_mouse_filter(MouseFilter::IGNORE);
        label.set_text("Input capture is preparing. Release all controls.");
        overlay.add_child(&label);
        scene.add_child(&overlay);
        session.label = Some(label);

        let mut recorder = InputCaptureRecorder::new_alloc();
        recorder.bind_mut().recording = Some(session.recording.clone());
        scene.add_child(&recorder);
        let recording = session.recording.clone();
        let connected = button.connect(
            "pressed",
            &Callable::from_fn("input_start", move |_| {
                if capture::phase() == capture::Phase::Running {
                    recording.borrow_mut().start_clicked = true;
                }
            }),
        );
        if connected != godot::global::Error::OK {
            return Err(format!(
                "cannot connect platformer Start button: {connected:?}"
            ));
        }
        button.set_focus_mode(godot::classes::control::FocusMode::NONE);
        scene
            .get_node_as::<Button>("Options/FullscreenButton")
            .set_disabled(true);
        scene
            .get_node_as::<Button>("Options/QuitButton")
            .set_disabled(true);
        return Ok(false);
    }
    let button = scene.get_node_as::<Button>("Options/StartButton");
    let sprite = scene.get_node_as::<AnimatedSprite2D>("Level1/Player/AnimatedSprite2D");
    let ready = button.is_node_ready()
        && button.get_size().x > 0.0
        && sprite
            .get_sprite_frames()
            .and_then(|frames| frames.get_frame_texture("idle", 0))
            .is_some_and(|texture| texture.get_width() > 0)
        && session.recording.borrow().draw_connected
        && session
            .recording
            .borrow()
            .actions
            .iter()
            .all(|action| !Input::singleton().is_action_pressed(action.as_str()));
    Ok(ready
        && world
            .query_filtered::<Entity, With<Player>>()
            .iter(world)
            .count()
            == 1)
}

fn reset(world: &mut World, _: &CaptureManifest) -> Result<(), String> {
    let mut session = world
        .get_non_send_mut::<Session>()
        .ok_or("missing input session")?;
    let scene = tree()
        .get_current_scene()
        .ok_or("missing platformer scene")?;
    let mut body = scene.get_node_as::<CharacterBody2D>("Level1/Player");
    body.set_velocity(Vector2::ZERO);
    session.next_step = 0;
    session.frame = 0;
    session.injections.clear();
    let mut recording = session.recording.borrow_mut();
    recording.origin = Instant::now();
    recording.prompt = None;
    recording.prompts.clear();
    recording.events.clear();
    recording.seen.clear();
    recording.held.clear();
    recording.start_clicked = session.name == "synthetic_input";
    recording.start_rect = scene
        .get_node_as::<Button>("Options/StartButton")
        .get_global_rect();
    Engine::singleton().set_max_fps(60);
    Ok(())
}

fn before_frame(world: &mut World, _: &CaptureManifest, frame: u32) -> Result<(), String> {
    let mut session = world
        .get_non_send_mut::<Session>()
        .ok_or("missing input session")?;
    session.frame = frame;
    if session.name == "physical_input" {
        if session.recording.borrow().prompt.is_none() && session.next_step < session.steps.len() {
            let step = session.steps[session.next_step].clone();
            let text = format!(
                "{}/{}: {}\nComplete within {} seconds.",
                session.next_step + 1,
                session.steps.len(),
                step.prompt,
                step.deadline_seconds
            );
            session
                .label
                .as_mut()
                .ok_or("missing input prompt")?
                .set_text(text.as_str());
            let mut recording = session.recording.borrow_mut();
            let previous_device = recording.held.get(&control(&step)).copied();
            recording.prompt = Some(Prompt {
                index: session.next_step,
                step,
                shown_ns: None,
                candidate: None,
                previous_device,
            });
        }
        return Ok(());
    }
    let cues: Vec<(usize, Cue)> = session
        .trace
        .iter()
        .enumerate()
        .filter(|(_, cue)| cue.frame == frame)
        .map(|(index, cue)| (index, cue.clone()))
        .collect();
    session
        .label
        .as_mut()
        .ok_or("missing input prompt")?
        .set_text("Replaying recorded keyboard input. Leave the controls untouched.");
    for (index, cue) in cues {
        let mut event = InputEventKey::new_gd();
        let key = Key::from_ord(cue.code);
        match cue.binding.as_str() {
            "keycode" => event.set_keycode(key),
            "physical_keycode" => event.set_physical_keycode(key),
            _ => return Err("unsupported replay key binding".into()),
        }
        event.set_pressed(cue.pressed);
        event.set_echo(false);
        event.set_device(0);
        session.injections.push(json!({
            "index": index, "frame": frame, "action": cue.action, "binding": cue.binding,
            "code": cue.code, "pressed": cue.pressed,
        }));
        Input::singleton().parse_input_event(&event);
    }
    Ok(())
}

fn record_frame(
    mut session: NonSendMut<Session>,
    actions: Res<GodotActions>,
    mut game: ResMut<InputGameplay>,
    _: GodotAccess,
) {
    let shared = session.recording.clone();
    let mut recording = shared.borrow_mut();
    let now = recording.now();
    let snapshot: serde_json::Map<String, Value> = recording
        .actions
        .iter()
        .map(|action| {
            (
                action.clone(),
                json!({
                    "pressed": actions.pressed(action.as_str()),
                    "just_pressed": actions.just_pressed(action.as_str()),
                    "just_released": actions.just_released(action.as_str()),
                    "strength": actions.strength(action.as_str()),
                }),
            )
        })
        .collect();
    let mut results = Vec::new();
    if let Some(prompt) = &recording.prompt
        && let Some(shown) = prompt.shown_ns
    {
        let deadline = shown + (prompt.step.deadline_seconds * 1_000_000_000.0).round() as u64;
        let action_observed = prompt.step.expected["action"]
            .as_str()
            .is_none_or(|action| {
                let pressed = prompt.step.expected["state"] == "pressed";
                snapshot[action]["pressed"] == pressed
                    && snapshot[action][if pressed {
                        "just_pressed"
                    } else {
                        "just_released"
                    }] == true
            });
        let success = prompt.candidate.is_some() && action_observed;
        if success || now > deadline {
            results.push(json!({
                "step": prompt.index, "time_ns": now,
                "status": if success { "success" } else { "timeout" },
            }));
            if success {
                let key = control(&prompt.step);
                let device = prompt.candidate.as_ref().unwrap()["device_id"]
                    .as_i64()
                    .unwrap();
                if is_release(&prompt.step) {
                    recording.held.remove(&key);
                } else {
                    recording.held.insert(key, device);
                }
            }
            recording.prompt = None;
            session.next_step += 1;
        }
    }
    let start = recording.start_clicked && !game.0;
    let mut facts = json!({
        "version": 1, "frame": session.frame, "time_ns": now,
        "events": std::mem::take(&mut recording.events),
        "actions": snapshot,
        "started": recording.start_clicked,
    });
    if session.name == "physical_input" {
        facts["prompts"] = json!(std::mem::take(&mut recording.prompts));
        facts["results"] = json!(results);
        facts["finished"] = json!(session.next_step == session.steps.len());
        if session.frame == session.frames {
            let requested: BTreeSet<&str> = session
                .steps
                .iter()
                .map(|step| step.device.as_str())
                .collect();
            facts["missing_devices"] = json!(
                requested
                    .into_iter()
                    .filter(|device| !recording.seen.contains(*device))
                    .collect::<Vec<_>>()
            );
        }
    } else {
        facts["injections"] = json!(std::mem::take(&mut session.injections));
    }
    drop(recording);
    if start {
        let scene = tree()
            .get_current_scene()
            .expect("platformer capture scene");
        for path in ["Options", "TitleLabel", "Label"] {
            scene.get_node_as::<CanvasItem>(path).set_visible(false);
        }
        let mut level = scene.get_node_as::<Node2D>("Level1");
        level.set_visible(true);
        level.set_process_mode(ProcessMode::INHERIT);
        level
            .get_node_as::<Camera2D>("Player/Camera2D")
            .set_enabled(true);
        game.0 = true;
    }
    if session.name == "physical_input" && session.next_step == session.steps.len() {
        session
            .label
            .as_mut()
            .expect("input prompt")
            .set_text("Device session complete. Retaining input evidence until capture ends.");
    }
    let mut stdout = std::io::stdout().lock();
    if writeln!(
        stdout,
        "CAPTURE_EXT adapter={} frame={} {}",
        session.name, session.frame, facts
    )
    .and_then(|()| stdout.flush())
    .is_err()
    {
        tree().quit_ex().exit_code(1).done();
    }
}

#[derive(GodotClass)]
#[class(base=Node, internal)]
struct InputCaptureRecorder {
    base: Base<Node>,
    recording: Option<Rc<RefCell<Recording>>>,
}

#[godot_api]
impl INode for InputCaptureRecorder {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            recording: None,
        }
    }

    fn ready(&mut self) {
        let connected = RenderingServer::singleton()
            .connect("frame_post_draw", &self.to_gd().callable("prompt_drawn"));
        if let Some(recording) = &self.recording {
            recording.borrow_mut().draw_connected = connected == godot::global::Error::OK;
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if capture::phase() != capture::Phase::Running {
            return;
        }
        let Some(shared) = &self.recording else {
            return;
        };
        let mut recording = shared.borrow_mut();
        let now = recording.now();
        let mut fact = describe_event(&event, &recording);
        fact["time_ns"] = json!(now);
        fact["step"] = json!(recording.prompt.as_ref().map(|prompt| prompt.index));
        fact["matched"] = json!(false);
        if let Some(prompt) = &mut recording.prompt
            && let Some(shown) = prompt.shown_ns
        {
            let same_device = !is_release(&prompt.step)
                || prompt
                    .previous_device
                    .is_some_and(|device| fact["device_id"] == device);
            let matched = same_device && matches(&prompt.step, &fact);
            fact["matched"] = json!(matched);
            if matched
                && now >= shown
                && now - shown <= (prompt.step.deadline_seconds * 1_000_000_000.0).round() as u64
                && prompt.candidate.is_none()
            {
                prompt.candidate = Some(fact.clone());
            }
        }
        recording
            .seen
            .insert(fact["device"].as_str().unwrap().to_owned());
        recording.events.push(fact);
    }
}

#[godot_api]
impl InputCaptureRecorder {
    #[func]
    fn prompt_drawn(&mut self) {
        if capture::phase() != capture::Phase::Running {
            return;
        }
        let Some(shared) = &self.recording else {
            return;
        };
        let mut recording = shared.borrow_mut();
        let now = recording.now();
        if let Some(prompt) = &mut recording.prompt
            && prompt.shown_ns.is_none()
        {
            prompt.shown_ns = Some(now);
            let fact = json!({"step": prompt.index, "time_ns": now, "prompt": prompt.step.prompt});
            recording.prompts.push(fact);
        }
    }
}

fn describe_event(event: &Gd<InputEvent>, recording: &Recording) -> Value {
    let mut fact = json!({
        "class": event.get_class().to_string(), "text": event.as_text().to_string(),
        "device_id": event.get_device(), "device": "other", "event": "other", "echo": event.is_echo(),
    });
    if let Ok(key) = event.clone().try_cast::<InputEventKey>() {
        fact["device"] = json!("keyboard");
        fact["event"] = json!("key");
        fact["keycode"] = json!(key.get_keycode().ord());
        fact["physical_keycode"] = json!(key.get_physical_keycode().ord());
        fact["pressed"] = json!(key.is_pressed());
        fact["unicode"] = json!(key.get_unicode());
        fact["modifiers"] = json!({
            "alt": key.is_alt_pressed(), "shift": key.is_shift_pressed(),
            "ctrl": key.is_ctrl_pressed(), "meta": key.is_meta_pressed(),
        });
    } else if let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() {
        fact["device"] = json!("mouse");
        fact["event"] = json!("mouse_button");
        fact["button"] = json!(button.get_button_index().ord());
        fact["pressed"] = json!(button.is_pressed());
        let position = button.get_position();
        fact["position"] = json!([position.x, position.y]);
        fact["over_start"] = json!(recording.start_rect.contains_point(position));
    } else if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
        fact["device"] = json!("mouse");
        fact["event"] = json!("mouse_motion");
        let position = motion.get_position();
        let relative = motion.get_relative();
        fact["position"] = json!([position.x, position.y]);
        fact["relative"] = json!([relative.x, relative.y]);
    } else if let Ok(button) = event.clone().try_cast::<InputEventJoypadButton>() {
        fact["device"] = json!("controller");
        fact["event"] = json!("joy_button");
        fact["button"] = json!(button.get_button_index().ord());
        fact["pressed"] = json!(button.is_pressed());
    } else if let Ok(axis) = event.clone().try_cast::<InputEventJoypadMotion>() {
        fact["device"] = json!("controller");
        fact["event"] = json!("joy_axis");
        fact["axis"] = json!(axis.get_axis().ord());
        fact["value"] = json!(axis.get_axis_value());
    }
    fact["identity"] = if fact["device"] == "controller" {
        let input = Input::singleton();
        json!(format!(
            "{}:{}",
            input.get_joy_name(event.get_device()),
            input.get_joy_guid(event.get_device())
        ))
    } else {
        json!(format!(
            "Godot {} device {}",
            fact["device"].as_str().unwrap(),
            event.get_device()
        ))
    };
    fact["actions"] = Value::Object(
        recording
            .actions
            .iter()
            .filter(|action| event.is_action(action.as_str()))
            .map(|action| {
                (
                    action.clone(),
                    json!({
                        "pressed": event.is_action_pressed(action.as_str()),
                        "released": event.is_action_released(action.as_str()),
                    }),
                )
            })
            .collect(),
    );
    fact
}

fn matches(step: &Step, event: &Value) -> bool {
    let expected = &step.expected;
    if event["device"] != step.device || event["echo"] == true {
        return false;
    }
    if let Some(action) = expected["action"].as_str() {
        return event["actions"][action][expected["state"].as_str().unwrap()] == true;
    }
    if event["event"] != expected["event"] {
        return false;
    }
    if expected["event"] == "joy_axis" {
        if event["axis"] != expected["axis"] {
            return false;
        }
        let value = event["value"].as_f64().unwrap();
        let direction = expected["direction"].as_f64().unwrap();
        return if direction == 0.0 {
            value.abs() <= expected["neutral_tolerance"].as_f64().unwrap()
        } else {
            value * direction >= expected["threshold"].as_f64().unwrap()
        };
    }
    if expected["event"] == "key" {
        if event[expected["binding"].as_str().unwrap()] != expected["code"] {
            return false;
        }
    } else if event["button"] != expected["button"] {
        return false;
    }
    (expected["target"] != "start" || event["over_start"] == true)
        && event["pressed"] == (expected["state"] == "pressed")
}

fn is_release(step: &Step) -> bool {
    step.expected["state"] == "released" || step.expected["direction"] == 0
}

fn control(step: &Step) -> String {
    let mut expected = step.expected.as_object().unwrap().clone();
    for field in [
        "state",
        "direction",
        "threshold",
        "neutral_tolerance",
        "target",
    ] {
        expected.remove(field);
    }
    json!([step.device, expected]).to_string()
}
