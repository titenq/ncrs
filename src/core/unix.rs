#![cfg(unix)]

use crate::core::duplex::{handle_duplex, handle_duplex_with_input, handle_duplex_with_timeout};
use crate::core::tcp::spawn_stdin_forwarder;
use colored::*;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast;

pub async fn run_unix_client(
    path: &str,
    verbose: bool,
    crlf: bool,
    read_timeout: Option<std::time::Duration>,
    shutdown_on_eof: bool,
    no_stdin: bool,
    quit_delay: Option<i32>,
    interval: Option<u64>,
    debug: bool,
) -> anyhow::Result<()> {
    if verbose {
        eprintln!("{} Connecting to Unix socket {}...", "[*]".yellow(), path);
    }

    let stream = match UnixStream::connect(path).await {
        Ok(s) => {
            if debug {
                let _ = crate::common::set_socket_debug(&s);
            }
            s
        },
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to connect to {}: {}", path, e));
        }
    };

    if verbose {
        eprintln!("{} Connected to {}", "[+]".green(), path);
    }

    if let Some(timeout_duration) = read_timeout {
        handle_duplex_with_timeout(
            stream,
            crlf,
            timeout_duration,
            shutdown_on_eof,
            no_stdin,
            quit_delay,
            interval,
        )
        .await
    } else {
        handle_duplex(
            stream,
            crlf,
            shutdown_on_eof,
            no_stdin,
            quit_delay,
            interval,
        )
        .await
    }
}

pub async fn run_unix_server(
    path: &str,
    verbose: bool,
    crlf: bool,
    read_timeout: Option<std::time::Duration>,
    shutdown_on_eof: bool,
    no_stdin: bool,
    quit_delay: Option<i32>,
    interval: Option<u64>,
    debug: bool,
) -> anyhow::Result<()> {
    let listener = bind_unix_listener(path, debug)?;

    let (stream, _) = listener.accept().await?;

    if verbose {
        eprintln!("{} Connection received", "[+]".green());
    }

    if let Some(timeout_duration) = read_timeout {
        handle_duplex_with_timeout(
            stream,
            crlf,
            timeout_duration,
            shutdown_on_eof,
            no_stdin,
            quit_delay,
            interval,
        )
        .await
    } else {
        handle_duplex(
            stream,
            crlf,
            shutdown_on_eof,
            no_stdin,
            quit_delay,
            interval,
        )
        .await
    }
}

pub async fn run_unix_server_persistent(
    path: &str,
    verbose: bool,
    crlf: bool,
    read_timeout: Option<std::time::Duration>,
    shutdown_on_eof: bool,
    no_stdin: bool,
    quit_delay: Option<i32>,
    interval: Option<u64>,
    debug: bool,
) -> anyhow::Result<()> {
    let listener = bind_unix_listener(path, debug)?;
    let (input_tx, _) = broadcast::channel(16);
    
    if !no_stdin {
        spawn_stdin_forwarder(input_tx.clone(), crlf);
    }

    loop {
        let (stream, _) = listener.accept().await?;
        let input_rx = input_tx.subscribe();

        if verbose {
            eprintln!("{} Connection received", "[+]".green());
        }

        if let Err(e) = handle_duplex_with_input(
            stream,
            input_rx,
            read_timeout,
            shutdown_on_eof,
            quit_delay,
            interval,
        )
        .await
        {
            if verbose {
                eprintln!("{} Connection closed or error: {}", "[!]".red(), e);
            }
        }
    }
}

fn bind_unix_listener(path: &str, debug: bool) -> anyhow::Result<UnixListener> {
    if std::path::Path::new(path).exists() {
        std::fs::remove_file(path)?;
    }

    let listener = UnixListener::bind(path)?;
    if debug {
        let _ = crate::common::set_socket_debug(&listener);
    }
    eprintln!("{} Listening on {}...", "[*]".yellow(), path);
    Ok(listener)
}
