# ncrs - Netcat Rust Edition

**ncrs** is a Rust implementation inspired by Netcat. It currently provides TCP client/server mode, UDP mode, asynchronous full-duplex I/O, basic port scanning, IPv6 support, CRLF line-ending conversion, persistent listen mode, and optional TLS.

The project is not yet a drop-in replacement for OpenBSD `nc`. Some flags have different meanings today, and several OpenBSD `nc` options are still not implemented.

Developed by **TitenQ** | [titenq.com.br](https://titenq.com.br) | [titenq@gmail.com](mailto:titenq@gmail.com)

## Features

- TCP client and listen modes.
- UDP client and listen modes.
- Full-duplex I/O with Tokio.
- Port scanning with `-z`, including port ranges such as `20-100`.
- Connection timeout with `-w`.
- IPv4 mode with `-4`.
- IPv6 mode with `-6`.
- CRLF conversion with `-C`.
- Persistent listen mode with `-k`.
- Local source address selection with `-s`.
- Optional TLS mode with `--tls`.

## Compatibility Notes

`ncrs` is currently compatible with only a subset of OpenBSD `nc`.

Implemented flags:

```text
-4 -6 -C -k -l -p -s -u -v -w -z
```

Important differences from OpenBSD `nc`:

- `--tls` is an `ncrs` extension and is not an OpenBSD `nc` flag.
- In OpenBSD `nc`, `-p` is the local source port for outbound connections. In `ncrs`, `-p` still also acts as the listen port when `-l` is used.
- `ncrs -k` accepts multiple sequential inbound connections, but it currently handles one active TCP connection at a time.
- OpenBSD options such as `-b`, `-D`, `-d`, `-F`, `-h`, `-I`, `-i`, `-M`, `-m`, `-N`, `-n`, `-O`, `-P`, `-q`, `-r`, `-S`, `-T`, `-t`, `-U`, `-V`, `-W`, `-X`, `-x`, and `-Z` are not implemented yet.

## Requirements

- Rust with Edition 2024 support.
- OpenSSL command-line tools only if you want to generate local certificates for TLS testing.

## Installation

```bash
git clone https://github.com/titenq/ncrs.git
cd ncrs
cargo build --release
```

To install globally:

```bash
sudo cp target/release/ncrs /usr/local/bin/
```

## Usage

### TCP

Terminal 1 (server):

```bash
ncrs -l -p 8080 -v
```

Terminal 2 (client):

```bash
ncrs 127.0.0.1 8080 -v
```

Use TCP mode for a basic bidirectional connection between two terminals.

### Persistent Listen Mode

Terminal 1 (server):

```bash
ncrs -l -p 8080 -k -v
```

Terminal 2 (client):

```bash
ncrs 127.0.0.1 8080 -v
```

Stop Terminal 2 and run it again. With `-k`, Terminal 1 keeps accepting sequential inbound connections.

### UDP

Terminal 1 (server):

```bash
ncrs -l -p 8080 -u -v
```

Terminal 2 (client):

```bash
ncrs 127.0.0.1 8080 -u -v
```

Use `-u` when you want datagram-based communication instead of TCP.

### Port Scan

```bash
ncrs localhost 20-100 -z -v
```

Use `-z` to test whether ports are open without entering interactive I/O mode.

### Timeout

```bash
ncrs 8.8.8.8 80 -w 10
```

Use `-w` to limit DNS resolution and TCP connection wait time.

### Source Address

Terminal 1 (server):

```bash
ncrs -l -p 8080 -4 -v
```

Terminal 2 (client):

```bash
ncrs 127.0.0.1 8080 -s 127.0.0.1 -4 -v
```

Use `-s` to choose the local address used for outbound connections. This is useful on hosts with multiple local IP addresses or when testing routing and firewall rules.

### IPv4

```bash
ncrs google.com 80 -4 -v
```

Use `-4` to force IPv4 address resolution and connection.

### IPv6

```bash
ncrs ::1 8080 -6 -v
```

Use `-6` to force IPv6 address resolution and connection.

### HTTP With CRLF

```bash
ncrs google.com 80 -v -C
```

After connecting, type:

```text
GET / HTTP/1.1
Host: google.com
Connection: close
```

The empty line finishes the HTTP headers.

## TLS Mode

TLS is enabled with `--tls`.

Generate a local certificate and key for server-side testing:

```bash
openssl req -new -x509 -newkey rsa:2048 -nodes \
    -keyout key.pem \
    -out cert.pem \
    -days 365 \
    -subj "/CN=localhost" \
    -addext "subjectAltName = DNS:localhost,IP:127.0.0.1" \
    -addext "basicConstraints = CA:FALSE" \
    -addext "keyUsage = digitalSignature, keyEncipherment"
```

Terminal 1 (server):

```bash
ncrs -l -p 8443 -v --tls
```

Terminal 2 (client):

```bash
ncrs localhost 8443 -v --tls
```

Use `--tls` when both sides should communicate through a TLS session.

Do not commit `key.pem`.

## Project Structure

```text
ncrs/
├── src/
│   ├── cli.rs
│   ├── common/
│   │   └── mod.rs
│   ├── core/
│   │   ├── address.rs
│   │   ├── duplex.rs
│   │   ├── mod.rs
│   │   ├── scan.rs
│   │   ├── tcp.rs
│   │   └── udp.rs
│   └── main.rs
├── Cargo.lock
├── Cargo.toml
├── LICENSE.txt
└── README.md
```

## Disclaimer

This project is intended for educational and cybersecurity research purposes only. Users are responsible for complying with applicable laws and regulations.

## License

This project is licensed under the GPL 3.0 License. See [LICENSE.txt](LICENSE.txt).
