#![cfg(unix)]

use crate::core::duplex::{DuplexOptions, handle_duplex, handle_duplex_with_input};
use crate::core::tcp::spawn_stdin_forwarder;
use colored::*;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub(crate) struct UnixOptions {
    pub path: String,
    pub verbose: bool,
    pub debug: bool,
    pub duplex: DuplexOptions,
}

pub(crate) async fn run_unix_client(options: UnixOptions) -> anyhow::Result<()> {
    if options.verbose {
        eprintln!(
            "{} Connecting to Unix socket {}...",
            "[*]".yellow(),
            options.path
        );
    }

    let stream = match UnixStream::connect(&options.path).await {
        Ok(s) => {
            if options.debug {
                let _ = crate::common::set_socket_debug(&s);
            }
            s
        }
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Failed to connect to {}: {}",
                options.path,
                e
            ));
        }
    };

    if options.verbose {
        eprintln!("{} Connected to {}", "[+]".green(), options.path);
    }

    handle_duplex(stream, options.duplex).await
}

pub(crate) async fn run_unix_server(options: UnixOptions) -> anyhow::Result<()> {
    let listener = bind_unix_listener(&options.path, options.debug)?;
    let (stream, _) = listener.accept().await?;

    if options.verbose {
        eprintln!("{} Connection received", "[+]".green());
    }

    handle_duplex(stream, options.duplex).await
}

pub(crate) async fn run_unix_server_persistent(options: UnixOptions) -> anyhow::Result<()> {
    let listener = bind_unix_listener(&options.path, options.debug)?;
    let (input_tx, _) = broadcast::channel(16);

    if !options.duplex.no_stdin {
        spawn_stdin_forwarder(input_tx.clone(), options.duplex.crlf);
    }

    loop {
        let (stream, _) = listener.accept().await?;
        let input_rx = input_tx.subscribe();

        if options.verbose {
            eprintln!("{} Connection received", "[+]".green());
        }

        if let Err(e) = handle_duplex_with_input(
            stream,
            input_rx,
            DuplexOptions {
                telnet: false,
                ..options.duplex
            },
        )
        .await
            && options.verbose
        {
            eprintln!("{} Connection closed or error: {}", "[!]".red(), e);
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
