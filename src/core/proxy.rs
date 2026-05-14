use crate::core::tcp::{TcpSocketOptions, connect_tcp};
use anyhow::{Context, Result, anyhow};
use base64::prelude::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Clone, Debug)]
pub(crate) struct ProxyOptions {
    pub proxy_addr: String,
    pub proxy_type: Option<String>,
    pub proxy_username: Option<String>,
    pub target_host: String,
    pub target_port: u16,
    pub timeout_duration: std::time::Duration,
    pub socket: TcpSocketOptions,
}

pub(crate) async fn connect_via_proxy(options: ProxyOptions) -> Result<TcpStream> {
    let p_type = options.proxy_type.as_deref().unwrap_or("5");
    let (p_host, p_port) = parse_proxy_address(&options.proxy_addr, p_type)?;
    let resolved_proxy = crate::core::address::resolve_address(
        &p_host,
        p_port,
        crate::core::address::AddressFamily::Any,
        options.timeout_duration,
        false,
    )
    .await
    .context("Failed to resolve proxy address")?;

    if options.socket.debug {
        eprintln!("[*] Connecting to proxy {}...", resolved_proxy);
    }

    let mut proxy_socket = options.socket.clone();

    proxy_socket.recv_bytes = None;
    proxy_socket.send_bytes = None;
    proxy_socket.ttl = None;
    proxy_socket.tos = None;
    proxy_socket.minttl = None;
    proxy_socket.tcp_md5sig = false;
    proxy_socket.dccp = false;

    let mut stream = connect_tcp(resolved_proxy, options.timeout_duration, &proxy_socket)
        .await
        .context("Failed to connect to proxy")?;

    if options.socket.debug {
        eprintln!(
            "[*] Connected to proxy, performing {} handshake to {}:{}...",
            p_type, options.target_host, options.target_port
        );
    }

    let handshake_future = async {
        match p_type {
            "4" => socks4_handshake(&mut stream, &options.target_host, options.target_port).await,
            "5" => socks5_handshake(&mut stream, &options.target_host, options.target_port).await,
            "connect" => {
                http_connect_handshake(
                    &mut stream,
                    &options.target_host,
                    options.target_port,
                    options.proxy_username.as_deref(),
                )
                .await
            }
            _ => Err(anyhow!("Unsupported proxy protocol: {}", p_type)),
        }
    };

    timeout(options.timeout_duration, handshake_future)
        .await
        .map_err(|_| anyhow!("Proxy handshake timed out"))??;

    if options.socket.debug {
        eprintln!("[+] Proxy handshake successful!");
    }

    Ok(stream)
}

fn parse_proxy_address(addr: &str, p_type: &str) -> Result<(String, u16)> {
    let default_port = match p_type {
        "4" | "5" => 1080,
        "connect" => 3128,
        _ => return Err(anyhow!("Unsupported proxy protocol: {}", p_type)),
    };

    if addr.starts_with('[')
        && let Some(end_idx) = addr.find(']')
    {
        let host = addr[1..end_idx].to_string();
        let port_str = &addr[end_idx + 1..];

        if port_str.starts_with(':') && port_str.len() > 1 {
            let port = port_str[1..].parse::<u16>().context("Invalid port")?;

            return Ok((host, port));
        } else {
            return Ok((host, default_port));
        }
    }

    if let Some((host, port_str)) = addr.rsplit_once(':') {
        if let Ok(port) = port_str.parse::<u16>() {
            Ok((host.to_string(), port))
        } else {
            Ok((addr.to_string(), default_port))
        }
    } else {
        Ok((addr.to_string(), default_port))
    }
}

async fn socks5_handshake(
    stream: &mut TcpStream,
    target_host: &str,
    target_port: u16,
) -> Result<()> {
    stream.write_all(&[0x05, 0x01, 0x00]).await?;

    let mut resp = [0u8; 2];

    stream.read_exact(&mut resp).await?;

    if resp[0] != 0x05 || resp[1] != 0x00 {
        return Err(anyhow!("SOCKS5 proxy rejected no-authentication method"));
    }

    let mut req = vec![0x05, 0x01, 0x00];

    if let Ok(ipv4) = target_host.parse::<std::net::Ipv4Addr>() {
        req.push(0x01);
        req.extend_from_slice(&ipv4.octets());
    } else if let Ok(ipv6) = target_host.parse::<std::net::Ipv6Addr>() {
        req.push(0x04);
        req.extend_from_slice(&ipv6.octets());
    } else {
        req.push(0x03);

        let host_bytes = target_host.as_bytes();

        if host_bytes.len() > 255 {
            return Err(anyhow!("Target host name too long for SOCKS5"));
        }

        req.push(host_bytes.len() as u8);
        req.extend_from_slice(host_bytes);
    }

    req.push((target_port >> 8) as u8);
    req.push((target_port & 0xFF) as u8);

    stream.write_all(&req).await?;

    let mut resp_header = [0u8; 4];

    stream.read_exact(&mut resp_header).await?;

    if resp_header[0] != 0x05 {
        return Err(anyhow!("Invalid SOCKS5 response version"));
    }

    if resp_header[1] != 0x00 {
        return Err(anyhow!(
            "SOCKS5 proxy connection failed with error code: {:#04x}",
            resp_header[1]
        ));
    }

    match resp_header[3] {
        0x01 => {
            let mut addr = [0u8; 4];
            stream.read_exact(&mut addr).await?;
        }
        0x03 => {
            let mut len_buf = [0u8; 1];
            stream.read_exact(&mut len_buf).await?;
            let mut domain = vec![0u8; len_buf[0] as usize];
            stream.read_exact(&mut domain).await?;
        }
        0x04 => {
            let mut addr = [0u8; 16];
            stream.read_exact(&mut addr).await?;
        }
        _ => return Err(anyhow!("Unknown SOCKS5 address type in response")),
    }

    let mut port = [0u8; 2];

    stream.read_exact(&mut port).await?;

    Ok(())
}

async fn socks4_handshake(
    stream: &mut TcpStream,
    target_host: &str,
    target_port: u16,
) -> Result<()> {
    let ipv4 = match target_host.parse::<std::net::Ipv4Addr>() {
        Ok(ip) => ip,
        Err(_) => {
            let addrs = tokio::net::lookup_host(format!("{}:{}", target_host, target_port)).await?;
            let mut resolved = None;

            for addr in addrs {
                if let std::net::SocketAddr::V4(v4) = addr {
                    resolved = Some(*v4.ip());
                    break;
                }
            }

            resolved.ok_or_else(|| anyhow!("Could not resolve target host to IPv4 for SOCKS4"))?
        }
    };

    let mut req = vec![0x04, 0x01];

    req.push((target_port >> 8) as u8);
    req.push((target_port & 0xFF) as u8);
    req.extend_from_slice(&ipv4.octets());
    req.push(0x00);

    stream.write_all(&req).await?;

    let mut resp = [0u8; 8];

    stream.read_exact(&mut resp).await?;

    if resp[0] != 0x00 || resp[1] != 0x5A {
        return Err(anyhow!(
            "SOCKS4 proxy connection failed with status: {:#04x}",
            resp[1]
        ));
    }

    Ok(())
}

async fn http_connect_handshake(
    stream: &mut TcpStream,
    target_host: &str,
    target_port: u16,
    proxy_username: Option<&str>,
) -> Result<()> {
    let mut req = format!(
        "CONNECT {0}:{1} HTTP/1.1\r\nHost: {0}:{1}\r\n",
        target_host, target_port
    );

    if let Some(user) = proxy_username {
        let auth = BASE64_STANDARD.encode(user.as_bytes());

        req.push_str(&format!("Proxy-Authorization: Basic {}\r\n", auth));
    }

    req.push_str("\r\n");

    stream.write_all(req.as_bytes()).await?;

    let mut response = Vec::new();
    let mut buf = [0u8; 1];

    loop {
        stream.read_exact(&mut buf).await?;
        response.push(buf[0]);

        if response.ends_with(b"\r\n\r\n") {
            break;
        }

        if response.len() > 8192 {
            return Err(anyhow!("Proxy HTTP response too large"));
        }
    }

    let resp_str = String::from_utf8_lossy(&response);
    let mut lines = resp_str.lines();
    let status_line = lines
        .next()
        .ok_or_else(|| anyhow!("Empty HTTP proxy response"))?;

    if !status_line.starts_with("HTTP/1.") {
        return Err(anyhow!("Invalid HTTP proxy response: {}", status_line));
    }

    let parts: Vec<&str> = status_line.split_whitespace().collect();

    if parts.len() < 2 {
        return Err(anyhow!("Invalid HTTP proxy status line: {}", status_line));
    }

    let status_code = parts[1];
    
    if !status_code.starts_with('2') {
        return Err(anyhow!("HTTP proxy connection failed: {}", status_line));
    }

    Ok(())
}
