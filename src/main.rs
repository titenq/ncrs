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

    if args.verbose {
        if let Some(figure) = FIGfont::standard().unwrap().convert("ncrs") {
            eprintln!("{}", figure.to_string().cyan().bold());
        }
    }

    if args.tls_gen || args.tls_gen_force {
        tls::generate_self_signed_cert(args.tls_gen_force)?;
        let paths = tls::config_tls_paths()?;
        eprintln!(
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

    if args.proxy.is_some() {
        if args.listen || args.udp || args.unix || args.source_addr.is_some() {
            eprintln!(
                "{} Error: A proxy cannot be used with any of the options -l, -u, -U, or -s.",
                "[!]".red()
            );
            std::process::exit(1);
        }
    }

    if args.randomize_ports {
        use rand::seq::SliceRandom;
        let mut rng = rand::rng();
        all_ports.shuffle(&mut rng);
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

    let parsed_tos = if let Some(tos_str) = &args.tos {
        match common::parse_tos(tos_str) {
            Some(t) => Some(t),
            None => {
                eprintln!("{} Error: Invalid TOS/Traffic Class keyword or value: {}", "[!]".red(), tos_str);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    if args.unix {
        #[cfg(unix)]
        {
            let target = match args.target {
                Some(ref t) => t.clone(),
                None => {
                    eprintln!(
                        "{} Error: Target path is required for Unix Sockets.",
                        "[!]".red()
                    );
                    std::process::exit(1);
                }
            };

            if args.listen {
                if args.keep_alive {
                    core::unix::run_unix_server_persistent(
                        &target,
                        args.verbose,
                        args.crlf,
                        read_timeout,
                        args.shutdown_on_eof,
                        args.no_stdin,
                        args.quit_delay,
                        args.interval,
                        args.debug,
                        args.recv_limit,
                    )
                    .await?;
                } else {
                    core::unix::run_unix_server(
                        &target,
                        args.verbose,
                        args.crlf,
                        read_timeout,
                        args.shutdown_on_eof,
                        args.no_stdin,
                        args.quit_delay,
                        args.interval,
                        args.debug,
                        args.recv_limit,
                    )
                    .await?;
                }
            } else {
                core::unix::run_unix_client(
                    &target,
                    args.verbose,
                    args.crlf,
                    read_timeout,
                    args.shutdown_on_eof,
                    args.no_stdin,
                    args.quit_delay,
                    args.interval,
                    args.debug,
                    args.recv_limit,
                )
                .await?;
            }

            return Ok(());
        }

        #[cfg(not(unix))]
        {
            eprintln!(
                "{} Error: Unix Domain Sockets (-U) are not supported on this platform.",
                "[!]".red()
            );
            std::process::exit(1);
        }
    }

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
                args.interval,
                args.debug,
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
            args.broadcast,
            args.debug,
            args.recv_limit,
            args.ttl,
            parsed_tos,
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
                eprintln!(
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
                args.shutdown_on_eof,
                args.no_stdin,
                args.quit_delay,
                args.interval,
                args.debug,
                args.recv_bytes,
                args.send_bytes,
                args.recv_limit,
                args.ttl,
                parsed_tos,
                args.telnet,
                args.pass_fd,
                args.minttl,
                args.tcp_md5sig,
                args.dccp,
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
                args.shutdown_on_eof,
                args.no_stdin,
                args.quit_delay,
                args.interval,
                args.debug,
                args.recv_bytes,
                args.send_bytes,
                args.recv_limit,
                args.ttl,
                parsed_tos,
                args.telnet,
                args.pass_fd,
                args.minttl,
                args.tcp_md5sig,
                args.dccp,
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
            args.shutdown_on_eof,
            args.no_stdin,
            args.quit_delay,
            args.interval,
            args.debug,
            args.recv_bytes,
            args.send_bytes,
            args.recv_limit,
            args.proxy,
            args.proxy_type,
            args.proxy_username,
            args.ttl,
            parsed_tos,
            args.telnet,
            args.pass_fd,
            args.minttl,
            args.tcp_md5sig,
            args.dccp,
        )
        .await?;
    } else {
        eprintln!(
            "{} Error: Usage ncrs [target] [port] or ncrs -l [port]",
            "[!]".red()
        );
    }

    Ok(())
}
