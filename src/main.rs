mod common;
mod core;

use clap::Parser;
use colored::*;
use figlet_rs::FIGfont;

#[derive(Parser, Debug)]
#[command(name = "ncrs", author = "TitenQ", version = "1.0.0")]
struct Args {
    target: Option<String>,
    ports: Vec<String>,

    #[arg(short = 'l', long)]
    listen: bool,

    #[arg(short = 'v', long)]
    verbose: bool,

    #[arg(short = 'z', long)]
    scan: bool,

    #[arg(short = 'p', long)]
    p_port: Option<u16>,

    #[arg(short = 's', long = "secure")]
    secure: bool,

    #[arg(short = 'w', long, default_value = "5")]
    timeout: u64,
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
    if args.scan {
        if let Some(target) = args.target {
            core::run_port_scan(target, all_ports, args.timeout, args.verbose).await?;
        }
    } else if args.listen {
        let port = args.p_port.unwrap_or(4444);
        core::run_server(port, args.verbose, args.secure).await?;
    } else if let (Some(target), Some(&port)) = (args.target, all_ports.first()) {
        core::run_client(target, port, args.verbose, args.secure).await?;
    } else {
        println!("{} Error: Usage ncrs [target] [port] or ncrs -l -p [port]", "[!]".red());
    }

    Ok(())
}
