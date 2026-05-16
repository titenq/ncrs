mod support;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Stdio;
use std::sync::mpsc;
use std::time::Duration;

use support::{free_tcp_port, ncrs, spawn, wait_for_stderr, write_stdin_and_close};

#[test]
fn tcp_client_sends_data_to_listener() {
    let port = free_tcp_port();

    let mut listener_cmd = ncrs();
    listener_cmd
        .arg("-l")
        .arg(port.to_string())
        .arg("-v")
        .arg("-d")
        .arg("-W")
        .arg("1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut listener = spawn(&mut listener_cmd);
    wait_for_stderr(&mut listener, "Listening on");

    let mut client_cmd = ncrs();
    client_cmd
        .arg("127.0.0.1")
        .arg(port.to_string())
        .arg("-N")
        .arg("-q")
        .arg("0")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut client = spawn(&mut client_cmd);
    write_stdin_and_close(&mut client, b"hello from client\n");

    let mut stdout = listener.take_stdout();
    let mut buf = Vec::new();
    stdout
        .read_to_end(&mut buf)
        .expect("failed to read listener stdout");

    client.wait();
    listener.wait();

    assert_eq!(buf, b"hello from client\n");
}

#[test]
fn scan_finds_open_port() {
    let port = free_tcp_port();

    let mut listener_cmd = ncrs();
    listener_cmd
        .arg("-l")
        .arg(port.to_string())
        .arg("-v")
        .arg("-d")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let mut listener = spawn(&mut listener_cmd);
    wait_for_stderr(&mut listener, "Listening on");

    let output = ncrs()
        .arg("-z")
        .arg("127.0.0.1")
        .arg(port.to_string())
        .arg("-v")
        .output()
        .expect("failed to run scan");

    listener.wait();

    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("open"));
}

#[test]
fn crlf_mode_converts_lf_before_sending() {
    let port = free_tcp_port();

    let mut listener_cmd = ncrs();
    listener_cmd
        .arg("-l")
        .arg(port.to_string())
        .arg("-d")
        .arg("-W")
        .arg("1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut listener = spawn(&mut listener_cmd);
    wait_for_stderr(&mut listener, "Listening on");

    let mut client_cmd = ncrs();
    client_cmd
        .arg("127.0.0.1")
        .arg(port.to_string())
        .arg("-C")
        .arg("-N")
        .arg("-q")
        .arg("0")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut client = spawn(&mut client_cmd);
    write_stdin_and_close(&mut client, b"line one\nline two\n");

    let mut stdout = listener.take_stdout();
    let mut buf = Vec::new();
    stdout
        .read_to_end(&mut buf)
        .expect("failed to read listener stdout");

    client.wait();
    listener.wait();

    assert_eq!(buf, b"line one\r\nline two\r\n");
}

#[test]
fn telnet_mode_rejects_do_and_will_negotiations() {
    let server = TcpListener::bind(("127.0.0.1", 0)).expect("failed to bind telnet test server");
    let port = server.local_addr().expect("missing local addr").port();
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let (mut stream, _) = server.accept().expect("failed to accept telnet client");

        stream
            .write_all(&[255, 253, 1, 255, 251, 3])
            .expect("failed to write telnet negotiation");

        let mut buf = [0u8; 6];

        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("failed to set read timeout");
        stream
            .read_exact(&mut buf)
            .expect("failed to read telnet response");

        tx.send(buf).expect("failed to send telnet response");
    });

    let output = ncrs()
        .arg("127.0.0.1")
        .arg(port.to_string())
        .arg("-t")
        .arg("-d")
        .arg("-w")
        .arg("1")
        .output()
        .expect("failed to run telnet client");

    assert!(output.status.success());

    let replies = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("timed out waiting for telnet replies");

    assert_eq!(replies, [255, 252, 1, 255, 254, 3]);
}

#[test]
fn persistent_listener_accepts_sequential_clients() {
    let port = free_tcp_port();

    let mut listener_cmd = ncrs();
    listener_cmd
        .arg("-l")
        .arg("-k")
        .arg(port.to_string())
        .arg("-v")
        .arg("-d")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut listener = spawn(&mut listener_cmd);
    wait_for_stderr(&mut listener, "Listening on");

    run_client_with_input(port, b"first\n");
    run_client_with_input(port, b"second\n");

    listener.kill();

    let mut stdout = listener.take_stdout();
    let mut buf = Vec::new();
    stdout
        .read_to_end(&mut buf)
        .expect("failed to read listener stdout");

    let stdout = String::from_utf8_lossy(&buf);

    assert!(stdout.contains("first\n"));
    assert!(stdout.contains("second\n"));
}

#[test]
fn socks5_proxy_forwards_client_data() {
    proxy_forwards_client_data(ProxyKind::Socks5);
}

#[test]
fn http_connect_proxy_forwards_client_data() {
    proxy_forwards_client_data(ProxyKind::HttpConnect);
}

fn run_client_with_input(port: u16, input: &[u8]) {
    let mut client_cmd = ncrs();
    client_cmd
        .arg("127.0.0.1")
        .arg(port.to_string())
        .arg("-N")
        .arg("-q")
        .arg("0")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut client = spawn(&mut client_cmd);

    write_stdin_and_close(&mut client, input);
    client.wait();
}

#[derive(Clone, Copy)]
enum ProxyKind {
    Socks5,
    HttpConnect,
}

fn proxy_forwards_client_data(kind: ProxyKind) {
    let target = TcpListener::bind(("127.0.0.1", 0)).expect("failed to bind target server");
    let target_port = target.local_addr().expect("missing target addr").port();
    let proxy = TcpListener::bind(("127.0.0.1", 0)).expect("failed to bind proxy server");
    let proxy_port = proxy.local_addr().expect("missing proxy addr").port();
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let (mut target_stream, _) = target.accept().expect("failed to accept target client");
        let mut buf = Vec::new();

        target_stream
            .read_to_end(&mut buf)
            .expect("failed to read proxied data");

        tx.send(buf).expect("failed to send proxied data");
    });

    std::thread::spawn(move || {
        let (mut client, _) = proxy.accept().expect("failed to accept proxy client");

        match kind {
            ProxyKind::Socks5 => handle_socks5_proxy(&mut client),
            ProxyKind::HttpConnect => handle_http_connect_proxy(&mut client),
        }

        let mut target_stream =
            TcpStream::connect(("127.0.0.1", target_port)).expect("failed to connect to target");

        std::io::copy(&mut client, &mut target_stream).expect("failed to relay proxy data");
    });

    let mut client_cmd = ncrs();
    client_cmd
        .arg("127.0.0.1")
        .arg(target_port.to_string())
        .arg("-x")
        .arg(format!("127.0.0.1:{proxy_port}"))
        .arg("-X")
        .arg(match kind {
            ProxyKind::Socks5 => "5",
            ProxyKind::HttpConnect => "connect",
        })
        .arg("-N")
        .arg("-q")
        .arg("0")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut client = spawn(&mut client_cmd);

    write_stdin_and_close(&mut client, b"proxied payload\n");
    client.wait();

    let received = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("timed out waiting for proxied data");

    assert_eq!(received, b"proxied payload\n");
}

fn handle_socks5_proxy(client: &mut TcpStream) {
    let mut greeting = [0u8; 3];

    client
        .read_exact(&mut greeting)
        .expect("failed to read SOCKS5 greeting");

    assert_eq!(greeting, [5, 1, 0]);

    client
        .write_all(&[5, 0])
        .expect("failed to write SOCKS5 greeting response");

    let mut header = [0u8; 4];

    client
        .read_exact(&mut header)
        .expect("failed to read SOCKS5 request header");

    assert_eq!(&header[..3], &[5, 1, 0]);

    match header[3] {
        1 => {
            let mut addr = [0u8; 4];
            client
                .read_exact(&mut addr)
                .expect("failed to read SOCKS5 IPv4 address");
        }
        3 => {
            let mut len = [0u8; 1];
            client
                .read_exact(&mut len)
                .expect("failed to read SOCKS5 hostname length");

            let mut host = vec![0u8; len[0] as usize];
            client
                .read_exact(&mut host)
                .expect("failed to read SOCKS5 hostname");
        }
        4 => {
            let mut addr = [0u8; 16];
            client
                .read_exact(&mut addr)
                .expect("failed to read SOCKS5 IPv6 address");
        }
        _ => panic!("unexpected SOCKS5 address type"),
    }

    let mut port = [0u8; 2];

    client
        .read_exact(&mut port)
        .expect("failed to read SOCKS5 target port");

    client
        .write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 0])
        .expect("failed to write SOCKS5 success response");
}

fn handle_http_connect_proxy(client: &mut TcpStream) {
    let mut request = Vec::new();
    let mut byte = [0u8; 1];

    loop {
        client
            .read_exact(&mut byte)
            .expect("failed to read HTTP CONNECT request");
        request.push(byte[0]);

        if request.ends_with(b"\r\n\r\n") {
            break;
        }
    }

    let request = String::from_utf8_lossy(&request);

    assert!(request.starts_with("CONNECT 127.0.0.1:"));

    client
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .expect("failed to write HTTP CONNECT response");
}
