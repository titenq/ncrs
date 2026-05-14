use crate::core::address::{AddressFamily, format_endpoint, parse_numeric_address};
use colored::*;
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::io::{self, AsyncWriteExt};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub(crate) struct UdpOptions {
    pub target: Option<String>,
    pub port: u16,
    pub listen: bool,
    pub verbose: bool,
    pub family: AddressFamily,
    pub source_addr: Option<String>,
    pub source_port: Option<u16>,
    pub numeric: bool,
    pub read_timeout: Option<std::time::Duration>,
    pub broadcast: bool,
    pub debug: bool,
    pub recv_limit: Option<u32>,
    pub ttl: Option<u32>,
    pub tos: Option<u8>,
}

pub(crate) async fn run_udp_node(options: UdpOptions) -> anyhow::Result<()> {
    let addr = udp_bind_addr(
        options.listen,
        options.port,
        options.family,
        options.source_addr.as_deref(),
        options.source_port,
    )?;

    let socket = UdpSocket::bind(&addr).await?;

    if let Some(t) = options.ttl {
        let _ = crate::common::set_socket_ttl(&socket, t, addr.is_ipv4());
    }

    if let Some(t) = options.tos {
        let _ = crate::common::set_socket_tos(&socket, t, addr.is_ipv4());
    }

    if options.broadcast {
        socket.set_broadcast(true)?;
    }

    if options.debug {
        let _ = crate::common::set_socket_debug(&socket);
    }

    let r_socket = Arc::new(socket);
    let s_socket = Arc::clone(&r_socket);

    if options.verbose {
        if options.listen {
            eprintln!("{} UDP listening on port {}", "[*]".yellow(), options.port);
        } else {
            eprintln!(
                "{} UDP socket bound to local port {}",
                "[*]".yellow(),
                r_socket.local_addr()?.port()
            );
        }
    }

    if options.listen {
        let mut buf = [0u8; 65535];
        let (len, peer) = recv_from_with_timeout(&r_socket, &mut buf, options.read_timeout).await?;

        io::stdout().write_all(&buf[..len]).await?;
        io::stdout().flush().await?;

        if options.verbose {
            eprintln!("\n{} UDP packet received from {}", "[+]".green(), peer);
        }

        tokio::select! {
            res = udp_idle_timeout(options.read_timeout) => res,
            res = async {
                let mut stdin_rx = spawn_stdin_reader();
                while let Some(data) = stdin_rx.recv().await {
                    s_socket.send_to(&data, peer).await?;
                }
                anyhow::Ok(())
            } => res,
            res = async {
                let mut recv_buf = [0u8; 65535];
                let mut reads = 1; // Since we already read one packet before the loop on line 52

                if let Some(limit) = options.recv_limit
                    && reads >= limit
                {
                    return anyhow::Ok(());
                }

                loop {
                    let (n, _) = recv_from_with_timeout(&r_socket, &mut recv_buf, options.read_timeout).await?;

                    io::stdout().write_all(&recv_buf[..n]).await?;
                    io::stdout().flush().await?;

                    reads += 1;

                    if let Some(limit) = options.recv_limit
                        && reads >= limit
                    {
                        break anyhow::Ok(());
                    }
                }
            } => res,
        }
    } else {
        let target_str = options
            .target
            .ok_or_else(|| anyhow::anyhow!("Target required"))?;
        let target_addr = if options.numeric {
            parse_numeric_address(&target_str, options.port, options.family)?.to_string()
        } else {
            format_endpoint(&target_str, options.port)
        };

        tokio::select! {
            res = udp_idle_timeout(options.read_timeout) => res,
            res = async {
                let mut stdin_rx = spawn_stdin_reader();
                while let Some(data) = stdin_rx.recv().await {
                    s_socket.send_to(&data, &target_addr).await?;
                }
                anyhow::Ok(())
            } => res,
            res = async {
                let mut recv_buf = [0u8; 65535];
                let mut reads = 0;
                loop {
                    let (n, _) = recv_from_with_timeout(&r_socket, &mut recv_buf, options.read_timeout).await?;

                    io::stdout().write_all(&recv_buf[..n]).await?;
                    io::stdout().flush().await?;

                    reads += 1;

                    if let Some(limit) = options.recv_limit
                        && reads >= limit
                    {
                        break anyhow::Ok(());
                    }
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
