use bevy::app::{App, Plugin};
use bevy::ecs::schedule::{IntoScheduleConfigs, ScheduleConfigs};
use bevy::ecs::system::{ScheduleSystem, SystemParam};
use bevy::prelude::*;
use std::marker::PhantomData;
use std::time::{Duration, Instant};
use transforms::GodotTransformsPlugin;

pub mod collisions;
pub use collisions::*;

pub mod scene_tree;
pub use scene_tree::*;

pub mod transforms;
pub use transforms::{Transform2D, Transform3D};

pub mod signals;
pub use signals::*;

pub struct GodotCorePlugin;

impl Plugin for GodotCorePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TaskPoolPlugin::default())
            .add_plugins(bevy::log::LogPlugin::default())
            .add_plugins(bevy::diagnostic::FrameCountPlugin)
            .add_plugins(bevy::diagnostic::DiagnosticsPlugin)
            .add_plugins(bevy::time::TimePlugin)
            .add_plugins(GodotSceneTreePlugin)
            .add_plugins(GodotTransformsPlugin)
            .add_plugins(GodotCollisionsPlugin)
            .add_plugins(GodotSignalsPlugin);
        // .add_plugins(GodotInputEventPlugin)
    }
}

/// Bevy Resource that is available when the app is updated through `process` callback
#[derive(Resource)]
pub struct GodotVisualFrame;

/// Bevy Resource that is available when the app is updated through `physics_process` callback
#[derive(Resource)]
pub struct GodotPhysicsFrame;

/// Adds `as_physics_system` that schedules a system only for the physics frame
pub trait AsPhysicsSystem<Marker> {
    #[allow(clippy::wrong_self_convention)]
    fn as_physics_system(self) -> ScheduleConfigs<ScheduleSystem>;
}

impl<Marker, T: IntoScheduleConfigs<ScheduleSystem, Marker>> AsPhysicsSystem<Marker> for T {
    fn as_physics_system(self) -> ScheduleConfigs<ScheduleSystem> {
        self.run_if(resource_exists::<GodotPhysicsFrame>)
    }
}

/// Adds `as_visual_system` that schedules a system only for the frame
pub trait AsVisualSystem<Marker> {
    #[allow(clippy::wrong_self_convention)]
    fn as_visual_system(self) -> ScheduleConfigs<ScheduleSystem>;
}

impl<Marker, T: IntoScheduleConfigs<ScheduleSystem, Marker>> AsVisualSystem<Marker> for T {
    fn as_visual_system(self) -> ScheduleConfigs<ScheduleSystem> {
        self.run_if(resource_exists::<GodotVisualFrame>)
    }
}

/// SystemParam to keep track of an independent delta time
///
/// Not every system runs on a Bevy update and Bevy can be updated multiple
/// during a "frame".
#[derive(SystemParam)]
pub struct SystemDeltaTimer<'w, 's> {
    last_time: Local<'s, Option<Instant>>,
    marker: PhantomData<&'w ()>,
}

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
