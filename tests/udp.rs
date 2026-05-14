mod support;

use std::io::Read;
use std::process::Stdio;

use support::{free_udp_port, ncrs, spawn, wait_for_stderr, write_stdin_and_close};

#[test]
fn udp_client_sends_datagram_to_listener() {
    let port = free_udp_port();

    let mut listener_cmd = ncrs();
    listener_cmd
        .arg("-u")
        .arg("-l")
        .arg(port.to_string())
        .arg("-v")
        .arg("-W")
        .arg("1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut listener = spawn(&mut listener_cmd);
    wait_for_stderr(&mut listener, "UDP listening");

    let mut client_cmd = ncrs();
    client_cmd
        .arg("-u")
        .arg("-w")
        .arg("1")
        .arg("127.0.0.1")
        .arg(port.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut client = spawn(&mut client_cmd);
    write_stdin_and_close(&mut client, b"hello udp\n");

    let mut stdout = listener.take_stdout();
    let mut buf = Vec::new();
    stdout
        .read_to_end(&mut buf)
        .expect("failed to read listener stdout");

    client.wait();
    listener.wait();

    assert_eq!(buf, b"hello udp\n");
}
