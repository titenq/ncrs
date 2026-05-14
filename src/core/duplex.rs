use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::broadcast;
use tokio::sync::mpsc;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DuplexOptions {
    pub crlf: bool,
    pub read_timeout: Option<std::time::Duration>,
    pub shutdown_on_eof: bool,
    pub no_stdin: bool,
    pub quit_delay: Option<i32>,
    pub interval: Option<u64>,
    pub recv_limit: Option<u32>,
    pub telnet: bool,
}

impl DuplexOptions {
    pub fn with_input(self) -> Self {
        Self {
            no_stdin: true,
            ..self
        }
    }
}

pub(crate) async fn handle_duplex<S>(stream: S, options: DuplexOptions) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let DuplexOptions {
        crlf,
        read_timeout,
        shutdown_on_eof,
        no_stdin,
        quit_delay,
        interval,
        recv_limit,
        telnet,
    } = options;

    let (mut reader, mut writer) = io::split(stream);
    let (telnet_tx, mut telnet_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    let stdin_to_socket = tokio::spawn(async move {
        let mut stdin = io::stdin();
        let mut buf = [0u8; 1024];
        let mut stdin_eof = no_stdin;
        let mut telnet_alive = telnet;

        while !stdin_eof || telnet_alive {
            tokio::select! {
                res = stdin.read(&mut buf), if !stdin_eof => {
                    let n = match res {
                        Ok(n) => n,
                        Err(e) => return Err(e.into()),
                    };

                    if n == 0 {
                        stdin_eof = true;

                        if !telnet_alive {
                            break;
                        }
                        continue;
                    }

                    if let Some(delay) = interval {
                        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                    }

                    if crlf {
                        let data = convert_lf_to_crlf(&buf[..n]);

                        writer.write_all(&data).await?;
                    } else {
                        writer.write_all(&buf[..n]).await?;
                    }

                    writer.flush().await?;
                }
                reply = telnet_rx.recv(), if telnet_alive => {
                    match reply {
                        Some(data) => {
                            if !data.is_empty() {
                                writer.write_all(&data).await?;
                                writer.flush().await?;
                            }
                        }
                        None => {
                            telnet_alive = false;

                            if stdin_eof {
                                break;
                            }
                        }
                    }
                }
            }
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
        let mut telnet_state = TelnetState::new();
        let telnet_tx_opt = if telnet { Some(telnet_tx) } else { None };

        if let Some(timeout_duration) = read_timeout {
            copy_with_idle_timeout(
                &mut reader,
                &mut stdout,
                timeout_duration,
                interval,
                recv_limit,
                telnet_tx_opt,
                &mut telnet_state,
            )
            .await
        } else {
            if interval.is_some() || recv_limit.is_some() || telnet {
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

                    if let Some(ref tx) = telnet_tx_opt {
                        let (clean_data, replies) = telnet_state.process(&buf[..n]);

                        if !replies.is_empty() {
                            let _ = tx.send(replies);
                        }

                        if !clean_data.is_empty() {
                            stdout.write_all(&clean_data).await?;
                            stdout.flush().await?;
                            copied += clean_data.len() as u64;
                        }
                    } else {
                        stdout.write_all(&buf[..n]).await?;
                        stdout.flush().await?;
                        copied += n as u64;
                    }

                    reads += 1;

                    if let Some(limit) = recv_limit
                        && reads >= limit
                    {
                        break;
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
    telnet_tx: Option<mpsc::UnboundedSender<Vec<u8>>>,
    telnet_state: &mut TelnetState,
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

        if let Some(ref tx) = telnet_tx {
            let (clean_data, replies) = telnet_state.process(&buf[..n]);

            if !replies.is_empty() {
                let _ = tx.send(replies);
            }

            if !clean_data.is_empty() {
                writer.write_all(&clean_data).await?;
                copied += clean_data.len() as u64;
            }
        } else {
            writer.write_all(&buf[..n]).await?;

            copied += n as u64;
        }

        reads += 1;

        if let Some(limit) = recv_limit
            && reads >= limit
        {
            break;
        }
    }

    writer.flush().await?;
    
    Ok(copied)
}

pub(crate) async fn handle_duplex_with_input<S>(
    stream: S,
    mut input_rx: broadcast::Receiver<Vec<u8>>,
    options: DuplexOptions,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let DuplexOptions {
        read_timeout,
        shutdown_on_eof,
        quit_delay,
        interval,
        recv_limit,
        telnet,
        ..
    } = options.with_input();

    let (mut reader, mut writer) = io::split(stream);
    let (telnet_tx, mut telnet_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let socket_to_stdout = tokio::spawn(async move {
        let mut stdout = io::stdout();
        let mut telnet_state = TelnetState::new();
        let telnet_tx_opt = if telnet { Some(telnet_tx) } else { None };

        if interval.is_some() || recv_limit.is_some() || telnet {
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

                if let Some(ref tx) = telnet_tx_opt {
                    let (clean_data, replies) = telnet_state.process(&buf[..n]);

                    if !replies.is_empty() {
                        let _ = tx.send(replies);
                    }

                    if !clean_data.is_empty() {
                        stdout.write_all(&clean_data).await?;
                        stdout.flush().await?;
                        copied += clean_data.len() as u64;
                    }
                } else {
                    stdout.write_all(&buf[..n]).await?;
                    stdout.flush().await?;
                    copied += n as u64;
                }

                reads += 1;

                if let Some(limit) = recv_limit
                    && reads >= limit
                {
                    break;
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

            reply = telnet_rx.recv(), if telnet => {
                if let Some(data) = reply
                    && !data.is_empty()
                {
                    writer.write_all(&data).await?;
                    writer.flush().await?;
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

const IAC: u8 = 255;
const DONT: u8 = 254;
const DO: u8 = 253;
const WONT: u8 = 252;
const WILL: u8 = 251;

pub(crate) struct TelnetState {
    buf: Vec<u8>,
}

impl TelnetState {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn process(&mut self, input: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let mut clean_data = Vec::new();
        let mut replies = Vec::new();
        let mut full_input = std::mem::take(&mut self.buf);

        full_input.extend_from_slice(input);

        let mut i = 0;

        while i < full_input.len() {
            if full_input[i] == IAC {
                if i + 2 < full_input.len() {
                    let cmd = full_input[i + 1];
                    let opt = full_input[i + 2];

                    match cmd {
                        DO | WILL | DONT | WONT => {
                            let reply_cmd = if cmd == DO {
                                WONT
                            } else if cmd == WILL {
                                DONT
                            } else {
                                0
                            };

                            if reply_cmd != 0 {
                                replies.extend_from_slice(&[IAC, reply_cmd, opt]);
                            }

                            i += 3;

                            continue;
                        }
                        _ => {}
                    }
                } else {
                    self.buf.extend_from_slice(&full_input[i..]);

                    break;
                }
            }

            clean_data.push(full_input[i]);

            i += 1;
        }

        (clean_data, replies)
    }
}
