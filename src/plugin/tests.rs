use std::sync::Arc;
use std::sync::atomic::AtomicI32;
use std::sync::mpsc::{self, Receiver};

use super::*;
use crate::doom_thread::{FrameHandle, SharedFrame};
use crate::test_input::{button, character, press};
use bevy_app::App;
use bevy_ecs::entity::Entity;
use doomgeneric::input::KeyData;
use plurimus::core::{CorePlugin, FrameBuffer, TerminalRenderApp, TerminalSize};
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
        position: [x, 0.5].into(),
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

#[test]
fn losing_focus_releases_the_held_button() {
    let DoomTestApp { mut app, keys, .. } = doom_app(red_frame());
    app.world_mut()
        .write_message(button(MouseButton::Left, ButtonState::Pressed));
    app.update();
    keys.try_iter().for_each(drop);

    app.world_mut().write_message(bevy_window::WindowFocused {
        window: Entity::PLACEHOLDER,
        focused: false,
    });
    app.update();

    let released: Vec<KeyData> = keys.try_iter().collect();
    assert_eq!(released.len(), 1);
    assert!(!released[0].pressed);
    assert_eq!(released[0].key, *doomgeneric::input::keys::KEY_FIRE);
}

fn top_row(app: &App) -> String {
    let frame = &app
        .sub_app(TerminalRenderApp)
        .world()
        .resource::<FrameBuffer>()
        .0;
    (0..frame.area.width)
        .filter_map(|x| frame.cell((x, 0)).map(|cell| cell.symbol().to_owned()))
        .collect()
}

#[test]
fn the_key_tier_note_follows_learned_capabilities() {
    const WIDE_COLUMNS: u16 = 80;
    let DoomTestApp { mut app, .. } = doom_app(red_frame());
    app.insert_resource(TerminalSize::new(WIDE_COLUMNS, 4));
    app.insert_resource(InputCapabilities::none());
    app.update();
    app.update();
    assert!(top_row(&app).contains("DEGRADED"));

    app.insert_resource(InputCapabilities::none().with_key_release(true));
    app.update();
    app.update();
    assert!(!top_row(&app).contains("DEGRADED"));
}
