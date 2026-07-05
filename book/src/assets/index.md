# Asset Loading

godot-bevy loads assets through Bevy's `AssetServer`, but Godot owns the filesystem. There are two lanes, and picking the wrong one is the mistake that works in the editor and 404s in an export.

## Two lanes

**Godot-imported resources** — anything Godot imports and manages: `.png`, `.ogg`, `.glb`, `.tscn`, `.tres`, `.res`, `.wav`. Load these as `GodotResource` and cast:

```rust,ignore
let scene: Handle<GodotResource> = asset_server.load("res://player.tscn");

// later, once loaded
if let Some(res) = assets.get_mut(&scene) {
    if let Some(packed) = res.try_cast::<PackedScene>() {
        // use the PackedScene
    }
}
```

This lane goes through Godot's `ResourceLoader`, which resolves imports, `.remap`s, and `uid://` references — the things a raw file read can't.

**Raw data files** — your own formats: `.ron`, `.toml`, `.json`, a custom binary. Write a normal Bevy `AssetLoader` for the type and load it directly. The reader hands your loader the real file bytes:

```rust,ignore
#[derive(Asset, TypePath)]
struct Level { /* ... */ }

struct LevelLoader;
impl AssetLoader for LevelLoader {
    type Asset = Level;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(&self, reader: &mut dyn Reader, _: &(), _: &mut LoadContext<'_>)
        -> Result<Level, std::io::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        // parse bytes into a Level
        # unimplemented!()
    }
    fn extensions(&self) -> &[&str] { &["level"] }
}
```

```rust,ignore
app.init_asset::<Level>().register_asset_loader(LevelLoader);
let level: Handle<Level> = asset_server.load("res://levels/one.level");
```

## The imported-vs-data rule

The lanes are not interchangeable, and the reason only shows up in an **exported** build.

When Godot imports `player.png`, the export `.pck` contains the *imported* form (a `.ctex`) plus a remap entry — **not** the original `player.png`. `ResourceLoader` follows the remap; a raw file read cannot. So a byte loader pointed at `res://player.png` works in the editor (the original file is right there on disk) and fails in an export (the original isn't in the `.pck`).

Rule of thumb: **load imported types as `GodotResource`; use byte loaders only for genuine data files** that Godot doesn't import.

## Export include filter

Godot only packs *resources* into an export by default. Your `.ron`/`.toml`/`.json`/custom-extension files are not resources, so they're excluded — and dev works while the export 404s.

Add them under the export preset's **Resources → "Filters to export non-resource files"**, e.g.:

```
*.ron, *.level
```

Without the filter the file simply isn't in the `.pck`.

## `uid://` is resource-only

`uid://<hash>` is a resource reference resolved by Godot's `ResourceUID`/`ResourceLoader`, not a path a file read can open. Load `uid://` targets as `GodotResource`. Byte loaders over `uid://` are not supported.

## `load_folder` is not supported over `res://`/`user://`

A Godot asset directory is full of `.import`/`.uid`/`.gd` sidecars that have no Bevy loader, and an untyped folder load aborts on the first one it can't load. `load_folder` over a Godot source returns an empty result rather than trying. Load the files you need explicitly.
