# Debugging

Godot-bevy's entity inspector shows Bevy entities, component values and scene-tree
relationships in the Godot editor while your game runs.

## Entity inspector

The **Entities** pane is the map of the running ECS world; Godot's **Inspector** shows
and edits the selected entity's components in a Bevy section below the node properties.
Node-backed entities contain their pure-ECS children through `GodotChildOf`. The pane hides
unnamed internal root entities by default, shows the hidden count, and has a toggle to reveal
them. Search matches names, decimal entity IDs, component type paths or an exact runtime node
path; matching entities keep their ancestors visible.

Selecting a node-backed entity selects it in Godot's Remote tree, where native node properties
remain editable. Remote selection also selects the entity in the pane. This adapter uses
version-specific editor widgets on 4.2–4.6; missing widgets show “not available”. Pure-ECS
entities use the same Bevy Inspector through a proxy object. Choose the running instance with
the session selector; the newest started session is the default.

The native Bevy section for node-backed entities requires Godot 4.4 or later. Godot 4.2
and 4.3 retain the previous component rows and identify that fallback in the section header;
the Entities pane remains available. The native section has one Bevy heading. Scalars and
tuples with one scalar field appear as one row named for the component, such as `Speed`. Components
with named fields or several values have foldable groups. Full Rust types are in tooltips;
generic labels preserve their arguments, such as `PreviousState<GameState>`, and expand to
full paths when short names collide. Runtime fields have no default-value revert.
Nested fields use Inspector sections; after three levels, field labels
show the remaining path as a breadcrumb with a copy action. Collection sizes are fixed,
and maps, sets, unsupported values and read-only fields retain their explanations.

### Enabling the inspector

`GodotDefaultPlugins` includes the inspector. To add it individually:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotDebuggerPlugin);
}
```

### Registering components

Enable automatic reflection registration on your Bevy dependency:

```toml
bevy = { version = "0.19", default-features = false, features = ["reflect_auto_register"] }
```

Derive `Reflect` on components you want to inspect. Add `#[reflect(Component)]`
to allow the runtime inspection service to edit their supported leaves:

```rust
use bevy::prelude::*;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Speed(pub f32);
```

The platformer example uses this automatic path for `Speed`, `JumpVelocity`,
`Gravity` and `Player`. Unregistered components display their unsupported reason.

For web and static builds, keep explicit registration as a fallback when automatic
registration has not been verified with your build and loader:

```rust
app.register_type::<Speed>();
```

Registration also includes reflected dependencies. Spawning a component or exporting
a Godot property does not register its type. Unregistered components remain listed;
`#[reflect(ignore)]` fields are hidden. Components without `ReflectComponent` can
still be read when registered, but cannot be edited generically.
`#[reflect(@InspectorReadOnly)]` shows a field without allowing edits;
`#[reflect(@InspectorRange::new(min, max))]` bounds scalar edits (both are in the prelude).

### Editing and refresh

Expand a component in the Inspector to edit supported scalar leaves or select a unit enum
variant. An edit displays the runtime's accepted value or an inline rejection reason; rejected
edits leave the accepted value unchanged. Wide integers use exact decimal text. Entity and
node references are navigation links. Game systems can overwrite accepted edits on later ticks.

Text and exact decimal integers commit on Enter; Escape restores the accepted value.
Submitting an edit disables its field and shows “Waiting for acknowledgement…”. The runtime
response supplies the accepted value; a rejection shows its reason beside a warning icon.
Value refreshes retain folds and unfinished input. Component shape or permission changes
rebuild the property rows while the proxy retains edit state and section folds.
The embedded section has its own property selection; the outer Inspector's property filter
does not filter its rows. Runtime edits do not enter the scene's undo history.

The pane subscribes only while visible, starting with a snapshot and applying later additions,
removals, renames, reparentings and component membership changes without rebuilding the tree.
Only the inspected entity's values are fetched at the subscription interval.
`DebuggerConfig.update_interval` supplies the default (0.5 seconds); configure it before
connecting the editor:

```rust
fn configure_debugger(mut config: ResMut<DebuggerConfig>) {
    config.enabled = cfg!(debug_assertions);
    config.update_interval = 1.0;
}
```

`DebuggerConfig::value_limits` bounds collection elements and nesting depth, with explicit
truncation markers. There is no unsolicited legacy entity stream.
`DebuggerConfig::snapshot_chunk_size` limits the initial snapshot to 256 entity summaries per
frame by default (zero uses one), sending one chunk per `First`; the pane shows a loading count
and keeps its last complete tree until the final chunk arrives.
The snapshot describes the world at subscription time; changes during loading follow in the
next delta, after the configured update interval.
Selections for entities still loading are retried after the final chunk. If the runtime
disables the debugger or a chunk arrives out of sequence, the pane discards the unfinished
snapshot and keeps its last complete tree; reopen the pane to subscribe again once the
debugger is enabled.

The runtime announces `godot.ready` once its debugger endpoint is registered. The editor then
reads the debugger config and subscribes while the Entities pane is visible. If the readiness
message is lost, it retries the config request every 500 ms for up to 20 seconds and reports
an unanswered endpoint in the pane. Entity summaries remain subscription-only.

### Limitations

Arbitrary not-yet-applied Bevy despawns cannot be observed through the public command queue,
which holds private erased commands without target metadata
([Bevy command queue source](https://docs.rs/bevy_ecs/0.19.0/src/bevy_ecs/world/command_queue.rs.html)).
Wide integers are edited as decimal text because a spin box value is a double and cannot
represent every integer exactly.
