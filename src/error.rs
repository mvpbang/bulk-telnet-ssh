use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Config parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Invalid host:port format: '{0}'")]
    InvalidAddress(String),
    #[error("SSH error: {0}")]
    Ssh(String),
}
