#![allow(dead_code, unused_variables)]

mod node_tree_view {
    use godot_bevy::prelude::{GodotNodeHandle, NodeTreeView};

    #[derive(NodeTreeView)]
    pub struct MenuUi {
        #[node("/root/Main/HUD/Message")]
        pub message_label: GodotNodeHandle,
    }
}

mod node_tree_paths {
    use godot_bevy::prelude::{GodotNodeHandle, NodeTreeView};

    #[derive(NodeTreeView)]
    pub struct MobNodes {
        #[node("AnimatedSprite2D")]
        animated_sprite: GodotNodeHandle,
        #[node("Node2D/*/VisibleOnScreenNotifier2D")]
        visibility_notifier: GodotNodeHandle,
    }

    const _: &str = MobNodes::ANIMATED_SPRITE_PATH;
    const _: &str = MobNodes::VISIBILITY_NOTIFIER_PATH;
}

mod bevy_components_fields {
    use bevy::prelude::Component;
    use godot::classes::Node2D;
    use godot::prelude::{Base, GodotClass};
    use godot_bevy::prelude::BevyComponents;

    #[derive(Component)]
    struct Speed(f32);

    #[derive(Component)]
    struct Health(f32);

    fn to_speed(value: f32) -> f32 {
        value
    }

    #[derive(GodotClass, BevyComponents)]
    #[class(init, base=Node2D)]
    struct PlayerNode {
        base: Base<Node2D>,
        #[export]
        #[gdbevy(component = Speed, with = to_speed)]
        speed: f32,
        #[export]
        #[gdbevy(component = Health)]
        health: f32,
    }
}

mod bevy_components_require {
    use bevy::prelude::Component;
    use godot::classes::Node2D;
    use godot::prelude::{Base, GodotClass};
    use godot_bevy::prelude::BevyComponents;

    #[derive(Component, Default)]
    struct Player;

    #[derive(Component, Default)]
    struct Stats {
        current: f32,
        max: f32,
    }

    #[derive(GodotClass, BevyComponents)]
    #[class(init, base=Node2D)]
    #[gdbevy(require(Player))]
    #[gdbevy(require(Stats { current: max_health, max: max_health }))]
    struct PlayerNode {
        base: Base<Node2D>,
        #[export]
        max_health: f32,
    }
}

mod godot_node {
    use bevy::prelude::Component;
    use godot_bevy::prelude::GodotNode;

    #[derive(Component)]
    struct Speed(f32);

    #[derive(Component, Default)]
    struct Stunned;

    #[derive(Component, GodotNode, Default)]
    #[gdbevy(base = CharacterBody2D, class_name = Player2D)]
    #[gdbevy(require(speed: Speed, as = f32, default = 250.0))]
    #[gdbevy(require(Stunned))]
    struct Player;
}

mod godot_node_marker {
    use bevy::prelude::Component;
    use godot_bevy::prelude::GodotNode;

    #[derive(Component, Default)]
    struct Stunned;

    #[derive(Component, GodotNode, Default)]
    #[gdbevy(class_name = MarkerExampleNode)]
    #[gdbevy(require(Stunned))]
    struct MarkerExample;
}

mod godot_node_newtype {
    use bevy::prelude::Component;
    use godot_bevy::prelude::GodotNode;

    #[derive(Component)]
    struct Speed(f32);

    fn to_speed(value: f32) -> f32 {
        value
    }

    #[derive(Component, GodotNode, Default)]
    #[gdbevy(class_name = NewtypeExampleNode)]
    #[gdbevy(require(speed: Speed, as = f32, default = 250.0, with = to_speed))]
    struct NewtypeExample;
}

mod godot_node_struct {
    use bevy::prelude::Component;
    use godot_bevy::prelude::GodotNode;

    #[derive(Component, Default)]
    struct Stats {
        current: i32,
        max: i32,
    }

    #[derive(Component, GodotNode, Default)]
    #[gdbevy(class_name = StructExampleNode)]
    #[gdbevy(require(stats: Stats { current(as = i32, default = 100), max(as = i32, default = 100) }))]
    struct StructExample;
}

mod godot_node_fields {
    use bevy::prelude::Component;
    use godot::prelude::{Export, GString, GodotConvert, Var};
    use godot_bevy::prelude::GodotNode;

    #[derive(Clone, Copy, Default, GodotConvert, Var, Export)]
    #[godot(via = GString)]
    pub enum LevelId {
        #[default]
        Level1,
    }

    fn meters_to_units(value: f32) -> f32 {
        value * 100.0
    }

    #[derive(Component, GodotNode, Default)]
    #[gdbevy(base = Area2D, class_name = Door2D)]
    struct Door {
        #[gdbevy(export, default = LevelId::Level1)]
        level_id: LevelId,
        #[gdbevy(export, as = f32, with = meters_to_units)]
        range: f32,
        /// Label shown to the player.
        #[gdbevy(export, as = GString, with = from_godot_string,
            default = GString::from("Open"), description = "Door label",
            hint = ENUM, hint_string = "Open,Locked")]
        label: String,
    }

    fn from_godot_string(value: GString) -> String {
        value.to_string()
    }
}

mod godot_node_tuple {
    use bevy::prelude::Component;
    use godot_bevy::prelude::GodotNode;

    #[derive(Component, GodotNode, Default)]
    struct Velocity(#[gdbevy(export)] f32, #[gdbevy(export)] f32);
}

mod itest {
    use godot_bevy_test::prelude::*;

    #[itest]
    async fn my_test(ctx: TestContext) {
        let app = TestApp::new(&ctx, |_| {}).await;
        app.update().await;
    }

    #[itest]
    fn my_sync_test(_ctx: &TestContext) {}

    #[itest(async)]
    fn my_async_test(_ctx: &TestContext) -> godot::task::TaskHandle {
        godot::task::spawn(async move {})
    }

    #[itest(skip)]
    fn skipped_test(_ctx: &TestContext) {}

    #[itest(focus)]
    fn focused_test(_ctx: &TestContext) {}
}

mod bench {
    use godot_bevy_test::bench;

    #[bench]
    fn my_benchmark() -> u64 {
        42
    }

    #[bench(repeat = 25)]
    fn expensive_benchmark() -> u64 {
        42
    }
}

mod attachable_component {
    use bevy::prelude::{Component, Vec2};
    use godot::classes::CharacterBody3D;
    use godot::obj::OnEditor;
    use godot::prelude::*;
    use godot_bevy::prelude::*;

    #[derive(AttachableComponent, GodotClass)]
    #[class(init, base=Node)]
    #[gdbevy(target = Movement)]
    struct MovementComponent {
        #[export]
        max_speed: f32,
        #[export]
        jump_height: f32,
        #[export]
        character_body: OnEditor<Gd<CharacterBody3D>>,
    }

    #[derive(Component)]
    struct Movement {
        max_speed: f32,
        jump_height: f32,
        character_body_3d: GodotNodeHandle,
        desired_direction: Vec2,
    }

    impl From<&MovementComponent> for Movement {
        fn from(value: &MovementComponent) -> Movement {
            Movement {
                max_speed: value.max_speed,
                jump_height: value.jump_height,
                character_body_3d: GodotNodeHandle::new(value.character_body.clone()),
                desired_direction: Vec2::ZERO,
            }
        }
    }
}
