use crate::GameState;
use crate::gameplay::audio::{GameAudio, play_capture_cue, reset_capture_audio};
use crate::main_menu::MenuAssets;
use bevy::prelude::{App, Assets, NonSendMut, PostUpdate, Res, Resource, State, World};
use godot::classes::{AudioEffectCapture, AudioServer, AudioStream, Engine};
use godot::prelude::*;
use godot_bevy::prelude::GodotResource;
use godot_bevy_test::capture::{self, CaptureAdapter, CaptureManifest, Pacing, Phase};
use serde::Deserialize;
use serde_json::{Value, json};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{Duration, Instant};

#[derive(Clone, Deserialize, Resource)]
struct AudioScenario {
    mixer_rate: u32,
    reference: String,
    cues: Vec<Cue>,
    stop_frame: u32,
    end_frame: u32,
}

#[derive(Clone, Deserialize)]
struct Cue {
    name: String,
    frame: u32,
}

struct MixerCapture {
    effect: Gd<AudioEffectCapture>,
    writer: BufWriter<File>,
    reset_pending: bool,
    origin: Option<Instant>,
    discarded_before: i64,
    sample_start: usize,
    frame: u32,
    issued_ms: f64,
    cue: Option<String>,
    stop: bool,
    error: Option<String>,
}

pub(crate) fn is_active() -> bool {
    capture::is_enabled()
        && std::env::var("GODOT_BEVY_CAPTURE").is_ok_and(|scenario| scenario == "cues")
}

pub(super) fn install(app: &mut App) {
    if !is_active() {
        return;
    }
    app.add_systems(PostUpdate, drain_audio);
    capture::install(
        app,
        CaptureAdapter {
            name: "audio",
            extension,
            scenarios: &["cues"],
            is_settled,
            reset,
            before_frame,
        },
    );
}

fn extension(world: &mut World, manifest: &CaptureManifest, value: &Value) -> Result<(), String> {
    let scenario: AudioScenario =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    if manifest.pacing != Pacing::Realtime || manifest.frames != scenario.end_frame {
        return Err("audio requires realtime pacing and frames equal to audio.end_frame".into());
    }
    if !matches!(scenario.mixer_rate, 44100 | 48000)
        || scenario.reference
            != format!(
                "examples/harness/audio/references/{}/{}.wav",
                manifest.example, manifest.scenario
            )
        || scenario.cues.len() != 2
        || scenario.cues[0].name != "jump"
        || scenario.cues[1].name != "gem"
        || !(0 < scenario.cues[0].frame
            && scenario.cues[0].frame < scenario.cues[1].frame
            && scenario.cues[1].frame < scenario.stop_frame
            && scenario.stop_frame < scenario.end_frame)
    {
        return Err("audio extension has an invalid mixer rate, reference or cue schedule".into());
    }
    Engine::singleton().set_max_fps(60);
    world.insert_resource(scenario);
    Ok(())
}

fn is_settled(world: &mut World, _: &CaptureManifest) -> Result<bool, String> {
    if !world.contains_resource::<AudioScenario>() {
        return Err("cues requires extensions.audio".into());
    }
    if let Some(capture) = world.get_non_send::<MixerCapture>() {
        if let Some(error) = &capture.error {
            return Err(error.clone());
        }
        if capture.reset_pending {
            return Ok(false);
        }
    }
    if world.resource::<State<GameState>>().get() != &GameState::MainMenu
        || !world.resource::<MenuAssets>().signals_connected
    {
        return Ok(false);
    }
    let Some(audio) = world.get_resource::<GameAudio>() else {
        return Ok(false);
    };
    let handles = [audio.jump_sound.clone(), audio.gem_sound.clone()];
    let mut assets = world.resource_mut::<Assets<GodotResource>>();
    for handle in handles {
        let Some(mut asset) = assets.get_mut(&handle) else {
            return Ok(false);
        };
        if !asset
            .try_cast::<AudioStream>()
            .is_some_and(|stream| stream.get_length() > 0.0)
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn reset(world: &mut World, _: &CaptureManifest) -> Result<(), String> {
    let declared = world.resource::<AudioScenario>().mixer_rate;
    let mut server = AudioServer::singleton();
    if server.get_mix_rate() != declared as f32 {
        return Err(format!(
            "audio declares a {declared} Hz mixer; AudioServer reports {} Hz",
            server.get_mix_rate()
        ));
    }
    let driver = server
        .call("get_driver_name", &[])
        .try_to::<GString>()
        .map_err(|error| error.to_string())?;
    if driver.is_empty() || driver.to_string().eq_ignore_ascii_case("dummy") {
        return Err("audio requires a real audio driver; Dummy cannot qualify mixer output".into());
    }
    let master = server.get_bus_index("Master");
    if master < 0 || server.get_bus_effect_count(master) != 0 {
        return Err("audio requires a Master bus without pre-existing effects".into());
    }
    for bus in 0..server.get_bus_count() {
        server.set_bus_solo(bus, false);
    }
    server.set_bus_mute(master, false);
    server.set_bus_volume_db(master, 0.0);
    server.set_bus_bypass_effects(master, false);
    server.set_playback_speed_scale(1.0);
    reset_capture_audio(world);
    let directory = capture::evidence_dir().ok_or("audio evidence directory is unavailable")?;
    let writer = BufWriter::new(
        File::create(directory.join("mixer.f32")).map_err(|error| error.to_string())?,
    );
    let mut effect = AudioEffectCapture::new_gd();
    effect.set_buffer_length(1.0);
    server.add_bus_effect(master, &effect);
    world.insert_non_send(MixerCapture {
        effect,
        writer,
        reset_pending: true,
        origin: None,
        discarded_before: 0,
        sample_start: 0,
        frame: 0,
        issued_ms: 0.0,
        cue: None,
        stop: false,
        error: None,
    });
    Ok(())
}

fn before_frame(world: &mut World, _: &CaptureManifest, frame: u32) -> Result<(), String> {
    let scenario = world.resource::<AudioScenario>().clone();
    let cue = scenario
        .cues
        .iter()
        .find(|cue| cue.frame == frame)
        .map(|cue| cue.name.clone());
    if frame == 0 {
        return Ok(());
    }
    {
        let mut capture = world.non_send_mut::<MixerCapture>();
        if let Some(error) = &capture.error {
            return Err(error.clone());
        }
        if capture.origin.is_none() {
            capture.effect.clear_buffer();
            capture.discarded_before = capture.effect.get_discarded_frames();
            capture.origin = Some(Instant::now());
        }
        let origin = capture.origin.unwrap();
        let deadline = origin + Duration::from_secs_f64(f64::from(frame) / 60.0);
        while let Some(delay) = deadline.checked_duration_since(Instant::now()) {
            std::thread::sleep(delay.min(Duration::from_millis(1)));
        }
        capture.frame = frame;
        capture.issued_ms = origin.elapsed().as_secs_f64() * 1000.0;
        capture.cue = cue.clone();
        capture.stop = frame == scenario.stop_frame;
    }
    if let Some(name) = cue {
        play_capture_cue(world, &name)?;
    }
    if frame == scenario.stop_frame {
        reset_capture_audio(world);
    }
    Ok(())
}

fn drain_audio(capture: Option<NonSendMut<MixerCapture>>, scenario: Option<Res<AudioScenario>>) {
    let (Some(mut capture), Some(scenario)) = (capture, scenario) else {
        return;
    };
    capture.reset_pending = false;
    if capture::phase() != Phase::Running || capture.error.is_some() {
        return;
    }
    if let Err(error) = drain_frame(&mut capture, &scenario) {
        capture.error = Some(error.clone());
        println!("CAPTURE_ERROR {}", json!(error));
        let _ = std::io::stdout().flush();
    }
}

fn drain_frame(capture: &mut MixerCapture, scenario: &AudioScenario) -> Result<(), String> {
    let available = capture.effect.get_frames_available();
    let samples = capture.effect.get_buffer(available);
    if samples.len() != available as usize {
        return Err("audio capture returned fewer samples than available".into());
    }
    let mut bytes = Vec::with_capacity(samples.len() * 8);
    for sample in samples.as_slice() {
        bytes.extend_from_slice(&sample.x.to_le_bytes());
        bytes.extend_from_slice(&sample.y.to_le_bytes());
    }
    capture
        .writer
        .write_all(&bytes)
        .and_then(|()| capture.writer.flush())
        .map_err(|error| error.to_string())?;
    let mut server = AudioServer::singleton();
    let driver = server
        .call("get_driver_name", &[])
        .try_to::<GString>()
        .map_err(|error| error.to_string())?;
    let facts = json!({
        "frame": capture.frame,
        "sample_start": capture.sample_start,
        "sample_count": samples.len(),
        "discarded_frames": capture.effect.get_discarded_frames() - capture.discarded_before,
        "mixer_rate": server.get_mix_rate(),
        "driver": driver.to_string(),
        "output_device": server.get_output_device().to_string(),
        "issued_ms": capture.issued_ms,
        "elapsed_ms": capture.origin.ok_or("audio capture has no clock origin")?.elapsed().as_secs_f64() * 1000.0,
        "cue": capture.cue,
        "stop": capture.stop,
        "final": capture.frame == scenario.end_frame,
    });
    println!("CAPTURE_EXT adapter=audio frame={} {facts}", capture.frame);
    std::io::stdout()
        .flush()
        .map_err(|error| error.to_string())?;
    capture.sample_start += samples.len();
    Ok(())
}
