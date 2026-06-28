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
}

// ── typed handle ────────────────────────────────────────────────────────────

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

// ── borrow helper ────────────────────────────────────────────────────────────

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

// ── accessors ────────────────────────────────────────────────────────────────

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
    /// Lazy-seeds the action list from `InputMap` on the first call (zero-alloc
    /// on subsequent calls -- `action_keys` keeps pre-stringified keys).
    pub(crate) fn poll(&mut self, clock: Clock) {
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

// ── driver helpers ───────────────────────────────────────────────────────────

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

// ── plugin ────────────────────────────────────────────────────────────────────

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

// ── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state_full(
        pressed: bool,
        just_pressed: bool,
        just_released: bool,
        strength: f32,
        raw_strength: f32,
    ) -> ActionState {
        ActionState {
            pressed,
            just_pressed,
            just_released,
            strength,
            raw_strength,
        }
    }

    // ── 1. Active-clock flip with divergent snapshots ────────────────────────

    #[test]
    fn active_clock_flip_divergent_snapshots() {
        let mut ga = GodotActions::default();

        // process: "a" pressed+just_pressed, "b" all-false
        ga.process
            .actions
            .insert("a".to_owned(), make_state_full(true, true, false, 0.0, 0.0));
        ga.process
            .actions
            .insert("b".to_owned(), ActionState::default());

        // physics: "a" all-false, "b" pressed+just_pressed
        ga.physics
            .actions
            .insert("a".to_owned(), ActionState::default());
        ga.physics
            .actions
            .insert("b".to_owned(), make_state_full(true, true, false, 0.0, 0.0));

        assert!(ga.pressed("a"), "process: a should be pressed");
        assert!(!ga.pressed("b"), "process: b should not be pressed");

        ga.set_active(Clock::Physics);
        assert!(!ga.pressed("a"), "physics: a should not be pressed");
        assert!(ga.pressed("b"), "physics: b should be pressed");

        ga.set_active(Clock::Process);
        assert!(ga.pressed("a"), "reverted: a should be pressed again");
        assert!(!ga.pressed("b"), "reverted: b should not be pressed again");
    }

    // ── 2. Shared helper returns the executing clock's data ──────────────────

    fn read_a(ga: &GodotActions) -> bool {
        ga.pressed("a")
    }

    #[test]
    fn shared_helper_sees_active_clock() {
        let mut ga = GodotActions::default();

        ga.process.actions.insert(
            "a".to_owned(),
            make_state_full(true, false, false, 0.0, 0.0),
        );
        ga.physics.actions.insert(
            "a".to_owned(),
            make_state_full(false, false, false, 0.0, 0.0),
        );

        assert!(read_a(&ga), "helper under Process should see pressed=true");
        ga.set_active(Clock::Physics);
        assert!(
            !read_a(&ga),
            "helper under Physics should see pressed=false"
        );
    }

    // ── 3. Edge independence -- no aliasing between fields ───────────────────

    #[test]
    fn edge_independence_no_aliasing() {
        let mut ga = GodotActions::default();

        ga.process.actions.insert(
            "held".to_owned(),
            make_state_full(true, false, false, 0.0, 0.0),
        );
        ga.process.actions.insert(
            "rising".to_owned(),
            make_state_full(true, true, false, 0.0, 0.0),
        );
        ga.process.actions.insert(
            "falling".to_owned(),
            make_state_full(false, false, true, 0.0, 0.0),
        );

        assert!(ga.pressed("held"));
        assert!(!ga.just_pressed("held"));
        assert!(!ga.just_released("held"));

        assert!(ga.pressed("rising"));
        assert!(ga.just_pressed("rising"));
        assert!(!ga.just_released("rising"));

        assert!(!ga.pressed("falling"));
        assert!(!ga.just_pressed("falling"));
        assert!(ga.just_released("falling"));
    }

    // ── 4. strength / raw_strength / axis / vector ───────────────────────────

    #[test]
    fn strength_raw_strength_axis_vector() {
        let mut ga = GodotActions::default();

        ga.process.actions.insert(
            "left".to_owned(),
            make_state_full(true, false, false, 0.8, 1.0),
        );
        ga.process.actions.insert(
            "right".to_owned(),
            make_state_full(true, false, false, 0.6, 0.9),
        );
        ga.process.actions.insert(
            "up".to_owned(),
            make_state_full(true, false, false, 0.4, 0.5),
        );
        ga.process.actions.insert(
            "down".to_owned(),
            make_state_full(true, false, false, 0.3, 0.7),
        );

        // strength != raw_strength
        assert_ne!(ga.strength("left"), ga.raw_strength("left"));
        assert!((ga.strength("left") - 0.8).abs() < f32::EPSILON);
        assert!((ga.raw_strength("left") - 1.0).abs() < f32::EPSILON);

        // axis == pos - neg
        let ax = ga.axis("left", "right");
        assert!(
            (ax - (0.6 - 0.8)).abs() < f32::EPSILON,
            "axis={ax}, expected {}",
            0.6 - 0.8
        );

        // vector componentwise
        let v = ga.vector("left", "right", "up", "down");
        assert!((v.x - (0.6 - 0.8)).abs() < f32::EPSILON, "v.x={}", v.x);
        assert!((v.y - (0.3 - 0.4)).abs() < f32::EPSILON, "v.y={}", v.y);
    }

    // ── 5. poll_physics_actions without resource is a noop ───────────────────

    #[test]
    fn poll_physics_actions_without_resource_is_noop() {
        let mut world = bevy_ecs::world::World::new();
        // Neither call should panic; the resource must remain absent.
        poll_physics_actions(&mut world);
        restore_process_clock(&mut world);
        assert!(
            world.get_resource::<GodotActions>().is_none(),
            "GodotActions must not be inserted by the driver helpers"
        );
    }

    // ── 6. Unknown &str -- no panic, defaults returned, warn-once ────────────

    #[test]
    fn unknown_action_returns_defaults_no_panic() {
        let ga = GodotActions::default();

        assert!(!ga.pressed("does_not_exist"));
        assert!((ga.strength("does_not_exist") - 0.0).abs() < f32::EPSILON);

        // Second call -- warn-once must not panic even if already warned.
        assert!(!ga.pressed("does_not_exist"));
        assert!((ga.strength("does_not_exist") - 0.0).abs() < f32::EPSILON);
    }
}
