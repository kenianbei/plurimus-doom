//! Shared Doom plumbing: input forwarding and the HUD line docked on
//! top, composed with the CPU blit or GPU frame path per build.

use std::sync::atomic::Ordering;

use bevy_app::{App, AppExit, Plugin, Startup, Update};
use bevy_ecs::prelude::{Commands, Local, MessageReader, MessageWriter, Res};
use bevy_window::{CursorMoved, WindowFocused};
use plurimus::core::bevy_camera::{Camera, ClearColorConfig};
use plurimus::core::{CellSize, Edge, TerminalCamera, UiCamera, UiWidget};
use plurimus::term::bevy_input::keyboard::{KeyCode, KeyboardInput};
use plurimus::term::bevy_input::mouse::MouseButtonInput;
use plurimus::term::{HeldModifiers, InputCapabilities, TermPlugin};
use ratatui_widgets::paragraph::Paragraph;

use crate::doom_thread::DoomHandle;
use crate::mouse_aim::AimTracker;
use crate::{doom_thread, keymap};

const HUD_ORDER: isize = 1;
const HUD_HEIGHT_CELLS: u16 = 1;

pub struct DoomPlugin;

impl Plugin for DoomPlugin {
    fn build(&self, app: &mut App) {
        // A terminal backend installs this itself; adding it here keeps
        // Doom's input path working in an app composed without one.
        if !app.is_plugin_added::<TermPlugin>() {
            app.add_plugins(TermPlugin);
        }
        app.add_systems(Startup, spawn_hud_camera);
        app.add_systems(
            Update,
            (forward_keys, forward_buttons, forward_aim, quit_on_ctrl_c),
        );
    }
}

fn spawn_hud_camera(mut commands: Commands, capabilities: Res<InputCapabilities>) {
    let hud_camera = commands
        .spawn((
            TerminalCamera::Docked {
                edge: Edge::Top,
                cells: HUD_HEIGHT_CELLS,
            },
            Camera {
                order: HUD_ORDER,
                clear_color: ClearColorConfig::None,
                ..Camera::default()
            },
        ))
        .id();
    commands.spawn((
        UiWidget::new(Paragraph::new(hud_text(&capabilities))),
        UiCamera(hud_camera),
    ));
}

#[cfg(feature = "gpu")]
const RENDER_KEYS_HELP: &str = " · t strategy · g edges";
#[cfg(not(feature = "gpu"))]
const RENDER_KEYS_HELP: &str = "";

fn hud_text(capabilities: &InputCapabilities) -> String {
    let keys_tier = if capabilities.key_release {
        ""
    } else {
        " · DEGRADED KEYS: no release events, movement will lurch (kitty-protocol terminal, no tmux)"
    };
    format!(
        "plurimus-doom · ctrl-c quits{RENDER_KEYS_HELP}{keys_tier} · engine log: {}",
        doom_thread::LOG_FILE
    )
}

fn forward_keys(
    mut keys: MessageReader<KeyboardInput>,
    modifiers: HeldModifiers,
    handle: Res<DoomHandle>,
) {
    let shifted = modifiers.get().shift;
    for input in keys.read() {
        for key in keymap::map_key(input, shifted) {
            let _ = handle.keys.send(key);
        }
    }
}

// Doom swallows every key it maps, so the quit chord is the app's own
// policy rather than anything the terminal backend provides.
fn quit_on_ctrl_c(
    mut keys: MessageReader<KeyboardInput>,
    modifiers: HeldModifiers,
    mut exit: MessageWriter<AppExit>,
) {
    let is_ctrl_held = modifiers.get().ctrl;
    for input in keys.read() {
        if is_ctrl_held && input.state.is_pressed() && input.key_code == KeyCode::KeyC {
            exit.write(AppExit::Success);
        }
    }
}

// A focus loss drops held buttons without a release message, so the
// engine would otherwise keep firing after an alt-tab.
fn forward_buttons(
    mut buttons: MessageReader<MouseButtonInput>,
    mut focus: MessageReader<WindowFocused>,
    handle: Res<DoomHandle>,
) {
    for key in buttons.read().filter_map(keymap::map_mouse) {
        let _ = handle.keys.send(key);
    }
    if focus.read().any(|change| !change.focused) {
        for key in keymap::button_releases() {
            let _ = handle.keys.send(key);
        }
    }
}

fn forward_aim(
    mut moves: MessageReader<CursorMoved>,
    mut tracker: Local<AimTracker>,
    cell: Res<CellSize>,
    handle: Res<DoomHandle>,
) {
    for moved in moves.read() {
        if let Some(dx) = tracker.track(moved.position.x, *cell) {
            handle.turn.fetch_add(dx, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests;
