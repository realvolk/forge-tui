use std::fs;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

pub fn open() -> io::Result<fs::File> {
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
}

pub fn redirect_stdout_to_tty() -> io::Result<OwnedFd> {
    let tty = open()?;
    let old = unsafe {
        let fd = libc::dup(1);
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        OwnedFd::from_raw_fd(fd)
    };
    let tty_fd = tty.as_raw_fd();
    if unsafe { libc::dup2(tty_fd, 1) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(old)
}