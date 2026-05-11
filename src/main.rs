use clap::Parser;
use colored::*;
use figlet_rs::FIGfont;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio::io::{self, AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::pki_types::CertificateDer;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};

#[derive(Parser, Debug)]
#[command(
    name = "ncrs",
    author = "TitenQ <titenq@gmail.com> (titenq.com.br)",
    version = "0.1.0",
    about = "Netcat in Rust focusing on performance and security",
    long_about = "A networking utility inspired by classic Netcat, built with the Tokio runtime."
)]
struct Args {
    /// Target IP address or Hostname
    target: Option<String>,

    /// Port to connect to or listen on
    port: Option<u16>,

    /// [Flag: -l] Listen mode: wait for incoming connections
    #[arg(short = 'l', long)]
    listen: bool,

    /// [Flag: -v] Verbose mode: display operation details
    #[arg(short = 'v', long)]
    verbose: bool,

    /// [Flag: -p] Local port (used in listen mode)
    #[arg(short = 'p', long)]
    p_port: Option<u16>,

    /// [Flag: -s] Secure mode: Enable TLS encryption
    #[arg(short = 's', long = "secure")]
    secure: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let font = FIGfont::standard().unwrap();
    if let Some(figure) = font.convert("ncrs") {
        println!("{}", figure.to_string().cyan().bold());
    }
    println!(
        "{} v{}\n",
        "Netcat Rust Edition".bright_black(),
        env!("CARGO_PKG_VERSION")
    );

    let final_port = args.p_port.or(args.port).unwrap_or(4444);

    if args.verbose {
        println!("{} Configured port: {}", "[*]".yellow(), final_port);
    }

    if args.listen {
        run_server(final_port, args.verbose, args.secure).await
    } else if let Some(target_ip) = args.target {
        run_client(target_ip, final_port, args.verbose, args.secure).await
    } else {
        println!(
            "{} Error: Define a target or use -l to listen.",
            "[!]".red()
        );
        Ok(())
    }
}

async fn run_client(target: String, port: u16, verbose: bool, secure: bool) -> anyhow::Result<()> {
    let addr = format!("{}:{}", target, port);

    if verbose {
        println!("{} Connecting to {}...", "[*]".yellow(), addr);
    }

    let stream = TcpStream::connect(&addr).await?;

    if secure {
        let mut root_cert_store = RootCertStore::empty();
        let cert_result = rustls_native_certs::load_native_certs();

        if verbose && !cert_result.errors.is_empty() {
            println!(
                "{} Warning: Some native certificates could not be loaded.",
                "[!]".yellow()
            );
        }

        root_cert_store.add_parsable_certificates(cert_result.certs);

        if std::path::Path::new("cert.pem").exists() {
            let cert_file = std::fs::File::open("cert.pem")?;
            let mut reader = std::io::BufReader::new(cert_file);
            let certs = rustls_pemfile::certs(&mut reader).collect::<Result<Vec<_>, _>>()?;
            root_cert_store.add_parsable_certificates(certs);
        }

        let config = ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(config));

        let domain = ServerName::try_from(target.as_str())
            .map_err(|_| anyhow::anyhow!("Invalid DNS name"))?
            .to_owned();

        let tls_stream = connector.connect(domain, stream).await?;

        if verbose {
            println!("{} TLS Connection established!", "[+]".green());
        }

        handle_duplex(tls_stream).await
    } else {
        if verbose {
            println!("{} Connected to {} (Insecure)", "[+]".green(), addr);
        }

        handle_duplex(stream).await
    }
}

async fn run_server(port: u16, verbose: bool, secure: bool) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;

    println!("{} Listening on {}...", "[*]".yellow(), addr);

    let (stream, remote_addr) = listener.accept().await?;

    if verbose {
        println!("{} Connection received from {}", "[+]".green(), remote_addr);
    }

    if secure {
        // 1. Load Certificate
        let cert_file = File::open("cert.pem")?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs: Vec<CertificateDer> =
            rustls_pemfile::certs(&mut cert_reader).collect::<Result<Vec<_>, _>>()?;

        // 2. Load Private Key
        let key_file = File::open("key.pem")?;
        let mut key_reader = BufReader::new(key_file);
        let key = rustls_pemfile::private_key(&mut key_reader)?
            .ok_or_else(|| anyhow::anyhow!("No private key found in key.pem"))?;

        // 3. Setup TLS Acceptor
        let config = tokio_rustls::rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        let acceptor = TlsAcceptor::from(Arc::new(config));

        // 4. Perform TLS Handshake
        let tls_stream = acceptor.accept(stream).await?;

        if verbose {
            println!(
                "{} TLS Handshake successful with {}",
                "[+]".green(),
                remote_addr
            );
        }

        handle_duplex(tls_stream).await
    } else {
        handle_duplex(stream).await
    }
}

async fn handle_duplex<S>(stream: S) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut reader, mut writer) = io::split(stream);

    let stdin_to_socket = tokio::spawn(async move {
        let mut stdin = io::stdin();
        io::copy(&mut stdin, &mut writer).await
    });

    let socket_to_stdout = tokio::spawn(async move {
        let mut stdout = io::stdout();
        io::copy(&mut reader, &mut stdout).await
    });

    tokio::select! {
        res = stdin_to_socket => { res??; },
        res = socket_to_stdout => { res??; },
    }

    Ok(())
}
