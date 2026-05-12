mod cli;
mod common;
mod core;
mod tls;

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

    if args.tls_gen || args.tls_gen_force {
        tls::generate_self_signed_cert(args.tls_gen_force)?;
        let paths = tls::config_tls_paths()?;
        println!(
            "{} Generated {} and {}",
            "[+]".green(),
            paths.cert.display(),
            paths.key.display()
        );
        return Ok(());
    }

    let mut all_ports = vec![];

    for p_arg in &args.ports {
        all_ports.extend(common::parse_port_range(p_arg));
    }

    let target_as_port = args
        .target
        .as_deref()
        .and_then(|target| target.parse::<u16>().ok());

    let family = if args.ipv4 {
        core::AddressFamily::Ipv4
    } else if args.ipv6 {
        core::AddressFamily::Ipv6
    } else {
        core::AddressFamily::Any
    };
    let connect_timeout = args.timeout.unwrap_or(5);
    let read_timeout = args.timeout.map(std::time::Duration::from_secs);

    if args.scan {
        if let Some(target) = args.target {
            core::run_port_scan(
                target,
                all_ports,
                connect_timeout,
                args.verbose,
                family,
                args.source_addr,
                args.source_port,
                args.numeric,
            )
            .await?;
        }
    } else if args.udp {
        let port = if args.listen {
            match all_ports.first().copied().or(target_as_port) {
                Some(port) => port,
                None => {
                    return Err(anyhow::anyhow!(
                        "Listen mode requires a positional port; -p is the source port for outbound connections"
                    ));
                }
            }
        } else {
            all_ports.first().copied().unwrap_or(4444)
        };
        core::run_udp_node(
            args.target,
            port,
            args.listen,
            args.verbose,
            family,
            args.source_addr,
            args.source_port,
            args.numeric,
            read_timeout,
        )
        .await?;
    } else if args.listen {
        let port = match all_ports.first().copied().or(target_as_port) {
            Some(port) => port,
            None => {
                return Err(anyhow::anyhow!(
                    "Listen mode requires a positional port; -p is the source port for outbound connections"
                ));
            }
        };

        if args.keep_alive {
            if args.verbose {
                println!(
                    "{} Persistent mode active (-k). Listener will keep accepting connections.",
                    "[*]".blue()
                );
            }
            core::run_server_persistent(
                port,
                args.verbose,
                args.tls,
                family,
                args.crlf,
                read_timeout,
            )
            .await?;
        } else {
            core::run_server(
                port,
                args.verbose,
                args.tls,
                family,
                args.crlf,
                read_timeout,
            )
            .await?;
        }
    } else if let (Some(target), Some(&port)) = (args.target, all_ports.first()) {
        core::run_client(
            target,
            port,
            args.verbose,
            args.tls,
            connect_timeout,
            family,
            args.crlf,
            args.source_addr,
            args.source_port,
            args.numeric,
            read_timeout,
        )
        .await?;
    } else {
        println!(
            "{} Error: Usage ncrs [target] [port] or ncrs -l [port]",
            "[!]".red()
        );
    }

    Ok(())
}
