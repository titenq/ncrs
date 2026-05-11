use colored::*;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio::io::{self, AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore, ServerConfig};
use tokio_rustls::{TlsAcceptor, TlsConnector};

pub async fn run_client(
    target: String,
    port: u16,
    verbose: bool,
    secure: bool,
) -> anyhow::Result<()> {
    let addr = format!("{}:{}", target, port);
    if verbose {
        println!("{} Connecting to {}...", "[*]".yellow(), addr);
    }

    let stream = TcpStream::connect(&addr).await?;

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

        handle_duplex(tls_stream).await
    } else {
        handle_duplex(stream).await
    }
}

pub async fn run_server(port: u16, verbose: bool, secure: bool) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    println!("{} Listening on {}...", "[*]".yellow(), addr);

    let (stream, remote_addr) = listener.accept().await?;
    if verbose {
        println!("{} Connection from {}", "[+]".green(), remote_addr);
    }

    if secure {
        if verbose { println!("{} Loading certificates and private key...", "[*]".yellow()); }
        
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
        
        if verbose { println!("{} Waiting for TLS handshake...", "[*]".yellow()); }
        let tls_stream = acceptor.accept(stream).await?;
        
        if verbose { println!("{} TLS Handshake successful!", "[+]".green()); }
        
        handle_duplex(tls_stream).await
    } else {
        handle_duplex(stream).await
    }
}

pub async fn run_port_scan(target: String, ports: Vec<u16>, timeout_secs: u64, verbose: bool) -> anyhow::Result<()> {
    let target = Arc::new(target);
    let mut handles = vec![];

    if verbose {
        println!("{} Scanning {} ports on {}...", "[*]".yellow(), ports.len(), target);
    }

    for port in ports {
        let t = Arc::clone(&target);
        handles.push(tokio::spawn(async move {
            let addr = format!("{}:{}", t, port);
            let timeout = std::time::Duration::from_secs(timeout_secs);
            if let Ok(Ok(_)) = tokio::time::timeout(timeout, TcpStream::connect(&addr)).await {
                println!("{} port {} open", "Connection to".green(), port);
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

async fn handle_duplex<S>(stream: S) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut reader, mut writer) = io::split(stream);
    let t1 = tokio::spawn(async move { io::copy(&mut io::stdin(), &mut writer).await });
    let t2 = tokio::spawn(async move { io::copy(&mut reader, &mut io::stdout()).await });
    tokio::select! {
        res = t1 => { res??; },
        res = t2 => { res??; },
    }
    Ok(())
}
