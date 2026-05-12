mod cli;
mod common;
mod core;

use clap::Parser;
use cli::Args;
use colored::*;
use figlet_rs::FIGfont;

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
            core::run_port_scan(
                target,
                all_ports,
                args.timeout,
                args.verbose,
                family,
                args.source_addr,
                args.p_port,
            )
            .await?;
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
                    core::run_server(port, args.verbose, args.tls, family, args.crlf).await
                {
                    if args.verbose {
                        eprintln!("{} Connection closed or error: {}", "[!]".red(), e);
                    }
                }

                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        } else {
            core::run_server(port, args.verbose, args.tls, family, args.crlf).await?;
        }
    } else if let (Some(target), Some(&port)) = (args.target, all_ports.first()) {
        core::run_client(
            target,
            port,
            args.verbose,
            args.tls,
            args.timeout,
            family,
            args.crlf,
            args.source_addr,
            args.p_port,
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
