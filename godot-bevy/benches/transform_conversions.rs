//! Benchmarks for transform conversions between Bevy and Godot coordinate systems
//!
//! These conversions happen frequently during transform sync and are pure computation,
//! making them ideal for Criterion benchmarking.

use bevy::math::{Quat, Vec3};
use bevy::prelude::Transform as BevyTransform;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

// Import the actual conversion traits from our library
use godot_bevy::plugins::transforms::{
    IntoBevyTransform, IntoGodotTransform, IntoGodotTransform2D,
};
// These helper traits are in the conversions module but not re-exported
use godot::builtin::{
    Quaternion, Transform2D as GodotTransform2D, Transform3D as GodotTransform3D, Vector3,
};
use godot_bevy::plugins::transforms::conversions::{
    IntoQuat, IntoQuaternion, IntoVec3, IntoVector3,
};

fn bench_transform_conversions_3d(c: &mut Criterion) {
    let mut group = c.benchmark_group("transform_conversions_3d");

    // Test different transform complexities
    let bevy_transforms = vec![
        ("identity", BevyTransform::IDENTITY),
        (
            "translation_only",
            BevyTransform::from_translation(Vec3::new(100.0, 50.0, -30.0)),
        ),
        (
            "rotation_only",
            BevyTransform::from_rotation(Quat::from_rotation_y(1.57)),
        ),
        (
            "complex",
            BevyTransform {
                translation: Vec3::new(100.0, 50.0, -30.0),
                rotation: Quat::from_euler(bevy::math::EulerRot::XYZ, 0.5, 1.0, 0.3),
                scale: Vec3::new(2.0, 0.5, 1.5),
            },
        ),
    ];

    // Benchmark Bevy to Godot 3D conversion using actual trait
    for (name, transform) in &bevy_transforms {
        group.bench_with_input(
            BenchmarkId::new("bevy_to_godot_3d", name),
            transform,
            |b, t| {
                b.iter(|| {
                    let godot_transform: GodotTransform3D = black_box(*t).to_godot_transform();
                    black_box(godot_transform)
                })
            },
        );
    }

    // Create Godot transforms for reverse conversion
    let godot_transforms: Vec<(&str, GodotTransform3D)> = bevy_transforms
        .iter()
        .map(|(name, t)| (*name, t.to_godot_transform()))
        .collect();

    // Benchmark Godot to Bevy 3D conversion using actual trait
    for (name, transform) in &godot_transforms {
        group.bench_with_input(
            BenchmarkId::new("godot_3d_to_bevy", name),
            transform,
            |b, t| {
                b.iter(|| {
                    let bevy_transform: BevyTransform = black_box(*t).to_bevy_transform();
                    black_box(bevy_transform)
                })
            },
        );
    }

    group.finish();
}

fn bench_transform_conversions_2d(c: &mut Criterion) {
    let mut group = c.benchmark_group("transform_conversions_2d");

    // Test different 2D transform complexities
    let bevy_transforms = vec![
        ("identity_2d", BevyTransform::IDENTITY),
        (
            "translation_2d",
            BevyTransform::from_translation(Vec3::new(100.0, 50.0, 0.0)),
        ),
        (
            "rotation_2d",
            BevyTransform::from_rotation(Quat::from_rotation_z(1.57)),
        ),
        (
            "complex_2d",
            BevyTransform {
                translation: Vec3::new(100.0, 50.0, 0.0),
                rotation: Quat::from_rotation_z(0.785), // 45 degrees
                scale: Vec3::new(2.0, 0.5, 1.0),
            },
        ),
    ];

    // Benchmark Bevy to Godot 2D conversion using actual trait
    for (name, transform) in &bevy_transforms {
        group.bench_with_input(
            BenchmarkId::new("bevy_to_godot_2d", name),
            transform,
            |b, t| {
                b.iter(|| {
                    let godot_transform: GodotTransform2D = black_box(*t).to_godot_transform_2d();
                    black_box(godot_transform)
                })
            },
        );
    }

    // Create Godot 2D transforms for reverse conversion
    let godot_2d_transforms: Vec<(&str, GodotTransform2D)> = bevy_transforms
        .iter()
        .map(|(name, t)| (*name, t.to_godot_transform_2d()))
        .collect();

    // Benchmark Godot 2D to Bevy conversion using actual trait
    for (name, transform) in &godot_2d_transforms {
        group.bench_with_input(
            BenchmarkId::new("godot_2d_to_bevy", name),
            transform,
            |b, t| {
                b.iter(|| {
                    let bevy_transform: BevyTransform = black_box(*t).to_bevy_transform();
                    black_box(bevy_transform)
                })
            },
        );
    }

    group.finish();
}

fn bench_bulk_conversions(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_transform_conversions");

    for count in [10, 100, 1000, 10000].iter() {
        let transforms: Vec<BevyTransform> = (0..*count)
            .map(|i| BevyTransform {
                translation: Vec3::new(i as f32, i as f32 * 0.5, i as f32 * -0.3),
                rotation: Quat::from_rotation_y(i as f32 * 0.1),
                scale: Vec3::new(1.0 + (i as f32 * 0.01), 1.0, 1.0),
            })
            .collect();

        // Benchmark bulk Bevy to Godot 3D conversions
        group.bench_with_input(
            BenchmarkId::new("bulk_bevy_to_godot_3d", count),
            &transforms,
            |b, transforms| {
                b.iter(|| {
                    for t in transforms {
                        let godot_t: GodotTransform3D = t.to_godot_transform();
                        black_box(godot_t);
                    }
                })
            },
        );

        // Benchmark bulk Bevy to Godot 2D conversions
        group.bench_with_input(
            BenchmarkId::new("bulk_bevy_to_godot_2d", count),
            &transforms,
            |b, transforms| {
                b.iter(|| {
                    for t in transforms {
                        let godot_t: GodotTransform2D = t.to_godot_transform_2d();
                        black_box(godot_t);
                    }
                })
            },
        );
    }

    group.finish();
}

fn bench_vector_conversions(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_conversions");

    let test_vectors = [Vec3::ZERO, Vec3::ONE, Vec3::new(123.456, -789.012, 345.678)];

    // Benchmark Vec3 to Vector3 conversion
    for (i, vec) in test_vectors.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("vec3_to_vector3", i), vec, |b, v| {
            b.iter(|| {
                let godot_vec: Vector3 = v.to_vector3();
                black_box(godot_vec)
            })
        });
    }

    // Benchmark Vector3 to Vec3 conversion
    let godot_vectors: Vec<Vector3> = test_vectors.iter().map(|v| v.to_vector3()).collect();

    for (i, vec) in godot_vectors.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("vector3_to_vec3", i), vec, |b, v| {
            b.iter(|| {
                let bevy_vec: Vec3 = v.to_vec3();
                black_box(bevy_vec)
            })
        });
    }

    group.finish();
}

fn bench_quaternion_conversions(c: &mut Criterion) {
    let mut group = c.benchmark_group("quaternion_conversions");

    let test_quats = [
        Quat::IDENTITY,
        Quat::from_rotation_x(1.57),
        Quat::from_rotation_y(0.785),
        Quat::from_euler(bevy::math::EulerRot::XYZ, 0.1, 0.2, 0.3),
    ];

    // Benchmark Quat to Quaternion conversion
    for (i, quat) in test_quats.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("quat_to_quaternion", i), quat, |b, q| {
            b.iter(|| {
                let godot_quat: Quaternion = q.to_quaternion();
                black_box(godot_quat)
            })
        });
    }

    // Benchmark Quaternion to Quat conversion
    let godot_quats: Vec<Quaternion> = test_quats.iter().map(|q| q.to_quaternion()).collect();

    for (i, quat) in godot_quats.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("quaternion_to_quat", i), quat, |b, q| {
            b.iter(|| {
                let bevy_quat: Quat = q.to_quat();
                black_box(bevy_quat)
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_transform_conversions_3d,
    bench_transform_conversions_2d,
    bench_bulk_conversions,
    bench_vector_conversions,
    bench_quaternion_conversions
);
criterion_main!(benches);
