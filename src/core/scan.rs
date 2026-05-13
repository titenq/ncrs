use crate::core::address::{AddressFamily, parse_numeric_address, resolve_address};
use crate::core::tcp::connect_tcp;
use colored::*;
use std::sync::Arc;

pub async fn run_port_scan(
    target: String,
    ports: Vec<u16>,
    timeout_secs: u64,
    verbose: bool,
    family: AddressFamily,
    source_addr: Option<String>,
    source_port: Option<u16>,
    numeric: bool,
) -> anyhow::Result<()> {
    if numeric {
        let port = ports.first().copied().unwrap_or(0);
        parse_numeric_address(&target, port, family)?;
    }

    let target = Arc::new(target);
    let mut handles = vec![];

    if verbose {
        println!(
            "{} Scanning {} ports on {}...",
            "[*]".yellow(),
            ports.len(),
            target
        );
    }

    for port in ports {
        let t = Arc::clone(&target);
        let source_addr = source_addr.clone();
        handles.push(tokio::spawn(async move {
            let timeout = std::time::Duration::from_secs(timeout_secs);
            if let Ok(addr) = resolve_address(&t, port, family, timeout, numeric).await {
                if connect_tcp(addr, source_addr.as_deref(), source_port, timeout)
                    .await
                    .is_ok()
                {
                    if verbose {
                        println!("{} port {} open", "Connection to".green(), port);
                    }
                }
            }
        }));
    }

    for h in handles {
        let _ = h.await;
    }

    if verbose {
        println!("{} Scan complete.", "[*]".yellow());
    }

    Ok(())
}
