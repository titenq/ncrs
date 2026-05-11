mod common;
mod core;

use clap::Parser;
use colored::*;
use figlet_rs::FIGfont;

#[derive(Parser, Debug)]
#[command(name = "ncrs", author = "TitenQ", version = "1.0.0")]
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Banner
    if let Some(figure) = FIGfont::standard().unwrap().convert("ncrs") {
        println!("{}", figure.to_string().cyan().bold());
    }

    // Port Parsing
    let mut all_ports = vec![];
    for p_arg in &args.ports {
        all_ports.extend(common::parse_port_range(p_arg));
    }

    // Logic Execution
    // Logic Execution
    if args.scan {
        if let Some(target) = args.target {
            core::run_port_scan(target, all_ports, args.timeout, args.verbose).await?;
        }
    } else if args.udp {
        // UDP
        let port = args.p_port.unwrap_or_else(|| {
            all_ports.first().copied().unwrap_or(4444)
        });
        core::run_udp_node(args.target, port, args.listen, args.verbose).await?;
    } else if args.listen {
        // Listen
        let port = args.p_port.unwrap_or(4444);
        core::run_server(port, args.verbose, args.secure).await?;
    } else if let (Some(target), Some(&port)) = (args.target, all_ports.first()) {
        // TCP Client
        core::run_client(target, port, args.verbose, args.secure).await?;
    } else {
        println!("{} Error: Usage ncrs [target] [port] or ncrs -l -p [port]", "[!]".red());
    }

    Ok(())
}
