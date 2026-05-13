use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::broadcast;

pub(crate) async fn handle_duplex<S>(
    stream: S,
    crlf: bool,
    shutdown_on_eof: bool,
    no_stdin: bool,
    quit_delay: Option<i32>,
    interval: Option<u64>,
    recv_limit: Option<u32>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    handle_duplex_inner(
        stream,
        crlf,
        None,
        shutdown_on_eof,
        no_stdin,
        quit_delay,
        interval,
        recv_limit,
    )
    .await
}

pub(crate) async fn handle_duplex_with_timeout<S>(
    stream: S,
    crlf: bool,
    timeout_duration: std::time::Duration,
    shutdown_on_eof: bool,
    no_stdin: bool,
    quit_delay: Option<i32>,
    interval: Option<u64>,
    recv_limit: Option<u32>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    handle_duplex_inner(
        stream,
        crlf,
        Some(timeout_duration),
        shutdown_on_eof,
        no_stdin,
        quit_delay,
        interval,
        recv_limit,
    )
    .await
}

async fn handle_duplex_inner<S>(
    stream: S,
    crlf: bool,
    read_timeout: Option<std::time::Duration>,
    shutdown_on_eof: bool,
    no_stdin: bool,
    quit_delay: Option<i32>,
    interval: Option<u64>,
    recv_limit: Option<u32>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut reader, mut writer) = io::split(stream);

    let stdin_to_socket = tokio::spawn(async move {
        if no_stdin {
            std::future::pending::<()>().await;
            return anyhow::Ok(());
        }

        let mut stdin = io::stdin();
        let mut buf = [0u8; 1024];

        loop {
            if let Some(delay) = interval {
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            }
            let n = stdin.read(&mut buf).await?;
            if n == 0 {
                break;
            }

            if crlf {
                let data = convert_lf_to_crlf(&buf[..n]);
                writer.write_all(&data).await?;
            } else {
                writer.write_all(&buf[..n]).await?;
            }

            writer.flush().await?;
        }

        if shutdown_on_eof {
            writer.shutdown().await?;
        }

        if let Some(delay) = quit_delay {
            if delay > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(delay as u64)).await;
            } else if delay < 0 {
                std::future::pending::<()>().await;
            }
        }

        anyhow::Ok(())
    });

    let socket_to_stdout = tokio::spawn(async move {
        let mut stdout = io::stdout();
        if let Some(timeout_duration) = read_timeout {
            copy_with_idle_timeout(&mut reader, &mut stdout, timeout_duration, interval, recv_limit).await
        } else {
            if interval.is_some() || recv_limit.is_some() {
                let mut buf = [0u8; 8192];
                let mut copied = 0;
                let mut reads = 0;
                loop {
                    if let Some(delay) = interval {
                        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                    }
                    let n = reader.read(&mut buf).await?;
                    if n == 0 {
                        break;
                    }
                    stdout.write_all(&buf[..n]).await?;
                    stdout.flush().await?;
                    copied += n as u64;
                    reads += 1;
                    if let Some(limit) = recv_limit {
                        if reads >= limit {
                            break;
                        }
                    }
                }
                Ok(copied)
            } else {
                io::copy(&mut reader, &mut stdout).await
            }
        }
    });

    tokio::pin!(stdin_to_socket);
    tokio::pin!(socket_to_stdout);

    tokio::select! {
        res = &mut socket_to_stdout => {
            res??;
            stdin_to_socket.abort();
        },
        res = &mut stdin_to_socket => {
            res??;
            if quit_delay.is_some() {
                socket_to_stdout.abort();
            } else {
                socket_to_stdout.await??;
            }
        },
    }

    Ok(())
}

async fn copy_with_idle_timeout<R, W>(
    reader: &mut R,
    writer: &mut W,
    timeout_duration: std::time::Duration,
    interval: Option<u64>,
    recv_limit: Option<u32>,
) -> std::io::Result<u64>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut buf = [0u8; 8192];
    let mut copied = 0;
    let mut reads = 0;

    loop {
        if let Some(delay) = interval {
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
        }
        let n = match tokio::time::timeout(timeout_duration, reader.read(&mut buf)).await {
            Ok(result) => result?,
            Err(_) => break,
        };

        if n == 0 {
            break;
        }

        writer.write_all(&buf[..n]).await?;
        copied += n as u64;
        reads += 1;
        if let Some(limit) = recv_limit {
            if reads >= limit {
                break;
            }
        }
    }

    writer.flush().await?;
    Ok(copied)
}

pub(crate) async fn handle_duplex_with_input<S>(
    stream: S,
    mut input_rx: broadcast::Receiver<Vec<u8>>,
    read_timeout: Option<std::time::Duration>,
    shutdown_on_eof: bool,
    quit_delay: Option<i32>,
    interval: Option<u64>,
    recv_limit: Option<u32>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut reader, mut writer) = io::split(stream);
    let socket_to_stdout = tokio::spawn(async move {
        let mut stdout = io::stdout();
        if interval.is_some() || recv_limit.is_some() {
            let mut buf = [0u8; 8192];
            let mut copied = 0;
            let mut reads = 0;
            loop {
                if let Some(delay) = interval {
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                }
                let n = reader.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                stdout.write_all(&buf[..n]).await?;
                stdout.flush().await?;
                copied += n as u64;
                reads += 1;
                if let Some(limit) = recv_limit {
                    if reads >= limit {
                        break;
                    }
                }
            }
            Ok(copied)
        } else {
            io::copy(&mut reader, &mut stdout).await
        }
    });

    tokio::pin!(socket_to_stdout);

    loop {
        tokio::select! {
            res = &mut socket_to_stdout => {
                res??;
                return Ok(());
            },
            _ = async {
                if let Some(timeout_duration) = read_timeout {
                    tokio::time::sleep(timeout_duration).await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                return Ok(());
            },
            input = input_rx.recv() => {
                match input {
                    Ok(data) => {
                        writer.write_all(&data).await?;
                        writer.flush().await?;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => {
                        if shutdown_on_eof {
                            writer.shutdown().await?;
                        }
                        if let Some(delay) = quit_delay {
                            if delay > 0 {
                                tokio::time::sleep(std::time::Duration::from_secs(delay as u64)).await;
                            } else if delay < 0 {
                                std::future::pending::<()>().await;
                            }
                            socket_to_stdout.abort();
                            return Ok(());
                        } else {
                            socket_to_stdout.await??;
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn convert_lf_to_crlf(input: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(input.len() * 2);
    let mut previous_was_cr = false;

    for &byte in input {
        if byte == b'\n' && !previous_was_cr {
            data.push(b'\r');
        }
        data.push(byte);
        previous_was_cr = byte == b'\r';
    }

    data
}
