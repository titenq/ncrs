use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::broadcast;

pub(crate) async fn handle_duplex<S>(stream: S, crlf: bool) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut reader, mut writer) = io::split(stream);

    let stdin_to_socket = tokio::spawn(async move {
        let mut stdin = io::stdin();
        let mut buf = [0u8; 1024];

        loop {
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

        writer.shutdown().await?;
        anyhow::Ok(())
    });

    let socket_to_stdout = tokio::spawn(async move {
        let mut stdout = io::stdout();
        io::copy(&mut reader, &mut stdout).await
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
            socket_to_stdout.await??;
        },
    }

    Ok(())
}

pub(crate) async fn handle_duplex_with_input<S>(
    stream: S,
    mut input_rx: broadcast::Receiver<Vec<u8>>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut reader, mut writer) = io::split(stream);
    let socket_to_stdout = tokio::spawn(async move {
        let mut stdout = io::stdout();
        io::copy(&mut reader, &mut stdout).await
    });

    tokio::pin!(socket_to_stdout);

    loop {
        tokio::select! {
            res = &mut socket_to_stdout => {
                res??;
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
                        writer.shutdown().await?;
                        socket_to_stdout.await??;
                        return Ok(());
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
