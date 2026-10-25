use bevy::prelude::Component;
use godot::prelude::*;
use godot_bevy::prelude::BevyComponents;

#[derive(Component, Default)]
pub struct ChallengeSelect {
    pub back_scene_path: String,
    pub accept_scene_path: String,
}

#[derive(GodotClass, BevyComponents)]
#[class(init, base = CanvasLayer)]
#[gdbevy(require(ChallengeSelect {
    back_scene_path: back_scene_path,
    accept_scene_path: accept_scene_path,
}))]
pub struct ChallengeSelectUI {
    #[export(file = "*.tscn")]
    #[gdbevy(with = String::from)]
    pub back_scene_path: GString,
    #[export(file = "*.tscn")]
    #[gdbevy(with = String::from)]
    pub accept_scene_path: GString,
}
