mod support;

use std::io::Read;
use std::process::Stdio;

use support::{
    free_tcp_port, ncrs, spawn, unique_temp_path, wait_for_stderr, write_stdin_and_close,
};

#[test]
fn tls_client_sends_data_to_listener() {
    let port = free_tcp_port();
    let config_dir = unique_temp_path("tls-config");
    std::fs::create_dir_all(&config_dir).expect("failed to create test config dir");

    let tls_gen = ncrs()
        .arg("--tls-gen-force")
        .env("XDG_CONFIG_HOME", &config_dir)
        .output()
        .expect("failed to run ncrs --tls-gen-force");
    assert!(tls_gen.status.success());

    let mut listener_cmd = ncrs();
    listener_cmd
        .arg("--tls")
        .arg("-l")
        .arg(port.to_string())
        .arg("-v")
        .arg("-d")
        .arg("-W")
        .arg("1")
        .env("XDG_CONFIG_HOME", &config_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut listener = spawn(&mut listener_cmd);
    wait_for_stderr(&mut listener, "Listening on");

    let mut client_cmd = ncrs();
    client_cmd
        .arg("--tls")
        .arg("127.0.0.1")
        .arg(port.to_string())
        .arg("-N")
        .arg("-q")
        .arg("0")
        .env("XDG_CONFIG_HOME", &config_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut client = spawn(&mut client_cmd);
    write_stdin_and_close(&mut client, b"hello tls\n");

    let mut stdout = listener.take_stdout();
    let mut buf = Vec::new();
    stdout
        .read_to_end(&mut buf)
        .expect("failed to read listener stdout");

    client.wait();
    listener.wait();
    let _ = std::fs::remove_dir_all(&config_dir);

    assert_eq!(buf, b"hello tls\n");
}
