use std::path::PathBuf;

use clap::Parser;

pub const DEFAULT_FPS: f32 = 60.0;

#[derive(Debug, Clone, Parser)]
#[command(name = "datastar-matrix")]
#[command(about = "Matrix rain in terminal and browser", long_about = None)]
pub struct Config {
    #[arg(long = "fps", default_value_t = DEFAULT_FPS)]
    pub target_fps: f32,
    #[arg(long = "port")]
    pub port: Option<u16>,
    #[arg(long = "server", default_value_t = false)]
    pub server: bool,
    #[arg(long = "insecure", default_value_t = false)]
    pub insecure: bool,
    #[arg(long = "tls-cert")]
    pub tls_cert: Option<PathBuf>,
    #[arg(long = "tls-key")]
    pub tls_key: Option<PathBuf>,
}

impl Config {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }

    #[cfg(feature = "web")]
    pub fn web_enabled(&self) -> bool {
        true
    }

    #[cfg(not(feature = "web"))]
    pub fn web_enabled(&self) -> bool {
        false
    }

    #[cfg(feature = "web")]
    pub fn web_transport(&self) -> anyhow::Result<WebTransport> {
        if self.insecure {
            if self.tls_cert.is_some() || self.tls_key.is_some() {
                anyhow::bail!("--insecure cannot be combined with --tls-cert/--tls-key");
            }
            return Ok(WebTransport::Http);
        }

        match (&self.tls_cert, &self.tls_key) {
            (Some(cert), Some(key)) => Ok(WebTransport::HttpsProvided {
                cert_path: cert.clone(),
                key_path: key.clone(),
            }),
            (None, Some(_)) => anyhow::bail!("missing required --tls-cert in secure mode"),
            (Some(_), None) => anyhow::bail!("missing required --tls-key in secure mode"),
            (None, None) => Ok(WebTransport::HttpsAuto),
        }
    }
}

#[cfg(feature = "web")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebTransport {
    Http,
    HttpsProvided { cert_path: PathBuf, key_path: PathBuf },
    HttpsAuto,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn defaults_to_sixty_fps() {
        let cfg = Config::try_parse_from(["datastar-matrix"]).expect("default parse should work");
        assert_eq!(cfg.target_fps, 60.0);
        assert_eq!(cfg.port, None);
        assert!(!cfg.server);
        assert!(!cfg.insecure);
        assert_eq!(cfg.tls_cert, None);
        assert_eq!(cfg.tls_key, None);
    }

    #[test]
    fn parses_fps_port_and_server() {
        let cfg = Config::try_parse_from([
            "datastar-matrix",
            "--fps",
            "30",
            "--port",
            "8123",
            "--server",
            "--insecure",
        ])
            .expect("parse should work");
        assert_eq!(cfg.target_fps, 30.0);
        assert_eq!(cfg.port, Some(8123));
        assert!(cfg.server);
        assert!(cfg.insecure);
    }

    #[cfg(feature = "web")]
    #[test]
    fn secure_mode_defaults_to_auto_tls() {
        let cfg = Config::try_parse_from(["datastar-matrix"]).expect("parse should work");
        assert_eq!(
            cfg.web_transport().expect("transport should resolve"),
            WebTransport::HttpsAuto
        );
    }

    #[cfg(feature = "web")]
    #[test]
    fn secure_mode_rejects_partial_tls_flags() {
        let cert_only = Config::try_parse_from([
            "datastar-matrix",
            "--tls-cert",
            "cert.pem",
        ])
        .expect("parse should work");
        assert!(cert_only.web_transport().is_err());

        let key_only = Config::try_parse_from([
            "datastar-matrix",
            "--tls-key",
            "key.pem",
        ])
        .expect("parse should work");
        assert!(key_only.web_transport().is_err());
    }

    #[cfg(feature = "web")]
    #[test]
    fn secure_mode_accepts_cert_and_key() {
        let cfg = Config::try_parse_from([
            "datastar-matrix",
            "--tls-cert",
            "/tmp/cert.pem",
            "--tls-key",
            "/tmp/key.pem",
        ])
        .expect("parse should work");

        assert_eq!(
            cfg.web_transport().expect("should resolve"),
            WebTransport::HttpsProvided {
                cert_path: PathBuf::from("/tmp/cert.pem"),
                key_path: PathBuf::from("/tmp/key.pem"),
            }
        );
    }

    #[cfg(feature = "web")]
    #[test]
    fn insecure_mode_rejects_tls_flags() {
        let cfg = Config::try_parse_from([
            "datastar-matrix",
            "--insecure",
            "--tls-cert",
            "/tmp/cert.pem",
            "--tls-key",
            "/tmp/key.pem",
        ])
        .expect("parse should work");
        assert!(cfg.web_transport().is_err());
    }
}
