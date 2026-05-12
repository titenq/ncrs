#[derive(Clone, Copy)]
pub enum AddressFamily {
    Any,
    Ipv4,
    Ipv6,
}

impl AddressFamily {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Any => "IPv4/IPv6/DNS",
            Self::Ipv4 => "IPv4",
            Self::Ipv6 => "IPv6",
        }
    }
}

pub(crate) fn format_endpoint(target: &str, port: u16) -> String {
    if !target.contains('[') && target.contains(':') {
        format!("[{}]:{}", target, port)
    } else {
        format!("{}:{}", target, port)
    }
}

pub(crate) async fn resolve_address(
    target: &str,
    port: u16,
    family: AddressFamily,
    timeout_duration: std::time::Duration,
) -> anyhow::Result<std::net::SocketAddr> {
    let endpoint = format_endpoint(target, port);
    let addrs =
        match tokio::time::timeout(timeout_duration, tokio::net::lookup_host(&endpoint)).await {
            Ok(Ok(iter)) => iter,
            Ok(Err(e)) => return Err(anyhow::anyhow!("Failed to resolve {}: {}", endpoint, e)),
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "DNS resolution timed out after {}s",
                    timeout_duration.as_secs()
                ));
            }
        };

    addrs
        .filter(|addr| match family {
            AddressFamily::Any => true,
            AddressFamily::Ipv4 => addr.is_ipv4(),
            AddressFamily::Ipv6 => addr.is_ipv6(),
        })
        .next()
        .ok_or_else(|| anyhow::anyhow!("Could not resolve to any {} address", family.label()))
}
