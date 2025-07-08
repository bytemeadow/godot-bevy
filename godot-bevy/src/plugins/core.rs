#![allow(deprecated)] // TODO: remove this once we've removed SystemDeltaTimer

use bevy::app::{App, Plugin, ScheduleRunnerPlugin};
use bevy::asset::{
    AssetMetaCheck, AssetPlugin,
    io::{AssetSource, AssetSourceId},
};
use bevy::ecs::schedule::{Schedule, ScheduleLabel};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

/// Schedule that runs during Godot's physics_process at physics frame rate.
/// Use this for movement, physics, and systems that need to sync with Godot's physics timing.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PhysicsUpdate;

/// Resource containing Godot's physics delta time for the current frame
#[derive(Resource, Default)]
pub struct PhysicsDelta {
    pub delta_seconds: f32,
}

impl PhysicsDelta {
    pub fn new(delta: f64) -> Self {
        Self {
            delta_seconds: delta as f32,
        }
    }

    pub fn delta(&self) -> Duration {
        Duration::from_secs_f32(self.delta_seconds)
    }
}

/// Resource marker to ensure systems accessing Godot APIs run on the main thread
#[derive(Resource, Default)]
pub struct MainThreadMarker;

/// Transform synchronization modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformSyncMode {
    /// No transform syncing - use direct Godot physics (move_and_slide, etc.)
    /// Best for: Platformers, physics-heavy games
    Disabled,
    /// One-way sync: ECS → Godot only
    /// Best for: Pure ECS games, simple movement
    OneWay,
    /// Two-way sync: ECS ↔ Godot
    /// Best for: Hybrid apps migrating from GDScript to ECS
    TwoWay,
}

impl Default for TransformSyncMode {
    fn default() -> Self {
        Self::OneWay
    }
}

/// Configuration resource for transform syncing behavior
#[derive(Resource, Debug, Clone)]
pub struct GodotTransformConfig {
    pub sync_mode: TransformSyncMode,
}

impl Default for GodotTransformConfig {
    fn default() -> Self {
        Self {
            sync_mode: TransformSyncMode::OneWay,
        }
    }
}

impl GodotTransformConfig {
    /// Disable all transform syncing - use direct Godot physics instead
    pub fn disabled() -> Self {
        Self {
            sync_mode: TransformSyncMode::Disabled,
        }
    }

    /// Enable one-way sync (ECS → Godot) - default behavior
    pub fn one_way() -> Self {
        Self {
            sync_mode: TransformSyncMode::OneWay,
        }
    }

    /// Enable two-way sync (ECS ↔ Godot) for hybrid apps
    pub fn two_way() -> Self {
        Self {
            sync_mode: TransformSyncMode::TwoWay,
        }
    }
}

/// Minimal core plugin with only essential Godot-Bevy integration.
/// This includes scene tree management, basic Bevy setup, and core resources.
#[derive(Default)]
pub struct GodotBaseCorePlugin;

impl Plugin for GodotBaseCorePlugin {
    fn build(&self, app: &mut App) {
        // IMPORTANT: Register custom AssetReader BEFORE setting up AssetPlugin
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSource::build()
                .with_reader(|| Box::new(crate::plugins::assets::GodotAssetReader::new())),
        );

        app.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>())
            // Configure AssetPlugin to bypass path verification for Godot resources
            .add_plugins(AssetPlugin {
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .add_plugins(bevy::log::LogPlugin::default())
            .add_plugins(bevy::diagnostic::DiagnosticsPlugin)
            .init_resource::<PhysicsDelta>()
            .init_non_send_resource::<MainThreadMarker>();

        // Add the PhysicsUpdate schedule
        app.add_schedule(Schedule::new(PhysicsUpdate));
    }
}

/// SystemParam to keep track of an independent delta time
///
/// Not every system runs on a Bevy update and Bevy can be updated multiple
/// during a "frame".
#[derive(SystemParam)]
#[deprecated(note = "Use PhysicsDelta instead")]
pub struct SystemDeltaTimer<'w, 's> {
    last_time: Local<'s, Option<Instant>>,
    marker: PhantomData<&'w ()>,
}

#[allow(deprecated)]
impl<'w, 's> SystemDeltaTimer<'w, 's> {
    /// Returns the time passed since the last invocation
    pub fn delta(&mut self) -> Duration {
        let now = Instant::now();
        let last_time = self.last_time.unwrap_or(now);

        *self.last_time = Some(now);

        now - last_time
    }

    pub fn delta_seconds(&mut self) -> f32 {
        self.delta().as_secs_f32()
    }

    pub fn delta_seconds_f64(&mut self) -> f64 {
        self.delta().as_secs_f64()
    }
}

pub trait FindEntityByNameExt<T> {
    fn find_entity_by_name(self, name: &str) -> Option<T>;
}

impl<'a, T: 'a, U> FindEntityByNameExt<T> for U
where
    U: Iterator<Item = (&'a Name, T)>,
{
    fn find_entity_by_name(mut self, name: &str) -> Option<T> {
        self.find_map(|(ent_name, t)| (ent_name.as_str() == name).then_some(t))
    }
}
