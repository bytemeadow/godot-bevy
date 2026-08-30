#[cfg(test)]
mod tests {
    use std::f32;

    use super::*;

    const EPSILON: f32 = 1e-5;

    fn assert_vec3_near(a: Vec3, b: Vec3, epsilon: f32) {
        assert!(
            (a.x - b.x).abs() < epsilon,
            "x component mismatch: {} vs {}",
            a.x,
            b.x
        );
        assert!(
            (a.y - b.y).abs() < epsilon,
            "y component mismatch: {} vs {}",
            a.y,
            b.y
        );
        assert!(
            (a.z - b.z).abs() < epsilon,
            "z component mismatch: {} vs {}",
            a.z,
            b.z
        );
    }

    fn assert_quat_near(a: Quat, b: Quat, epsilon: f32) {
        assert!(
            !quats_differ(a, b, epsilon),
            "quaternion mismatch: {a:?} vs {b:?}"
        );
    }

    #[test]
    fn test_vector3_conversions() {
        let bevy_vec = Vec3::new(1.0, 2.0, 3.0);
        let godot_vec = bevy_vec.to_vector3();
        assert_eq!(godot_vec.x, bevy_vec.x);
        assert_eq!(godot_vec.y, bevy_vec.y);
        assert_eq!(godot_vec.z, bevy_vec.z);

        let godot_vec = Vector3::new(4.0, 5.0, 6.0);
        let bevy_vec = godot_vec.to_vec3();
        assert_eq!(bevy_vec.x, godot_vec.x);
        assert_eq!(bevy_vec.y, godot_vec.y);
        assert_eq!(bevy_vec.z, godot_vec.z);

        let original = Vec3::new(1.5, -2.7, f32::consts::PI);
        let round_trip = original.to_vector3().to_vec3();
        assert_vec3_near(original, round_trip, EPSILON);
    }

    #[test]
    fn test_quaternion_conversions() {
        let bevy_quat = Quat::from_rotation_y(std::f32::consts::PI / 4.0);
        let godot_quat = bevy_quat.to_quaternion();
        assert!((godot_quat.x - bevy_quat.x).abs() < EPSILON);
        assert!((godot_quat.y - bevy_quat.y).abs() < EPSILON);
        assert!((godot_quat.z - bevy_quat.z).abs() < EPSILON);
        assert!((godot_quat.w - bevy_quat.w).abs() < EPSILON);

        let godot_quat = Quaternion::new(0.0, 0.707, 0.0, 0.707);
        let bevy_quat = godot_quat.to_quat();
        assert!((bevy_quat.x - godot_quat.x).abs() < EPSILON);
        assert!((bevy_quat.y - godot_quat.y).abs() < EPSILON);
        assert!((bevy_quat.z - godot_quat.z).abs() < EPSILON);
        assert!((bevy_quat.w - godot_quat.w).abs() < EPSILON);

        let original = Quat::from_euler(bevy_math::EulerRot::XYZ, 0.1, 0.2, 0.3);
        let round_trip = original.to_quaternion().to_quat();
        assert_quat_near(original, round_trip, EPSILON);
    }

    #[test]
    fn test_transform_3d_identity() {
        let bevy_transform = BevyTransform::IDENTITY;
        let godot_transform = bevy_transform.to_godot_transform();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(back_to_bevy.translation, Vec3::ZERO, EPSILON);
        assert_quat_near(back_to_bevy.rotation, Quat::IDENTITY, EPSILON);
        assert_vec3_near(back_to_bevy.scale, Vec3::ONE, EPSILON);
    }

    #[test]
    fn test_transform_3d_translation_only() {
        let bevy_transform = BevyTransform::from_translation(Vec3::new(10.0, 20.0, 30.0));
        let godot_transform = bevy_transform.to_godot_transform();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(
            back_to_bevy.translation,
            bevy_transform.translation,
            EPSILON,
        );
        assert_quat_near(back_to_bevy.rotation, Quat::IDENTITY, EPSILON);
        assert_vec3_near(back_to_bevy.scale, Vec3::ONE, EPSILON);
    }

    #[test]
    fn test_transform_3d_rotation_only() {
        let bevy_transform =
            BevyTransform::from_rotation(Quat::from_rotation_y(std::f32::consts::PI / 3.0));
        let godot_transform = bevy_transform.to_godot_transform();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(back_to_bevy.translation, Vec3::ZERO, EPSILON);
        assert_quat_near(back_to_bevy.rotation, bevy_transform.rotation, EPSILON);
        assert_vec3_near(back_to_bevy.scale, Vec3::ONE, EPSILON);
    }

    #[test]
    fn test_transform_3d_scale_only() {
        let bevy_transform = BevyTransform::from_scale(Vec3::new(2.0, 0.5, 3.0));
        let godot_transform = bevy_transform.to_godot_transform();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(back_to_bevy.translation, Vec3::ZERO, EPSILON);
        assert_quat_near(back_to_bevy.rotation, Quat::IDENTITY, EPSILON);
        assert_vec3_near(back_to_bevy.scale, bevy_transform.scale, EPSILON);
    }

    #[test]
    fn test_transform_3d_complex() {
        let bevy_transform = BevyTransform {
            translation: Vec3::new(5.0, -10.0, 15.0),
            rotation: Quat::from_euler(bevy_math::EulerRot::XYZ, 0.1, 0.2, 0.3),
            scale: Vec3::new(1.5, 2.0, 0.75),
        };
        let godot_transform = bevy_transform.to_godot_transform();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(
            back_to_bevy.translation,
            bevy_transform.translation,
            EPSILON,
        );
        assert_quat_near(back_to_bevy.rotation, bevy_transform.rotation, EPSILON);
        assert_vec3_near(back_to_bevy.scale, bevy_transform.scale, EPSILON);
    }

    #[test]
    fn test_transform_2d_identity() {
        let bevy_transform = BevyTransform::IDENTITY;
        let godot_transform = bevy_transform.to_godot_transform_2d();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(back_to_bevy.translation, Vec3::ZERO, EPSILON);
        // For 2D, we only care about Z rotation
        assert!((back_to_bevy.scale.x - 1.0).abs() < EPSILON);
        assert!((back_to_bevy.scale.y - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_transform_2d_translation_only() {
        let bevy_transform = BevyTransform::from_translation(Vec3::new(10.0, 20.0, 0.0));
        let godot_transform = bevy_transform.to_godot_transform_2d();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert!((back_to_bevy.translation.x - bevy_transform.translation.x).abs() < EPSILON);
        assert!((back_to_bevy.translation.y - bevy_transform.translation.y).abs() < EPSILON);
        assert!((back_to_bevy.scale.x - 1.0).abs() < EPSILON);
        assert!((back_to_bevy.scale.y - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_transform_2d_rotation_only() {
        let angle = std::f32::consts::PI / 4.0;
        let bevy_transform = BevyTransform::from_rotation(Quat::from_rotation_z(angle));
        let godot_transform = bevy_transform.to_godot_transform_2d();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(back_to_bevy.translation, Vec3::ZERO, EPSILON);

        let (_, _, z_rot) = back_to_bevy.rotation.to_euler(bevy_math::EulerRot::XYZ);
        assert!(
            (z_rot - angle).abs() < EPSILON,
            "Z rotation mismatch: {z_rot} vs {angle}"
        );

        assert!((back_to_bevy.scale.x - 1.0).abs() < EPSILON);
        assert!((back_to_bevy.scale.y - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_transform_2d_scale_only() {
        let bevy_transform = BevyTransform::from_scale(Vec3::new(2.0, 0.5, 1.0));
        let godot_transform = bevy_transform.to_godot_transform_2d();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(back_to_bevy.translation, Vec3::ZERO, EPSILON);
        assert!((back_to_bevy.scale.x - bevy_transform.scale.x).abs() < EPSILON);
        assert!((back_to_bevy.scale.y - bevy_transform.scale.y).abs() < EPSILON);
    }

    #[test]
    fn test_transform_2d_complex() {
        let bevy_transform = BevyTransform {
            translation: Vec3::new(5.0, -10.0, 0.0),
            rotation: Quat::from_rotation_z(0.785), // 45 degrees
            scale: Vec3::new(1.5, 2.0, 1.0),
        };
        let godot_transform = bevy_transform.to_godot_transform_2d();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert!((back_to_bevy.translation.x - bevy_transform.translation.x).abs() < EPSILON);
        assert!((back_to_bevy.translation.y - bevy_transform.translation.y).abs() < EPSILON);

        let (_, _, original_z) = bevy_transform.rotation.to_euler(bevy_math::EulerRot::XYZ);
        let (_, _, back_z) = back_to_bevy.rotation.to_euler(bevy_math::EulerRot::XYZ);
        assert!(
            (back_z - original_z).abs() < EPSILON,
            "Z rotation mismatch: {back_z} vs {original_z}"
        );

        assert!((back_to_bevy.scale.x - bevy_transform.scale.x).abs() < EPSILON);
        assert!((back_to_bevy.scale.y - bevy_transform.scale.y).abs() < EPSILON);
    }

    #[test]
    fn test_vector2_to_vec3() {
        let vec2 = Vector2::new(1.0, 2.0);
        let vec3 = vec2.to_vec3();
        assert_eq!(vec3.x, 1.0);
        assert_eq!(vec3.y, 2.0);
        assert_eq!(vec3.z, 0.0);
    }

    #[test]
    fn transform_2d_scale_threshold_and_pi_sign_are_deterministic() {
        let threshold = GodotTransform2D {
            a: Vector2::new(0.0, 1e-6),
            b: Vector2::new(-1e-6, 0.0),
            origin: Vector2::ZERO,
        }
        .to_bevy_transform();
        assert_eq!(threshold.rotation, Quat::IDENTITY);

        let positive_pi = GodotTransform2D {
            a: Vector2::new(-1.0, 0.0),
            b: Vector2::new(0.0, -1.0),
            origin: Vector2::ZERO,
        }
        .to_bevy_transform();
        assert_eq!(positive_pi.rotation.z, 1.0);
        assert_eq!(positive_pi.rotation.w, 0.0);

        let negative_pi = GodotTransform2D {
            a: Vector2::new(-1.0, -1e-20),
            b: Vector2::new(1e-20, -1.0),
            origin: Vector2::ZERO,
        }
        .to_bevy_transform();
        assert_eq!(negative_pi.rotation.z, -1.0);
        assert_eq!(negative_pi.rotation.w, 0.0);
    }

    #[test]
    fn transform_2d_general_quaternion_uses_full_euler_rotation() {
        let rotation = Quat::from_xyzw(0.5, 0.0, 0.5, std::f32::consts::FRAC_1_SQRT_2);
        let transform = BevyTransform {
            translation: Vec3::new(7.0, 11.0, 13.0),
            rotation,
            scale: Vec3::new(2.0, 3.0, 1.0),
        }
        .to_godot_transform_2d();

        assert!((transform.a.x - 1.154_700_5).abs() < 1e-6);
        assert!((transform.a.y - 1.632_993_2).abs() < 1e-6);
        assert!((transform.b.x + 2.449_489_8).abs() < 1e-6);
        assert!((transform.b.y - 1.732_050_8).abs() < 1e-6);
        assert_eq!(transform.origin, Vector2::new(7.0, 11.0));
    }

    #[test]
    fn transform_2d_y_component_alone_takes_the_euler_path() {
        let rotation = Quat::from_xyzw(0.0, 0.5, 0.5, std::f32::consts::FRAC_1_SQRT_2);
        let transform = BevyTransform {
            translation: Vec3::new(7.0, 11.0, 13.0),
            rotation,
            scale: Vec3::new(2.0, 3.0, 1.0),
        }
        .to_godot_transform_2d();

        // euler z is exactly PI/2 for this quat; the pure-z fast path would
        // instead produce a.x = (w*w - z*z) * 2.0 = 0.5
        assert!(transform.a.x.abs() < 1e-6);
        assert!((transform.a.y - 2.0).abs() < 1e-6);
        assert!((transform.b.x + 3.0).abs() < 1e-6);
        assert!(transform.b.y.abs() < 1e-6);
    }

    #[test]
    fn transform_2d_fast_path_thresholds_fall_back_to_euler() {
        for rotation in [
            Quat::from_xyzw(1e-6, 0.5e-6, 1.0, -0.5e-12),
            Quat::from_xyzw(0.5e-6, 1e-6, 1.0, -0.5e-12),
        ] {
            let transform = BevyTransform::from_rotation(rotation).to_godot_transform_2d();
            assert!(transform.a.y.abs() > 1e-8);
        }
    }

    #[test]
    fn quaternion_difference_checks_sign_boundary_and_each_component() {
        let canonical = Quat::from_xyzw(0.5, 0.5, 0.5, 0.5);
        assert!(!quats_differ(canonical, -canonical, 0.01));

        let dot_zero_a = Quat::from_xyzw(0.75, 0.5, 0.5, 0.5);
        let dot_zero_b = Quat::from_xyzw(0.75, -0.5, -0.3125, -0.3125);
        assert_eq!(dot_zero_a.dot(dot_zero_b), 0.0);
        assert!(!quats_differ(dot_zero_a, dot_zero_b, 1.0));

        let zero = Quat::from_xyzw(0.0, 0.0, 0.0, 0.0);
        for boundary in [
            Quat::from_xyzw(0.25, 0.0, 0.0, 0.0),
            Quat::from_xyzw(0.0, 0.25, 0.0, 0.0),
            Quat::from_xyzw(0.0, 0.0, 0.25, 0.0),
            Quat::from_xyzw(0.0, 0.0, 0.0, 0.25),
        ] {
            assert!(!quats_differ(zero, boundary, 0.25));
        }
        for different in [
            Quat::from_xyzw(0.5, 0.0, 0.0, 0.0),
            Quat::from_xyzw(0.0, 0.5, 0.0, 0.0),
            Quat::from_xyzw(0.0, 0.0, 0.5, 0.0),
            Quat::from_xyzw(0.0, 0.0, 0.0, 0.5),
        ] {
            assert!(quats_differ(zero, different, 0.25));
        }
    }
}
