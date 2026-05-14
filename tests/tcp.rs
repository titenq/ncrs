mod support;

use std::io::Read;
use std::process::Stdio;

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
