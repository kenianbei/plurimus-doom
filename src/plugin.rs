//! Shared Doom plumbing: input forwarding and the HUD line docked on
//! top, composed with the CPU blit or GPU frame path per build.

use std::sync::atomic::Ordering;

use bevy_app::{App, AppExit, Plugin, Startup, Update};
use bevy_ecs::prelude::{Commands, Local, MessageReader, MessageWriter, Res};
use bevy_window::CursorMoved;
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

fn forward_buttons(mut buttons: MessageReader<MouseButtonInput>, handle: Res<DoomHandle>) {
    for key in buttons.read().filter_map(keymap::map_mouse) {
        let _ = handle.keys.send(key);
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
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicI32;
    use std::sync::mpsc::{self, Receiver};

    use super::*;
    use crate::doom_thread::{FrameHandle, SharedFrame};
    use crate::keymap::test_input::{button, character, press};
    use bevy_app::App;
    use bevy_ecs::entity::Entity;
    use bevy_math::Vec2;
    use doomgeneric::input::KeyData;
    use plurimus::core::{CorePlugin, FrameBuffer, TerminalRenderApp, TerminalSize};
    use plurimus::term::MouseButton;
    use plurimus::term::bevy_input::ButtonState;
    use plurimus::term::bevy_input::keyboard::Key;

    const RED: u32 = 0xff0000;

    struct DoomTestApp {
        app: App,
        keys: Receiver<KeyData>,
        turn: Arc<AtomicI32>,
    }

    fn doom_app(frame: SharedFrame) -> DoomTestApp {
        let (keys, key_receiver) = mpsc::channel();
        let turn = Arc::new(AtomicI32::new(0));
        let mut app = App::new();
        app.add_plugins((CorePlugin, DoomPlugin));
        #[cfg(not(feature = "gpu"))]
        app.add_plugins(crate::cpu_blit::CpuBlitPlugin);
        app.insert_resource(TerminalSize::new(10, 4));
        app.insert_resource(DoomHandle {
            frame: FrameHandle::new(frame),
            keys,
            turn: Arc::clone(&turn),
        });
        // One update for Startup to spawn cameras, one for their extraction.
        app.update();
        app.update();
        DoomTestApp {
            app,
            keys: key_receiver,
            turn,
        }
    }

    fn cursor_at(x: f32) -> CursorMoved {
        CursorMoved {
            window: Entity::PLACEHOLDER,
            position: Vec2::new(x, 0.5),
            delta: None,
        }
    }

    fn red_frame() -> SharedFrame {
        SharedFrame {
            pixels: vec![RED; 4],
            width: 2,
            height: 2,
            version: 1,
        }
    }

    #[cfg(not(feature = "gpu"))]
    #[test]
    fn frame_pixels_reach_the_composited_frame() {
        use plurimus::core::ratatui_core::style::Color;

        let DoomTestApp { app, .. } = doom_app(red_frame());

        let frame = app
            .sub_app(TerminalRenderApp)
            .world()
            .resource::<FrameBuffer>();
        let content = frame.0.cell((4, 2)).unwrap();
        assert_eq!(content.symbol(), "▀");
        assert_eq!(content.fg, Color::Rgb(255, 0, 0));
        let bar = frame.0.cell((0, 2)).unwrap();
        assert_eq!(bar.bg, Color::Reset);
    }

    #[test]
    fn hud_line_overlays_the_top_row() {
        let DoomTestApp { app, .. } = doom_app(red_frame());

        let frame = app
            .sub_app(TerminalRenderApp)
            .world()
            .resource::<FrameBuffer>();
        assert_eq!(frame.0.cell((0, 0)).unwrap().symbol(), "p");
    }

    #[test]
    fn mouse_clicks_forward_to_the_doom_thread() {
        let DoomTestApp {
            mut app,
            keys,
            turn,
            ..
        } = doom_app(red_frame());

        app.world_mut()
            .write_message(button(MouseButton::Left, ButtonState::Pressed));
        app.update();

        let key = keys.try_recv().unwrap();
        assert!(key.pressed);
        assert_eq!(key.key, *doomgeneric::input::keys::KEY_FIRE);
        assert_eq!(turn.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn mouse_motion_forwards_aim_deltas() {
        let DoomTestApp { mut app, turn, .. } = doom_app(red_frame());

        for x in [10.5, 14.5] {
            app.world_mut().write_message(cursor_at(x));
        }
        app.update();

        assert!(turn.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn the_quit_chord_holds_under_shift() {
        let DoomTestApp { mut app, .. } = doom_app(red_frame());

        for key in [Key::Control, Key::Shift, character("C")] {
            app.world_mut().write_message(press(key));
        }
        app.update();

        assert!(app.should_exit().is_some());
    }

    #[test]
    fn key_messages_forward_to_the_doom_thread() {
        let DoomTestApp { mut app, keys, .. } = doom_app(red_frame());

        app.world_mut().write_message(press(character(" ")));
        app.update();

        let key = keys.try_recv().unwrap();
        assert!(key.pressed);
        assert_eq!(key.key, *doomgeneric::input::keys::KEY_FIRE);
    }

    #[test]
    fn held_shift_turns_arrows_into_strafe() {
        let DoomTestApp { mut app, keys, .. } = doom_app(red_frame());

        for key in [Key::Shift, Key::ArrowRight] {
            app.world_mut().write_message(press(key));
        }
        app.update();

        let key = keys.try_recv().unwrap();
        assert_eq!(key.key, *doomgeneric::input::keys::KEY_STRAFERIGHT);
    }
}
