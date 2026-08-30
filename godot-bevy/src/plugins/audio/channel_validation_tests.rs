    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_clamp_volume() {
            assert_eq!(clamp_volume(-0.5), 0.0);
            assert_eq!(clamp_volume(0.5), 0.5);
            assert_eq!(clamp_volume(1.5), 1.0);
            assert_eq!(clamp_volume(0.0), 0.0);
            assert_eq!(clamp_volume(1.0), 1.0);
        }

        #[test]
        fn test_clamp_pitch() {
            assert_eq!(clamp_pitch(0.05), 0.1);
            assert_eq!(clamp_pitch(2.0), 2.0);
            assert_eq!(clamp_pitch(5.0), 4.0);
            assert_eq!(clamp_pitch(0.1), 0.1);
            assert_eq!(clamp_pitch(4.0), 4.0);
        }

        #[test]
        fn test_clamp_panning() {
            assert_eq!(clamp_panning(-2.0), -1.0);
            assert_eq!(clamp_panning(0.0), 0.0);
            assert_eq!(clamp_panning(2.0), 1.0);
            assert_eq!(clamp_panning(-1.0), -1.0);
            assert_eq!(clamp_panning(1.0), 1.0);
        }

        #[test]
        fn test_validation_functions() {
            assert!(is_valid_volume(0.5));
            assert!(!is_valid_volume(-0.1));
            assert!(!is_valid_volume(1.1));
            assert!(!is_valid_volume(f32::NAN));

            assert!(is_valid_pitch(2.0));
            assert!(!is_valid_pitch(0.05));
            assert!(!is_valid_pitch(5.0));

            assert!(is_valid_panning(0.0));
            assert!(!is_valid_panning(-1.5));
            assert!(!is_valid_panning(1.5));
        }
    }
