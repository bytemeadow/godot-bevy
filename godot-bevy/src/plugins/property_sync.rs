use bevy_ecs::schedule::SystemSet;

/// Observation and publication boundaries for live mapped properties.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertySyncSet {
    /// Sample Godot properties during `PreUpdate`.
    Read,
    /// Publish Bevy changes during `PostUpdate`.
    Write,
}

#[doc(hidden)]
#[inline]
pub fn equal<T: PartialEq + 'static>(a: &T, b: &T) -> bool {
    use std::any::Any;
    if let (Some(a), Some(b)) = (
        (a as &dyn Any).downcast_ref::<f32>(),
        (b as &dyn Any).downcast_ref::<f32>(),
    ) {
        return a.to_bits() == b.to_bits();
    }
    if let (Some(a), Some(b)) = (
        (a as &dyn Any).downcast_ref::<f64>(),
        (b as &dyn Any).downcast_ref::<f64>(),
    ) {
        return a.to_bits() == b.to_bits();
    }
    a == b
}

#[cfg(feature = "test-frame-signal")]
#[doc(hidden)]
#[derive(bevy_ecs::resource::Resource, Default, Clone, Copy, Debug)]
pub struct PropertySyncCounters {
    pub reads: u64,
    pub writes: u64,
    pub read_passes: u64,
    pub write_passes: u64,
}

#[doc(hidden)]
#[derive(bevy_ecs::system::SystemParam)]
pub struct PropertySyncStats<'w> {
    #[cfg(feature = "test-frame-signal")]
    counters: bevy_ecs::system::ResMut<'w, PropertySyncCounters>,
    marker: std::marker::PhantomData<&'w ()>,
}

impl PropertySyncStats<'_> {
    pub fn read(&mut self) {
        #[cfg(feature = "test-frame-signal")]
        {
            self.counters.reads += 1;
        }
    }

    pub fn write(&mut self) {
        #[cfg(feature = "test-frame-signal")]
        {
            self.counters.writes += 1;
        }
    }

    pub fn read_pass(&mut self) {
        #[cfg(feature = "test-frame-signal")]
        {
            self.counters.read_passes += 1;
        }
    }

    pub fn write_pass(&mut self) {
        #[cfg(feature = "test-frame-signal")]
        {
            self.counters.write_passes += 1;
        }
    }
}

#[doc(hidden)]
pub fn init_stats(_app: &mut bevy_app::App) {
    #[cfg(feature = "test-frame-signal")]
    _app.init_resource::<PropertySyncCounters>();
}

#[doc(hidden)]
pub struct Value<'a, T>(pub &'a T);

#[doc(hidden)]
pub trait Capture {
    type Snapshot: Clone + PartialEq + Send + Sync + 'static;
    fn capture(self) -> Self::Snapshot;
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Capture for &Value<'_, T> {
    type Snapshot = T;
    fn capture(self) -> T {
        self.0.clone()
    }
}

// The extra reference selects owned text even when the Godot type is an alias.
impl Capture for &&Value<'_, godot::builtin::GString> {
    type Snapshot = String;
    fn capture(self) -> String {
        self.0.to_string()
    }
}

impl Capture for &&Value<'_, godot::builtin::StringName> {
    type Snapshot = String;
    fn capture(self) -> String {
        self.0.to_string()
    }
}

#[doc(hidden)]
pub fn empty_snapshot<G, S>(_: fn(&G) -> S) -> Option<S> {
    None
}

#[doc(hidden)]
pub fn valid_components(types: &[(std::any::TypeId, &'static str)], class: &str) -> bool {
    for (index, (id, name)) in types.iter().enumerate() {
        if types[..index].iter().any(|(previous, _)| previous == id) {
            tracing::warn!(
                "godot-bevy: {class} constructs {name} more than once; skipping its bundle and property sync"
            );
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::equal;

    #[test]
    fn floats_compare_representations() {
        assert!(equal(&f32::NAN, &f32::NAN));
        assert!(equal(&f64::NAN, &f64::NAN));
        assert!(!equal(&0.0f32, &-0.0f32));
        assert!(!equal(&0.0f64, &-0.0f64));
        assert!(!equal(&f32::from_bits(0x7fc00001), &f32::NAN));
        assert!(equal(&String::from("same"), &String::from("same")));
        assert!(!equal(&1, &2));
    }
}
