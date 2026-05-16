use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ncrs", author = "TitenQ", version = "0.3.0")]
pub struct Args {
    /// Target IP address or Hostname
    #[arg(value_name = "destination")]
    pub target: Option<String>,

    /// Port(s) to connect to (e.g., 80, 80 443 8080, or 20-100)
    #[arg(value_name = "port")]
    pub ports: Vec<String>,

    /// [Flag: -l] Listen mode: waits for incoming connections
    #[arg(short = 'l', long)]
    pub listen: bool,

    /// [Flag: -v] Verbose mode: prints detailed connection info
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// [Flag: -z] Zero-I/O mode: used for port scanning
    #[arg(short = 'z', long)]
    pub scan: bool,

    /// [Flag: -p] Local source port for outbound connections
    #[arg(short = 'p', long = "source-port", value_name = "PORT")]
    pub source_port: Option<u16>,

    /// [Flag: -s] Local source address for outbound connections
    #[arg(short = 's', long = "sourceaddr", value_name = "SOURCEADDR")]
    pub source_addr: Option<String>,

    /// [Flag: -w] Connection timeout: maximum seconds to wait for a response
    #[arg(short = 'w', long, value_name = "TIMEOUT")]
    pub timeout: Option<u64>,

    /// [Flag: -u] UDP mode: uses UDP instead of the default TCP
    #[arg(short = 'u', long)]
    pub udp: bool,

    /// [Flag: -n] Suppress name resolution; destination must be a numeric IP
    #[arg(short = 'n', long)]
    pub numeric: bool,

    /// [Flag: -b] Allow broadcast (SO_BROADCAST)
    #[arg(short = 'b', long)]
    pub broadcast: bool,

    /// [Flag: -4] IPv4 mode: force usage of IPv4 addresses
    #[arg(short = '4', long, conflicts_with = "ipv6")]
    pub ipv4: bool,

    /// [Flag: -6] IPv6 mode: force usage of IPv6 addresses
    #[arg(short = '6', long)]
    pub ipv6: bool,

    /// [Flag: -C] Send CRLF as line-ending instead of just LF
    #[arg(short = 'C', long)]
    pub crlf: bool,

    /// [Flag: -k] Keep-alive: accept multiple connections in listen mode
    #[arg(short = 'k', long)]
    pub keep_alive: bool,

    /// [Flag: -N] shutdown the network socket after EOF on stdin
    #[arg(short = 'N', long = "shutdown-on-eof")]
    pub shutdown_on_eof: bool,

    /// [Flag: -d] Do not attempt to read from stdin
    #[arg(short = 'd', long = "no-stdin")]
    pub no_stdin: bool,

    /// [Flag: -D] Enable debugging on the socket
    #[arg(short = 'D', long = "debug")]
    pub debug: bool,

    /// [Flag: -t] Answer RFC 854 DON'T and WON'T to RFC 854 DO and WILL requests
    #[arg(short = 't', long = "telnet")]
    pub telnet: bool,

    /// [Flag: -q] Quit delay: after EOF on stdin, wait the specified number of seconds and then quit
    #[arg(
        short = 'q',
        long = "quit-delay",
        value_name = "SECONDS",
        allow_hyphen_values = true
    )]
    pub quit_delay: Option<i32>,

    /// [Flag: -i] Interval delay: specifies a delay time interval between lines/chunks of text sent and received
    #[arg(short = 'i', long = "interval", value_name = "SECONDS")]
    pub interval: Option<u64>,

    /// [Flag: -r] Randomize remote ports
    #[arg(short = 'r', long = "randomize-ports")]
    pub randomize_ports: bool,

    /// [Flag: -I] Specify the size of the TCP receive buffer
    #[arg(short = 'I', long = "recv-bytes", value_name = "BYTES")]
    pub recv_bytes: Option<u32>,

    /// [Flag: -O] Specify the size of the TCP send buffer in bytes
    #[arg(short = 'O', long = "send-bytes", value_name = "BYTES")]
    pub send_bytes: Option<u32>,

    /// [Flag: -W] Receive limit: Terminate after receiving the specified number of packets/chunks
    #[arg(short = 'W', long = "recv-limit", value_name = "LIMIT")]
    pub recv_limit: Option<u32>,

    /// [Flag: -U] Use Unix Domain Sockets
    #[arg(short = 'U', long = "unixsock")]
    pub unix: bool,

    /// [Flag: -M] Set the TTL / hop limit of outgoing packets
    #[arg(short = 'M', long = "ttl", value_name = "TTL")]
    pub ttl: Option<u32>,

    /// [Flag: -T] Change IPv4 TOS or IPv6 traffic class value (keywords: critical, inetcontrol, lowcost, lowdelay, netcontrol, throughput, reliability, or hex/dec)
    #[arg(short = 'T', long = "tos", value_name = "KEYWORD")]
    pub tos: Option<String>,

    /// [Flag: -x] Proxy address and port
    #[arg(short = 'x', long = "proxy", value_name = "ADDRESS[:PORT]")]
    pub proxy: Option<String>,

    /// [Flag: -X] Proxy protocol: "4" (SOCKS v.4), "5" (SOCKS v.5), or "connect" (HTTP)
    #[arg(short = 'X', long = "proxy-type", value_name = "PROTOCOL")]
    pub proxy_type: Option<String>,

    /// [Flag: -P] Proxy username for authentication (only for HTTP CONNECT proxies at present)
    #[arg(short = 'P', long = "proxy-username", value_name = "USERNAME")]
    pub proxy_username: Option<String>,

    /// [Flag: -F] Pass the first connected socket using sendmsg(2) to stdout and exit
    #[arg(short = 'F', long = "pass-fd")]
    pub pass_fd: bool,

    /// [Flag: -m] Ask the kernel to drop incoming packets whose TTL/hop limit is under minttl
    #[arg(short = 'm', long = "minttl", value_name = "TTL")]
    pub minttl: Option<u32>,

    /// [Flag: -S] Enable the RFC 2385 TCP MD5 signature option
    #[arg(short = 'S', long = "tcp-md5sig")]
    pub tcp_md5sig: bool,

    /// [Flag: -Z] [EXPERIMENTAL/STUB] DCCP mode; currently returns an unsupported error
    #[arg(short = 'Z', long = "dccp")]
    pub dccp: bool,

    /// [ncrs extension] Use TLS for the connection
    #[arg(long = "tls")]
    pub tls: bool,

    /// [ncrs extension] Generate TLS files in the OS-specific data directory
    #[arg(long = "tls-gen")]
    pub tls_gen: bool,

    /// [ncrs extension] Generate TLS files in the OS-specific data directory, overwriting existing files
    #[arg(long = "tls-gen-force")]
    pub tls_gen_force: bool,
}
