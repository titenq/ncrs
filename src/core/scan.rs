use crate::core::address::{AddressFamily, parse_numeric_address, resolve_address};
use crate::core::tcp::{TcpSocketOptions, connect_tcp};
use colored::*;
use std::sync::Arc;
use tokio::sync::Semaphore;

const MAX_CONCURRENT_SCANS: usize = 256;

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
    let first_port = options.ports.first().copied().unwrap_or(0);
    let timeout = std::time::Duration::from_secs(options.timeout_secs);

    let resolved_target = if options.numeric {
        parse_numeric_address(&options.target, first_port, options.family)?
    } else {
        resolve_address(
            &options.target,
            first_port,
            options.family,
            timeout,
            options.numeric,
        )
        .await?
    };

    let target = Arc::new(options.target);
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_SCANS));
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

        let permit = Arc::clone(&semaphore).acquire_owned().await?;
        let socket = TcpSocketOptions {
            source_addr: options.source_addr.clone(),
            source_port: options.source_port,
            debug: options.debug,
            ..Default::default()
        };
        let mut addr = resolved_target;
        let verbose = options.verbose;

        addr.set_port(port);

        handles.push(tokio::spawn(async move {
            let _permit = permit;

            if connect_tcp(addr, timeout, &socket).await.is_ok() && verbose {
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
