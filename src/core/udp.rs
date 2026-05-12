use crate::core::address::{AddressFamily, format_endpoint};
use colored::*;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::UdpSocket;

pub async fn run_udp_node(
    target: Option<String>,
    port: u16,
    listen: bool,
    verbose: bool,
    family: AddressFamily,
    source_addr: Option<String>,
    source_port: Option<u16>,
) -> anyhow::Result<()> {
    let addr = udp_bind_addr(listen, port, family, source_addr.as_deref(), source_port)?;

    let socket = UdpSocket::bind(&addr).await?;
    let r_socket = Arc::new(socket);
    let s_socket = Arc::clone(&r_socket);

    if verbose {
        if listen {
            println!("{} UDP listening on port {}", "[*]".yellow(), port);
        } else {
            println!(
                "{} UDP socket bound to local port {}",
                "[*]".yellow(),
                r_socket.local_addr()?.port()
            );
        }
    }

    if listen {
        let mut buf = [0u8; 65535];
        let (len, peer) = r_socket.recv_from(&mut buf).await?;

        io::stdout().write_all(&buf[..len]).await?;
        io::stdout().flush().await?;

        if verbose {
            println!("\n{} UDP packet received from {}", "[+]".green(), peer);
        }

        tokio::select! {
            res = async {
                let mut stdin = io::stdin();
                let mut input_buf = [0u8; 65535];
                loop {
                    let n = stdin.read(&mut input_buf).await?;
                    if n == 0 { break; }
                    s_socket.send_to(&input_buf[..n], peer).await?;
                }
                anyhow::Ok(())
            } => res,
            res = async {
                let mut recv_buf = [0u8; 65535];

                loop {
                    let (n, _) = r_socket.recv_from(&mut recv_buf).await?;
                    io::stdout().write_all(&recv_buf[..n]).await?;
                    io::stdout().flush().await?;
                }
            } => res,
        }
    } else {
        let target_str = target.ok_or_else(|| anyhow::anyhow!("Target required"))?;
        let target_addr = format_endpoint(&target_str, port);

        tokio::select! {
            res = async {
                let mut stdin = io::stdin();
                let mut input_buf = [0u8; 65535];
                loop {
                    let n = stdin.read(&mut input_buf).await?;
                    if n == 0 { break; }
                    s_socket.send_to(&input_buf[..n], &target_addr).await?;
                }
                anyhow::Ok(())
            } => res,
            res = async {
                let mut recv_buf = [0u8; 65535];
                loop {
                    let (n, _) = r_socket.recv_from(&mut recv_buf).await?;
                    io::stdout().write_all(&recv_buf[..n]).await?;
                    io::stdout().flush().await?;
                }
            } => res,
        }
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
