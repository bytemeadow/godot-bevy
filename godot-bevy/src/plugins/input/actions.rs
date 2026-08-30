use std::collections::{HashMap, HashSet};

use bevy_app::{App, Plugin, Update};
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::{IntoScheduleConfigs, SystemSet};
use bevy_ecs::system::ResMut;
use bevy_ecs::world::World;
use bevy_math::Vec2;
use godot::builtin::StringName;
use godot::classes::{Input, InputMap};
use godot::obj::Singleton;
use parking_lot::Mutex;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum Clock {
    #[default]
    Process,
    Physics,
}

#[derive(Default, Clone, Copy)]
pub(crate) struct ActionState {
    pub(crate) pressed: bool,
    pub(crate) just_pressed: bool,
    pub(crate) just_released: bool,
    pub(crate) strength: f32,
    pub(crate) raw_strength: f32,
}

#[derive(Default)]
pub(crate) struct Snapshot {
    pub(crate) actions: HashMap<String, ActionState>,
}

/// Godot action input state -- held state for both the process and physics clocks.
///
/// Read the same resource in `Update`, `FixedUpdate`, or a shared helper and
/// it returns the snapshot that matches the currently-executing clock. The
/// active-clock flag is set by the fixed-schedule driver, not by readers.
#[derive(Resource, Default)]
pub struct GodotActions {
    pub(crate) process: Snapshot,
    pub(crate) physics: Snapshot,
    pub(crate) active: Clock,
    /// Cached action names from InputMap -- rebuilt on first poll.
    pub(crate) action_set: Vec<StringName>,
    /// Pre-stringified keys matching `action_set` for HashMap lookups without alloc.
    pub(crate) action_keys: Vec<String>,
    /// Actions already warned about in debug so each unknown name warns only once.
    warned: Mutex<HashSet<String>>,
    #[cfg(test)]
    poll_override: Option<fn(&mut GodotActions, Clock)>,
}

/// A typed action handle. Construct once; reuse across systems.
///
/// The key is pre-computed from the `StringName` so lookups never allocate.
/// Reading via `&Action` suppresses the warn-on-unknown path -- an `Action`
/// that's in scope was constructed intentionally.
pub struct Action {
    pub name: StringName,
    pub(crate) key: String,
}

impl Action {
    pub fn new(name: impl Into<StringName>) -> Self {
        let name = name.into();
        let key = name.to_string();
        Self { name, key }
    }
}

/// Short-lived borrow used by every accessor. Never allocates.
pub struct ActionRef<'a> {
    key: &'a str,
    warn_if_unknown: bool,
}

impl<'a> From<&'a str> for ActionRef<'a> {
    fn from(s: &'a str) -> Self {
        ActionRef {
            key: s,
            warn_if_unknown: true,
        }
    }
}

impl<'a> From<&'a String> for ActionRef<'a> {
    fn from(s: &'a String) -> Self {
        ActionRef {
            key: s.as_str(),
            warn_if_unknown: true,
        }
    }
}

impl<'a> From<&'a Action> for ActionRef<'a> {
    fn from(a: &'a Action) -> Self {
        ActionRef {
            key: &a.key,
            // Typed handles are constructed deliberately; skip the warn path.
            warn_if_unknown: false,
        }
    }
}

impl GodotActions {
    fn snapshot(&self) -> &Snapshot {
        match self.active {
            Clock::Process => &self.process,
            Clock::Physics => &self.physics,
        }
    }

    fn lookup(&self, r: ActionRef<'_>) -> ActionState {
        match self.snapshot().actions.get(r.key) {
            Some(&state) => state,
            None => {
                if r.warn_if_unknown && cfg!(debug_assertions) {
                    let mut warned = self.warned.lock();
                    if warned.insert(r.key.to_owned()) {
                        tracing::warn!(
                            "GodotActions: unknown action {:?} -- not in InputMap",
                            r.key
                        );
                    }
                }
                ActionState::default()
            }
        }
    }

    pub fn pressed<'a>(&self, action: impl Into<ActionRef<'a>>) -> bool {
        self.lookup(action.into()).pressed
    }

    pub fn just_pressed<'a>(&self, action: impl Into<ActionRef<'a>>) -> bool {
        self.lookup(action.into()).just_pressed
    }

    pub fn just_released<'a>(&self, action: impl Into<ActionRef<'a>>) -> bool {
        self.lookup(action.into()).just_released
    }

    pub fn strength<'a>(&self, action: impl Into<ActionRef<'a>>) -> f32 {
        self.lookup(action.into()).strength
    }

    pub fn raw_strength<'a>(&self, action: impl Into<ActionRef<'a>>) -> f32 {
        self.lookup(action.into()).raw_strength
    }

    /// `strength(pos) - strength(neg)` -- positive axis wins.
    pub fn axis<'a>(&self, neg: impl Into<ActionRef<'a>>, pos: impl Into<ActionRef<'a>>) -> f32 {
        self.lookup(pos.into()).strength - self.lookup(neg.into()).strength
    }

    /// Directional vector: `Vec2(strength(pos_x)-strength(neg_x), strength(pos_y)-strength(neg_y))`.
    /// Argument order matches Godot's `Input.get_vector`.
    pub fn vector<'a>(
        &self,
        neg_x: impl Into<ActionRef<'a>>,
        pos_x: impl Into<ActionRef<'a>>,
        neg_y: impl Into<ActionRef<'a>>,
        pos_y: impl Into<ActionRef<'a>>,
    ) -> Vec2 {
        Vec2::new(
            self.lookup(pos_x.into()).strength - self.lookup(neg_x.into()).strength,
            self.lookup(pos_y.into()).strength - self.lookup(neg_y.into()).strength,
        )
    }

    pub(crate) fn set_active(&mut self, clock: Clock) {
        self.active = clock;
    }

    /// Poll Godot's `Input` singleton and refresh the snapshot for `clock`.
    ///
    /// Lazy-seeds the action list from `InputMap` on the first poll that returns
    /// any actions; zero-alloc once seeded -- `action_keys` holds the
    /// pre-stringified keys.
    pub(crate) fn poll(&mut self, clock: Clock) {
        #[cfg(test)]
        if let Some(poll_override) = self.poll_override {
            poll_override(self, clock);
            return;
        }
        if self.action_set.is_empty() {
            let acts: Vec<StringName> = InputMap::singleton().get_actions().iter_shared().collect();
            let keys: Vec<String> = acts.iter().map(|n| n.to_string()).collect();
            for key in &keys {
                self.process
                    .actions
                    .insert(key.clone(), ActionState::default());
                self.physics
                    .actions
                    .insert(key.clone(), ActionState::default());
            }
            self.action_set = acts;
            self.action_keys = keys;
        }
        // Destructure for disjoint borrows -- the compiler can't see that
        // `process`/`physics` and `action_set`/`action_keys` don't alias.
        let Self {
            process,
            physics,
            action_set,
            action_keys,
            ..
        } = self;
        let snap = match clock {
            Clock::Process => process,
            Clock::Physics => physics,
        };
        let input = Input::singleton();
        for (name, key) in action_set.iter().zip(action_keys.iter()) {
            let st = snap.actions.get_mut(key).unwrap();
            st.pressed = input.is_action_pressed(name);
            st.just_pressed = input.is_action_just_pressed(name);
            st.just_released = input.is_action_just_released(name);
            st.strength = input.get_action_strength(name);
            st.raw_strength = input.get_action_raw_strength(name);
        }
    }
}

/// Called by `godot_fixed_driver` before `FixedMain` runs. Sets the active
/// clock to `Physics` and refreshes the physics snapshot via FFI. No-op if
/// `GodotActions` hasn't been added (allows apps that don't use action input
/// to run the fixed driver without any FFI cost).
pub(crate) fn poll_physics_actions(world: &mut World) {
    let Some(mut ga) = world.get_resource_mut::<GodotActions>() else {
        return;
    };
    ga.set_active(Clock::Physics);
    ga.poll(Clock::Physics);
}

/// Called by `godot_fixed_driver` after `FixedMain` returns. Restores the
/// active clock to `Process` so subsequent `Update` reads see the process snapshot.
pub(crate) fn restore_process_clock(world: &mut World) {
    if let Some(mut ga) = world.get_resource_mut::<GodotActions>() {
        ga.set_active(Clock::Process);
    }
}

fn poll_process_actions(mut ga: ResMut<GodotActions>) {
    ga.set_active(Clock::Process);
    ga.poll(Clock::Process);
}

/// Marker set for the `Update`-schedule process-clock poll.
///
/// User systems in `Update` that read `GodotActions` must run `.after(GodotInputSet)`;
/// otherwise they may run before the process snapshot is refreshed and see stale state.
/// `FixedUpdate` systems read the physics snapshot, which is polled automatically by
/// the fixed-schedule driver -- no ordering constraint needed there.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GodotInputSet;

/// Registers `GodotActions` and polls it each `Update` schedule tick (process clock).
///
/// Polling in `First` would evaluate just-pressed/just-released edges against the
/// physics frame counter and corrupt the process snapshot. `Update` is the correct
/// schedule; readers run `.after(GodotInputSet)`.
///
/// The physics-clock poll is wired by the fixed-schedule driver automatically
/// whenever this resource is present -- no extra plugin needed.
pub struct GodotActionsPlugin;

impl Plugin for GodotActionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GodotActions>();
        app.add_systems(Update, poll_process_actions.in_set(GodotInputSet));
    }
}

#[cfg(test)]
include!("actions_tests.rs");
