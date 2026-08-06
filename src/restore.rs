//! Restores the terminal when the C engine exits the process directly.

/// Registers an `atexit` hook running plurimus's terminal restore. Doom's
/// in-game quit calls C `exit()`, which skips every Rust drop path; this
/// hook is the only restore that runs on that route.
pub fn install() {
    // SAFETY: restore_terminal is an extern "C" fn that does not unwind —
    // plurimus's restore is best-effort and ignores every error.
    unsafe {
        libc::atexit(restore_terminal);
    }
}

extern "C" fn restore_terminal() {
    plurimus::crossterm::restore();
}
