pub fn parse_port_range(port_str: &str) -> Vec<u16> {
    if let Ok(p) = port_str.parse::<u16>() {
        return vec![p];
    }

    if port_str.contains('-') {
        let parts: Vec<&str> = port_str.split('-').collect();
        if parts.len() == 2 {
            if let (Ok(start), Ok(end)) = (parts[0].parse::<u16>(), parts[1].parse::<u16>()) {
                if start <= end {
                    return (start..=end).collect();
                }
            }
        }
    }
    vec![]
}

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

#[cfg(unix)]
pub fn set_socket_debug<S: AsRawFd>(socket: &S) -> std::io::Result<()> {
    let fd = socket.as_raw_fd();
    let optval: libc::c_int = 1;
    let ret = unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_DEBUG,
            &optval as *const _ as *const libc::c_void,
            std::mem::size_of_val(&optval) as libc::socklen_t,
        )
    };
    if ret == -1 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
pub fn set_socket_debug<S>(_: &S) -> std::io::Result<()> {
    Ok(())
}
