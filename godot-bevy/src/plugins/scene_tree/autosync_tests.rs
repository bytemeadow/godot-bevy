#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::App;
    use bevy_ecs::prelude::Component;

    #[derive(Component)]
    struct Primary;
    #[derive(Component, PartialEq, Debug, Default)]
    struct Companion(u8);

    fn register_primary(world: &mut World) {
        let _ = world.try_register_required_components_with::<Primary, Companion>(|| Companion(7));
    }
    crate::inventory::submit! {
        GodotRequiredComponents { component_name: "Primary", registrar_fn: register_primary }
    }

    #[test]
    fn registrar_entries_apply_to_new_world() {
        let mut app = App::new();
        register_all_required_components(&mut app);
        let e = app.world_mut().spawn(Primary).id();
        assert_eq!(app.world().get::<Companion>(e), Some(&Companion(7)));
    }
}
