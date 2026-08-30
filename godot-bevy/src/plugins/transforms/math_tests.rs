#[cfg(test)]
mod tests {
    use super::*;
    use bevy_math::Vec3;
    use std::f32::consts::PI;

    #[test]
    fn test_extract_rotation_from_2d_matrix() {
        // Test identity matrix (no rotation)
        assert!((extract_rotation_from_2d_matrix(1.0, 0.0) - 0.0).abs() < 1e-6);

        // Test 90-degree rotation
        assert!((extract_rotation_from_2d_matrix(0.0, 1.0) - PI / 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_extract_scale_from_2d_matrix() {
        let (scale_x, scale_y) = extract_scale_from_2d_matrix(3.0, 4.0, 5.0, 12.0);
        assert_eq!(scale_x, 5.0);
        assert_eq!(scale_y, 13.0);
    }

    #[test]
    fn test_create_2d_rotation_matrix() {
        let ((a_x, a_y), (b_x, b_y)) = create_2d_rotation_matrix(PI / 6.0, 2.0, 3.0);
        assert!((a_x - 1.732_050_8).abs() < 1e-6);
        assert!((a_y - 1.0).abs() < 1e-6);
        assert!((b_x + 1.5).abs() < 1e-6);
        assert!((b_y - 2.598_076).abs() < 1e-6);
    }

    #[test]
    fn test_validate_transform_for_conversion() {
        // Valid transform
        let valid_transform = Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::new(1.0, 1.0, 1.0),
        };
        assert!(validate_transform_for_conversion(&valid_transform));

        // Invalid translation (NaN)
        let invalid_transform = Transform {
            translation: Vec3::new(f32::NAN, 2.0, 3.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::new(1.0, 1.0, 1.0),
        };
        assert!(!validate_transform_for_conversion(&invalid_transform));

        let non_normalized_rotation = Transform {
            translation: Vec3::ZERO,
            rotation: Quat::from_xyzw(0.0, 0.0, 0.0, 2.0),
            scale: Vec3::ONE,
        };
        assert!(!validate_transform_for_conversion(&non_normalized_rotation));

        let zero_scale = Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::new(1.0, 0.0, 1.0),
        };
        assert!(!validate_transform_for_conversion(&zero_scale));
    }

    #[test]
    fn test_extract_z_rotation_from_quat() {
        // Test identity quaternion
        assert!(extract_z_rotation_from_quat(Quat::IDENTITY).abs() < 1e-6);

        // Test Z rotation
        let z_rot_quat = Quat::from_rotation_z(PI / 4.0);
        assert!((extract_z_rotation_from_quat(z_rot_quat) - PI / 4.0).abs() < 1e-6);
    }
}
