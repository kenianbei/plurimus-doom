//! Feeds mouse turn deltas to the engine through `D_PostEvent`,
//! bypassing the doomgeneric crate's keyboard-only input hook. Buttons
//! stay on the key path: the engine's own mouse bindings default to
//! strafe/forward, not the use action this demo maps.

use std::os::raw::c_int;

use plurimus::input::{MouseKind, MouseMessage};

const EV_MOUSE: c_int = 2;
const ENGINE_DX_PER_CELL: i32 = 32;

#[repr(C)]
struct DoomEvent {
    kind: c_int,
    data1: c_int,
    data2: c_int,
    data3: c_int,
    data4: c_int,
}

unsafe extern "C" {
    fn D_PostEvent(event: *mut DoomEvent);
}

/// Posts one `ev_mouse` turn to the engine. Doom-thread only: the
/// engine's event ring buffer is unsynchronized.
pub fn post_turn(dx: i32) {
    let mut event = DoomEvent {
        kind: EV_MOUSE,
        data1: 0,
        data2: dx,
        data3: 0,
        data4: 0,
    };
    // SAFETY: the struct matches the engine's `event_t` layout and the
    // engine copies the event out before `D_PostEvent` returns.
    unsafe { D_PostEvent(&mut event) };
}

/// Differences the absolute cell positions of the mouse message stream
/// into engine turn deltas.
#[derive(Default)]
pub struct AimTracker {
    last_column: Option<u16>,
}

impl AimTracker {
    pub fn track(&mut self, message: &MouseMessage) -> Option<i32> {
        match message.kind {
            MouseKind::Drag(_) | MouseKind::Moved => self.turn(message.position.x),
            _ => None,
        }
    }

    fn turn(&mut self, column: u16) -> Option<i32> {
        let previous = self.last_column.replace(column)?;
        let cells = i32::from(column) - i32::from(previous);
        (cells != 0).then_some(cells * ENGINE_DX_PER_CELL)
    }
}

#[cfg(test)]
mod tests {
    use plurimus::core::ratatui_core::layout::Position;
    use plurimus::input::{KeyModifiers, MouseButton};

    use super::*;

    fn mouse(kind: MouseKind, column: u16) -> MouseMessage {
        MouseMessage {
            kind,
            position: Position::new(column, 0),
            modifiers: KeyModifiers::default(),
        }
    }

    #[test]
    fn first_motion_primes_without_turning() {
        let mut tracker = AimTracker::default();
        assert!(tracker.track(&mouse(MouseKind::Moved, 10)).is_none());
    }

    #[test]
    fn motion_scales_cell_delta_into_engine_units() {
        let mut tracker = AimTracker::default();
        tracker.track(&mouse(MouseKind::Moved, 10));

        let dx = tracker.track(&mouse(MouseKind::Moved, 14)).unwrap();
        assert_eq!(dx, 4 * ENGINE_DX_PER_CELL);

        let dx = tracker.track(&mouse(MouseKind::Moved, 9)).unwrap();
        assert_eq!(dx, -5 * ENGINE_DX_PER_CELL);
    }

    #[test]
    fn drag_turns_like_plain_motion() {
        let mut tracker = AimTracker::default();
        tracker.track(&mouse(MouseKind::Drag(MouseButton::Left), 0));

        let dx = tracker
            .track(&mouse(MouseKind::Drag(MouseButton::Left), 2))
            .unwrap();
        assert_eq!(dx, 2 * ENGINE_DX_PER_CELL);
    }

    #[test]
    fn clicks_and_scroll_are_ignored() {
        let mut tracker = AimTracker::default();
        assert!(
            tracker
                .track(&mouse(MouseKind::Down(MouseButton::Left), 0))
                .is_none()
        );
        assert!(tracker.track(&mouse(MouseKind::ScrollUp, 0)).is_none());
    }
}
