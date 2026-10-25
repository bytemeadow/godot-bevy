use bevy::prelude::*;
use godot::classes::{AudioStream, AudioStreamOggVorbis, AudioStreamWav, Node};
use godot::prelude::*;
use godot_bevy::prelude::*;
use godot_bevy_test::prelude::*;

#[itest(async)]
fn test_audio_looping_preserves_shared_stream(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx, |app| {
            app.add_plugins((GodotAssetsPlugin, GodotAudioPlugin));
        })
        .await;

        let mut wav = AudioStreamWav::new_gd();
        wav.set_data(&PackedByteArray::from(vec![0u8; 441_000].as_slice()));
        wav.set_loop_end(441_000);
        let ogg = AudioStreamOggVorbis::load_from_buffer(&PackedByteArray::from(
            include_bytes!("../../../examples/dodge-the-creeps-2d/godot/audio/plop.ogg").as_slice(),
        ))
        .unwrap();

        for mut stream in [wav.upcast::<AudioStream>(), ogg.upcast::<AudioStream>()] {
            let path = format!("res://itest_audio_{}.tres", stream.get_class());
            stream.take_over_path(&path);
            let handle: Handle<GodotResource> =
                app.with_world(|world| world.resource::<AssetServer>().load(path));
            for _ in 0..120 {
                app.update().await;
                if app
                    .with_world(|world| world.resource::<Assets<GodotResource>>().contains(&handle))
                {
                    break;
                }
            }
            assert!(
                app.with_world(|world| world.resource::<Assets<GodotResource>>().contains(&handle))
            );

            for spatial in 0..3 {
                for looping in [false, true, false] {
                    app.with_world(|world| {
                        let channel = world.resource::<AudioChannel<MainAudioTrack>>();
                        let command = match spatial {
                            0 => channel.play(handle.clone()),
                            1 => channel.play_2d(handle.clone(), Vec2::ZERO),
                            _ => channel.play_3d(handle.clone(), Vec3::ZERO),
                        }
                        .pitch(0.1);
                        if looping {
                            command.looped();
                        }
                    });
                }
                app.update().await;

                let root = ctx.scene_tree.get_tree().get_root().unwrap();
                let players: Vec<Gd<Node>> = root
                    .get_children()
                    .iter_shared()
                    .filter(|node| {
                        node.get_class()
                            .to_string()
                            .starts_with("AudioStreamPlayer")
                    })
                    .collect();
                let loops: Vec<bool> = players
                    .iter()
                    .map(|node| {
                        let stream = node.get("stream").to::<Gd<AudioStream>>();
                        stream_loops(&stream)
                    })
                    .collect();
                let original_loops = stream_loops(&stream);
                app.with_world(|world| world.resource::<AudioChannel<MainAudioTrack>>().stop());
                app.updates(2).await;

                assert_eq!(loops, [false, true, false]);
                assert!(
                    !original_loops,
                    "looping playback must not modify the asset"
                );
            }
            drop(handle);
            app.updates(3).await;
        }
        app.cleanup().await;
    })
}

fn stream_loops(stream: &Gd<AudioStream>) -> bool {
    if let Ok(wav) = stream.clone().try_cast::<AudioStreamWav>() {
        wav.get_loop_mode() != godot::classes::audio_stream_wav::LoopMode::DISABLED
    } else {
        stream.clone().cast::<AudioStreamOggVorbis>().has_loop()
    }
}
