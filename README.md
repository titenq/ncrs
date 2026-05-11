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
- IPv6 mode with `-6`.
- CRLF conversion with `-C`.
- Persistent listen mode with `-k`.
- Optional TLS mode with `-s` / `--secure`.

## Compatibility Notes

`ncrs` is currently compatible with only a subset of OpenBSD `nc`.

Implemented flags:

```text
-6 -C -k -l -p -s -u -v -w -z
```

Important differences from OpenBSD `nc`:

- In OpenBSD `nc`, `-s` means local source address. In `ncrs`, `-s` currently enables TLS.
- In OpenBSD `nc`, `-p` is the local source port for outbound connections. In `ncrs`, `-p` is mainly used as the listen port.
- `ncrs -k` accepts multiple sequential inbound connections, but it currently handles one active TCP connection at a time.
- OpenBSD options such as `-4`, `-b`, `-D`, `-d`, `-F`, `-h`, `-I`, `-i`, `-M`, `-m`, `-N`, `-n`, `-O`, `-P`, `-q`, `-r`, `-S`, `-T`, `-t`, `-U`, `-V`, `-W`, `-X`, `-x`, and `-Z` are not implemented yet.

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

Server:

```bash
ncrs -l -p 8080 -v
```

Client:

```bash
ncrs 127.0.0.1 8080 -v
```

### Persistent Listen Mode

```bash
ncrs -l -p 8080 -k -v
```

Then connect more than once from another terminal:

```bash
ncrs 127.0.0.1 8080 -v
```

### UDP

Server:

```bash
ncrs -l -p 8080 -u -v
```

Client:

```bash
ncrs 127.0.0.1 8080 -u -v
```

### Port Scan

```bash
ncrs localhost 20-100 -z -v
```

### Timeout

```bash
ncrs 8.8.8.8 80 -w 10
```

### IPv6

```bash
ncrs ::1 8080 -6 -v
```

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

TLS is enabled with `-s` / `--secure` in the current CLI.

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

Server:

```bash
ncrs -l -p 8443 -v -s
```

Client:

```bash
ncrs localhost 8443 -v -s
```

Do not commit `key.pem`.

## Project Structure

```text
ncrs/
├── src/
│   ├── common/
│   │   └── mod.rs
│   ├── core/
│   │   └── mod.rs
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
