use crate::error::AppError;
use serde::{Deserialize, Deserializer};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub auth: AuthConfig,
    pub ips: Vec<String>,
    pub target: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AuthConfig {
    pub user: String,
    pub password: String,
    #[serde(deserialize_with = "deserialize_port")]
    pub port: u16,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
}

fn default_concurrency() -> usize {
    8
}

fn deserialize_port<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum PortValue {
        Num(u16),
        Str(String),
    }

    match PortValue::deserialize(deserializer)? {
        PortValue::Num(n) => Ok(n),
        PortValue::Str(s) => s.trim().parse::<u16>().map_err(serde::de::Error::custom),
    }
}

pub fn normalize_host_port(addr: &str, default_port: u16) -> Result<(String, u16), AppError> {
    let trimmed = addr.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidAddress("empty address".to_string()));
    }

    if let Some((host, port_str)) = trimmed.rsplit_once(':') {
        let port = port_str
            .parse::<u16>()
            .map_err(|_| AppError::InvalidAddress(addr.to_string()))?;
        Ok((host.to_string(), port))
    } else {
        Ok((trimmed.to_string(), default_port))
    }
}

impl Config {
    pub fn parse_yaml(content: &str) -> Result<Self, AppError> {
        let cfg: Config = serde_yaml::from_str(content)?;
        Ok(cfg)
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, AppError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_yaml(&content)
    }

    pub fn resolved_ips(&self) -> Result<Vec<(String, u16)>, AppError> {
        self.ips
            .iter()
            .map(|ip| normalize_host_port(ip, self.auth.port))
            .collect()
    }

    pub fn resolved_targets(&self) -> Result<Vec<(String, u16)>, AppError> {
        self.target
            .iter()
            .map(|tgt| normalize_host_port(tgt, 0))
            .collect()
    }
}
