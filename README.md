# ncrs - Netcat Rust Edition 🚀

**ncrs** is a modern, high-performance, and secure implementation of the classic Netcat utility, built from the ground up in **Rust**. This project focuses on network security, leveraging the **Tokio** runtime for asynchronous concurrency and **Rustls** for mandatory/optional end-to-end encryption (TLS).

Developed by **TitenQ** | [titenq.com.br](https://titenq.com.br) | [titenq@gmail.com](mailto:titenq@gmail.com)

---

## 🛡️ Key Features

- **Asynchronous Full-Duplex I/O:** Simultaneous bidirectional communication using `tokio::select!`.

- **TLS 1.3 Encryption:** Native support for secure tunnels in both Client and Server modes.

- **Port Scanning (-z):** High-speed asynchronous port scanning with range support (e.g., 20-100).

- **UDP Support (-u):** Bidirectional datagram communication for both Client and Server modes.

- **Smart Timeout (-w):** Applies to both DNS resolution and TCP connection establishment.

- **IPv6 Ready (-6):** Full support for modern IPv6 addressing and dual-stack connectivity.

- **Line Ending Control (-C):** Optional CRLF (\r\n) support for compatibility with strict protocols like HTTP and SMTP.

- **Intuitive CLI:** Powered by `clap` for a professional command-line experience.

- **Custom Banner:** Professional identity with ASCII art branding.

- **Memory Safety:** Built with Rust's strict safety guarantees, eliminating common C-based vulnerabilities like buffer overflows.

- **Static Binaries:** Easy to distribute without worrying about complex system dependencies.

---

## 🛠️ Tech Stack

*   **Rust:** Core language for performance and memory safety.

*   **Tokio:** The industry-standard asynchronous runtime for Rust.

*   **Rustls:** A modern, fast, and safe TLS library (no OpenSSL dependency for the binary).

*   **Clap:** Powerful command-line argument parsing.

*   **Colored:** Terminal-based visual feedback with colors.

## 📋 Requirements

- Rust 1.70+
- OpenSSL (only for generating local certificates/keys)

## 🚀 Installation

### 1. Clone and Compile
```bash
git clone https://github.com/titenq/ncrs.git
cd ncrs
cargo build --release
```

### 2. Global Installation
To use ncrs from anywhere in your terminal:
```bash
sudo cp target/release/ncrs /usr/local/bin/
```

---

## 🔐 TLS Setup (Self-Signed)

To use Secure Mode (-s), you must generate local certificates on your Linux system:

```bash
openssl req -new -x509 -key key.pem -out cert.pem -days 365 \
    -subj "/CN=localhost" \
    -addext "subjectAltName = DNS:localhost,IP:127.0.0.1" \
    -addext "basicConstraints = CA:FALSE" \
    -addext "keyUsage = digitalSignature, keyEncipherment"
```

*Note: Make sure to add key.pem to your .gitignore file to prevent leaking your private key.*

## 📖 Usage Guide

### Basic Chat (TCP & UDP)
**TCP Server:**
```bash
ncrs -l -p 8080 -v
```

**UDP Server:**
```bash
ncrs -l -p 8080 -u -v
```

**UDP Client:**
```bash
ncrs localhost 8080 -u -v
```

### Secure Tunnel (TLS)
Server:
```bash
ncrs -l -p 8443 -v -s
```

Client:
```bash
ncrs localhost 8443 -v -s
```

### Port Scan
```bash
ncrs localhost 20-100 -z -v
```

**Connection with Timeout:**
```bash
ncrs 8.8.8.8 80 -w 10
```

**IPv6 Connection:**
```bash
ncrs ::1 8080 -v
```

**Testing HTTP (Requires -C):**
```bash
ncrs google.com 80 -v -C
# Once connected, type:
GET / HTTP/1.1
Host: google.com
(Press Enter twice)
```
---

### Network Auditing (Example: Google)
You can use ncrs to audit real-world server certificates and HTTP headers:

```bash
ncrs google.com 443 -v -s
```

*Once connected, type GET / HTTP/1.1 and hit Enter twice.*

## 📂 Project Structure

```text
ncrs/
├── src/
│   ├── common/
│   │   └── mod.rs      # Utility functions (port parsing, etc.)
│   ├── core/
│   │   └── mod.rs      # Connection engines and scan logic
│   └── main.rs         # CLI entry point and orchestration
├── .gitignore          # Ignores target/ and key.pem
├── Cargo.lock          # Fixed dependency versions
├── Cargo.toml          # Project metadata and dependencies
├── cert.pem            # Public certificate (shared with clients)
├── key.pem             # Private key (SECRET - kept locally)
├── LICENSE.txt         # GPL 3.0 License terms
└── README.md           # Project documentation
```

---

## ⚖️ Disclaimer

This project is intended for **educational and cybersecurity research purposes only**. The author is not responsible for any misuse, damage, or illegal activities performed with this tool. Users are responsible for complying with local laws and regulations.

---

## 📜 License

This project is licensed under the GPL3.0 License - see the [LICENSE](LICENSE.txt) file for details.

---
