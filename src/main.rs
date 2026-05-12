mod common;
mod core;

use clap::Parser;
use colored::*;
use figlet_rs::FIGfont;

#[derive(Parser, Debug)]
#[command(name = "ncrs", author = "TitenQ", version = "0.1.0")]
struct Args {
    /// Target IP address or Hostname
    target: Option<String>,

    /// Port(s) to connect to (e.g., 80 or 20-100)
    ports: Vec<String>,

    /// [Flag: -l] Listen mode: waits for incoming connections
    #[arg(short = 'l', long)]
    listen: bool,

    /// [Flag: -v] Verbose mode: prints detailed connection info
    #[arg(short = 'v', long)]
    verbose: bool,

    /// [Flag: -z] Zero-I/O mode: used for port scanning
    #[arg(short = 'z', long)]
    scan: bool,

    /// [Flag: -p] Source port: specific port to bind to in listen mode
    #[arg(short = 'p', long)]
    p_port: Option<u16>,

    /// [Flag: -s] Secure mode: uses TLS for the connection
    #[arg(short = 's', long = "secure")]
    secure: bool,

    /// [Flag: -w] Connection timeout: maximum seconds to wait for a response
    #[arg(short = 'w', long, default_value = "5")]
    timeout: u64,

    /// [Flag: -u] UDP mode: uses UDP instead of the default TCP
    #[arg(short = 'u', long)]
    udp: bool,

    /// [Flag: -4] IPv4 mode: force usage of IPv4 addresses
    #[arg(short = '4', long, conflicts_with = "ipv6")]
    ipv4: bool,

    /// [Flag: -6] IPv6 mode: force usage of IPv6 addresses
    #[arg(short = '6', long)]
    ipv6: bool,

    /// [Flag: -C] Send CRLF as line-ending instead of just LF
    #[arg(short = 'C', long)]
    crlf: bool,

    /// [Flag: -k] Keep-alive: accept multiple connections in listen mode
    #[arg(short = 'k', long)]
    keep_alive: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if let Some(figure) = FIGfont::standard().unwrap().convert("ncrs") {
        println!("{}", figure.to_string().cyan().bold());
    }

    let mut all_ports = vec![];

    for p_arg in &args.ports {
        all_ports.extend(common::parse_port_range(p_arg));
    }

    let family = if args.ipv4 {
        core::AddressFamily::Ipv4
    } else if args.ipv6 {
        core::AddressFamily::Ipv6
    } else {
        core::AddressFamily::Any
    };

    if args.scan {
        if let Some(target) = args.target {
            core::run_port_scan(target, all_ports, args.timeout, args.verbose, family).await?;
        }
    } else if args.udp {
        let port = args
            .p_port
            .unwrap_or_else(|| all_ports.first().copied().unwrap_or(4444));
        core::run_udp_node(args.target, port, args.listen, args.verbose, family).await?;
    } else if args.listen {
        let port = args.p_port.unwrap_or(4444);

        if args.keep_alive {
            if args.verbose {
                println!(
                    "{} Persistent mode active (-k). Server will restart after logout.",
                    "[*]".blue()
                );
            }
            loop {
                if let Err(e) =
                    core::run_server(port, args.verbose, args.secure, family, args.crlf).await
                {
                    if args.verbose {
                        eprintln!("{} Connection closed or error: {}", "[!]".red(), e);
                    }
                }

                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        } else {
            core::run_server(port, args.verbose, args.secure, family, args.crlf).await?;
        }
    } else if let (Some(target), Some(&port)) = (args.target, all_ports.first()) {
        core::run_client(
            target,
            port,
            args.verbose,
            args.secure,
            args.timeout,
            family,
            args.crlf,
        )
        .await?;
    } else {
        println!(
            "{} Error: Usage ncrs [target] [port] or ncrs -l -p [port]",
            "[!]".red()
        );
    }

    Ok(())
}
