//! Runs the Doom engine on its own thread and bridges frames, keys, and
//! mouse aim.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;

use bevy_ecs::prelude::Resource;
use doomgeneric::game::{self, DoomGeneric};
use doomgeneric::input::KeyData;

use crate::mouse_aim;

pub const LOG_FILE: &str = "doom.log";

#[derive(Default)]
pub struct SharedFrame {
    pub pixels: Vec<u32>,
    pub width: usize,
    pub height: usize,
    pub version: u64,
}

impl SharedFrame {
    fn store(&mut self, pixels: &[u32], width: usize, height: usize) {
        self.pixels.clear();
        self.pixels.extend_from_slice(pixels);
        self.width = width;
        self.height = height;
        self.version += 1;
    }
}

/// Shared view of the latest engine frame; `version` orders frames and
/// stays 0 until the engine has drawn once.
#[derive(Clone, Default)]
pub struct FrameHandle(Arc<Mutex<SharedFrame>>);

impl FrameHandle {
    #[cfg(test)]
    pub fn new(frame: SharedFrame) -> Self {
        Self(Arc::new(Mutex::new(frame)))
    }

    /// Swaps the newest frame into `destination` and reports whether it
    /// changed. The shared side keeps the consumer's previous buffer —
    /// stale content guarded by the version check — so neither thread
    /// copies pixels under the lock. Single-consumer by design.
    pub fn fetch_if_newer(&self, destination: &mut SharedFrame) -> bool {
        let mut latest = self.lock();
        if latest.version == destination.version {
            return false;
        }
        std::mem::swap(&mut destination.pixels, &mut latest.pixels);
        destination.width = latest.width;
        destination.height = latest.height;
        destination.version = latest.version;
        true
    }

    fn lock(&self) -> MutexGuard<'_, SharedFrame> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Bridges the Doom thread into the app: latest frame in, key events
/// and accumulated turn deltas out.
#[derive(Resource)]
pub struct DoomHandle {
    pub frame: FrameHandle,
    pub keys: Sender<KeyData>,
    pub turn: Arc<AtomicI32>,
}

struct DoomShim {
    frame: FrameHandle,
    keys: Receiver<KeyData>,
}

impl DoomGeneric for DoomShim {
    fn draw_frame(&mut self, screen_buffer: &[u32], xres: usize, yres: usize) {
        self.frame.lock().store(screen_buffer, xres, yres);
    }

    fn get_key(&mut self) -> Option<KeyData> {
        self.keys.try_recv().ok()
    }

    fn set_window_title(&mut self, _title: &str) {}
}

/// Runs Doom on a detached thread. Engine output goes wherever fds 1/2
/// point — redirect them to the log before calling this.
pub fn spawn_doom(iwad: PathBuf) -> DoomHandle {
    let frame = FrameHandle::default();
    let (key_sender, key_receiver) = mpsc::channel();
    let turn = Arc::new(AtomicI32::new(0));
    let shim = DoomShim {
        frame: frame.clone(),
        keys: key_receiver,
    };
    let engine_turn = Arc::clone(&turn);
    thread::spawn(move || run_doom(&iwad, shim, &engine_turn));
    DoomHandle {
        frame,
        keys: key_sender,
        turn,
    }
}

fn run_doom(iwad: &Path, shim: DoomShim, turn: &AtomicI32) {
    let arguments = [
        "plurimus-doom".to_owned(),
        "-iwad".to_owned(),
        iwad.to_string_lossy().into_owned(),
    ];
    game::init(&arguments, shim);
    loop {
        let dx = turn.swap(0, Ordering::Relaxed);
        if dx != 0 {
            mouse_aim::post_turn(dx);
        }
        game::tick();
    }
}
