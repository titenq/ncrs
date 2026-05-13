use crate::core::address::{AddressFamily, format_endpoint, parse_numeric_address};
use colored::*;
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::io::{self, AsyncWriteExt};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

pub async fn run_udp_node(
    target: Option<String>,
    port: u16,
    listen: bool,
    verbose: bool,
    family: AddressFamily,
    source_addr: Option<String>,
    source_port: Option<u16>,
    numeric: bool,
    read_timeout: Option<std::time::Duration>,
    broadcast: bool,
    debug: bool,
) -> anyhow::Result<()> {
    let addr = udp_bind_addr(listen, port, family, source_addr.as_deref(), source_port)?;

    let socket = UdpSocket::bind(&addr).await?;

    if broadcast {
        socket.set_broadcast(true)?;
    }
    
    if debug {
        let _ = crate::common::set_socket_debug(&socket);
    }

    let r_socket = Arc::new(socket);
    let s_socket = Arc::clone(&r_socket);

    if verbose {
        if listen {
            eprintln!("{} UDP listening on port {}", "[*]".yellow(), port);
        } else {
            eprintln!(
                "{} UDP socket bound to local port {}",
                "[*]".yellow(),
                r_socket.local_addr()?.port()
            );
        }
    }

    if listen {
        let mut buf = [0u8; 65535];
        let (len, peer) = recv_from_with_timeout(&r_socket, &mut buf, read_timeout).await?;

        io::stdout().write_all(&buf[..len]).await?;
        io::stdout().flush().await?;

        if verbose {
            eprintln!("\n{} UDP packet received from {}", "[+]".green(), peer);
        }

        tokio::select! {
            res = udp_idle_timeout(read_timeout) => res,
            res = async {
                let mut stdin_rx = spawn_stdin_reader();
                while let Some(data) = stdin_rx.recv().await {
                    s_socket.send_to(&data, peer).await?;
                }
                anyhow::Ok(())
            } => res,
            res = async {
                let mut recv_buf = [0u8; 65535];

                loop {
                    let (n, _) = recv_from_with_timeout(&r_socket, &mut recv_buf, read_timeout).await?;
                    io::stdout().write_all(&recv_buf[..n]).await?;
                    io::stdout().flush().await?;
                }
            } => res,
        }
    } else {
        let target_str = target.ok_or_else(|| anyhow::anyhow!("Target required"))?;
        let target_addr = if numeric {
            parse_numeric_address(&target_str, port, family)?.to_string()
        } else {
            format_endpoint(&target_str, port)
        };

        tokio::select! {
            res = udp_idle_timeout(read_timeout) => res,
            res = async {
                let mut stdin_rx = spawn_stdin_reader();
                while let Some(data) = stdin_rx.recv().await {
                    s_socket.send_to(&data, &target_addr).await?;
                }
                anyhow::Ok(())
            } => res,
            res = async {
                let mut recv_buf = [0u8; 65535];
                loop {
                    let (n, _) = recv_from_with_timeout(&r_socket, &mut recv_buf, read_timeout).await?;
                    io::stdout().write_all(&recv_buf[..n]).await?;
                    io::stdout().flush().await?;
                }
            } => res,
        }
    }
}

fn spawn_stdin_reader() -> mpsc::UnboundedReceiver<Vec<u8>> {
    let (tx, rx) = mpsc::unbounded_channel();

    std::thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 65535];

        loop {
            match stdin.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    rx
}

async fn udp_idle_timeout(read_timeout: Option<std::time::Duration>) -> anyhow::Result<()> {
    if let Some(timeout_duration) = read_timeout {
        tokio::time::sleep(timeout_duration).await;
        Err(anyhow::anyhow!(
            "UDP receive timed out after {}s",
            timeout_duration.as_secs()
        ))
    } else {
        std::future::pending().await
    }
}

async fn recv_from_with_timeout(
    socket: &UdpSocket,
    buf: &mut [u8],
    read_timeout: Option<std::time::Duration>,
) -> anyhow::Result<(usize, SocketAddr)> {
    if let Some(timeout_duration) = read_timeout {
        match tokio::time::timeout(timeout_duration, socket.recv_from(buf)).await {
            Ok(result) => Ok(result?),
            Err(_) => Err(anyhow::anyhow!(
                "UDP receive timed out after {}s",
                timeout_duration.as_secs()
            )),
        }
    } else {
        Ok(socket.recv_from(buf).await?)
    }
}

fn udp_bind_addr(
    listen: bool,
    port: u16,
    family: AddressFamily,
    source_addr: Option<&str>,
    source_port: Option<u16>,
) -> anyhow::Result<SocketAddr> {
    if listen {
        return Ok(match family {
            AddressFamily::Any | AddressFamily::Ipv4 => {
                SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)
            }
            AddressFamily::Ipv6 => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port),
        });
    }

    let ip = match source_addr {
        Some(addr) => addr.parse::<IpAddr>()?,
        None if matches!(family, AddressFamily::Ipv6) => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        None => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
    };

    if matches!(family, AddressFamily::Ipv4) && !ip.is_ipv4() {
        return Err(anyhow::anyhow!(
            "Source address family does not match requested IPv4 mode"
        ));
    }

    if matches!(family, AddressFamily::Ipv6) && !ip.is_ipv6() {
        return Err(anyhow::anyhow!(
            "Source address family does not match requested IPv6 mode"
        ));
    }

    Ok(SocketAddr::new(ip, source_port.unwrap_or(0)))
}
