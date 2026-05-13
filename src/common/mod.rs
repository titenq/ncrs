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

pub fn parse_tos(tos: &str) -> Option<u8> {
    match tos.to_lowercase().as_str() {
        "critical" => Some(0xa0),
        "inetcontrol" => Some(0xc0),
        "lowcost" => Some(0x02),
        "lowdelay" => Some(0x10),
        "netcontrol" => Some(0xe0),
        "throughput" => Some(0x08),
        "reliability" => Some(0x04),
        _ => {
            if tos.starts_with("0x") {
                u8::from_str_radix(&tos[2..], 16).ok()
            } else {
                tos.parse::<u8>().ok()
            }
        }
    }
}

#[cfg(unix)]
pub fn set_socket_ttl<S: AsRawFd>(socket: &S, ttl: u32, is_ipv4: bool) -> std::io::Result<()> {
    let fd = socket.as_raw_fd();
    let optval: libc::c_int = ttl as libc::c_int;
    let ret = unsafe {
        if is_ipv4 {
            libc::setsockopt(
                fd,
                libc::IPPROTO_IP,
                libc::IP_TTL,
                &optval as *const _ as *const libc::c_void,
                std::mem::size_of_val(&optval) as libc::socklen_t,
            )
        } else {
            libc::setsockopt(
                fd,
                libc::IPPROTO_IPV6,
                libc::IPV6_UNICAST_HOPS,
                &optval as *const _ as *const libc::c_void,
                std::mem::size_of_val(&optval) as libc::socklen_t,
            )
        }
    };
    if ret == -1 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
pub fn set_socket_ttl<S>(_: &S, _: u32, _: bool) -> std::io::Result<()> {
    Ok(())
}

#[cfg(unix)]
pub fn set_socket_tos<S: AsRawFd>(socket: &S, tos: u8, is_ipv4: bool) -> std::io::Result<()> {
    let fd = socket.as_raw_fd();
    let optval: libc::c_int = tos as libc::c_int;
    let ret = unsafe {
        if is_ipv4 {
            libc::setsockopt(
                fd,
                libc::IPPROTO_IP,
                libc::IP_TOS,
                &optval as *const _ as *const libc::c_void,
                std::mem::size_of_val(&optval) as libc::socklen_t,
            )
        } else {
            libc::setsockopt(
                fd,
                libc::IPPROTO_IPV6,
                libc::IPV6_TCLASS,
                &optval as *const _ as *const libc::c_void,
                std::mem::size_of_val(&optval) as libc::socklen_t,
            )
        }
    };
    if ret == -1 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
pub fn set_socket_tos<S>(_: &S, _: u8, _: bool) -> std::io::Result<()> {
    Ok(())
}

