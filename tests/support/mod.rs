#![allow(dead_code)]

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, UdpSocket};
use std::path::PathBuf;
use std::process::{Child, ChildStderr, Command, ExitStatus};
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn ncrs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ncrs"))
}

pub struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    pub fn take_stdin(&mut self) -> std::process::ChildStdin {
        self.child.stdin.take().expect("child stdin was not piped")
    }

    pub fn take_stdout(&mut self) -> std::process::ChildStdout {
        self.child
            .stdout
            .take()
            .expect("child stdout was not piped")
    }

    pub fn take_stderr(&mut self) -> ChildStderr {
        self.child
            .stderr
            .take()
            .expect("child stderr was not piped")
    }

    pub fn wait(&mut self) {
        let _ = self.child.wait();
    }

    pub fn wait_status(&mut self) -> ExitStatus {
        self.child.wait().expect("failed to wait for child")
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
        }

        let _ = self.child.wait();
    }
}

#[allow(clippy::zombie_processes)]
pub fn spawn(command: &mut Command) -> ChildGuard {
    ChildGuard {
        child: command.spawn().expect("failed to spawn ncrs"),
    }
}

pub fn write_stdin_and_close(child: &mut ChildGuard, data: &[u8]) {
    let mut stdin = child.take_stdin();
    stdin.write_all(data).expect("failed to write child stdin");
    stdin.flush().expect("failed to flush child stdin");
}

pub fn wait_for_stderr(child: &mut ChildGuard, needle: &str) {
    let stderr = child.take_stderr();
    let needle = needle.to_string();
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let mut found = false;

        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if !found && line.contains(&needle) {
                let _ = tx.send(());
                found = true;
            }
        }
    });

    rx.recv_timeout(Duration::from_secs(5))
        .expect("timed out waiting for child readiness message");
}

pub fn free_tcp_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .expect("failed to bind temporary TCP listener")
        .local_addr()
        .expect("temporary TCP listener has no local address")
        .port()
}

pub fn free_udp_port() -> u16 {
    UdpSocket::bind(("127.0.0.1", 0))
        .expect("failed to bind temporary UDP socket")
        .local_addr()
        .expect("temporary UDP socket has no local address")
        .port()
}

pub fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is before Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("ncrs-{name}-{}-{nanos}", std::process::id()))
}
