use crate::prelude::bevy_prelude::{App, ChildOf, Commands, Component, Entity, Query, With, debug};
use bevy::app::Plugin;

pub struct ChildComponentRegistry {
    pub create_system_fn: fn(&mut App),
}

inventory::collect!(ChildComponentRegistry);

#[derive(Default, Debug)]
pub struct ComponentAsGodotNodeChildPlugin;

impl Plugin for ComponentAsGodotNodeChildPlugin {
    fn build(&self, app: &mut App) {
        inventory::iter::<ChildComponentRegistry>()
            .for_each(|registry| (registry.create_system_fn)(app));
        debug!(
            "Registered child node components: Count={}",
            inventory::iter::<ChildComponentRegistry>().count()
        );
    }
}

#[derive(Default, Debug, Component)]
pub struct UninitializedBevyComponentNode;

pub fn move_component_from_child_to_parent<ComponentToMove: Component + Clone>(
    query: Query<(Entity, &ChildOf), (With<UninitializedBevyComponentNode>, With<ComponentToMove>)>,
    mut commands: Commands,
) {
    for (entity, child_of) in query.iter() {
        commands
            .entity(entity)
            .remove::<UninitializedBevyComponentNode>()
            .move_components::<ComponentToMove>(child_of.parent());
    }
}
