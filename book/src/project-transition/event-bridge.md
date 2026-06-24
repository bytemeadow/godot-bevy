# Godot to Bevy Event Bridge (Godot -> Bevy Events)

The event bridge sends Bevy `Event`s into the ECS **from outside a Bevy system**
— from a Godot custom-node Rust method or from a plain GDScript script — without
ever handing the caller `&mut App` or `world_mut()`. Events arrive as Bevy
observer triggers (`On<T>` / `add_observer`), exactly like Godot signals.

## Two entry points

- **Rust** — `godot_bevy::send_event(&app, event)`, where `app` is a resolved
  `Gd<BevyApp>`. No `Variant`, no registration, no stringly-typed name.
- **GDScript** — `get_node("/root/BevyAppSingleton").send_event(name, payload)`,
  backed by a Rust-side `add_godot_event::<T>(name, mapper)` registration that
  decodes the payload into a typed event.

Both feed the existing per-instance signal channel and reach `On<T>` observers.

## Rust path (single-app / autoload)

```rust,ignore
use godot_bevy::prelude::*;

#[derive(Event, Clone)]
struct Damage { amount: i32 }

#[godot_api]
impl Enemy {
    #[func]
    fn take_hit(&mut self, amount: i32) {
        // Free-function form — explicit, works anywhere you have a Gd<BevyApp>.
        if let Some(app) = BevyApp::try_singleton() {
            godot_bevy::send_event(&app, Damage { amount });
        }

        // Method form — equally natural, compiles to the same channel enqueue.
        if let Some(app) = BevyApp::try_singleton() {
            app.bind().send_event(Damage { amount });
        }
    }
}

// In your Bevy build fn:
app.add_plugins(GodotEventBridgePlugin)
    .add_observer(|trigger: On<Damage>, mut hp: ResMut<Health>| {
        hp.0 -= trigger.event().amount;
    });
```

For multiple `BevyApp` instances, pass the specific instance instead of
resolving the singleton:

```rust,ignore
godot_bevy::send_event(&viewport_app, Damage { amount: 5 });
```

`Damage` does **not** need `GodotConvert` for the Rust path — the
typed event goes straight into the channel. (Values crossing from GDScript still
need a decode; see below.)

## GDScript path (registered mapper)

GDScript cannot see Rust types, so a Rust-side registration maps a string name
and a payload `Variant` to a typed event:

```rust,ignore
#[derive(Event, Clone)]
struct Damage { amount: i64 }   // i64 — GDScript ints arrive as i64

#[derive(Event, Clone)]
struct GameOver;

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotEventBridgePlugin)
        .add_godot_event::<Damage>("damage", |payload| {
            let dict = payload.try_to::<VarDictionary>().ok()?;
            Some(Damage { amount: dict.get("amount")?.try_to::<i64>().ok()? })
        })
        .add_godot_event::<GameOver>("game_over", |_payload| Some(GameOver))
        .add_observer(|t: On<Damage>, /* ... */| { /* ... */ });
}
```

```gdscript
var bridge = get_node("/root/BevyAppSingleton")
bridge.send_event("damage", { "amount": 10 })   # payload dictionary
bridge.send_event("game_over", null)            # unit event, nil payload
```

### Newtype convenience

Events that implement `FromGodot` (transparent newtypes) skip the closure:

```rust,ignore
// Volume is a #[godot(transparent)] f64 newtype
#[derive(Event, Clone, GodotConvert)]
#[godot(transparent)]
struct Volume(f64);

app.add_godot_event_from::<Volume>("volume");
```

`GodotConvert` is the only derive needed — gdext 0.5.1 derives `FromGodot`
automatically from it for transparent newtypes.

### Diagnostics

The conversion is strict: a `null` / wrong-typed / out-of-range payload (e.g. a
float where an `i64` is expected) is dropped with a `warn!`. An unknown name is
dropped with a `warn!` that lists the registered names, so typos are easy to
spot. Mappers must return `None` on failure — never panic.

## Timing guarantee

The channel is drained twice per frame: in `First` (process time, display rate)
and in `PrePhysicsUpdate` (before each physics tick). Each queued item is
consumed exactly once by whichever drain runs first.

- An event **enqueued before this frame's `physics_process`** (from Godot
  `_input`, the test thread, or a previous frame's process tail) is delivered
  **before `PhysicsUpdate` this frame** — same-frame for physics.
- An event **enqueued after `physics_process` has run this frame** (from a
  GDScript `_process` handler or a process-time `Update` system) is seen by
  physics **next frame**; the `First` drain still delivers it to process-time
  consumers this frame.
- An event enqueued **from inside an observer during a drain** is delivered on
  the **next** drain, never within the current one.

> Note: adding the `PrePhysicsUpdate` drain also affects existing Godot signals
> — they now reach physics systems same-frame too. To order systems against
> delivery, use the public `SignalDrainSet` system set (e.g.
> `.after(SignalDrainSet::Drain)`).

## Signals vs send_event — which to use

Built-in Godot node signals (`GodotSignalsPlugin::<T>` + `GodotSignals::connect`)
and bridge events (`GodotEventBridgePlugin` + `add_godot_event` / `send_event`)
both end up calling `world.trigger(T)` through the **same** channel and reach the
**same** `On<T>` observers. Pick the entry point by where the event originates:

- **Built-in Godot node signal** — a `Button`'s `pressed`, an `Area2D`'s
  `body_entered`, etc. → use `GodotSignalsPlugin` + `GodotSignals::connect`.
- **Arbitrary GDScript-initiated event** — a custom GDScript script calling
  `send_event("damage", …)` → use `add_godot_event` + the GDScript `send_event`.
- **Arbitrary Rust-initiated event from a Godot custom-node method** →
  `godot_bevy::send_event(&app, event)` (no registration, no `Variant`).

> Sending from **inside** a Bevy system or observer is not what `send_event` is
> for. There, prefer `Commands::trigger` / `world.trigger` — `send_event` enqueues
> onto the channel and the event is delivered one drain later, not immediately.

## Boilerplate accounting

The "low boilerplate" claim, counted honestly for a single event:

| Approach | Per-event cost | Type-safe across the boundary? |
|---|---|---|
| **Rust `send_event`** | `#[derive(Event, Clone)]` + `send_event(&app, Ev { .. })` — no `Variant`, no string, no registration | Yes — the typed value is already in hand |
| **GDScript `send_event`** | `#[derive(Event, Clone)]` + `add_godot_event::<Ev>("name", \|p\| { decode })` (3-6 lines/field) + GDScript `send_event("name", { .. })` | No — string name + dict keys, checked at runtime |
| **GDScript, newtype via `add_godot_event_from`** | `#[derive(Event, Clone, GodotConvert, FromGodot)]` transparent newtype + `add_godot_event_from::<Ev>("name")` — no closure | Partial (single transparent field) |

**The Rust path is a genuine boilerplate win**: no `Variant`, no glue, no
registration. The **GDScript path for multi-field events is roughly a wash** —
it moves the manual `Variant` decode into a closure but does not eliminate it,
and it reintroduces a stringly-typed name. `add_godot_event_from` removes the
closure for newtype/single-field events. We ship the GDScript half anyway
because it is the only mechanism that lets **pure GDScript scripts** — which
cannot call Rust functions — drive Bevy events at all.

Multi-field GDScript example, so the compounding cost is explicit:

```rust,ignore
#[derive(Event, Clone)]
struct Hit { amount: i64, crit: bool, source: GString }

app.add_godot_event::<Hit>("hit", |payload| {
    let d = payload.try_to::<VarDictionary>().ok()?;
    Some(Hit {
        amount: d.get("amount")?.try_to::<i64>().ok()?,
        crit:   d.get("crit")?.try_to::<bool>().ok()?,
        source: d.get("source")?.try_to::<GString>().ok()?,
    })
});
```

```gdscript
bridge.send_event("hit", { "amount": 10, "crit": true, "source": "trap" })
```
