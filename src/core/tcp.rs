use crate::core::address::{AddressFamily, format_endpoint, resolve_address};
use crate::core::duplex::{
    convert_lf_to_crlf, handle_duplex, handle_duplex_with_input, handle_duplex_with_timeout,
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

pub(crate) async fn connect_tcp(
    target_addr: SocketAddr,
    source_addr: Option<&str>,
    source_port: Option<u16>,
    timeout_duration: std::time::Duration,
    debug: bool,
    recv_bytes: Option<u32>,
    send_bytes: Option<u32>,
    ttl: Option<u32>,
    tos: Option<u8>,
    minttl: Option<u32>,
    tcp_md5sig: bool,
    dccp: bool,
) -> anyhow::Result<TcpStream> {
    if dccp {
        return Err(anyhow::anyhow!("DCCP mode (-Z) is not currently supported by tokio networking."));
    }
    
    let local_ip = match source_addr {
        Some(addr) => Some(addr.parse::<IpAddr>()?),
        None => None,
    };

    if let Some(ip) = local_ip {
        if target_addr.is_ipv4() != ip.is_ipv4() {
            return Err(anyhow::anyhow!(
                "Source address family does not match target address family"
            ));
        }
    }

    let socket = if target_addr.is_ipv4() {
        TcpSocket::new_v4()?
    } else {
        TcpSocket::new_v6()?
    };

    if debug {
        let _ = crate::common::set_socket_debug(&socket);
    }

    if let Some(size) = recv_bytes {
        socket.set_recv_buffer_size(size)?;
    }

    if let Some(size) = send_bytes {
        socket.set_send_buffer_size(size)?;
    }

    if let Some(t) = ttl {
        let _ = crate::common::set_socket_ttl(&socket, t, target_addr.is_ipv4());
    }

    if let Some(t) = tos {
        let _ = crate::common::set_socket_tos(&socket, t, target_addr.is_ipv4());
    }

    if let Some(mttl) = minttl {
        let _ = crate::common::set_socket_minttl(&socket, mttl);
    }

    if tcp_md5sig {
        let _ = crate::common::set_socket_tcp_md5sig(&socket);
    }

    if local_ip.is_some() || source_port.is_some() {
        let ip = local_ip.unwrap_or_else(|| {
            if target_addr.is_ipv4() {
                IpAddr::V4(Ipv4Addr::UNSPECIFIED)
            } else {
                IpAddr::V6(Ipv6Addr::UNSPECIFIED)
            }
        });
        let local_addr = SocketAddr::new(ip, source_port.unwrap_or(0));
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

pub async fn run_client(
    target: String,
    port: u16,
    verbose: bool,
    tls: bool,
    timeout_secs: u64,
    family: AddressFamily,
    crlf: bool,
    source_addr: Option<String>,
    source_port: Option<u16>,
    numeric: bool,
    read_timeout: Option<std::time::Duration>,
    shutdown_on_eof: bool,
    no_stdin: bool,
    quit_delay: Option<i32>,
    interval: Option<u64>,
    debug: bool,
    recv_bytes: Option<u32>,
    send_bytes: Option<u32>,
    recv_limit: Option<u32>,
    proxy: Option<String>,
    proxy_type: Option<String>,
    proxy_username: Option<String>,
    ttl: Option<u32>,
    tos: Option<u8>,
    telnet: bool,
    pass_fd: bool,
    minttl: Option<u32>,
    tcp_md5sig: bool,
    dccp: bool,
) -> anyhow::Result<()> {
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
        crate::core::proxy::connect_via_proxy(
            &proxy_addr,
            proxy_type.as_deref(),
            proxy_username.as_deref(),
            &target,
            port,
            timeout_duration,
            source_addr.as_deref(),
            source_port,
            debug,
        )
        .await?
    } else {
        connect_tcp(
            target_addr,
            source_addr.as_deref(),
            source_port,
            timeout_duration,
            debug,
            recv_bytes,
            send_bytes,
            ttl,
            tos,
            minttl,
            tcp_md5sig,
            dccp,
        )
        .await?
    };

    if tls {
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
        let domain = ServerName::try_from(target.as_str())?.to_owned();
        let tls_stream = connector.connect(domain, stream).await?;

        if pass_fd {
            crate::common::pass_fd_and_exit(&tls_stream)?;
        }

        print_connected(verbose, crlf);

        if let Some(timeout_duration) = read_timeout {
            handle_duplex_with_timeout(
                tls_stream,
                crlf,
                timeout_duration,
                shutdown_on_eof,
                no_stdin,
                quit_delay,
                interval,
                recv_limit,
                telnet,
            )
            .await
        } else {
            handle_duplex(
                tls_stream,
                crlf,
                shutdown_on_eof,
                no_stdin,
                quit_delay,
                interval,
                recv_limit,
                telnet,
            )
            .await
        }
    } else {
        if pass_fd {
            crate::common::pass_fd_and_exit(&stream)?;
        }

        print_connected(verbose, crlf);
        if let Some(timeout_duration) = read_timeout {
            handle_duplex_with_timeout(
                stream,
                crlf,
                timeout_duration,
                shutdown_on_eof,
                no_stdin,
                quit_delay,
                interval,
                recv_limit,
                telnet,
            )
            .await
        } else {
            handle_duplex(
                stream,
                crlf,
                shutdown_on_eof,
                no_stdin,
                quit_delay,
                interval,
                recv_limit,
                telnet,
            )
            .await
        }
    }
}

pub async fn run_server(
    port: u16,
    verbose: bool,
    tls: bool,
    family: AddressFamily,
    crlf: bool,
    read_timeout: Option<std::time::Duration>,
    shutdown_on_eof: bool,
    no_stdin: bool,
    quit_delay: Option<i32>,
    interval: Option<u64>,
    debug: bool,
    recv_bytes: Option<u32>,
    send_bytes: Option<u32>,
    recv_limit: Option<u32>,
    ttl: Option<u32>,
    tos: Option<u8>,
    telnet: bool,
    pass_fd: bool,
    minttl: Option<u32>,
    tcp_md5sig: bool,
    dccp: bool,
) -> anyhow::Result<()> {
    let listener = bind_listener(port, family, debug, recv_bytes, send_bytes, ttl, tos, minttl, tcp_md5sig, dccp).await?;
    let (stream, remote_addr) = listener.accept().await?;

    handle_server_stream(
        stream,
        remote_addr,
        verbose,
        tls,
        crlf,
        read_timeout,
        shutdown_on_eof,
        no_stdin,
        quit_delay,
        interval,
        recv_limit,
        telnet,
        pass_fd,
    )
    .await
}

pub async fn run_server_persistent(
    port: u16,
    verbose: bool,
    tls: bool,
    family: AddressFamily,
    crlf: bool,
    read_timeout: Option<std::time::Duration>,
    shutdown_on_eof: bool,
    no_stdin: bool,
    quit_delay: Option<i32>,
    interval: Option<u64>,
    debug: bool,
    recv_bytes: Option<u32>,
    send_bytes: Option<u32>,
    recv_limit: Option<u32>,
    ttl: Option<u32>,
    tos: Option<u8>,
    telnet: bool,
    pass_fd: bool,
    minttl: Option<u32>,
    tcp_md5sig: bool,
    dccp: bool,
) -> anyhow::Result<()> {
    let listener = bind_listener(port, family, debug, recv_bytes, send_bytes, ttl, tos, minttl, tcp_md5sig, dccp).await?;
    let (input_tx, _) = broadcast::channel(16);
    if !no_stdin {
        spawn_stdin_forwarder(input_tx.clone(), crlf);
    }

    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let input_rx = input_tx.subscribe();

        if let Err(e) = handle_server_stream_with_input(
            stream,
            remote_addr,
            verbose,
            tls,
            input_rx,
            read_timeout,
            shutdown_on_eof,
            quit_delay,
            interval,
            recv_limit,
            telnet,
            pass_fd,
        )
        .await
        {
            if verbose {
                eprintln!("{} Connection closed or error: {}", "[!]".red(), e);
            }
        }
    }
}

async fn bind_listener(
    port: u16,
    family: AddressFamily,
    debug: bool,
    recv_bytes: Option<u32>,
    send_bytes: Option<u32>,
    ttl: Option<u32>,
    tos: Option<u8>,
    minttl: Option<u32>,
    tcp_md5sig: bool,
    dccp: bool,
) -> anyhow::Result<TcpListener> {
    if dccp {
        return Err(anyhow::anyhow!("DCCP mode (-Z) is not currently supported by tokio networking."));
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

    if debug {
        let _ = crate::common::set_socket_debug(&socket);
    }

    if let Some(size) = recv_bytes {
        socket.set_recv_buffer_size(size)?;
    }

    if let Some(size) = send_bytes {
        socket.set_send_buffer_size(size)?;
    }

    let is_ipv4 = !matches!(family, AddressFamily::Ipv6);

    if let Some(t) = ttl {
        let _ = crate::common::set_socket_ttl(&socket, t, is_ipv4);
    }

    if let Some(t) = tos {
        let _ = crate::common::set_socket_tos(&socket, t, is_ipv4);
    }

    if let Some(mttl) = minttl {
        let _ = crate::common::set_socket_minttl(&socket, mttl);
    }

    if tcp_md5sig {
        let _ = crate::common::set_socket_tcp_md5sig(&socket);
    }

    socket.set_reuseaddr(true)?;

    let local_addr = addr.parse::<SocketAddr>()?;
    socket.bind(local_addr)?;
    let listener = socket.listen(1024)?;

    if debug {
        eprintln!("{} Listening on {}...", "[*]".yellow(), addr);
    } else {
        eprintln!("{} Listening on {}...", "[*]".yellow(), addr);
    }

    Ok(listener)
}

async fn handle_server_stream(
    stream: TcpStream,
    remote_addr: SocketAddr,
    verbose: bool,
    tls: bool,
    crlf: bool,
    read_timeout: Option<std::time::Duration>,
    shutdown_on_eof: bool,
    no_stdin: bool,
    quit_delay: Option<i32>,
    interval: Option<u64>,
    recv_limit: Option<u32>,
    telnet: bool,
    pass_fd: bool,
) -> anyhow::Result<()> {
    if verbose {
        eprintln!("{} Connection from {}", "[+]".green(), remote_addr);
    }

    if tls {
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

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        let acceptor = TlsAcceptor::from(Arc::new(config));

        if verbose {
            eprintln!("{} Waiting for TLS handshake...", "[*]".yellow());
        }

        let tls_stream = acceptor.accept(stream).await?;

        if verbose {
            eprintln!("{} TLS Handshake successful!", "[+]".green());
        }

        if let Some(timeout_duration) = read_timeout {
            handle_duplex_with_timeout(
                tls_stream,
                crlf,
                timeout_duration,
                shutdown_on_eof,
                no_stdin,
                quit_delay,
                interval,
                recv_limit,
                telnet,
            )
            .await
        } else {
            handle_duplex(
                tls_stream,
                crlf,
                shutdown_on_eof,
                no_stdin,
                quit_delay,
                interval,
                recv_limit,
                telnet,
            )
            .await
        }
    } else {
        if pass_fd {
            crate::common::pass_fd_and_exit(&stream)?;
        }

        if let Some(timeout_duration) = read_timeout {
            handle_duplex_with_timeout(
                stream,
                crlf,
                timeout_duration,
                shutdown_on_eof,
                no_stdin,
                quit_delay,
                interval,
                recv_limit,
                telnet,
            )
            .await
        } else {
            handle_duplex(
                stream,
                crlf,
                shutdown_on_eof,
                no_stdin,
                quit_delay,
                interval,
                recv_limit,
                telnet,
            )
            .await
        }
    }
}

async fn handle_server_stream_with_input(
    stream: TcpStream,
    remote_addr: SocketAddr,
    verbose: bool,
    tls: bool,
    input_rx: broadcast::Receiver<Vec<u8>>,
    read_timeout: Option<std::time::Duration>,
    shutdown_on_eof: bool,
    quit_delay: Option<i32>,
    interval: Option<u64>,
    recv_limit: Option<u32>,
    telnet: bool,
    pass_fd: bool,
) -> anyhow::Result<()> {
    if verbose {
        eprintln!("{} Connection from {}", "[+]".green(), remote_addr);
    }

    if tls {
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

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        let acceptor = TlsAcceptor::from(Arc::new(config));

        if verbose {
            eprintln!("{} Waiting for TLS handshake...", "[*]".yellow());
        }

        let tls_stream = acceptor.accept(stream).await?;

        if verbose {
            eprintln!("{} TLS Handshake successful!", "[+]".green());
        }

        if pass_fd {
            crate::common::pass_fd_and_exit(&tls_stream)?;
        }

        handle_duplex_with_input(
            tls_stream,
            input_rx,
            read_timeout,
            shutdown_on_eof,
            quit_delay,
            interval,
            recv_limit,
            telnet,
        )
        .await
    } else {
        if pass_fd {
            crate::common::pass_fd_and_exit(&stream)?;
        }

        handle_duplex_with_input(
            stream,
            input_rx,
            read_timeout,
            shutdown_on_eof,
            quit_delay,
            interval,
            recv_limit,
            telnet,
        )
        .await
    }
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
