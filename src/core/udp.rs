use crate::core::address::{AddressFamily, format_endpoint};
use colored::*;
use std::sync::Arc;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::UdpSocket;

pub async fn run_udp_node(
    target: Option<String>,
    port: u16,
    listen: bool,
    verbose: bool,
    family: AddressFamily,
) -> anyhow::Result<()> {
    let addr = match (listen, family) {
        (true, AddressFamily::Ipv6) => format!("[::]:{}", port),
        (true, AddressFamily::Any | AddressFamily::Ipv4) => format!("0.0.0.0:{}", port),
        (false, AddressFamily::Ipv6) => "[::]:0".to_string(),
        (false, AddressFamily::Any | AddressFamily::Ipv4) => "0.0.0.0:0".to_string(),
    };

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
