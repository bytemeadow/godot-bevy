#[cfg(test)]
mod tests {
    #[test]
    fn public_exports_keep_their_paths() {
        let config: crate::prelude::DebuggerConfig = crate::plugins::DebuggerConfig::default();
        let _: crate::prelude::GodotDebuggerPlugin = crate::plugins::GodotDebuggerPlugin;
        let _: crate::prelude::InspectorReadOnly = crate::plugins::InspectorReadOnly;
        let range: crate::prelude::InspectorRange = crate::plugins::InspectorRange::new(0.0, 10.0);
        assert_eq!((range.min, range.max), (0.0, 10.0));
        let _: crate::prelude::DebuggerSet = crate::plugins::DebuggerSet::Drain;
        let _: crate::prelude::SceneTreeSet = crate::plugins::SceneTreeSet::Apply;
        let _: crate::plugins::scene_tree::SceneTreeSet = crate::plugins::SceneTreeSet::Apply;
        assert!(config.enabled);
        assert_eq!(config.update_interval, 0.5);
    }
}
