//! Points the process's stdout/stderr at the engine log for the process
//! lifetime; the terminal renderer presents through `/dev/tty` instead.

use std::fs::File;
use std::io;
use std::os::fd::{IntoRawFd, RawFd};
use std::path::Path;

pub fn redirect_to_log(path: &Path) -> io::Result<()> {
    let log_fd = File::create(path)?.into_raw_fd();
    // SAFETY: dup2/close on fds this process owns; the log file stays
    // open through fds 1 and 2 after the original descriptor is closed.
    unsafe {
        checked(libc::dup2(log_fd, libc::STDOUT_FILENO))?;
        checked(libc::dup2(log_fd, libc::STDERR_FILENO))?;
        libc::close(log_fd);
    }
    Ok(())
}

fn checked(result: RawFd) -> io::Result<RawFd> {
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(result)
}
