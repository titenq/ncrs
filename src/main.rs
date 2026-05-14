mod cli;
mod common;
mod core;
mod tls;

use clap::Parser;
use cli::Args;
use colored::*;
use figlet_rs::FIGfont;

fn duplex_options(
    args: &Args,
    read_timeout: Option<std::time::Duration>,
    telnet: bool,
) -> core::DuplexOptions {
    core::DuplexOptions {
        crlf: args.crlf,
        read_timeout,
        shutdown_on_eof: args.shutdown_on_eof,
        no_stdin: args.no_stdin,
        quit_delay: args.quit_delay,
        interval: args.interval,
        recv_limit: args.recv_limit,
        telnet,
    }
}

fn socket_options(args: &Args, tos: Option<u8>) -> core::TcpSocketOptions {
    core::TcpSocketOptions {
        source_addr: args.source_addr.clone(),
        source_port: args.source_port,
        debug: args.debug,
        recv_bytes: args.recv_bytes,
        send_bytes: args.send_bytes,
        ttl: args.ttl,
        tos,
        minttl: args.minttl,
        tcp_md5sig: args.tcp_md5sig,
        dccp: args.dccp,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.verbose
        && let Some(figure) = FIGfont::standard().unwrap().convert("ncrs")
    {
        eprintln!("{}", figure.to_string().cyan().bold());
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

    if args.proxy.is_some() && (args.listen || args.udp || args.unix || args.source_addr.is_some())
    {
        eprintln!(
            "{} Error: A proxy cannot be used with any of the options -l, -u, -U, or -s.",
            "[!]".red()
        );

        std::process::exit(1);
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
                eprintln!(
                    "{} Error: Invalid TOS/Traffic Class keyword or value: {}",
                    "[!]".red(),
                    tos_str
                );
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
                    core::unix::run_unix_server_persistent(core::unix::UnixOptions {
                        path: target.clone(),
                        verbose: args.verbose,
                        debug: args.debug,
                        duplex: duplex_options(&args, read_timeout, false),
                    })
                    .await?;
                } else {
                    core::unix::run_unix_server(core::unix::UnixOptions {
                        path: target.clone(),
                        verbose: args.verbose,
                        debug: args.debug,
                        duplex: duplex_options(&args, read_timeout, false),
                    })
                    .await?;
                }
            } else {
                core::unix::run_unix_client(core::unix::UnixOptions {
                    path: target,
                    verbose: args.verbose,
                    debug: args.debug,
                    duplex: duplex_options(&args, read_timeout, false),
                })
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
        let Some(target) = args.target else {
            return Err(anyhow::anyhow!("Scan mode requires a destination"));
        };

        if all_ports.is_empty() {
            return Err(anyhow::anyhow!("Scan mode requires at least one port"));
        }

        core::run_port_scan(core::ScanOptions {
            target,
            ports: all_ports,
            timeout_secs: connect_timeout,
            verbose: args.verbose,
            family,
            source_addr: args.source_addr,
            source_port: args.source_port,
            numeric: args.numeric,
            interval: args.interval,
            debug: args.debug,
        })
        .await?;
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
        core::run_udp_node(core::UdpOptions {
            target: args.target,
            port,
            listen: args.listen,
            verbose: args.verbose,
            family,
            source_addr: args.source_addr,
            source_port: args.source_port,
            numeric: args.numeric,
            read_timeout,
            broadcast: args.broadcast,
            debug: args.debug,
            recv_limit: args.recv_limit,
            ttl: args.ttl,
            tos: parsed_tos,
        })
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

            core::run_server_persistent(core::TcpServerOptions {
                port,
                verbose: args.verbose,
                tls: args.tls,
                family,
                duplex: duplex_options(&args, read_timeout, args.telnet),
                socket: socket_options(&args, parsed_tos),
                pass_fd: args.pass_fd,
            })
            .await?;
        } else {
            core::run_server(core::TcpServerOptions {
                port,
                verbose: args.verbose,
                tls: args.tls,
                family,
                duplex: duplex_options(&args, read_timeout, args.telnet),
                socket: socket_options(&args, parsed_tos),
                pass_fd: args.pass_fd,
            })
            .await?;
        }
    } else if let (Some(target), Some(&port)) = (args.target.clone(), all_ports.first()) {
        core::run_client(core::TcpClientOptions {
            target,
            port,
            verbose: args.verbose,
            tls: args.tls,
            timeout_secs: connect_timeout,
            family,
            numeric: args.numeric,
            duplex: duplex_options(&args, read_timeout, args.telnet),
            socket: socket_options(&args, parsed_tos),
            proxy: args.proxy,
            proxy_type: args.proxy_type,
            proxy_username: args.proxy_username,
            pass_fd: args.pass_fd,
        })
        .await?;
    } else {
        eprintln!(
            "{} Error: Usage ncrs [target] [port] or ncrs -l [port]",
            "[!]".red()
        );

        std::process::exit(1);
    }

    Ok(())
}
