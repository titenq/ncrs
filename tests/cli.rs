use std::process::Command;

fn ncrs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ncrs"))
}

#[test]
fn prints_version() {
    let output = ncrs()
        .arg("--version")
        .output()
        .expect("failed to run ncrs --version");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn prints_help() {
    let output = ncrs()
        .arg("--help")
        .output()
        .expect("failed to run ncrs --help");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("--listen"));
    assert!(stdout.contains("--udp"));
}
#[test]
fn errors_on_missing_port_in_listen_mode() {
    let output = ncrs().arg("-l").output().expect("failed to run ncrs -l");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("requires a positional port") || stderr.contains("error:"));
}

#[test]
fn errors_on_invalid_port() {
    let output = ncrs()
        .arg("127.0.0.1")
        .arg("invalid")
        .output()
        .expect("failed to run ncrs 127.0.0.1 invalid");

    assert!(!output.status.success());
}

#[test]
fn errors_on_conflicting_ipv4_ipv6() {
    let output = ncrs()
        .arg("-4")
        .arg("-6")
        .arg("127.0.0.1")
        .arg("80")
        .output()
        .expect("failed to run ncrs -4 -6");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot be used with"));
}

#[test]
fn errors_on_scan_without_destination() {
    let output = ncrs().arg("-z").output().expect("failed to run ncrs -z");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Scan mode requires a destination"));
}

#[test]
fn errors_on_scan_without_ports() {
    let output = ncrs()
        .arg("-z")
        .arg("127.0.0.1")
        .output()
        .expect("failed to run ncrs -z 127.0.0.1");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Scan mode requires at least one port"));
}
