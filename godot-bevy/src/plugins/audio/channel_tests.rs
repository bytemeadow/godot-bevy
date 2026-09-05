#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::audio::AudioEasing;
    use std::time::Duration;

    #[derive(Resource)]
    struct TestTrack;

    impl AudioChannelMarker for TestTrack {
        const CHANNEL_NAME: &'static str = "test";
    }

    #[test]
    fn channel_controls_queue_exact_commands() {
        let channel = AudioChannel::<TestTrack>::new(ChannelId("test"));
        let fade = AudioTween::linear(Duration::from_millis(250));

        channel.stop();
        channel.stop_with_fade(fade.clone());
        channel.pause();
        channel.resume();
        channel.set_volume(1.5);
        channel.set_volume_with_fade(-0.5, fade.clone());
        channel.set_pitch(8.0);
        channel.set_panning(-2.0);

        let commands = channel.commands.read();
        assert_eq!(commands.len(), 8);
        assert!(matches!(
            &commands[0],
            AudioCommand::Stop(ChannelId("test"), None)
        ));
        match &commands[1] {
            AudioCommand::Stop(ChannelId("test"), Some(tween)) => {
                assert_eq!(tween.duration, Duration::from_millis(250));
                assert!(matches!(tween.easing, AudioEasing::Linear));
            }
            command => panic!("unexpected command: {command:?}"),
        }
        assert!(matches!(
            &commands[2],
            AudioCommand::Pause(ChannelId("test"), None)
        ));
        assert!(matches!(
            &commands[3],
            AudioCommand::Resume(ChannelId("test"), None)
        ));
        assert!(matches!(
            &commands[4],
            AudioCommand::SetVolume(ChannelId("test"), 1.0, None)
        ));
        match &commands[5] {
            AudioCommand::SetVolume(ChannelId("test"), volume, Some(tween)) => {
                assert_eq!(*volume, 0.0);
                assert_eq!(tween.duration, Duration::from_millis(250));
            }
            command => panic!("unexpected command: {command:?}"),
        }
        assert!(matches!(
            &commands[6],
            AudioCommand::SetPitch(ChannelId("test"), 4.0, None)
        ));
        assert!(matches!(
            &commands[7],
            AudioCommand::SetPanning(ChannelId("test"), -1.0, None)
        ));
    }

    #[test]
    fn dropped_play_builder_queues_exact_settings() {
        let channel = AudioChannel::<TestTrack>::new(ChannelId("test"));
        let handle = Handle::<GodotResource>::default();

        channel
            .play(handle)
            .volume(0.25)
            .pitch(1.5)
            .looped()
            .start_from(2.5)
            .panning(-2.0);

        let mut commands = channel.commands.write();
        assert_eq!(commands.len(), 1);
        let command = commands.pop_front().unwrap();
        match command {
            AudioCommand::Play(play) => {
                assert_eq!(play.channel_id, ChannelId("test"));
                assert!(matches!(play.player_type, AudioPlayerType::NonPositional));
                assert_eq!(play.settings.volume, 0.25);
                assert_eq!(play.settings.pitch, 1.5);
                assert!(play.settings.looping);
                assert_eq!(play.settings.start_position, 2.5);
                assert_eq!(play.settings.panning, Some(-1.0));
            }
            command => panic!("unexpected command: {command:?}"),
        }
    }
}
