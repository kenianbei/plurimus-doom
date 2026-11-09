//! Shared Doom plumbing: input forwarding and the HUD line docked on
//! top, composed with the CPU blit or GPU frame path per build.

use std::sync::atomic::Ordering;

use bevy_app::{App, AppExit, Plugin, Startup, Update};
use bevy_ecs::prelude::{
    Commands, Component, IntoScheduleConfigs, Local, MessageReader, MessageWriter, Query, Res,
    With, resource_changed,
};
use bevy_window::CursorMoved;
use plurimus::core::bevy_camera::{Camera, ClearColorConfig};
use plurimus::core::{CellSize, Edge, TerminalCamera, UiCamera, UiWidget};
use plurimus::term::bevy_input::keyboard::{KeyCode, KeyboardInput};
use plurimus::term::{ButtonInput, HeldModifiers, InputCapabilities, MouseButton, TermPlugin};
use ratatui_widgets::paragraph::Paragraph;

use crate::doom_thread::DoomHandle;
use crate::mouse_aim::AimTracker;
use crate::{doom_thread, keymap};

const HUD_ORDER: isize = 1;
const HUD_HEIGHT_CELLS: u16 = 1;

pub struct DoomPlugin;

#[derive(Component)]
struct HudLine;

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
            (
                forward_keys,
                forward_buttons,
                forward_aim,
                quit_on_ctrl_c,
                refresh_hud.run_if(resource_changed::<InputCapabilities>),
            ),
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
        HudLine,
        UiWidget::new(Paragraph::new(hud_text(&capabilities))),
        UiCamera(hud_camera),
    ));
}

// The backend learns key releases from the first one it sees, after
// startup, so the key-tier note must follow the resource.
fn refresh_hud(capabilities: Res<InputCapabilities>, mut hud: Query<&mut UiWidget, With<HudLine>>) {
    for mut line in &mut hud {
        *line = UiWidget::new(Paragraph::new(hud_text(&capabilities)));
    }
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

// Polled rather than read as messages: a focus loss releases held buttons
// into `just_released` without writing a release message.
fn forward_buttons(buttons: Res<ButtonInput<MouseButton>>, handle: Res<DoomHandle>) {
    let presses = buttons.get_just_pressed().map(|button| (*button, true));
    let releases = buttons.get_just_released().map(|button| (*button, false));
    for (button, pressed) in presses.chain(releases) {
        if let Some(key) = keymap::map_mouse(button, pressed) {
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
