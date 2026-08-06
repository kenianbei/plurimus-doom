//! Locates the IWAD from the command line or the working directory.

use std::env;
use std::path::PathBuf;

const CANDIDATE_WADS: [&str; 4] = ["doom1.wad", "doom.wad", "freedoom1.wad", "freedoom2.wad"];

pub fn locate() -> Result<PathBuf, String> {
    match iwad_argument() {
        Some(path) if path.exists() => Ok(path),
        Some(path) => Err(format!("iwad not found: {}", path.display())),
        None => CANDIDATE_WADS
            .iter()
            .map(PathBuf::from)
            .find(|path| path.exists())
            .ok_or_else(missing_wad_help),
    }
}

fn iwad_argument() -> Option<PathBuf> {
    let mut arguments = env::args();
    arguments.find(|argument| argument == "-iwad")?;
    arguments.next().map(PathBuf::from)
}

fn missing_wad_help() -> String {
    format!(
        "no IWAD found: pass -iwad <path> or put one of {} in the\n\
         current directory.\n\n\
         \x20 ./fetch-wad.sh               downloads freedoom1.wad (free, BSD-licensed)\n\
         \x20 https://freedoom.github.io   manual download",
        CANDIDATE_WADS.join(", ")
    )
}
