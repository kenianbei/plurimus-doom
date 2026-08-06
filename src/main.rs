//! Doom in the terminal: doomgeneric rendered as halfblock cells through
//! plurimus.

#[cfg(not(feature = "gpu"))]
mod cpu_blit;
mod doom_thread;
#[cfg(feature = "gpu")]
mod gpu;
mod keymap;
mod mouse_aim;
mod plugin;
mod restore;
mod stdio;
mod wad;

use std::path::Path;
use std::time::Duration;

use bevy_app::{App, AppExit, ScheduleRunnerPlugin};
use plurimus::core::CorePlugin;
use plurimus::crossterm::CrosstermPlugin;

use crate::plugin::DoomPlugin;

const FRAME_INTERVAL: Duration = Duration::from_millis(16);

fn main() -> AppExit {
    let iwad = match wad::locate() {
        Ok(path) => path,
        Err(message) => {
            eprintln!("{message}");
            return AppExit::error();
        }
    };
    let terminal = match CrosstermPlugin::tty() {
        Ok(plugin) => plugin,
        Err(error) => {
            eprintln!("no controlling terminal: {error}");
            return AppExit::error();
        }
    };
    // Terminal init before the log redirect: crossterm's kitty-support
    // query falls back to writing through stdout, so stdout must still
    // reach the terminal while CrosstermPlugin builds.
    let mut app = App::new();
    app.add_plugins((
        ScheduleRunnerPlugin::run_loop(FRAME_INTERVAL),
        CorePlugin,
        terminal,
        DoomPlugin,
    ));
    if let Err(error) = stdio::redirect_to_log(Path::new(doom_thread::LOG_FILE)) {
        eprintln!("warning: engine output not redirected: {error}");
    }
    restore::install();
    #[cfg(not(feature = "gpu"))]
    app.add_plugins(cpu_blit::CpuBlitPlugin);
    #[cfg(feature = "gpu")]
    app.add_plugins(gpu::GpuPlugin);
    let doom = doom_thread::spawn_doom(iwad);
    app.insert_resource(doom);
    let capabilities = app.world().resource::<plurimus::input::InputCapabilities>();
    eprintln!(
        "input capabilities: key_release={}",
        capabilities.key_release
    );
    app.run()
}
