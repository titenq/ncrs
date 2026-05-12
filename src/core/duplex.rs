use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

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
                let mut data = Vec::with_capacity(n * 2);
                let mut previous_was_cr = false;

                for &byte in &buf[..n] {
                    if byte == b'\n' && !previous_was_cr {
                        data.push(b'\r');
                    }
                    data.push(byte);
                    previous_was_cr = byte == b'\r';
                }

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
