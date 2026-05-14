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
- Numeric-only mode with `-n`.
- Local source address selection with `-s`.
- Local source port selection with `-p`.
- Shutdown the network socket after EOF on stdin with `-N`.
- Disable stdin reading with `-d` for background execution.
- Quit delay after EOF on stdin with `-q`.
- Receive limit with `-W` to terminate after exactly X packets/chunks of data.
- Interval delay for throttling data and port scanning with `-i`.
- Randomize remote ports with `-r` to obfuscate port scans.
- Unix Domain Sockets support with `-U` (cross-platform safe, Unix-only execution).
- Specify the size of the TCP receive and send buffers in bytes with `-I` and `-O`.
- Allow broadcast (SO_BROADCAST) on the socket with `-b`.
- Enable debugging (SO_DEBUG) on the socket with `-D`.
- Set the Time To Live (TTL) of outgoing packets with `-M`.
- Change the IPv4 Type of Service (TOS) or IPv6 Traffic Class with `-T`.
- Verbose mode with `-v` to print detailed connection info.
- Silent execution by default (logs directed to stderr) for safe data piping.
- Telnet negotiation handling with `-t` to answer RFC 854 requests automatically.
- Proxy support for TCP connections via SOCKS4, SOCKS5, and HTTP CONNECT with `-x`, `-X`, and `-P`.
- Optional TLS mode with `--tls`.
- Local TLS certificate generation with `--tls-gen` and `--tls-gen-force`.
- Pass the first connected socket to stdout and exit with `-F`.
- Set minimum TTL for incoming packets with `-m`.
- Enable TCP MD5 signature option with `-S`.
- DCCP mode support with `-Z`.

## Compatibility Notes

`ncrs` is currently compatible with only a subset of OpenBSD `nc`.

Implemented interface:

```text
Arguments:
  [destination]  Target IP address or Hostname
  [port]...      Port(s) to connect to (e.g., 80, 80 443 8080, or 20-100)

Options:
  -l, --listen                   Listen mode: waits for incoming connections
  -v, --verbose                  Verbose mode: prints detailed connection info
  -z, --scan                     Zero-I/O mode: used for port scanning
  -p, --source-port <PORT>       Local source port for outbound connections
  -s, --sourceaddr <SOURCEADDR>  Local source address for outbound connections
  -w, --timeout <TIMEOUT>        Connection timeout in seconds
  -u, --udp                      UDP mode
  -n, --numeric                  Suppress name resolution
  -4, --ipv4                     Force IPv4
  -6, --ipv6                     Force IPv6
  -b, --broadcast                Allow broadcast (SO_BROADCAST) on the socket
  -D, --debug                    Enable debugging on the socket
  -t, --telnet                   Answer RFC 854 DON'T and WON'T to RFC 854 DO and WILL requests
  -M, --ttl <TTL>                Set the TTL / hop limit of outgoing packets
  -T, --tos <KEYWORD>            Change IPv4 TOS or IPv6 traffic class value
  -m, --minttl <TTL>             Ask the kernel to drop incoming packets whose TTL/hop limit is under minttl
  -S, --tcp-md5sig               Enable the RFC 2385 TCP MD5 signature option
  -Z, --dccp                     DCCP mode (currently a stub returning an unsupported error)
  -F, --pass-fd                  Pass the first connected socket using sendmsg(2) to stdout and exit
  -C, --crlf                     Send CRLF as line-ending
  -d, --no-stdin                 Do not attempt to read from stdin
  -I, --recv-bytes <BYTES>       Specify the size of the TCP receive buffer in bytes
  -O, --send-bytes <BYTES>       Specify the size of the TCP send buffer in bytes
  -i, --interval <SECONDS>       Interval delay: delay between lines of text and port scan connections
  -k, --keep-alive               Keep accepting sequential inbound connections
  -N, --shutdown-on-eof          Shutdown the network socket after EOF on stdin
  -q, --quit-delay <SECONDS>     Quit delay: wait the specified seconds after EOF on stdin and quit
  -W, --recv-limit <LIMIT>       Receive limit: Terminate after receiving the specified number of packets/chunks
  -U, --unixsock                 Use Unix Domain Sockets
  -r, --randomize-ports          Randomize remote ports
  -x, --proxy <ADDRESS[:PORT]>   Proxy address and port (default port 1080 for SOCKS, 3128 for HTTP)
  -X, --proxy-type <PROTOCOL>    Proxy protocol: "4" (SOCKSv4), "5" (SOCKSv5), or "connect" (HTTP)
  -P, --proxy-username <USER>    Proxy username for authentication (only for HTTP CONNECT proxies)
      --tls                      ncrs extension: use TLS for the connection
      --tls-gen                  ncrs extension: generate TLS files in the OS-specific data directory
      --tls-gen-force            ncrs extension: generate and overwrite TLS files
```

Important differences from OpenBSD `nc`:

- `--tls` is an `ncrs` extension and is not an OpenBSD `nc` flag.
- `--tls-gen` and `--tls-gen-force` are `ncrs` extensions and are not OpenBSD `nc` flags.

## Requirements

- Rust with Edition 2024 support.

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

## Releases

`ncrs` has an automated CI/CD pipeline via GitHub Actions. Whenever a new tag starting with `v` is pushed, it automatically compiles native executables for Linux, Windows, and macOS, and attaches them to a new GitHub Release.

To trigger a release:

```bash
# 1. Commit your changes
git commit -m "feat: your new feature"

# 2. Create a version tag
git tag v0.1.0

# 3. Push the tags to GitHub
git push origin --tags
```

You can then download the compiled binaries from the **Releases** tab of the GitHub repository.

## Usage

### TCP

Terminal 1 (server):

```bash
ncrs -l 8080 -v
```

Terminal 2 (client):

```bash
ncrs 127.0.0.1 8080 -v
```

Use TCP mode for a basic bidirectional connection between two terminals.

### Persistent Listen Mode

Terminal 1 (server):

```bash
ncrs -l 8080 -k -v
```

Terminal 2 (client):

```bash
ncrs 127.0.0.1 8080 -v
```

Stop Terminal 2 and run it again. With `-k`, Terminal 1 keeps accepting sequential inbound connections.

### UDP

Terminal 1 (server):

```bash
ncrs -l 8080 -u -v
```

Terminal 2 (client):

```bash
ncrs 127.0.0.1 8080 -u -v
```

Use `-u` when you want datagram-based communication instead of TCP.

### Broadcast

**Terminal 1 (Listener on a second device or local):**
```bash
ncrs -l 8080 -u -v
```

**Terminal 2 (Broadcaster):**
```bash
ncrs 255.255.255.255 8080 -u -b -v
```

Use `-b` to allow sending UDP packets to network broadcast addresses. Any device on your local network listening on that port will receive the messages.

### Debug (-D)

**Terminal 1 (Server):**
```bash
ncrs -l 8080 -D -v
```

**Terminal 2 (Client):**
```bash
ncrs localhost 8080 -D -v
```

Use `-D` to enable debugging on the socket (`SO_DEBUG`). Note that actual debugging output depends on the underlying operating system and network stack configuration (and often requires root privileges).

### Port Scan

```bash
ncrs localhost 20-100 -z -v
```

Use `-z` to test whether ports are open without entering interactive I/O mode.

You can also use `-r` to randomize the order in which ports are scanned, which is helpful to evade detection or simple port scan limits:

```bash
ncrs localhost 20-100 -z -r -v
```

### Timeout

```bash
ncrs 8.8.8.8 80 -w 5 -v
```

Use `-w` to limit DNS resolution, TCP connection wait time, and idle waits for final TCP/UDP reads.

For a quick UDP receive-timeout check:

```bash
ncrs 127.0.0.1 9999 -u -w 2 -v
```

### Numeric-Only Mode

```bash
ncrs 127.0.0.1 80 -n -v
```

Use `-n` to suppress name resolution. With `-n`, the destination must be a numeric IP address.

This should fail because DNS is disabled:

```bash
ncrs localhost 80 -n -v
```

### Source Address

Terminal 1 (server):

```bash
ncrs -l 8080 -4 -v
```

Terminal 2 (client):

```bash
ncrs 127.0.0.1 8080 -s 127.0.0.1 -4 -v
```

Use `-s` to choose the local address used for outbound connections. This is useful on hosts with multiple local IP addresses or when testing routing and firewall rules.

### Source Port

Terminal 1 (server):

```bash
ncrs -l 8080 -4 -v
```

Terminal 2 (client):

```bash
ncrs 127.0.0.1 8080 -p 19001 -4 -v
```

Use `-p` to choose the local source port used for outbound connections.

### IPv4

**Terminal 1 (Server):**
```bash
ncrs -l 8080 -4 -v
```

**Terminal 2 (Client):**
```bash
ncrs localhost 8080 -4 -v
```

Use `-4` to force IPv4 address resolution and connection.

### IPv6

**Terminal 1 (Server):**
```bash
ncrs -l 8080 -6 -v
```

**Terminal 2 (Client):**
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

### Shutdown on EOF

For automated scripts using pipes:

```bash
echo -e "GET / HTTP/1.0\r\n\r\n" | ncrs google.com 80 -v -N
```

For manual usage, run:

```bash
ncrs google.com 80 -v -C -N
```

After connecting, type your request and press `Ctrl+D` (EOF):

```text
GET / HTTP/1.0
```

Use `-N` to instruct `ncrs` to close the sending half of the connection immediately after it finishes reading from standard input. This is very useful in shell scripts or when sending EOF manually, preventing `ncrs` from hanging forever waiting for the server to close the connection.

### Run in Background (No Stdin)

When running `ncrs` in the background (using `&`), the operating system may suspend the process if it tries to read from the terminal (`stdin`). Use `-d` to disable `stdin` reading.

**Terminal 1 (Server in Background):**
```bash
ncrs -l 8080 -d > received_data.txt &
```
*Note: Because it's running in the background, your terminal prompt will return immediately. Any text you type now goes to your shell, not to `ncrs`.*

**Terminal 2 (Client):**
Send data to the background server from another terminal:
```bash
echo "Secret message over the network!" | ncrs localhost 8080
```

**Back to Terminal 1:**
Check the contents of the file to see the received data:
```bash
cat received_data.txt
```

With `-d`, `ncrs` will solely receive data from the network without expecting or reading any input from your keyboard, making it perfect for silent daemon-like execution.

### Quit Delay (-q)

When sending data through a pipeline, the `-N` flag shuts down the socket immediately. If the server takes a few seconds to process your data and send a response, `-N` will close the connection too early and you won't see the reply. 

Use `-q <SECONDS>` to specify a maximum delay before quitting. To see it in action waiting exactly 5 seconds, try this with two terminals:

**Terminal 1 (Server):**
Start a simple server that keeps the connection open:
```bash
ncrs -l 8080
```

**Terminal 2 (Client):**
Send a message and instruct `ncrs` to wait 5 seconds after sending it:
```bash
echo "Process this data" | ncrs localhost 8080 -q 5
```

Because the server (Terminal 1) is keeping the connection open, the client (Terminal 2) will send the message, detect the end of the `echo` text, **wait exactly 5 seconds**, and then gracefully exit. 

*Note: If the server finishes sending its response and closes the connection **before** the 5 seconds are up, `ncrs` exits immediately. You can use `-q 0` to exit instantly (aborting everything) or `-q -1` to wait forever.*

### Receive Limit (-W)

Use `-W <LIMIT>` to automatically terminate the connection after receiving exactly `LIMIT` network reads (packets for UDP, chunks of data for TCP).

To wait for exactly 1 UDP reply from an NTP server and exit immediately:

```bash
echo -n -e '\x1b\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00' | ncrs 162.159.200.1 123 -u -W 1 -v
```

This is extremely useful in bash scripts where you don't want to deal with manual timeouts, knowing you just need exactly one response.

### Interval Delay (-i)

The `-i` flag introduces a delay between each block of data sent and received. It also introduces a delay between each port connection attempt when scanning multiple ports (`-z`).

To simulate a slow client (Slowloris-style) sending data with a 3-second delay between each chunk:
```bash
cat large_file.txt | ncrs server.com 8080 -i 3
```

To scan ports 20-100 with a 1-second delay between each port check (useful to evade detection or rate limits):
```bash
ncrs server.com 20-100 -z -i 1
```

### Receive and Send Buffer Size (-I, -O)

**Terminal 1 (Server):**
```bash
ncrs -l 8080 -I 8192 -v
```

**Terminal 2 (Client):**
```bash
ncrs localhost 8080 -O 4096 -v
```

Use `-I <BYTES>` to specify the size of the TCP receive buffer (`SO_RCVBUF`) and `-O <BYTES>` to specify the size of the TCP send buffer (`SO_SNDBUF`). This is useful for tuning network performance or testing specific buffer limits.

### Unix Domain Sockets (-U)

`ncrs` supports connecting to and listening on Unix domain sockets (on supported platforms like Linux and macOS). This is highly useful for interacting with local daemons or IPC (Inter-Process Communication) sockets without using TCP/UDP.

**Listen on a Unix socket (Server):**
To create a local server that listens on a Unix socket:
```bash
ncrs -l -U /tmp/echo.sock -v
```

**Connect to a Unix socket (Client):**
To interact with a local service (like Docker, Redis, or our server above) that exposes a `.sock` file:
```bash
ncrs -U /tmp/echo.sock -v
```

When you use `-U`, `ncrs` interprets the target destination as a file path instead of an IP/hostname, and you do not need to provide a port. The socket file will be automatically removed if it already exists when starting a server.

*Note: The `-U` flag is strictly available when compiled on Unix-like environments. Executing this flag on Windows will gracefully abort with an unsupported platform error.*

### Proxy Support (-x, -X, -P)

`ncrs` supports connecting to remote hosts through proxies (SOCKS4, SOCKS5, and HTTP CONNECT). This is useful for pivoting or hiding your origin IP.

Connect to `example.com:80` through a SOCKS5 proxy running on `10.2.3.4:1080`:
```bash
ncrs -x 10.2.3.4:1080 -X 5 example.com 80 -v
```
*(Note: If `-X` is omitted, it defaults to SOCKS5 `5`)*

Connect through an HTTP proxy on `10.2.3.4:8080`:
```bash
ncrs -x 10.2.3.4:8080 -X connect example.com 80 -v
```

Connect through an HTTP proxy requiring basic authentication:
```bash
ncrs -x 10.2.3.4:8080 -X connect -P myusername:mypassword example.com 80 -v
```

*Note: Proxy support is only available for outbound TCP connections. It cannot be used with listen (`-l`), UDP (`-u`), Unix sockets (`-U`), or custom source addresses (`-s`).*

### Telnet Negotiation (-t)

`ncrs` can automatically respond to Telnet negotiation requests (RFC 854). This is useful when interacting with services that expect initial Telnet handshakes before establishing a raw text connection.

```bash
ncrs telehack.com 23 -t -v
```

Use `-t` to answer RFC 854 `DO` and `WILL` requests with `DON'T` and `WON'T`, preventing the connection from hanging due to unhandled negotiation bytes.

### Time To Live (-M)

Use `-M <ttl>` to set the IP TTL (or IPv6 hop limit) of outgoing packets. Useful for testing routing and firewall limits.
```bash
ncrs example.com 80 -M 5 -v
```

### Type of Service (-T)

Use `-T <keyword>` to change the IPv4 TOS or IPv6 Traffic Class. You can use hexadecimal values (e.g., `0x10`) or OpenBSD `nc` keywords like `critical`, `inetcontrol`, `lowcost`, `lowdelay`, `netcontrol`, `throughput`, or `reliability`.
```bash
ncrs example.com 80 -T lowdelay -v
```

### Pass File Descriptor (-F)

Use `-F` to pass the first connected socket using `sendmsg(2)` to `stdout` and exit. This is typically used by other programs to establish a connection and then hand off the file descriptor for the parent process to use.

**Terminal 1 (Server):**
```bash
ncrs -l 8080 -F
```

**Terminal 2 (Client):**
```bash
ncrs localhost 8080
```

### Minimum TTL (-m)

Use `-m <ttl>` to ask the kernel to drop incoming packets whose TTL (Time To Live) or hop limit is under the specified value. This is useful for spoofing protection (e.g., BGP GTSM).

**Terminal 1 (Server):**
```bash
sudo ncrs -l 8080 -m 255 -v
```

**Terminal 2 (Client):**
```bash
ncrs localhost 8080 -v
```

*Note: Setting custom socket options like `IP_MINTTL` often requires root/administrator privileges (`sudo`).*

### TCP MD5 Signature (-S)

Use `-S` to enable the RFC 2385 TCP MD5 signature option. This is typically used to protect BGP routing sessions.

**Terminal 1 (Server):**
```bash
sudo ncrs -l 179 -S -v
```

**Terminal 2 (Client):**
```bash
sudo ncrs target_ip 179 -S -v
```

*Note: Enabling `TCP_MD5SIG` require root privileges (`sudo`). Additionally, the actual MD5 password must be configured in the OS networking stack beforehand (e.g., via `ip tcp_metrics` on Linux). `ncrs` simply enables the socket option.*

*Note: The actual MD5 password must be configured in the OS networking stack beforehand. `ncrs` simply enables the `TCP_MD5SIG` socket option on the connection.*

### DCCP Mode (-Z)

Use `-Z` to select DCCP (Datagram Congestion Control Protocol) mode.

**Terminal 1 (Server):**
```bash
ncrs -l 8080 -Z -v
```

**Terminal 2 (Client):**
```bash
ncrs localhost 8080 -Z -v
```

*Note: In `ncrs`, this is currently a stub returning an "unsupported error", as DCCP is not universally supported by the async networking stack across all platforms.*

*Note: In `ncrs`, this is currently a stub returning an unsupported error, as DCCP is not universally supported across all platforms.*

### TLS Mode

TLS is enabled with `--tls`.

Generate a local certificate and key for server-side testing:

```bash
ncrs --tls-gen
```

This creates:

```text
~/.config/ncrs/cert.pem
~/.config/ncrs/key.pem
```

Use `--tls-gen-force` to overwrite existing `cert.pem` and `key.pem`:

```bash
ncrs --tls-gen-force
```

When `--tls` is used, `ncrs` reads TLS files from `~/.config/ncrs` first. If they are not found there, it falls back to `cert.pem` and `key.pem` in the current directory.

Terminal 1 (server):

```bash
ncrs -l 8443 -v --tls
```

Terminal 2 (client):

```bash
ncrs localhost 8443 -v --tls
```

Use `--tls` when both sides should communicate through a TLS session.

Do not commit `key.pem` if you generate TLS files in a project directory.

## Project Structure

```text
ncrs/
├── src/
│   ├── common/
│   │   └── mod.rs
│   ├── core/
│   │   ├── address.rs
│   │   ├── duplex.rs
│   │   ├── mod.rs
│   │   ├── proxy.rs
│   │   ├── scan.rs
│   │   ├── tcp.rs
│   │   ├── udp.rs
│   │   └── unix.rs
│   ├── cli.rs
│   ├── main.rs
│   └── tls.rs
├── Cargo.lock
├── Cargo.toml
├── LICENSE.txt
└── README.md
```

## Disclaimer

This project is intended for educational and cybersecurity research purposes only. Users are responsible for complying with applicable laws and regulations.

## License

This project is licensed under the GPL 3.0 License. See [LICENSE.txt](LICENSE.txt).
