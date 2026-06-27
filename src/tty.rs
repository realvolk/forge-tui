use std::fs;
use std::io;

pub fn open() -> io::Result<fs::File> {
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
}

pub fn redirect_stdout() -> io::Result<std::os::unix::io::OwnedFd> {
    let tty = open()?;
    let old = unsafe {
        let fd = libc::dup(1);
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        std::os::unix::io::OwnedFd::from_raw_fd(fd)
    };
    let tty_fd = tty.as_raw_fd();
    if unsafe { libc::dup2(tty_fd, 1) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(old)
}

pub fn restore_stdout(old: std::os::unix::io::OwnedFd) {
    let old_fd = old.as_raw_fd();
    unsafe {
        libc::dup2(old_fd, 1);
    }
}