use std::path::Path;
use std::process::Command;

pub fn generate_self_signed_cert(force: bool) -> anyhow::Result<()> {
    let cert_path = Path::new("cert.pem");
    let key_path = Path::new("key.pem");

    if !force && (cert_path.exists() || key_path.exists()) {
        return Err(anyhow::anyhow!(
            "cert.pem or key.pem already exists; use --tls-gen-force to overwrite"
        ));
    }

    let status = Command::new("openssl")
        .args([
            "req",
            "-new",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            "key.pem",
            "-out",
            "cert.pem",
            "-days",
            "365",
            "-subj",
            "/CN=localhost",
            "-addext",
            "subjectAltName = DNS:localhost,IP:127.0.0.1",
            "-addext",
            "basicConstraints = CA:FALSE",
            "-addext",
            "keyUsage = digitalSignature, keyEncipherment",
        ])
        .status()
        .map_err(|e| anyhow::anyhow!("failed to execute openssl: {}", e))?;

    if !status.success() {
        return Err(anyhow::anyhow!(
            "openssl failed while generating TLS certificate"
        ));
    }

    Ok(())
}
