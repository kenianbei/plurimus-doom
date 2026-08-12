//! Maps plurimus key and mouse messages to Doom key codes.

use doomgeneric::input::KeyData;
use doomgeneric::input::keys;
use plurimus::term::{KeyCode, KeyKind, KeyMessage, MouseButton, MouseKind, MouseMessage};

pub fn map_mouse(message: &MouseMessage) -> Option<KeyData> {
    let (pressed, button) = match message.kind {
        MouseKind::Down(button) => (true, button),
        MouseKind::Up(button) => (false, button),
        _ => return None,
    };
    let key = match button {
        MouseButton::Left => *keys::KEY_FIRE,
        MouseButton::Right => *keys::KEY_USE,
        _ => return None,
    };
    Some(KeyData { pressed, key })
}

pub fn map_key(message: &KeyMessage) -> Vec<KeyData> {
    let pressed = match message.kind {
        KeyKind::Press => true,
        KeyKind::Release => false,
        KeyKind::Repeat => return Vec::new(),
    };
    let codes = match strafe_codes(message.code, message.modifiers.shift, pressed) {
        Some(codes) => codes,
        None => doom_key(message.code).into_iter().collect(),
    };
    codes
        .into_iter()
        .map(|key| KeyData { pressed, key })
        .collect()
}

/// Left/right turn, or strafe while shift is held. A release covers
/// both variants, so dropping shift mid-hold cannot leave either key
/// stuck down.
fn strafe_codes(code: KeyCode, shifted: bool, pressed: bool) -> Option<Vec<u8>> {
    let (turn, strafe) = match code {
        KeyCode::Left => (*keys::KEY_LEFT, *keys::KEY_STRAFELEFT),
        KeyCode::Right => (*keys::KEY_RIGHT, *keys::KEY_STRAFERIGHT),
        _ => return None,
    };
    Some(match (pressed, shifted) {
        (false, _) => vec![turn, strafe],
        (true, true) => vec![strafe],
        (true, false) => vec![turn],
    })
}

fn doom_key(code: KeyCode) -> Option<u8> {
    match code {
        KeyCode::Up => Some(*keys::KEY_UP),
        KeyCode::Down => Some(*keys::KEY_DOWN),
        KeyCode::Esc => Some(keys::KEY_ESCAPE),
        KeyCode::Enter => Some(keys::KEY_ENTER),
        KeyCode::Char(character) => char_key(character.to_ascii_lowercase()),
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
mod tests {
    use plurimus::core::ratatui_core::layout::Position;
    use plurimus::term::KeyModifiers;

    use super::*;

    fn message(code: KeyCode, kind: KeyKind) -> KeyMessage {
        KeyMessage::new(code, KeyModifiers::default(), kind)
    }

    fn shifted(code: KeyCode, kind: KeyKind) -> KeyMessage {
        KeyMessage::new(code, KeyModifiers::default().with_shift(true), kind)
    }

    fn mouse(kind: MouseKind) -> MouseMessage {
        MouseMessage::new(kind, Position::new(0, 0), KeyModifiers::default())
    }

    #[test]
    fn left_click_maps_to_fire() {
        let key = map_mouse(&mouse(MouseKind::Down(MouseButton::Left))).unwrap();
        assert!(key.pressed);
        assert_eq!(key.key, *keys::KEY_FIRE);
    }

    #[test]
    fn right_release_maps_to_unpressed_use() {
        let key = map_mouse(&mouse(MouseKind::Up(MouseButton::Right))).unwrap();
        assert!(!key.pressed);
        assert_eq!(key.key, *keys::KEY_USE);
    }

    #[test]
    fn motion_and_other_buttons_are_dropped() {
        assert!(map_mouse(&mouse(MouseKind::Moved)).is_none());
        assert!(map_mouse(&mouse(MouseKind::Down(MouseButton::Middle))).is_none());
    }

    #[test]
    fn arrows_move_and_turn() {
        let keys_down = map_key(&message(KeyCode::Up, KeyKind::Press));
        assert_eq!(keys_down.len(), 1);
        assert!(keys_down[0].pressed);
        assert_eq!(keys_down[0].key, *keys::KEY_UP);

        let turn = map_key(&message(KeyCode::Left, KeyKind::Press));
        assert_eq!(turn.len(), 1);
        assert_eq!(turn[0].key, *keys::KEY_LEFT);
    }

    #[test]
    fn shifted_arrows_strafe() {
        let strafe = map_key(&shifted(KeyCode::Right, KeyKind::Press));
        assert_eq!(strafe.len(), 1);
        assert_eq!(strafe[0].key, *keys::KEY_STRAFERIGHT);
    }

    #[test]
    fn arrow_releases_cover_turn_and_strafe() {
        let released = map_key(&message(KeyCode::Right, KeyKind::Release));
        let codes: Vec<u8> = released.iter().map(|key| key.key).collect();
        assert!(released.iter().all(|key| !key.pressed));
        assert_eq!(codes, [*keys::KEY_RIGHT, *keys::KEY_STRAFERIGHT]);
    }

    #[test]
    fn wasd_moves_and_strafes() {
        let pairs = [
            ('w', *keys::KEY_UP),
            ('s', *keys::KEY_DOWN),
            ('a', *keys::KEY_STRAFELEFT),
            ('d', *keys::KEY_STRAFERIGHT),
        ];
        for (character, expected) in pairs {
            let pressed = map_key(&message(KeyCode::Char(character), KeyKind::Press));
            assert_eq!(pressed.len(), 1);
            assert!(pressed[0].pressed);
            assert_eq!(pressed[0].key, expected);
        }
    }

    #[test]
    fn wasd_release_unpresses() {
        let released = map_key(&message(KeyCode::Char('a'), KeyKind::Release));
        assert_eq!(released.len(), 1);
        assert!(!released[0].pressed);
        assert_eq!(released[0].key, *keys::KEY_STRAFELEFT);
    }

    #[test]
    fn shifted_wasd_still_strafes() {
        let strafe = map_key(&shifted(KeyCode::Char('D'), KeyKind::Press));
        assert_eq!(strafe.len(), 1);
        assert_eq!(strafe[0].key, *keys::KEY_STRAFERIGHT);
    }

    #[test]
    fn space_fires_and_e_uses() {
        let fire = map_key(&message(KeyCode::Char(' '), KeyKind::Press));
        assert_eq!(fire[0].key, *keys::KEY_FIRE);

        let along = map_key(&message(KeyCode::Char('E'), KeyKind::Press));
        assert_eq!(along[0].key, *keys::KEY_USE);
    }

    #[test]
    fn release_maps_to_unpressed() {
        let released = map_key(&message(KeyCode::Esc, KeyKind::Release));
        assert_eq!(released.len(), 1);
        assert!(!released[0].pressed);
        assert_eq!(released[0].key, keys::KEY_ESCAPE);
    }

    #[test]
    fn repeat_is_dropped() {
        assert!(map_key(&message(KeyCode::Char('e'), KeyKind::Repeat)).is_empty());
    }

    #[test]
    fn unmapped_ascii_passes_through() {
        let key = map_key(&message(KeyCode::Char('y'), KeyKind::Press));
        assert_eq!(key[0].key, b'y');
    }

    #[test]
    fn non_ascii_keys_are_dropped() {
        assert!(map_key(&message(KeyCode::Home, KeyKind::Press)).is_empty());
    }
}
