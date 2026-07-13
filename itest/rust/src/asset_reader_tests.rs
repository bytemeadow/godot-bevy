/*
 * Asset-reader itests: the FileAccess-backed GodotAssetReader.
 *
 * These pin the three load-bearing properties: a byte loader gets real file
 * bytes through the reader, the GodotResource/ResourceLoader path still works
 * (the reader's lazy Ok never interferes), and a missing data file fails cleanly
 * without aborting the world.
 *
 * GodotCorePlugins (the itest autoload) has no asset system, so each test's setup
 * adds GodotAssetsPlugin. Loads are async with +-1-frame slop, so we poll for the
 * terminal LoadState within a bounded budget rather than asserting on a fixed tick.
 */

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext, LoadState};
use bevy::prelude::*;
use godot::classes::PackedScene;
use godot_bevy::prelude::*;
use godot_bevy_test::prelude::*;
use std::time::{Duration, Instant};

/// Wall-clock budget for an async load to reach a terminal `LoadState`. Loads run on a worker
/// thread, so the number of frames until completion is undetermined -- bound the wait by real
/// time (as production code does), not a frame count that doesn't map to wall-clock in headless.
/// Generous because it's only a hang guard; a healthy load resolves in a handful of frames.
const LOAD_TIMEOUT: Duration = Duration::from_secs(10);

/// Test-only byte loader: reads raw bytes and stores them as a string. `.greeting`
/// is not in `GodotResourceAssetLoader::extensions()`, so it routes here and proves
/// `read()` delivers real bytes (no `ron`/`toml` dependency needed).
#[derive(Asset, TypePath, Debug)]
struct Greeting(String);

#[derive(TypePath)]
struct GreetingLoader;

impl AssetLoader for GreetingLoader {
    type Asset = Greeting;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Greeting, std::io::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(Greeting(String::from_utf8_lossy(&bytes).into_owned()))
    }

    fn extensions(&self) -> &[&str] {
        &["greeting"]
    }
}

/// A byte loader reading `res://...greeting` receives the real file bytes.
/// Fails hard against the old stub (empty stream -> `Greeting("")`).
#[itest(async)]
fn test_read_delivers_real_bytes(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotAssetsPlugin);
            app.init_asset::<Greeting>();
            app.register_asset_loader(GreetingLoader);
        })
        .await;

        let handle: Handle<Greeting> = app.with_world(|w| {
            w.resource::<AssetServer>()
                .load("res://itest_assets/hello.greeting")
        });

        let deadline = Instant::now() + LOAD_TIMEOUT;
        let mut loaded = false;
        while Instant::now() < deadline {
            app.update().await;
            match app.with_world(|w| w.resource::<AssetServer>().get_load_state(&handle)) {
                Some(LoadState::Loaded) => {
                    loaded = true;
                    break;
                }
                Some(LoadState::Failed(e)) => panic!("greeting load failed: {e:?}"),
                _ => {}
            }
        }
        assert!(loaded, "greeting asset should reach Loaded within budget");

        let value = app.with_world(|w| {
            w.resource::<Assets<Greeting>>()
                .get(&handle)
                .map(|g| g.0.clone())
        });
        assert_eq!(
            value.as_deref(),
            Some("hello world"),
            "reader must deliver the real file bytes, not an empty stream"
        );

        app.cleanup().await;
    })
}

/// The `GodotResource`/`ResourceLoader` path still loads a `.tscn`. The reader's
/// lazy `Ok` keeps the loader running and its ignored reader never interferes.
#[itest(async)]
fn test_godot_resource_tscn_does_not_regress(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotAssetsPlugin);
        })
        .await;

        let handle: Handle<GodotResource> = app.with_world(|w| {
            w.resource::<AssetServer>()
                .load("res://test_spawn_scene.tscn")
        });

        let deadline = Instant::now() + LOAD_TIMEOUT;
        let mut loaded = false;
        while Instant::now() < deadline {
            app.update().await;
            match app.with_world(|w| w.resource::<AssetServer>().get_load_state(&handle)) {
                Some(LoadState::Loaded) => {
                    loaded = true;
                    break;
                }
                Some(LoadState::Failed(e)) => panic!("GodotResource .tscn load failed: {e:?}"),
                _ => {}
            }
        }
        assert!(
            loaded,
            "GodotResource .tscn should reach Loaded within budget"
        );

        let is_scene = app.with_world_mut(|w| {
            w.resource_mut::<Assets<GodotResource>>()
                .get_mut(&handle)
                .and_then(|mut res| res.try_cast::<PackedScene>())
                .is_some()
        });
        assert!(
            is_scene,
            "the loaded GodotResource must cast to PackedScene (ResourceLoader path intact)"
        );

        app.cleanup().await;
    })
}

/// A missing data file surfaces as a clean `Failed` and does not abort the world:
/// a subsequent good load still succeeds.
#[itest(async)]
fn test_missing_data_file_fails_cleanly(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotAssetsPlugin);
            app.init_asset::<Greeting>();
            app.register_asset_loader(GreetingLoader);
        })
        .await;

        let missing: Handle<Greeting> = app.with_world(|w| {
            w.resource::<AssetServer>()
                .load("res://itest_assets/nope.greeting")
        });

        let deadline = Instant::now() + LOAD_TIMEOUT;
        let mut failed = false;
        while Instant::now() < deadline {
            app.update().await;
            if let Some(LoadState::Failed(_)) =
                app.with_world(|w| w.resource::<AssetServer>().get_load_state(&missing))
            {
                failed = true;
                break;
            }
        }
        assert!(
            failed,
            "a missing data file should reach Failed within budget"
        );

        let good: Handle<Greeting> = app.with_world(|w| {
            w.resource::<AssetServer>()
                .load("res://itest_assets/hello.greeting")
        });

        let deadline = Instant::now() + LOAD_TIMEOUT;
        let mut loaded = false;
        while Instant::now() < deadline {
            app.update().await;
            match app.with_world(|w| w.resource::<AssetServer>().get_load_state(&good)) {
                Some(LoadState::Loaded) => {
                    loaded = true;
                    break;
                }
                Some(LoadState::Failed(e)) => {
                    panic!("good load failed after a missing one: {e:?}")
                }
                _ => {}
            }
        }
        assert!(
            loaded,
            "a good load must still succeed after a missing one failed cleanly"
        );

        app.cleanup().await;
    })
}
