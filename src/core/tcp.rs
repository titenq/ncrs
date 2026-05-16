use crate::core::address::{AddressFamily, format_endpoint, resolve_address};
use crate::core::duplex::{
    DuplexOptions, convert_lf_to_crlf, handle_duplex, handle_duplex_with_input,
};
use crate::tls;
use colored::*;
use std::fs::File;
use std::io::BufReader;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::sync::broadcast;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore, ServerConfig};
use tokio_rustls::{TlsAcceptor, TlsConnector};

#[derive(Clone, Debug, Default)]
pub(crate) struct TcpSocketOptions {
    pub source_addr: Option<String>,
    pub source_port: Option<u16>,
    pub debug: bool,
    pub recv_bytes: Option<u32>,
    pub send_bytes: Option<u32>,
    pub ttl: Option<u32>,
    pub tos: Option<u8>,
    pub minttl: Option<u32>,
    pub tcp_md5sig: bool,
    pub dccp: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct TcpClientOptions {
    pub target: String,
    pub port: u16,
    pub verbose: bool,
    pub tls: bool,
    pub timeout_secs: u64,
    pub family: AddressFamily,
    pub numeric: bool,
    pub duplex: DuplexOptions,
    pub socket: TcpSocketOptions,
    pub proxy: Option<String>,
    pub proxy_type: Option<String>,
    pub proxy_username: Option<String>,
    pub pass_fd: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct TcpServerOptions {
    pub port: u16,
    pub verbose: bool,
    pub tls: bool,
    pub family: AddressFamily,
    pub duplex: DuplexOptions,
    pub socket: TcpSocketOptions,
    pub pass_fd: bool,
}

#[derive(Clone, Copy, Debug)]
struct ServerStreamOptions {
    verbose: bool,
    tls: bool,
    duplex: DuplexOptions,
    pass_fd: bool,
}

pub(crate) async fn connect_tcp(
    target_addr: SocketAddr,
    timeout_duration: std::time::Duration,
    options: &TcpSocketOptions,
) -> anyhow::Result<TcpStream> {
    if options.dccp {
        return Err(anyhow::anyhow!(
            "DCCP mode (-Z) is not currently supported by tokio networking."
        ));
    }

    let local_ip = match options.source_addr.as_deref() {
        Some(addr) => Some(addr.parse::<IpAddr>()?),
        None => None,
    };

    if let Some(ip) = local_ip
        && target_addr.is_ipv4() != ip.is_ipv4()
    {
        return Err(anyhow::anyhow!(
            "Source address family does not match target address family"
        ));
    }

    let socket = if target_addr.is_ipv4() {
        TcpSocket::new_v4()?
    } else {
        TcpSocket::new_v6()?
    };

    if options.debug {
        let _ = crate::common::set_socket_debug(&socket);
    }

    if let Some(size) = options.recv_bytes {
        socket.set_recv_buffer_size(size)?;
    }

    if let Some(size) = options.send_bytes {
        socket.set_send_buffer_size(size)?;
    }

    if let Some(t) = options.ttl {
        let _ = crate::common::set_socket_ttl(&socket, t, target_addr.is_ipv4());
    }

    if let Some(t) = options.tos {
        let _ = crate::common::set_socket_tos(&socket, t, target_addr.is_ipv4());
    }

    if let Some(mttl) = options.minttl {
        let _ = crate::common::set_socket_minttl(&socket, mttl);
    }

    if options.tcp_md5sig {
        let _ = crate::common::set_socket_tcp_md5sig(&socket);
    }

    if local_ip.is_some() || options.source_port.is_some() {
        let ip = local_ip.unwrap_or_else(|| {
            if target_addr.is_ipv4() {
                IpAddr::V4(Ipv4Addr::UNSPECIFIED)
            } else {
                IpAddr::V6(Ipv6Addr::UNSPECIFIED)
            }
        });

        let local_addr = SocketAddr::new(ip, options.source_port.unwrap_or(0));

        socket.bind(local_addr)?;
    }

    match tokio::time::timeout(timeout_duration, socket.connect(target_addr)).await {
        Ok(Ok(s)) => Ok(s),
        Ok(Err(e)) => Err(anyhow::anyhow!(
            "Failed to connect to {}: {}",
            target_addr,
            e
        )),
        Err(_) => Err(anyhow::anyhow!(
            "Connection to {} timed out after {}s",
            target_addr,
            timeout_duration.as_secs()
        )),
    }
}

pub(crate) async fn run_client(options: TcpClientOptions) -> anyhow::Result<()> {
    let TcpClientOptions {
        target,
        port,
        verbose,
        tls,
        timeout_secs,
        family,
        numeric,
        duplex,
        socket,
        proxy,
        proxy_type,
        proxy_username,
        pass_fd,
    } = options;

    let addr = format_endpoint(&target, port);

    if verbose {
        let mode = if numeric { "numeric" } else { family.label() };

        eprintln!("{} [{}] Connecting to {}...", "[*]".yellow(), mode, addr);
    }

    let timeout_duration = std::time::Duration::from_secs(timeout_secs);

    if verbose && !numeric {
        eprintln!("{} Resolving address...", "[*]".yellow());
    }

    let target_addr = resolve_address(&target, port, family, timeout_duration, numeric).await?;

    if verbose {
        if numeric {
            eprintln!("{} Using numeric address: {}", "[*]".yellow(), target_addr);
        } else {
            eprintln!("{} Resolved to: {}", "[*]".yellow(), target_addr);
        }

        eprintln!("{} Connecting...", "[*]".yellow());
    }

    let stream = if let Some(proxy_addr) = proxy {
        crate::core::proxy::connect_via_proxy(crate::core::proxy::ProxyOptions {
            proxy_addr,
            proxy_type,
            proxy_username,
            target_host: target.clone(),
            target_port: port,
            timeout_duration,
            socket: socket.clone(),
        })
        .await?
    } else {
        connect_tcp(target_addr, timeout_duration, &socket).await?
    };

    if tls {
        let tls_stream = connect_client_tls(stream, &target).await?;

        if pass_fd {
            crate::common::pass_fd_and_exit(&tls_stream)?;
        }

        print_connected(verbose, duplex.crlf);

        handle_duplex(tls_stream, duplex).await
    } else {
        if pass_fd {
            crate::common::pass_fd_and_exit(&stream)?;
        }

        print_connected(verbose, duplex.crlf);
        handle_duplex(stream, duplex).await
    }
}

pub(crate) async fn run_server(options: TcpServerOptions) -> anyhow::Result<()> {
    let listener = bind_listener(options.port, options.family, &options.socket).await?;
    let (stream, remote_addr) = listener.accept().await?;

    handle_server_stream(
        stream,
        remote_addr,
        ServerStreamOptions {
            verbose: options.verbose,
            tls: options.tls,
            duplex: options.duplex,
            pass_fd: options.pass_fd,
        },
    )
    .await
}

pub(crate) async fn run_server_persistent(options: TcpServerOptions) -> anyhow::Result<()> {
    let listener = bind_listener(options.port, options.family, &options.socket).await?;
    let (input_tx, _) = broadcast::channel(16);

    if !options.duplex.no_stdin {
        spawn_stdin_forwarder(input_tx.clone(), options.duplex.crlf);
    }

    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let input_rx = input_tx.subscribe();

        if let Err(e) = handle_server_stream_with_input(
            stream,
            remote_addr,
            input_rx,
            ServerStreamOptions {
                verbose: options.verbose,
                tls: options.tls,
                duplex: options.duplex,
                pass_fd: options.pass_fd,
            },
        )
        .await
            && options.verbose
        {
            eprintln!("{} Connection closed or error: {}", "[!]".red(), e);
        }
    }
}

async fn bind_listener(
    port: u16,
    family: AddressFamily,
    options: &TcpSocketOptions,
) -> anyhow::Result<TcpListener> {
    if options.dccp {
        return Err(anyhow::anyhow!(
            "DCCP mode (-Z) is not currently supported by tokio networking."
        ));
    }

    let addr = match family {
        AddressFamily::Any | AddressFamily::Ipv4 => format!("0.0.0.0:{}", port),
        AddressFamily::Ipv6 => format!("[::]:{}", port),
    };

    let socket = if matches!(family, AddressFamily::Ipv6) {
        TcpSocket::new_v6()?
    } else {
        TcpSocket::new_v4()?
    };

    if options.debug {
        let _ = crate::common::set_socket_debug(&socket);
    }

    if let Some(size) = options.recv_bytes {
        socket.set_recv_buffer_size(size)?;
    }

    if let Some(size) = options.send_bytes {
        socket.set_send_buffer_size(size)?;
    }

    let is_ipv4 = !matches!(family, AddressFamily::Ipv6);

    if let Some(t) = options.ttl {
        let _ = crate::common::set_socket_ttl(&socket, t, is_ipv4);
    }

    if let Some(t) = options.tos {
        let _ = crate::common::set_socket_tos(&socket, t, is_ipv4);
    }

    if let Some(mttl) = options.minttl {
        let _ = crate::common::set_socket_minttl(&socket, mttl);
    }

    if options.tcp_md5sig {
        let _ = crate::common::set_socket_tcp_md5sig(&socket);
    }

    socket.set_reuseaddr(true)?;

    let local_addr = addr.parse::<SocketAddr>()?;
    socket.bind(local_addr)?;
    let listener = socket.listen(1024)?;

    eprintln!("{} Listening on {}...", "[*]".yellow(), addr);

    Ok(listener)
}

async fn handle_server_stream(
    stream: TcpStream,
    remote_addr: SocketAddr,
    options: ServerStreamOptions,
) -> anyhow::Result<()> {
    if options.verbose {
        eprintln!("{} Connection from {}", "[+]".green(), remote_addr);
    }

    if options.tls {
        let tls_stream = accept_server_tls(stream, options.verbose).await?;

        if options.pass_fd {
            crate::common::pass_fd_and_exit(&tls_stream)?;
        }

        handle_duplex(tls_stream, options.duplex).await
    } else {
        if options.pass_fd {
            crate::common::pass_fd_and_exit(&stream)?;
        }

        handle_duplex(stream, options.duplex).await
    }
}

async fn handle_server_stream_with_input(
    stream: TcpStream,
    remote_addr: SocketAddr,
    input_rx: broadcast::Receiver<Vec<u8>>,
    options: ServerStreamOptions,
) -> anyhow::Result<()> {
    if options.verbose {
        eprintln!("{} Connection from {}", "[+]".green(), remote_addr);
    }

    if options.tls {
        let tls_stream = accept_server_tls(stream, options.verbose).await?;

        if options.pass_fd {
            crate::common::pass_fd_and_exit(&tls_stream)?;
        }

        handle_duplex_with_input(tls_stream, input_rx, options.duplex).await
    } else {
        if options.pass_fd {
            crate::common::pass_fd_and_exit(&stream)?;
        }

        handle_duplex_with_input(stream, input_rx, options.duplex).await
    }
}

async fn connect_client_tls(
    stream: TcpStream,
    target: &str,
) -> anyhow::Result<tokio_rustls::client::TlsStream<TcpStream>> {
    let mut root_cert_store = RootCertStore::empty();
    let cert_result = rustls_native_certs::load_native_certs();

    root_cert_store.add_parsable_certificates(cert_result.certs);

    if let Some(cert_path) = tls::resolve_client_cert_path()? {
        let cert_file = File::open(cert_path)?;
        let mut reader = BufReader::new(cert_file);
        let certs = rustls_pemfile::certs(&mut reader).collect::<Result<Vec<_>, _>>()?;

        root_cert_store.add_parsable_certificates(certs);
    }

    let config = ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));
    let domain = ServerName::try_from(target.to_string())?;

    Ok(connector.connect(domain, stream).await?)
}

async fn accept_server_tls(
    stream: TcpStream,
    verbose: bool,
) -> anyhow::Result<tokio_rustls::server::TlsStream<TcpStream>> {
    let acceptor = TlsAcceptor::from(Arc::new(load_server_tls_config(verbose)?));

    if verbose {
        eprintln!("{} Waiting for TLS handshake...", "[*]".yellow());
    }

    let tls_stream = acceptor.accept(stream).await?;

    if verbose {
        eprintln!("{} TLS Handshake successful!", "[+]".green());
    }

    Ok(tls_stream)
}

fn load_server_tls_config(verbose: bool) -> anyhow::Result<ServerConfig> {
    if verbose {
        eprintln!("{} Loading certificates and private key...", "[*]".yellow());
    }

    let tls_paths = tls::resolve_server_tls_paths()?;
    let cert_file = File::open(tls_paths.cert)?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs = rustls_pemfile::certs(&mut cert_reader).collect::<Result<Vec<_>, _>>()?;

    let key_file = File::open(tls_paths.key)?;
    let mut key_reader = BufReader::new(key_file);
    let key = rustls_pemfile::private_key(&mut key_reader)?
        .ok_or_else(|| anyhow::anyhow!("No private key found"))?;

    Ok(ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?)
}

pub(crate) fn spawn_stdin_forwarder(input_tx: broadcast::Sender<Vec<u8>>, crlf: bool) {
    tokio::spawn(async move {
        let mut stdin = tokio::io::stdin();
        let mut buf = [0u8; 1024];

        loop {
            let n = match stdin.read(&mut buf).await {
                Ok(0) => return,
                Ok(n) => n,
                Err(_) => return,
            };

            let data = if crlf {
                convert_lf_to_crlf(&buf[..n])
            } else {
                buf[..n].to_vec()
            };

            let _ = input_tx.send(data);
        }
    });
}

fn print_connected(verbose: bool, crlf: bool) {
    if !verbose {
        return;
    }

    eprintln!(
        "{} Connected! Type your messages and press Enter.",
        "[+]".green()
    );

    if crlf {
        eprintln!(
            "{} CRLF mode active (-C): line endings typed as LF are sent as CRLF.",
            "[*]".blue()
        );
        eprintln!(
            "{} For HTTP, finish headers with an empty line.",
            "[*]".blue()
        );
    }
}
