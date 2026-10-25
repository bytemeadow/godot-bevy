// ANCHOR: health
use bevy::prelude::{Component, Query};
use godot_bevy::prelude::GodotNode;

#[derive(Component, Default, GodotNode)]
#[gdbevy(class_name = HealthView)]
pub struct Health {
    #[gdbevy(export, default = 100, sync = to_godot)]
    pub current: i32,
}

pub fn take_damage(mut health: Query<&mut Health>) {
    for mut health in &mut health {
        health.current -= 1;
    }
}
// ANCHOR_END: health

pub mod tuning {
    // ANCHOR: tuning
    use bevy::prelude::Component;
    use godot::prelude::*;
    use godot_bevy::prelude::BevyComponents;

    #[derive(Component, Default)]
    pub struct Movement {
        pub speed: f32,
        pub acceleration: f32,
    }

    #[derive(GodotClass, BevyComponents)]
    #[class(base = CharacterBody2D, init)]
    #[gdbevy(require(Movement { speed: speed, acceleration: acceleration }))]
    pub struct PlayerTuning {
        #[export]
        #[init(val = 250.0)]
        #[gdbevy(sync = two_way)]
        pub speed: f32,
        #[export]
        #[init(val = 800.0)]
        pub acceleration: f32,
    }
    // ANCHOR_END: tuning
}

pub mod conversion {
    // ANCHOR: conversion
    use bevy::prelude::Component;
    use godot_bevy::prelude::GodotNode;

    #[derive(Component, Default, GodotNode)]
    #[gdbevy(class_name = ChargeView)]
    pub struct Charge {
        #[gdbevy(export, as = f32, sync = two_way,
                 with = from_percent, into = to_percent)]
        pub fraction: f32,
    }

    fn from_percent(value: f32) -> f32 {
        value / 100.0
    }

    fn to_percent(value: f32) -> f32 {
        value * 100.0
    }
    // ANCHOR_END: conversion
}

pub mod companion {
    // ANCHOR: companion
    use bevy::prelude::Component;
    use godot_bevy::prelude::GodotNode;

    #[derive(Component)]
    pub struct Speed(pub f32);

    #[derive(Component, Default)]
    pub struct Stats {
        pub health: i32,
        pub mana: i32,
    }

    #[derive(Component, Default, GodotNode)]
    #[gdbevy(class_name = ActorView)]
    #[gdbevy(require(speed: Speed, as = f32, default = 250.0, sync = two_way))]
    #[gdbevy(require(stats: Stats {
        health(as = i32, default = 100, sync = to_godot),
        mana(as = i32, default = 50, sync = two_way),
    }))]
    pub struct Actor;

    #[derive(Component, Default, GodotNode)]
    #[gdbevy(class_name = LevelView)]
    pub struct Level(#[gdbevy(export, sync = two_way)] pub i32);
    // ANCHOR_END: companion
}

pub mod strings {
    // ANCHOR: strings
    use bevy::prelude::Component;
    use godot::prelude::*;
    use godot_bevy::prelude::BevyComponents;

    #[derive(Component)]
    pub struct Caption(pub String);

    #[derive(GodotClass, BevyComponents)]
    #[class(base = Node, init)]
    pub struct CaptionView {
        #[export]
        #[gdbevy(component = Caption, sync = two_way,
                 with = from_godot, into = to_godot)]
        pub caption: GString,
    }

    fn from_godot(value: GString) -> String {
        value.to_string()
    }

    fn to_godot(value: String) -> GString {
        GString::from(&value)
    }
    // ANCHOR_END: strings
}

// ANCHOR: ordering
use bevy::prelude::{App, IntoScheduleConfigs, PostUpdate, PreUpdate};
use godot_bevy::prelude::PropertySyncSet;

pub fn configure_property_consumers(app: &mut App) {
    app.add_systems(PreUpdate, read_health.after(PropertySyncSet::Read));
    app.add_systems(PostUpdate, take_damage.before(PropertySyncSet::Write));
}

fn read_health(health: Query<&Health>) {
    for health in &health {
        println!("Health: {}", health.current);
    }
}
// ANCHOR_END: ordering
