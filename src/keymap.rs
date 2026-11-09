//! Maps bevy key and mouse-button input to Doom key codes.

use doomgeneric::input::KeyData;
use doomgeneric::input::keys;
use plurimus::term::MouseButton;
use plurimus::term::bevy_input::keyboard::{Key, KeyboardInput};
use plurimus::term::bevy_input::mouse::MouseButtonInput;

pub fn map_mouse(input: &MouseButtonInput) -> Option<KeyData> {
    let key = match input.button {
        MouseButton::Left => *keys::KEY_FIRE,
        MouseButton::Right => *keys::KEY_USE,
        _ => return None,
    };
    Some(KeyData {
        pressed: input.state.is_pressed(),
        key,
    })
}

/// Releases of every key a mouse button maps to.
pub fn button_releases() -> [KeyData; 2] {
    [*keys::KEY_FIRE, *keys::KEY_USE].map(|key| KeyData {
        pressed: false,
        key,
    })
}

pub fn map_key(input: &KeyboardInput, shifted: bool) -> Vec<KeyData> {
    if input.repeat {
        return Vec::new();
    }
    let pressed = input.state.is_pressed();
    strafe_codes(&input.logical_key, shifted, pressed)
        .unwrap_or_else(|| doom_key(&input.logical_key).into_iter().collect())
        .into_iter()
        .map(|key| KeyData { pressed, key })
        .collect()
}

/// Left/right turn, or strafe while shift is held. A release covers
/// both variants, so dropping shift mid-hold cannot leave either key
/// stuck down.
fn strafe_codes(key: &Key, shifted: bool, pressed: bool) -> Option<Vec<u8>> {
    let (turn, strafe) = match key {
        Key::ArrowLeft => (*keys::KEY_LEFT, *keys::KEY_STRAFELEFT),
        Key::ArrowRight => (*keys::KEY_RIGHT, *keys::KEY_STRAFERIGHT),
        _ => return None,
    };
    Some(match (pressed, shifted) {
        (false, _) => vec![turn, strafe],
        (true, true) => vec![strafe],
        (true, false) => vec![turn],
    })
}

fn doom_key(key: &Key) -> Option<u8> {
    match key {
        Key::ArrowUp => Some(*keys::KEY_UP),
        Key::ArrowDown => Some(*keys::KEY_DOWN),
        Key::Escape => Some(keys::KEY_ESCAPE),
        Key::Enter => Some(keys::KEY_ENTER),
        Key::Character(text) => single_char(text).and_then(char_key),
        _ => None,
    }
}

fn single_char(text: &str) -> Option<char> {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(character), None) => Some(character.to_ascii_lowercase()),
        _ => None,
    }
}

/// `a`/`d` strafe instead of turning, since turning is the mouse's job.
fn char_key(character: char) -> Option<u8> {
    match character {
        ' ' => Some(*keys::KEY_FIRE),
        'w' => Some(*keys::KEY_UP),
        's' => Some(*keys::KEY_DOWN),
        'a' => Some(*keys::KEY_STRAFELEFT),
        'd' => Some(*keys::KEY_STRAFERIGHT),
        'e' => Some(*keys::KEY_USE),
        'r' => Some(*keys::KEY_SPEED),
        _ => keys::from_char(character),
    }
}

#[cfg(test)]
pub mod test_input {
    use bevy_ecs::entity::Entity;
    use plurimus::term::MouseButton;
    use plurimus::term::bevy_input::ButtonState;
    use plurimus::term::bevy_input::keyboard::{Key, KeyboardInput};
    use plurimus::term::bevy_input::mouse::MouseButtonInput;
    use plurimus::term::physical_code;

    pub fn key(logical_key: Key, state: ButtonState) -> KeyboardInput {
        KeyboardInput {
            key_code: physical_code(&logical_key),
            logical_key,
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        }
    }

    pub fn press(logical_key: Key) -> KeyboardInput {
        key(logical_key, ButtonState::Pressed)
    }

    pub fn character(text: &str) -> Key {
        Key::Character(text.into())
    }

    pub fn button(button: MouseButton, state: ButtonState) -> MouseButtonInput {
        MouseButtonInput {
            button,
            state,
            window: Entity::PLACEHOLDER,
        }
    }
}

#[cfg(test)]
mod tests {
    use plurimus::term::bevy_input::ButtonState;

    use super::test_input::{button, character, key, press};
    use super::*;

    fn release(logical_key: Key) -> KeyboardInput {
        key(logical_key, ButtonState::Released)
    }

    #[test]
    fn left_click_maps_to_fire() {
        let key = map_mouse(&button(MouseButton::Left, ButtonState::Pressed)).unwrap();
        assert!(key.pressed);
        assert_eq!(key.key, *keys::KEY_FIRE);
    }

    #[test]
    fn right_release_maps_to_unpressed_use() {
        let key = map_mouse(&button(MouseButton::Right, ButtonState::Released)).unwrap();
        assert!(!key.pressed);
        assert_eq!(key.key, *keys::KEY_USE);
    }

    #[test]
    fn other_buttons_are_dropped() {
        assert!(map_mouse(&button(MouseButton::Middle, ButtonState::Pressed)).is_none());
    }

    #[test]
    fn arrows_move_and_turn() {
        let keys_down = map_key(&press(Key::ArrowUp), false);
        assert_eq!(keys_down.len(), 1);
        assert!(keys_down[0].pressed);
        assert_eq!(keys_down[0].key, *keys::KEY_UP);

        let turn = map_key(&press(Key::ArrowLeft), false);
        assert_eq!(turn.len(), 1);
        assert_eq!(turn[0].key, *keys::KEY_LEFT);
    }

    #[test]
    fn shifted_arrows_strafe() {
        let strafe = map_key(&press(Key::ArrowRight), true);
        assert_eq!(strafe.len(), 1);
        assert_eq!(strafe[0].key, *keys::KEY_STRAFERIGHT);
    }

    #[test]
    fn arrow_releases_cover_turn_and_strafe() {
        let released = map_key(&release(Key::ArrowRight), false);
        let codes: Vec<u8> = released.iter().map(|key| key.key).collect();
        assert!(released.iter().all(|key| !key.pressed));
        assert_eq!(codes, [*keys::KEY_RIGHT, *keys::KEY_STRAFERIGHT]);
    }

    #[test]
    fn wasd_moves_and_strafes() {
        let pairs = [
            ("w", *keys::KEY_UP),
            ("s", *keys::KEY_DOWN),
            ("a", *keys::KEY_STRAFELEFT),
            ("d", *keys::KEY_STRAFERIGHT),
        ];
        for (text, expected) in pairs {
            let pressed = map_key(&press(character(text)), false);
            assert_eq!(pressed.len(), 1);
            assert!(pressed[0].pressed);
            assert_eq!(pressed[0].key, expected);
        }
    }

    #[test]
    fn wasd_release_unpresses() {
        let released = map_key(&release(character("a")), false);
        assert_eq!(released.len(), 1);
        assert!(!released[0].pressed);
        assert_eq!(released[0].key, *keys::KEY_STRAFELEFT);
    }

    #[test]
    fn shifted_wasd_still_strafes() {
        let strafe = map_key(&press(character("D")), true);
        assert_eq!(strafe.len(), 1);
        assert_eq!(strafe[0].key, *keys::KEY_STRAFERIGHT);
    }

    #[test]
    fn space_fires_and_e_uses() {
        let fire = map_key(&press(character(" ")), false);
        assert_eq!(fire[0].key, *keys::KEY_FIRE);

        let along = map_key(&press(character("E")), true);
        assert_eq!(along[0].key, *keys::KEY_USE);
    }

    #[test]
    fn release_maps_to_unpressed() {
        let released = map_key(&release(Key::Escape), false);
        assert_eq!(released.len(), 1);
        assert!(!released[0].pressed);
        assert_eq!(released[0].key, keys::KEY_ESCAPE);
    }

    #[test]
    fn repeat_is_dropped() {
        let repeat = KeyboardInput {
            repeat: true,
            ..press(character("e"))
        };
        assert!(map_key(&repeat, false).is_empty());
    }

    #[test]
    fn unmapped_ascii_passes_through() {
        let key = map_key(&press(character("y")), false);
        assert_eq!(key[0].key, b'y');

        let upper = map_key(&press(character("Y")), true);
        assert_eq!(upper[0].key, b'y');

        let punctuation = map_key(&press(character("-")), false);
        assert_eq!(punctuation[0].key, b'-');
    }

    #[test]
    fn non_ascii_keys_are_dropped() {
        assert!(map_key(&press(Key::Home), false).is_empty());
    }
}
