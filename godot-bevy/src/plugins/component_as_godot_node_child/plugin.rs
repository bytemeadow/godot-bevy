use crate::interop::GodotNodeHandle;
use crate::prelude::bevy_prelude::{
    App, Commands, Component, Entity, ParamSet, Query, With, debug, warn,
};
use bevy::app::Plugin;
use godot::classes::Node;
use godot::obj::Gd;
use std::collections::HashMap;

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
    mut param_set: ParamSet<(
        Query<
            (Entity, &mut GodotNodeHandle, &ComponentToMove),
            With<UninitializedBevyComponentNode>,
        >,
        Query<(&mut GodotNodeHandle, Entity)>,
    )>,
    mut commands: Commands,
) {
    // TODO: Is there a way child components can be moved to their parent during
    // TODO: scene_tree::plugin::create_scene_tree_entity? Is that even needed?
    if param_set.p0().iter().count() > 0 {
        let entities_param = param_set.p1();
        let ent_mapping = entities_param
            .iter()
            .map(|(reference, ent)| (reference.instance_id(), ent))
            .collect::<HashMap<_, _>>();

        let mut uninitialized_comp_param = param_set.p0();
        for (entity, mut handle, component) in uninitialized_comp_param.iter_mut() {
            let node: Gd<Node> = handle.get::<Node>();
            if let Some(parent) = node.get_parent() {
                let parent_entity = ent_mapping[&parent.instance_id()];
                commands.entity(parent_entity).insert(component.clone());
                commands
                    .entity(entity)
                    .remove::<ComponentToMove>()
                    .remove::<UninitializedBevyComponentNode>();
                // TODO: Clean up logging
                // godot_print!(
                //     "Registered Component '{}' with parent '{}'",
                //     node.get_name(),
                //     parent.get_name()
                // );
            } else {
                warn!("Entity {} has no parent", entity);
            }
        }
    }
}
