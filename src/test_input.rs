//! Hand-built bevy input messages for tests.

use bevy_ecs::entity::Entity;
use plurimus::term::MouseButton;
use plurimus::term::bevy_input::ButtonState;
use plurimus::term::bevy_input::keyboard::{Key, KeyboardInput};
use plurimus::term::bevy_input::mouse::MouseButtonInput;
use plurimus::term::physical_code;

fn key(logical_key: Key, state: ButtonState) -> KeyboardInput {
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

pub fn release(logical_key: Key) -> KeyboardInput {
    key(logical_key, ButtonState::Released)
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
