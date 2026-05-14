use std::path::{Path, PathBuf};

pub struct TlsPaths {
    pub cert: PathBuf,
    pub key: PathBuf,
}

pub fn generate_self_signed_cert(force: bool) -> anyhow::Result<()> {
    let paths = config_tls_paths()?;

    std::fs::create_dir_all(
        paths
            .cert
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid TLS config directory"))?,
    )?;

    if !force && (paths.cert.exists() || paths.key.exists()) {
        return Err(anyhow::anyhow!(
            "{} or {} already exists; use --tls-gen-force to overwrite",
            paths.cert.display(),
            paths.key.display()
        ));
    }

    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let cert = rcgen::generate_simple_self_signed(subject_alt_names)
        .map_err(|e| anyhow::anyhow!("failed to generate certificate: {}", e))?;

    std::fs::write(&paths.cert, cert.cert.pem())?;
    std::fs::write(&paths.key, cert.signing_key.serialize_pem())?;

    Ok(())
}

pub fn config_tls_paths() -> anyhow::Result<TlsPaths> {
    let proj_dirs = directories::ProjectDirs::from("", "", "ncrs")
        .ok_or_else(|| anyhow::anyhow!("could not determine configuration directory"))?;
    let dir = proj_dirs.config_dir();

    Ok(TlsPaths {
        cert: dir.join("cert.pem"),
        key: dir.join("key.pem"),
    })
}

pub fn resolve_client_cert_path() -> anyhow::Result<Option<PathBuf>> {
    let config_paths = config_tls_paths()?;

    if config_paths.cert.exists() {
        return Ok(Some(config_paths.cert));
    }

    let local_cert = Path::new("cert.pem");

    if local_cert.exists() {
        return Ok(Some(local_cert.to_path_buf()));
    }

    Ok(None)
}

pub fn resolve_server_tls_paths() -> anyhow::Result<TlsPaths> {
    let config_paths = config_tls_paths()?;

    if config_paths.cert.exists() && config_paths.key.exists() {
        return Ok(config_paths);
    }

    let local_paths = TlsPaths {
        cert: PathBuf::from("cert.pem"),
        key: PathBuf::from("key.pem"),
    };

    if local_paths.cert.exists() && local_paths.key.exists() {
        return Ok(local_paths);
    }

    Err(anyhow::anyhow!(
        "TLS certificate files not found; run ncrs --tls-gen"
    ))
}
