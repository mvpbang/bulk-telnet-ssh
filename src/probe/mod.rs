pub mod bash_tcp;
pub mod direct_tcp;
pub mod engine;

pub use bash_tcp::parse_bash_output;
pub use engine::probe_target;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeStatus {
    Pong,
    Refused,
    TimedOut,
    NoRoute,
    SshLoginFailed(String),
    Error(String),
}

impl ProbeStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, ProbeStatus::Pong)
    }

    pub fn legacy_keyword(&self) -> &str {
        match self {
            ProbeStatus::Pong => "pong",
            ProbeStatus::Refused => "refused",
            ProbeStatus::TimedOut => "timed out",
            ProbeStatus::NoRoute => "no route",
            ProbeStatus::SshLoginFailed(_) => "login false",
            ProbeStatus::Error(_) => "error",
        }
    }

    pub fn display_badge(&self) -> &str {
        match self {
            ProbeStatus::Pong => "[✓ 成功]",
            ProbeStatus::Refused => "[✗ 拒绝]",
            ProbeStatus::TimedOut => "[✗ 超时]",
            ProbeStatus::NoRoute => "[✗ 无路由]",
            ProbeStatus::SshLoginFailed(_) => "[! 登录失败]",
            ProbeStatus::Error(_) => "[! 异常]",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeResult {
    pub src: String,
    pub target: String,
    pub status: ProbeStatus,
    pub duration_ms: u64,
}
