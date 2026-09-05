#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::input::events::GodotKeyboardInput;
    use bevy_app::{App, First, Update};
    use bevy_ecs::{
        message::{MessageReader, Messages},
        resource::Resource,
        system::ResMut,
    };
    use bevy_input::ButtonInput;

    #[derive(Resource, Default)]
    struct Collected(Vec<BevyKeyboardInput>);

    fn collect(mut reader: MessageReader<BevyKeyboardInput>, mut out: ResMut<Collected>) {
        for ev in reader.read() {
            out.0.push(ev.clone());
        }
    }

    fn make_app() -> App {
        let mut app = App::new();
        app.add_message::<GodotKeyboardInput>()
            .add_message::<BevyKeyboardInput>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Collected>()
            .add_systems(First, bridge_keyboard_input)
            .add_systems(Update, collect);
        app
    }

    fn send(app: &mut App, msg: GodotKeyboardInput) {
        app.world_mut()
            .resource_mut::<Messages<GodotKeyboardInput>>()
            .write(msg);
    }

    fn drain(app: &mut App) -> Vec<BevyKeyboardInput> {
        core::mem::take(&mut app.world_mut().resource_mut::<Collected>().0)
    }

    fn godot_key_msg(
        keycode: godot::global::Key,
        unicode: u32,
        pressed: bool,
        echo: bool,
    ) -> GodotKeyboardInput {
        GodotKeyboardInput {
            keycode,
            physical_keycode: None,
            pressed,
            echo,
            unicode,
        }
    }

    // (a) printable: Key::A + unicode 'a' -> KeyCode::KeyA + Key::Character("a") + text Some("a") + Pressed
    #[test]
    fn printable_key_maps_to_character() {
        let mut app = make_app();
        send(
            &mut app,
            godot_key_msg(godot::global::Key::A, 'a' as u32, true, false),
        );
        app.update();
        let events = drain(&mut app);
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.key_code, KeyCode::KeyA);
        assert_eq!(ev.logical_key, Key::Character("a".to_string().into()));
        assert_eq!(ev.text, Some("a".to_string().into()));
        assert_eq!(ev.state, ButtonState::Pressed);
        assert!(!ev.repeat);
    }

    // (b) named non-printable: Key::ESCAPE + unicode 0 -> KeyCode::Escape + Key::Escape + text None
    #[test]
    fn non_printable_key_maps_to_named_variant() {
        let mut app = make_app();
        send(
            &mut app,
            godot_key_msg(godot::global::Key::ESCAPE, 0, true, false),
        );
        app.update();
        let events = drain(&mut app);
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.key_code, KeyCode::Escape);
        assert_eq!(ev.logical_key, Key::Escape);
        assert_eq!(ev.text, None);
        assert_eq!(ev.state, ButtonState::Pressed);
    }

    // (c) unmapped key + unicode 0 -> both Unidentified, still emitted
    #[test]
    fn unmapped_key_emits_unidentified() {
        let mut app = make_app();
        // Key::NONE has no keycode mapping and no unicode
        send(
            &mut app,
            godot_key_msg(godot::global::Key::NONE, 0, true, false),
        );
        app.update();
        let events = drain(&mut app);
        assert_eq!(events.len(), 1, "unmapped key must still emit an event");
        let ev = &events[0];
        assert!(
            matches!(ev.key_code, KeyCode::Unidentified(_)),
            "key_code should be Unidentified"
        );
        assert!(
            matches!(ev.logical_key, Key::Unidentified(_)),
            "logical_key should be Unidentified"
        );
    }

    // (d) echo=true -> repeat true
    #[test]
    fn echo_maps_to_repeat() {
        let mut app = make_app();
        send(
            &mut app,
            godot_key_msg(godot::global::Key::A, 'a' as u32, true, true),
        );
        app.update();
        let events = drain(&mut app);
        assert_eq!(events.len(), 1);
        assert!(events[0].repeat);
    }

    // (e) regression guard: bridge alone must not touch ButtonInput<KeyCode>
    #[test]
    fn bridge_does_not_press_button_input() {
        let mut app = make_app();
        send(
            &mut app,
            godot_key_msg(godot::global::Key::A, 'a' as u32, true, false),
        );
        app.update();
        let input = app.world().resource::<ButtonInput<KeyCode>>();
        assert!(
            input.get_just_pressed().count() == 0 && input.get_pressed().count() == 0,
            "bridge must only write events, never press ButtonInput<KeyCode> directly"
        );
    }
}
