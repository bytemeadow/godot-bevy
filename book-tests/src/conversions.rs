use bevy::prelude::Vec3;
use godot_bevy::prelude::*;

pub fn convert_position() {
    let position = Vec3::new(1.0, 2.0, 3.0);
    let godot_position = position.to_vector3();
    let bevy_position = godot_position.to_vec3();
    assert_eq!(bevy_position, position);
}
