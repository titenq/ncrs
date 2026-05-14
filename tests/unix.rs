#![cfg(unix)]

mod support;

use std::io::Read;
use std::process::Stdio;

use support::{ncrs, spawn, unique_temp_path, wait_for_stderr, write_stdin_and_close};

#[test]
fn unix_client_sends_data_to_listener() {
    let socket_path = unique_temp_path("unix.sock");
    let socket_path = socket_path.to_string_lossy().into_owned();
    let _ = std::fs::remove_file(&socket_path);

    let mut listener_cmd = ncrs();
    listener_cmd
        .arg("-U")
        .arg("-l")
        .arg(&socket_path)
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
        .arg("-U")
        .arg(&socket_path)
        .arg("-N")
        .arg("-q")
        .arg("0")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut client = spawn(&mut client_cmd);
    write_stdin_and_close(&mut client, b"hello unix\n");

    let mut stdout = listener.take_stdout();
    let mut buf = Vec::new();
    stdout
        .read_to_end(&mut buf)
        .expect("failed to read listener stdout");

    client.wait();
    listener.wait();
    let _ = std::fs::remove_file(&socket_path);

    assert_eq!(buf, b"hello unix\n");
}
