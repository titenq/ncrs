use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ncrs", author = "TitenQ", version = "0.1.0")]
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

    /// [ncrs extension] Use TLS for the connection
    #[arg(long = "tls")]
    pub tls: bool,

    /// [ncrs extension] Generate TLS files in ~/.config/ncrs
    #[arg(long = "tls-gen")]
    pub tls_gen: bool,

    /// [ncrs extension] Generate TLS files in ~/.config/ncrs, overwriting existing files
    #[arg(long = "tls-gen-force")]
    pub tls_gen_force: bool,

    /// [Flag: -w] Connection timeout: maximum seconds to wait for a response
    #[arg(short = 'w', long, value_name = "TIMEOUT")]
    pub timeout: Option<u64>,

    /// [Flag: -u] UDP mode: uses UDP instead of the default TCP
    #[arg(short = 'u', long)]
    pub udp: bool,

    /// [Flag: -n] Suppress name resolution; destination must be a numeric IP
    #[arg(short = 'n', long)]
    pub numeric: bool,

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
}
