//! Shared Doom plumbing: input forwarding and the HUD line docked on
//! top, composed with the CPU blit or GPU frame path per build.

use std::sync::atomic::Ordering;

use bevy_app::{App, AppExit, Plugin, Startup, Update};
use bevy_ecs::prelude::{Commands, Local, MessageReader, MessageWriter, Res};
use plurimus::core::{Background, Edge, TerminalCamera, Viewport};
use plurimus::core::{UiCamera, UiWidget};
use plurimus::input::{InputCapabilities, InputPlugin, KeyCode, KeyKind, KeyMessage, MouseMessage};
use ratatui_widgets::paragraph::Paragraph;

use crate::doom_thread::DoomHandle;
use crate::mouse_aim::AimTracker;
use crate::{doom_thread, keymap};

const HUD_ORDER: isize = 1;

pub struct DoomPlugin;

impl Plugin for DoomPlugin {
    fn build(&self, app: &mut App) {
        // Doom reads terminal input directly, so it owns that dependency
        // rather than relying on whatever installed the backend.
        if !app.is_plugin_added::<InputPlugin>() {
            app.add_plugins(InputPlugin);
        }
        app.add_systems(Startup, spawn_hud_camera);
        app.add_systems(Update, (forward_keys, forward_mouse, quit_on_ctrl_c));
    }
}

fn spawn_hud_camera(mut commands: Commands, capabilities: Res<InputCapabilities>) {
    let hud_camera = commands
        .spawn(TerminalCamera {
            order: HUD_ORDER,
            background: Background::Transparent,
            viewport: Viewport::Docked {
                edge: Edge::Top,
                cells: 1,
            },
            ..TerminalCamera::default()
        })
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

fn forward_keys(mut keys: MessageReader<KeyMessage>, handle: Res<DoomHandle>) {
    for message in keys.read() {
        for key in keymap::map_key(message) {
            let _ = handle.keys.send(key);
        }
    }
}

// Doom swallows every key it maps, so the quit chord is the app's own
// policy rather than anything the terminal backend provides.
fn quit_on_ctrl_c(mut keys: MessageReader<KeyMessage>, mut exit: MessageWriter<AppExit>) {
    for message in keys.read() {
        if message.kind == KeyKind::Press
            && message.modifiers.ctrl
            && message.code == KeyCode::Char('c')
        {
            exit.write(AppExit::Success);
        }
    }
}

fn forward_mouse(
    mut mice: MessageReader<MouseMessage>,
    mut tracker: Local<AimTracker>,
    handle: Res<DoomHandle>,
) {
    for message in mice.read() {
        if let Some(key) = keymap::map_mouse(message) {
            let _ = handle.keys.send(key);
        }
        if let Some(dx) = tracker.track(message) {
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
    use bevy_app::App;
    use doomgeneric::input::KeyData;
    use plurimus::core::{CorePlugin, FrameBuffer, TerminalRenderApp, TerminalSize};
    use plurimus::input::{KeyCode, KeyKind, KeyModifiers};

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
        app.insert_resource(TerminalSize { cols: 10, rows: 4 });
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
        assert_eq!(bar.bg, Color::Black);
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
        use plurimus::core::ratatui_core::layout::Position;
        use plurimus::input::{MouseButton, MouseKind};

        let DoomTestApp {
            mut app,
            keys,
            turn,
            ..
        } = doom_app(red_frame());

        app.world_mut().write_message(MouseMessage {
            kind: MouseKind::Down(MouseButton::Left),
            position: Position::new(0, 0),
            modifiers: KeyModifiers::default(),
        });
        app.update();

        let key = keys.try_recv().unwrap();
        assert!(key.pressed);
        assert_eq!(key.key, *doomgeneric::input::keys::KEY_FIRE);
        assert_eq!(turn.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn mouse_motion_forwards_aim_deltas() {
        use plurimus::core::ratatui_core::layout::Position;
        use plurimus::input::MouseKind;

        let DoomTestApp { mut app, turn, .. } = doom_app(red_frame());

        for column in [10, 14] {
            app.world_mut().write_message(MouseMessage {
                kind: MouseKind::Moved,
                position: Position::new(column, 0),
                modifiers: KeyModifiers::default(),
            });
        }
        app.update();

        assert!(turn.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn key_messages_forward_to_the_doom_thread() {
        let DoomTestApp { mut app, keys, .. } = doom_app(red_frame());

        app.world_mut().write_message(KeyMessage {
            code: KeyCode::Char(' '),
            modifiers: KeyModifiers::default(),
            kind: KeyKind::Press,
        });
        app.update();

        let key = keys.try_recv().unwrap();
        assert!(key.pressed);
        assert_eq!(key.key, *doomgeneric::input::keys::KEY_FIRE);
    }
}
