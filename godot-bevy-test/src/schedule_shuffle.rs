use std::sync::Mutex;

use bevy::ecs::schedule::{Schedules, SingleThreadedExecutor};
use bevy::prelude::*;

pub(crate) static ACTIVE_SCHEDULE_SEED: Mutex<Option<u64>> = Mutex::new(None);

pub(crate) fn test_schedule_seed(base_seed: u64, name: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in name.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let mut seed = hash ^ base_seed;
    seed = (seed ^ (seed >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    seed = (seed ^ (seed >> 27)).wrapping_mul(0x94d049bb133111eb);
    seed ^ (seed >> 31)
}

pub(crate) struct ScheduleShufflePlugin(pub(crate) u64);

impl Plugin for ScheduleShufflePlugin {
    fn build(&self, _app: &mut App) {}

    fn cleanup(&self, app: &mut App) {
        for (_, schedule) in app.world_mut().resource_mut::<Schedules>().iter_mut() {
            let mut settings = schedule.get_build_settings();
            settings.shuffle_seed = Some(self.0);
            schedule.set_build_settings(settings);
            schedule.set_executor(SingleThreadedExecutor::new());
        }
    }
}

#[cfg(test)]
include!("schedule_shuffle_tests.rs");
