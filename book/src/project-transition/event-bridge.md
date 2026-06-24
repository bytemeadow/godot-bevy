# The Event Bridge

Bevy systems react to Godot through observers. The event bridge is how something *outside* a Bevy system — a Godot node's Rust method, or a plain GDScript script — gets a Bevy [`Event`](https://docs.rs/bevy/latest/bevy/ecs/event/trait.Event.html) into that observer loop. The event lands as an `On<T>` trigger, the same place a Godot signal lands.

There are two ways in, depending on who's sending.

## From Rust

If you have a `Gd<BevyApp>`, you send a typed event straight in -- no `Variant`, no string name, no registration:

```rust,ignore
#[derive(Event, Clone)]
struct Damage { amount: i32 }

#[godot_api]
impl Enemy {
    #[func]
    fn take_hit(&mut self, amount: i32) {
        if let Some(app) = BevyApp::try_singleton() {
            app.bind().send_event(Damage { amount });
        }
    }
}
```

`try_singleton()` resolves the `/root/BevyAppSingleton` autoload. If you run more than one `BevyApp`, skip it and send to the one you mean: `godot_bevy::send_event(&that_app, Damage { amount })` -- the same call, spelled as a free function.

Handle it like any observer:

```rust,ignore
app.add_plugins(GodotEventBridgePlugin)
   .add_observer(|trigger: On<Damage>, mut hp: ResMut<Health>| {
       hp.0 -= trigger.event().amount;
   });
```

`send_event` *enqueues* -- the event is delivered on the next channel drain, not the instant you call it. From inside a Bevy system you'd use `Commands::trigger` instead; the bridge is for callers who aren't in the ECS yet.

## From GDScript

GDScript can't name a Rust type, so you register the name once on the Rust side with a mapper from the payload `Variant`:

```rust,ignore
#[derive(Event, Clone)]
struct Damage { amount: i64 }   // GDScript ints arrive as i64

app.add_plugins(GodotEventBridgePlugin)
   .add_godot_event::<Damage>("damage", |payload| {
       let dict = payload.try_to::<VarDictionary>().ok()?;
       Some(Damage { amount: dict.get("amount")?.try_to::<i64>().ok()? })
   });
```

Then any script fires it through the autoload:

```gdscript
get_node("/root/BevyAppSingleton").send_event("damage", { "amount": 10 })
```

For a transparent newtype you can drop the mapper -- `GodotConvert` gives you the decode for free (gdext derives `FromGodot` from it):

```rust,ignore
#[derive(Event, Clone, GodotConvert)]
#[godot(transparent)]
struct Volume(f64);

app.add_godot_event_from::<Volume>("volume");
```

A unit event takes a `null` payload: `send_event("game_over", null)`.

The decode is strict on purpose: a float where you asked for an `i64`, or anything out of range, is dropped with a warning rather than quietly coerced. An unknown name is dropped with a warning that lists the names that *are* registered, so a typo is quick to spot. Your mapper returns `None` to reject a payload -- it must never panic.

## Timing

The channel drains twice a frame -- in `First` (process, display rate) and in `PrePhysicsUpdate` (before each physics tick) -- and each event is consumed once, by whichever drain runs first. So:

- Enqueue *before* this frame's `physics_process` (from `_input`, for instance) and physics sees it the same frame.
- Enqueue *after* physics has run (from a `_process` handler) and physics sees it next frame; process-time systems still see it this frame.

Godot signals ride the same channel, so they reach physics the same way. If you need a system ordered around delivery, the drains run in the public `SignalDrainSet`, so `.after(SignalDrainSet::Drain)` does what you'd expect.

## Signals or send_event?

Both arrive at the same `On<T>` observers through the same channel, so pick by where the event comes from:

- A built-in node signal -- a `Button`'s `pressed`, an `Area2D`'s `body_entered` -> `GodotSignalsPlugin` + `GodotSignals::connect`.
- A GDScript script raising its own event -> `add_godot_event` + `send_event`.
- A Rust node method raising its own event -> `send_event(&app, event)`.
