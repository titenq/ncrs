use crate::core::address::{AddressFamily, parse_numeric_address, resolve_address};
use crate::core::tcp::{TcpSocketOptions, connect_tcp};
use colored::*;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(crate) struct ScanOptions {
    pub target: String,
    pub ports: Vec<u16>,
    pub timeout_secs: u64,
    pub verbose: bool,
    pub family: AddressFamily,
    pub source_addr: Option<String>,
    pub source_port: Option<u16>,
    pub numeric: bool,
    pub interval: Option<u64>,
    pub debug: bool,
}

pub(crate) async fn run_port_scan(options: ScanOptions) -> anyhow::Result<()> {
    if options.numeric {
        let port = options.ports.first().copied().unwrap_or(0);

        parse_numeric_address(&options.target, port, options.family)?;
    }

    let target = Arc::new(options.target);
    let mut handles = vec![];

    if options.verbose {
        eprintln!(
            "{} Scanning {} ports on {}...",
            "[*]".yellow(),
            options.ports.len(),
            target
        );
    }

    for port in options.ports {
        if let Some(delay) = options.interval {
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
        }

        let t = Arc::clone(&target);
        let socket = TcpSocketOptions {
            source_addr: options.source_addr.clone(),
            source_port: options.source_port,
            debug: options.debug,
            ..Default::default()
        };
        let family = options.family;
        let numeric = options.numeric;
        let timeout_secs = options.timeout_secs;
        let verbose = options.verbose;

        handles.push(tokio::spawn(async move {
            let timeout = std::time::Duration::from_secs(timeout_secs);

            if let Ok(addr) = resolve_address(&t, port, family, timeout, numeric).await
                && connect_tcp(addr, timeout, &socket).await.is_ok()
                && verbose
            {
                eprintln!("{} port {} open", "Connection to".green(), port);
            }
        }));
    }

    for h in handles {
        let _ = h.await;
    }

    if options.verbose {
        eprintln!("{} Scan complete.", "[*]".yellow());
    }

    Ok(())
}
