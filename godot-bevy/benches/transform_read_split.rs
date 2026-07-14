//! Investigative bench (not part of CI): does splitting the two-way transform READ
//! into a main-thread FFI-gather + a parallel convert/merge actually pay off?
//!
//! We can't measure the FFI half without a live Godot, but that half stays serial
//! regardless. What's in question is the *parallelizable* half: `to_bevy_transform`
//! (pure glam -- orthonormalize + quaternion extraction + column-length scale) plus
//! the per-axis shadow merge. This bench runs that half over a real Bevy `Query`,
//! serial (`iter_mut`) vs parallel (`par_iter_mut`), so we learn whether the CPU
//! cost is large enough to beat `par_iter` dispatch overhead.
//!
//! Run: cargo bench -p godot-bevy --bench transform_read_split

use bevy_ecs::prelude::*;
use bevy_math::{Quat, Vec3};
use bevy_tasks::{ComputeTaskPool, TaskPool};
use bevy_transform::components::Transform as BevyTransform;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use godot::builtin::{Basis, Transform3D, Vector3};
use godot_bevy::plugins::transforms::IntoBevyTransform;
use std::hint::black_box;

// The value most recently read from Godot for this entity (the gather-phase output).
#[derive(Component)]
struct GodotVal(Transform3D);

// The sync shadow: what Bevy and Godot last agreed on.
#[derive(Component)]
struct Shadow(BevyTransform);

const SCALE_EPSILON: f32 = 1e-5;
const ROTATION_EPSILON: f32 = 1e-5;

// Verbatim copies of the private helpers in sync_systems.rs so the bench exercises
// the real per-entity work.
fn rotation_differs(a: Quat, b: Quat) -> bool {
    let b = if a.dot(b) < 0.0 { -b } else { b };
    (a.x - b.x).abs() > ROTATION_EPSILON
        || (a.y - b.y).abs() > ROTATION_EPSILON
        || (a.z - b.z).abs() > ROTATION_EPSILON
        || (a.w - b.w).abs() > ROTATION_EPSILON
}

fn merge_godot_into_bevy(
    bevy: &mut BevyTransform,
    godot: &BevyTransform,
    shadow: &mut BevyTransform,
) -> bool {
    let mut merged = *bevy;
    let mut changed = false;
    for i in 0..3 {
        if godot.translation[i] != shadow.translation[i] {
            merged.translation[i] = godot.translation[i];
            shadow.translation[i] = godot.translation[i];
            changed = true;
        }
    }
    for i in 0..3 {
        if (godot.scale[i] - shadow.scale[i]).abs() > SCALE_EPSILON {
            merged.scale[i] = godot.scale[i];
            shadow.scale[i] = godot.scale[i];
            changed = true;
        }
    }
    if rotation_differs(godot.rotation, shadow.rotation) {
        merged.rotation = godot.rotation;
        shadow.rotation = godot.rotation;
        changed = true;
    }
    if changed {
        *bevy = merged;
    }
    changed
}

// A non-axis-aligned, non-identity Godot transform so orthonormalization and
// quaternion extraction do real work. Index varies it so entities aren't identical.
fn sample_godot_transform(i: usize) -> Transform3D {
    let t = i as f32 * 0.001;
    let basis = Basis::from_euler(
        godot::builtin::EulerOrder::XYZ,
        Vector3::new(0.3 + t, 0.7 - t, 1.1 + t),
    )
    .scaled(Vector3::new(1.5, 2.0, 0.75));
    Transform3D {
        basis,
        origin: Vector3::new(t, -t, t * 2.0),
    }
}

fn setup_world(n: usize) -> World {
    let mut world = World::new();
    for i in 0..n {
        // Shadow deliberately differs from the incoming value so the merge writes
        // (worst case for the write-back; the conversion runs unconditionally anyway).
        world.spawn((
            BevyTransform::IDENTITY,
            GodotVal(sample_godot_transform(i)),
            Shadow(BevyTransform::from_translation(Vec3::splat(-1.0))),
        ));
    }
    world
}

fn read_serial(query: &mut Query<(&mut BevyTransform, &GodotVal, &mut Shadow)>) {
    for (mut bevy, godot_val, mut shadow) in query.iter_mut() {
        let gt = godot_val.0.to_bevy_transform();
        // Edit through a plain &mut so we don't trip Changed on a no-op (mirrors
        // the real system's deref-only-when-changed contract).
        let mut t = *bevy;
        if merge_godot_into_bevy(&mut t, &gt, &mut shadow.0) {
            *bevy = t;
        }
        black_box(&*bevy);
    }
}

fn read_parallel(query: &mut Query<(&mut BevyTransform, &GodotVal, &mut Shadow)>) {
    query
        .par_iter_mut()
        .for_each(|(mut bevy, godot_val, mut shadow)| {
            let gt = godot_val.0.to_bevy_transform();
            let mut t = *bevy;
            if merge_godot_into_bevy(&mut t, &gt, &mut shadow.0) {
                *bevy = t;
            }
            black_box(&*bevy);
        });
}

fn bench(c: &mut Criterion) {
    ComputeTaskPool::get_or_init(TaskPool::default);
    let threads = ComputeTaskPool::get().thread_num();

    let mut group = c.benchmark_group("transform_read");
    for &n in &[50usize, 100, 250, 500, 1_000, 2_000, 5_000] {
        let mut world = setup_world(n);
        let mut state = world.query::<(&mut BevyTransform, &GodotVal, &mut Shadow)>();

        group.bench_with_input(BenchmarkId::new("serial", n), &n, |b, _| {
            b.iter(|| {
                let mut q = state.query_mut(&mut world);
                read_serial(&mut q);
            });
        });
        group.bench_with_input(
            BenchmarkId::new(format!("parallel_{threads}t"), n),
            &n,
            |b, _| {
                b.iter(|| {
                    let mut q = state.query_mut(&mut world);
                    read_parallel(&mut q);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
