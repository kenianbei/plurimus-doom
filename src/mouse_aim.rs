//! Feeds mouse turn deltas to the engine through `D_PostEvent`,
//! bypassing the doomgeneric crate's keyboard-only input hook. Buttons
//! stay on the key path: the engine's own mouse bindings default to
//! strafe/forward, not the use action this demo maps.

use std::os::raw::c_int;

use plurimus::core::CellSize;

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

/// Differences the pointer's absolute positions into engine turn deltas,
/// in whole columns so the turn rate holds at any [`CellSize`].
#[derive(Default)]
pub struct AimTracker {
    last_column: Option<i32>,
}

impl AimTracker {
    pub fn track(&mut self, position_x: f32, cell: CellSize) -> Option<i32> {
        let column = (position_x / cell.x).floor() as i32;
        let previous = self.last_column.replace(column)?;
        let cells = column - previous;
        (cells != 0).then_some(cells * ENGINE_DX_PER_CELL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: CellSize = CellSize { x: 1.0, y: 1.0 };
    const CENTER: f32 = 0.5;

    fn at_column(column: u16, cell: CellSize) -> f32 {
        (f32::from(column) + CENTER) * cell.x
    }

    #[test]
    fn first_motion_primes_without_turning() {
        let mut tracker = AimTracker::default();
        assert!(tracker.track(at_column(10, SQUARE), SQUARE).is_none());
    }

    #[test]
    fn motion_scales_cell_delta_into_engine_units() {
        let mut tracker = AimTracker::default();
        tracker.track(at_column(10, SQUARE), SQUARE);

        let dx = tracker.track(at_column(14, SQUARE), SQUARE).unwrap();
        assert_eq!(dx, 4 * ENGINE_DX_PER_CELL);

        let dx = tracker.track(at_column(9, SQUARE), SQUARE).unwrap();
        assert_eq!(dx, -5 * ENGINE_DX_PER_CELL);
    }

    #[test]
    fn wide_cells_turn_per_column_not_per_pixel() {
        let wide = CellSize { x: 8.0, y: 16.0 };
        let mut tracker = AimTracker::default();
        tracker.track(at_column(0, wide), wide);

        let dx = tracker.track(at_column(2, wide), wide).unwrap();
        assert_eq!(dx, 2 * ENGINE_DX_PER_CELL);
    }
}
