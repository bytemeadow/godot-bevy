use bevy::prelude::Component;
use godot::prelude::GString;
use godot_bevy::prelude::GodotNode;

#[derive(Component)]
pub struct WeaponKind(pub String);

#[derive(Component, GodotNode, Default)]
#[gdbevy(class_name = WeaponNode)]
#[gdbevy(require(
    kind: WeaponKind,
    as = GString,
    with = from_godot_string,
    default = GString::from("Hands"),
    description = "Weapon selected by the designer",
    hint = ENUM,
    hint_string = "Hands,Knife"
))]
pub struct Weapon;

fn from_godot_string(value: GString) -> String {
    value.to_string()
}
