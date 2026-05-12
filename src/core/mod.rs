use colored::*;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio::io::{self, AsyncRead, AsyncWrite};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UdpSocket;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore, ServerConfig};
use tokio_rustls::{TlsAcceptor, TlsConnector};

#[derive(Clone, Copy)]
pub enum AddressFamily {
    Any,
    Ipv4,
    Ipv6,
}

impl AddressFamily {
    fn label(self) -> &'static str {
        match self {
            Self::Any => "IPv4/IPv6/DNS",
            Self::Ipv4 => "IPv4",
            Self::Ipv6 => "IPv6",
        }
    }
}

fn format_endpoint(target: &str, port: u16) -> String {
    if !target.contains('[') && target.contains(':') {
        format!("[{}]:{}", target, port)
    } else {
        format!("{}:{}", target, port)
    }
}

async fn resolve_address(
    target: &str,
    port: u16,
    family: AddressFamily,
    timeout_duration: std::time::Duration,
) -> anyhow::Result<std::net::SocketAddr> {
    let endpoint = format_endpoint(target, port);
    let addrs =
        match tokio::time::timeout(timeout_duration, tokio::net::lookup_host(&endpoint)).await {
            Ok(Ok(iter)) => iter,
            Ok(Err(e)) => return Err(anyhow::anyhow!("Failed to resolve {}: {}", endpoint, e)),
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "DNS resolution timed out after {}s",
                    timeout_duration.as_secs()
                ));
            }
        };

    addrs
        .filter(|addr| match family {
            AddressFamily::Any => true,
            AddressFamily::Ipv4 => addr.is_ipv4(),
            AddressFamily::Ipv6 => addr.is_ipv6(),
        })
        .next()
        .ok_or_else(|| anyhow::anyhow!("Could not resolve to any {} address", family.label()))
}

pub async fn run_client(
    target: String,
    port: u16,
    verbose: bool,
    secure: bool,
    timeout_secs: u64,
    family: AddressFamily,
    crlf: bool,
) -> anyhow::Result<()> {
    let addr = format_endpoint(&target, port);

    if verbose {
        println!(
            "{} [{}] Connecting to {}...",
            "[*]".yellow(),
            family.label(),
            addr
        );
    }

    let timeout_duration = std::time::Duration::from_secs(timeout_secs);

    if verbose {
        println!("{} Resolving address...", "[*]".yellow());
    }

    let target_addr = resolve_address(&target, port, family, timeout_duration).await?;

    if verbose {
        println!("{} Resolved to: {}", "[*]".yellow(), target_addr);
    }

    if verbose {
        println!("{} Connecting...", "[*]".yellow());
    }

    let stream = match tokio::time::timeout(timeout_duration, TcpStream::connect(target_addr)).await
    {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            return Err(anyhow::anyhow!(
                "Failed to connect to {}: {}",
                target_addr,
                e
            ));
        }
        Err(_) => {
            return Err(anyhow::anyhow!(
                "Connection to {} timed out after {}s",
                target_addr,
                timeout_secs
            ));
        }
    };

    if secure {
        let mut root_cert_store = RootCertStore::empty();
        let cert_result = rustls_native_certs::load_native_certs();

        root_cert_store.add_parsable_certificates(cert_result.certs);

        if std::path::Path::new("cert.pem").exists() {
            let cert_file = File::open("cert.pem")?;
            let mut reader = BufReader::new(cert_file);
            let certs = rustls_pemfile::certs(&mut reader).collect::<Result<Vec<_>, _>>()?;

            root_cert_store.add_parsable_certificates(certs);
        }

        let config = ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(config));
        let domain = ServerName::try_from(target.as_str())?.to_owned();
        let tls_stream = connector.connect(domain, stream).await?;

        if verbose {
            println!(
                "{} Connected! Type your messages and press Enter.",
                "[+]".green()
            );

            if crlf {
                println!(
                    "{} CRLF mode active (-C): line endings typed as LF are sent as CRLF.",
                    "[*]".blue()
                );
                println!(
                    "{} For HTTP, finish headers with an empty line.",
                    "[*]".blue()
                );
            }
        }

        handle_duplex(tls_stream, crlf).await
    } else {
        if verbose {
            println!(
                "{} Connected! Type your messages and press Enter.",
                "[+]".green()
            );

            if crlf {
                println!(
                    "{} CRLF mode active (-C): line endings typed as LF are sent as CRLF.",
                    "[*]".blue()
                );
                println!(
                    "{} For HTTP, finish headers with an empty line.",
                    "[*]".blue()
                );
            }
        }

        handle_duplex(stream, crlf).await
    }
}

pub async fn run_server(
    port: u16,
    verbose: bool,
    secure: bool,
    family: AddressFamily,
    crlf: bool,
) -> anyhow::Result<()> {
    let addr = match family {
        AddressFamily::Any | AddressFamily::Ipv4 => format!("0.0.0.0:{}", port),
        AddressFamily::Ipv6 => format!("[::]:{}", port),
    };

    let listener = TcpListener::bind(&addr).await?;
    println!("{} Listening on {}...", "[*]".yellow(), addr);

    let (stream, remote_addr) = listener.accept().await?;
    if verbose {
        println!("{} Connection from {}", "[+]".green(), remote_addr);
    }

    if secure {
        if verbose {
            println!("{} Loading certificates and private key...", "[*]".yellow());
        }

        let cert_file = File::open("cert.pem")?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs = rustls_pemfile::certs(&mut cert_reader).collect::<Result<Vec<_>, _>>()?;

        let key_file = File::open("key.pem")?;
        let mut key_reader = BufReader::new(key_file);
        let key = rustls_pemfile::private_key(&mut key_reader)?
            .ok_or_else(|| anyhow::anyhow!("No private key found"))?;

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        let acceptor = TlsAcceptor::from(Arc::new(config));

        if verbose {
            println!("{} Waiting for TLS handshake...", "[*]".yellow());
        }
        let tls_stream = acceptor.accept(stream).await?;

        if verbose {
            println!("{} TLS Handshake successful!", "[+]".green());
        }

        handle_duplex(tls_stream, crlf).await
    } else {
        handle_duplex(stream, crlf).await
    }
}

pub async fn run_port_scan(
    target: String,
    ports: Vec<u16>,
    timeout_secs: u64,
    verbose: bool,
    family: AddressFamily,
) -> anyhow::Result<()> {
    let target = Arc::new(target);
    let mut handles = vec![];

    if verbose {
        println!(
            "{} Scanning {} ports on {}...",
            "[*]".yellow(),
            ports.len(),
            target
        );
    }

    for port in ports {
        let t = Arc::clone(&target);
        handles.push(tokio::spawn(async move {
            let timeout = std::time::Duration::from_secs(timeout_secs);
            if let Ok(addr) = resolve_address(&t, port, family, timeout).await {
                if let Ok(Ok(_)) = tokio::time::timeout(timeout, TcpStream::connect(addr)).await {
                    println!("{} port {} open", "Connection to".green(), port);
                }
            }
        }));
    }

    for h in handles {
        let _ = h.await;
    }

    if verbose {
        println!("{} Scan complete.", "[*]".yellow());
    }

    Ok(())
}

async fn handle_duplex<S>(stream: S, crlf: bool) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut reader, mut writer) = io::split(stream);

    let stdin_to_socket = tokio::spawn(async move {
        let mut stdin = io::stdin();
        let mut buf = [0u8; 1024];

        loop {
            let n = stdin.read(&mut buf).await?;
            if n == 0 {
                break;
            }

            if crlf {
                let mut data = Vec::with_capacity(n * 2);
                let mut previous_was_cr = false;

                for &byte in &buf[..n] {
                    if byte == b'\n' && !previous_was_cr {
                        data.push(b'\r');
                    }
                    data.push(byte);
                    previous_was_cr = byte == b'\r';
                }

                writer.write_all(&data).await?;
            } else {
                writer.write_all(&buf[..n]).await?;
            }

            writer.flush().await?;
        }

        writer.shutdown().await?;
        anyhow::Ok(())
    });

    let socket_to_stdout = tokio::spawn(async move {
        let mut stdout = io::stdout();
        io::copy(&mut reader, &mut stdout).await
    });

    tokio::pin!(stdin_to_socket);
    tokio::pin!(socket_to_stdout);

    tokio::select! {
        res = &mut socket_to_stdout => {
            res??;
            stdin_to_socket.abort();
        },
        res = &mut stdin_to_socket => {
            res??;
            socket_to_stdout.await??;
        },
    }

    Ok(())
}

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
