extern crate self as godot;
extern crate self as godot_bevy_test;

use godot_bevy_test_macros::bench;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct BenchRegistration {
    function: fn(),
    repetitions: usize,
}

inventory::collect!(BenchRegistration);

#[macro_export]
#[allow(clippy::crate_in_macro_def)] // deliberately the consumer crate: this mocks the registry in-crate
macro_rules! shard_add {
    (
        $registry:path;
        $benchmark:path {
            name: $name:expr,
            file: $file:expr,
            line: $line:expr,
            function: $function:path,
            repetitions: $repetitions:expr,
        }
    ) => {
        inventory::submit! {
            crate::BenchRegistration {
                function: $function,
                repetitions: $repetitions,
            }
        }
    };
}

pub mod sys {
    pub use crate::shard_add;
}

static CALLS: AtomicUsize = AtomicUsize::new(0);

#[bench(repeat = 3)]
fn generated_benchmark() -> usize {
    CALLS.fetch_add(1, Ordering::SeqCst)
}

#[test]
fn bench_attribute_emits_callable_repetition_loop() {
    let registrations = inventory::iter::<BenchRegistration>
        .into_iter()
        .collect::<Vec<_>>();
    assert_eq!(registrations.len(), 1);
    assert_eq!(registrations[0].repetitions, 3);
    (registrations[0].function)();
    assert_eq!(CALLS.load(Ordering::SeqCst), 3);
}
