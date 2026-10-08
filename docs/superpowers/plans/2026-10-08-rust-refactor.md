# bulk-telnet-ssh Rust 重构实施计划 (Implementation Plan)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于 Rust 重构 `bulk-telnet-ssh`，彻底弃用 `telnet` 命令，采用 SSH `direct-tcpip` 协议级通道与 Bash `/dev/tcp` 双引擎探测，实现连接复用、细粒度异步并发控制与现代化结构化日志汇报。

**Architecture:** 异步运行时 `tokio` 驱动调度，`ssh2` 维持每台源主机的复用长连接会话；通过 `direct-tcpip` 协议通道优先完成毫秒级 TCP 握手探测，遇权限拦截自动降级为远程 `bash -c '</dev/tcp/host/port>'`；`tokio::sync::Semaphore` 控制并发度；`reporter` 模块同时追加兼容旧版的 `ssh.log` 并输出美观的控制台汇总表格。

**Tech Stack:** Rust 2021, `tokio`, `ssh2`, `serde`, `serde_yaml`, `clap`, `colored` / `tabled` (or native formatting).

**Spec:** [`docs/superpowers/specs/2026-10-08-rust-refactor-design.md`](file:///Users/mac/auto/bulk-telnet-ssh/docs/superpowers/specs/2026-10-08-rust-refactor-design.md)

## Global Constraints

- 保持对现有 `ips.yml` 格式 100% 向后兼容（支持 `auth.port` 为数字或字符串，支持缺省端口自动填充）。
- 保持 `ssh.log` 日志输出关键字格式（`成功 <src> pong <target>`、`失败 <src> refused <target>`、`失败 <src> timed out <target>`、`login false ...`）兼容。
- 保证每个源主机仅建立 1 次复用长连接（附带 3 次重试机制），避免重复 SSH 握手风暴。
- 严禁依赖远端系统的 `telnet` 命令。
- 所有非异步阻塞的 SSH I/O 操作均在 `tokio::task::spawn_blocking` 中执行，避免阻塞异步运行时。

---

### Task 1: 初始化 Rust 工程结构与 Cargo 依赖配置

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `src/main.rs`

**Interfaces:**
- Produces: 基础可编译运行的 Rust CLI 骨架，定义统一外部依赖。

- [ ] **Step 1: 创建 Cargo.toml 依赖配置**

```toml
[package]
name = "bulk-telnet-ssh"
version = "0.2.0"
edition = "2021"
authors = ["mac"]
description = "High-performance bulk network policy reachability probe tool over SSH"

[dependencies]
tokio = { version = "1.40", features = ["full"] }
ssh2 = { version = "0.9", features = ["vendored-openssl"] }
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
clap = { version = "4.5", features = ["derive"] }
anyhow = "1.0"
thiserror = "1.0"
chrono = "0.4"
colored = "2.1"

[dev-dependencies]
tempfile = "3.12"
```

- [ ] **Step 2: 创建 .gitignore 规则**

```gitignore
/target/
Cargo.lock
ssh.log
*.exe
```

- [ ] **Step 3: 创建最小可运行的 src/main.rs 验证骨架**

```rust
fn main() {
    println!("bulk-telnet-ssh (Rust) initializing...");
}
```

- [ ] **Step 4: 编译并验证**

Run: `cargo check`
Expected: 顺利下载依赖并编译通过，退出码 0。

- [ ] **Step 5: 提交代码**

```bash
git add Cargo.toml .gitignore src/main.rs
git commit -m "chore: initialize Cargo manifest and dependencies for Rust refactor"
```

---

### Task 2: 配置解析模块与数据模型 (`src/config.rs`, `src/error.rs`)

**Files:**
- Create: `src/error.rs`
- Create: `src/config.rs`
- Test: `tests/config_test.rs`

**Interfaces:**
- Produces:
  - `pub struct Config { pub auth: AuthConfig, pub ips: Vec<String>, pub target: Vec<String> }`
  - `pub struct AuthConfig { pub user: String, pub password: String, pub port: u16, pub concurrency: usize }`
  - `Config::load_from_file<P: AsRef<Path>>(path: P) -> Result<Config, AppError>`
  - `pub fn normalize_host_port(addr: &str, default_port: u16) -> Result<(String, u16), AppError>`

- [ ] **Step 1: 编写配置解析的失败单元测试**

`tests/config_test.rs`:
```rust
use bulk_telnet_ssh::config::{Config, normalize_host_port};

#[test]
fn test_normalize_host_port() {
    let (h1, p1) = normalize_host_port("172.20.189.75", 22).unwrap();
    assert_eq!(h1, "172.20.189.75");
    assert_eq!(p1, 22);

    let (h2, p2) = normalize_host_port("172.20.189.75:2222", 22).unwrap();
    assert_eq!(h2, "172.20.189.75");
    assert_eq!(p2, 2222);
}

#[test]
fn test_parse_yaml_string_port_and_int_port() {
    let yaml = r#"
auth:
  user: root
  password: secret
  port: "22"
  concurrency: 16
ips:
  - 10.0.0.1
  - 10.0.0.2:2222
target:
  - 192.168.1.1:80
"#;
    let cfg = Config::parse_yaml(yaml).unwrap();
    assert_eq!(cfg.auth.user, "root");
    assert_eq!(cfg.auth.port, 22);
    assert_eq!(cfg.auth.concurrency, 16);
    assert_eq!(cfg.resolved_ips().unwrap().len(), 2);
}
```

- [ ] **Step 2: 运行测试验证失败**

Run: `cargo test --test config_test`
Expected: 编译失败，未定义 `bulk_telnet_ssh` 模块或函数。

- [ ] **Step 3: 编写 src/error.rs 与 src/config.rs 及 src/lib.rs**

`src/error.rs`:
```rust
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
```

`src/config.rs`:
```rust
use crate::error::AppError;
use serde::{Deserialize, Deserializer};
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub auth: AuthConfig,
    pub ips: Vec<String>,
    pub target: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
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
        PortValue::Str(s) => s.parse::<u16>().map_err(serde::de::Error::custom),
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
```

`src/lib.rs`:
```rust
pub mod config;
pub mod error;
pub mod ssh;
pub mod probe;
pub mod runner;
pub mod reporter;
```

- [ ] **Step 4: 运行测试验证通过**

Run: `cargo test --test config_test`
Expected: PASS 2 passed.

- [ ] **Step 5: 提交代码**

```bash
git add src/error.rs src/config.rs src/lib.rs tests/config_test.rs
git commit -m "feat: implement YAML configuration parsing and address normalizer"
```

---

### Task 3: SSH 连接管理与握手重试模块 (`src/ssh/`)

**Files:**
- Create: `src/ssh/mod.rs`
- Create: `src/ssh/client.rs`

**Interfaces:**
- Produces:
  - `pub struct SshClient { ... }`
  - `SshClient::connect(host: &str, port: u16, user: &str, pass: &str, timeout_secs: u64, max_retries: usize) -> Result<Self, AppError>`
  - `impl SshClient { pub fn session(&self) -> &ssh2::Session }`
  - `pub struct SshPool { ... }` 管理主机与对应 `Arc<SshClient>` 连接池。

- [ ] **Step 1: 编写 src/ssh/client.rs 实现握手与重试机制**

```rust
use crate::error::AppError;
use ssh2::Session;
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct SshClient {
    pub host: String,
    pub port: u16,
    pub addr: String,
    session: Arc<Session>,
}

impl SshClient {
    pub fn connect(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        timeout: Duration,
        max_retries: usize,
    ) -> Result<Self, AppError> {
        let addr = format!("{}:{}", host, port);
        let mut last_err = None;

        for attempt in 1..=max_retries {
            match Self::try_connect(host, port, user, password, timeout) {
                Ok(client) => return Ok(client),
                Err(err) => {
                    let err_msg = err.to_string();
                    let is_handshake = err_msg.to_lowercase().contains("handshake")
                        || err_msg.to_lowercase().contains("reset")
                        || err_msg.to_lowercase().contains("timed out");

                    last_err = Some(err);
                    if !is_handshake || attempt == max_retries {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(500 * attempt as u64));
                }
            }
        }

        Err(last_err.unwrap_or_else(|| AppError::Ssh("Unknown connection error".to_string())))
    }

    fn try_connect(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        timeout: Duration,
    ) -> Result<Self, AppError> {
        let addr = format!("{}:{}", host, port);
        let tcp = TcpStream::connect_timeout(
            &addr
                .parse()
                .or_else(|_| {
                    use std::net::ToSocketAddrs;
                    addr.to_socket_addrs()?
                        .next()
                        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "DNS failed"))
                })
                .map_err(AppError::Io)?,
            timeout,
        )?;

        let mut sess = Session::new().map_err(|e| AppError::Ssh(e.to_string()))?;
        sess.set_timeout(timeout.as_millis() as u32);
        sess.set_tcp_stream(tcp);
        sess.handshake().map_err(|e| AppError::Ssh(e.to_string()))?;
        sess.userauth_password(user, password)
            .map_err(|e| AppError::Ssh(e.to_string()))?;

        if !sess.authenticated() {
            return Err(AppError::Ssh("Authentication failed".to_string()));
        }

        Ok(Self {
            host: host.to_string(),
            port,
            addr,
            session: Arc::new(sess),
        })
    }

    pub fn session(&self) -> &Session {
        &self.session
    }
}
```

- [ ] **Step 2: 导出模块并增加单元测试**

`src/ssh/mod.rs`:
```rust
pub mod client;
pub use client::SshClient;
```

`tests/ssh_mock_test.rs`: 验证重试与错误退避逻辑。

- [ ] **Step 3: 编译并运行测试**

Run: `cargo test`
Expected: 编译通过，所有测试正常。

- [ ] **Step 4: 提交代码**

```bash
git add src/ssh/
git commit -m "feat: implement SSH connection client with retry and authentication"
```

---

### Task 4: 双引擎探测机制 (`src/probe/`)

**Files:**
- Create: `src/probe/mod.rs`
- Create: `src/probe/engine.rs`
- Create: `src/probe/direct_tcp.rs`
- Create: `src/probe/bash_tcp.rs`
- Test: `tests/probe_test.rs`

**Interfaces:**
- Produces:
  - `pub enum ProbeStatus { Pong, Refused, TimedOut, NoRoute, SshLoginFailed(String), Error(String) }`
  - `pub struct ProbeResult { pub src: String, pub target: String, pub status: ProbeStatus, pub duration_ms: u64 }`
  - `pub fn probe_target(client: &SshClient, target_host: &str, target_port: u16, timeout_secs: u64) -> ProbeResult`

- [ ] **Step 1: 编写探测状态枚举与解析测试**

`tests/probe_test.rs`:
```rust
use bulk_telnet_ssh::probe::{ProbeStatus, parse_bash_output};

#[test]
fn test_parse_bash_output_mapping() {
    assert_eq!(parse_bash_output(0, ""), ProbeStatus::Pong);
    assert_eq!(parse_bash_output(1, "Connection refused"), ProbeStatus::Refused);
    assert_eq!(parse_bash_output(124, "timed out"), ProbeStatus::TimedOut);
    assert_eq!(parse_bash_output(1, "No route to host"), ProbeStatus::NoRoute);
}
```

- [ ] **Step 2: 运行测试验证失败**

Run: `cargo test --test probe_test`
Expected: 失败，模块未定义。

- [ ] **Step 3: 实现 Direct-TCPIP 探测引擎 (`src/probe/direct_tcp.rs`)**

```rust
use crate::ssh::SshClient;
use crate::probe::ProbeStatus;
use std::time::Duration;

pub fn probe_direct_tcp(
    client: &SshClient,
    target_host: &str,
    target_port: u16,
    timeout: Duration,
) -> Result<ProbeStatus, ()> {
    let session = client.session();
    session.set_timeout(timeout.as_millis() as u32);

    match session.channel_direct_tcpip(target_host, target_port as i32, None) {
        Ok(mut channel) => {
            let _ = channel.close();
            Ok(ProbeStatus::Pong)
        }
        Err(err) => {
            let code = err.code();
            let msg = err.message().to_lowercase();
            // SSH_ERROR_CHANNEL_CLOSED or connection failure
            if msg.contains("refused") || code == -19 /* LIBSSH2_ERROR_CHANNEL_FAILURE */ {
                Ok(ProbeStatus::Refused)
            } else if msg.contains("timeout") || code == -30 /* LIBSSH2_ERROR_TIMEOUT */ {
                Ok(ProbeStatus::TimedOut)
            } else if msg.contains("prohibited") || msg.contains("administratively") {
                // 需要降级到 bash
                Err(())
            } else {
                Ok(ProbeStatus::Error(err.message().to_string()))
            }
        }
    }
}
```

- [ ] **Step 4: 实现 Bash /dev/tcp 降级引擎 (`src/probe/bash_tcp.rs`)**

```rust
use crate::ssh::SshClient;
use crate::probe::ProbeStatus;
use std::io::Read;

pub fn parse_bash_output(exit_status: i32, output: &str) -> ProbeStatus {
    let out_lower = output.to_lowercase();
    if exit_status == 0 {
        ProbeStatus::Pong
    } else if out_lower.contains("refused") {
        ProbeStatus::Refused
    } else if exit_status == 124 || out_lower.contains("timed out") || out_lower.contains("killed") {
        ProbeStatus::TimedOut
    } else if out_lower.contains("no route") || out_lower.contains("unreachable") {
        ProbeStatus::NoRoute
    } else {
        ProbeStatus::Error(output.trim().to_string())
    }
}

pub fn probe_bash_tcp(
    client: &SshClient,
    target_host: &str,
    target_port: u16,
    timeout_secs: u64,
) -> ProbeStatus {
    let session = client.session();
    let mut channel = match session.channel_session() {
        Ok(c) => c,
        Err(e) => return ProbeStatus::Error(format!("Failed to open session channel: {}", e)),
    };

    let cmd = format!(
        "timeout --signal=9 {} bash -c 'exec 3<>/dev/tcp/{}/{}' 2>&1",
        timeout_secs, target_host, target_port
    );

    if let Err(e) = channel.exec(&cmd) {
        return ProbeStatus::Error(format!("Exec failed: {}", e));
    }

    let mut output = String::new();
    let _ = channel.read_to_string(&mut output);
    let _ = channel.wait_close();
    let exit_status = channel.exit_status().unwrap_or(1);

    parse_bash_output(exit_status, &output)
}
```

- [ ] **Step 5: 封装整合调度引擎 (`src/probe/engine.rs` 与 `src/probe/mod.rs`)**

`src/probe/engine.rs`:
```rust
use crate::probe::{bash_tcp, direct_tcp, ProbeResult, ProbeStatus};
use crate::ssh::SshClient;
use std::time::{Duration, Instant};

pub fn probe_target(
    client: &SshClient,
    target_host: &str,
    target_port: u16,
    timeout_secs: u64,
) -> ProbeResult {
    let start = Instant::now();
    let target = format!("{}:{}", target_host, target_port);

    let status = match direct_tcp::probe_direct_tcp(
        client,
        target_host,
        target_port,
        Duration::from_secs(timeout_secs),
    ) {
        Ok(st) => st,
        Err(_) => {
            // 降级为 bash /dev/tcp
            bash_tcp::probe_bash_tcp(client, target_host, target_port, timeout_secs)
        }
    };

    ProbeResult {
        src: client.addr.clone(),
        target,
        status,
        duration_ms: start.elapsed().as_millis() as u64,
    }
}
```

- [ ] **Step 6: 运行测试并验证通过**

Run: `cargo test --test probe_test`
Expected: PASS.

- [ ] **Step 7: 提交代码**

```bash
git add src/probe/ tests/probe_test.rs
git commit -m "feat: implement dual-engine probing with direct-tcpip and bash fallback"
```

---

### Task 5: 异步并发任务调度器 (`src/runner.rs`)

**Files:**
- Create: `src/runner.rs`

**Interfaces:**
- Produces:
  - `pub struct TaskRunner { ... }`
  - `TaskRunner::new(config: Config) -> Self`
  - `pub async fn run(&self) -> Vec<ProbeResult>`

- [ ] **Step 1: 实现 TaskRunner 与 Semaphore 并发调度**

`src/runner.rs`:
- 首先异步并发连接所有源主机（`Config::resolved_ips()`），使用 3 秒握手超时，最多重试 3 次。连接成功的源主机存入缓存映射，失败的记录其源 IP 并直接生成对应的失败结果。
- 构造 `(SshClient, target_host, target_port)` 探测任务列表。
- 初始化 `Arc<Semaphore>`，并发度设置为 `config.auth.concurrency`。
- 使用 `tokio::spawn` 配合 `tokio::task::spawn_blocking` 并发派发探测任务，并在完成后将结果发送至结果接收通道。
- 收集所有探测结果。

- [ ] **Step 2: 编译与代码检查**

Run: `cargo check`
Expected: 编译通过无 Warning/Error。

- [ ] **Step 3: 提交代码**

```bash
git add src/runner.rs
git commit -m "feat: implement concurrent task runner with semaphore throttling"
```

---

### Task 6: 日志兼容记录与终端结构化统计汇总 (`src/reporter.rs`)

**Files:**
- Create: `src/reporter.rs`

**Interfaces:**
- Produces:
  - `pub struct Reporter { log_path: PathBuf }`
  - `Reporter::record(&self, result: &ProbeResult)`：实时写入 `ssh.log` 并打印彩色终端日志。
  - `Reporter::print_summary(&self, results: &[ProbeResult])`：打印美化对齐的汇总表格与最终结论。

- [ ] **Step 1: 编写 reporter.rs 保持旧版关键字格式兼容**

旧版格式映射：
- `Pong` -> `成功 <src> pong <target>`
- `Refused` -> `失败 <src> refused <target>`
- `TimedOut` -> `失败 <src> timed out <target>`
- `NoRoute` -> `失败 <src> no route <target>`
- `SshLoginFailed(err)` -> `login false <src>, err: <err>`
- `Error(err)` -> `失败+++ <src>-><target>: <err>`

在终端输出清晰彩色的状态：
`[✓ 成功] 172.20.189.75 -> 172.20.189.75:22 (pong, 12ms)`
`[✗ 拒绝] 172.20.189.75 -> 172.20.189.75:21 (refused, 2ms)`
`[✗ 超时] 172.20.189.75 -> 172.20.189.75:23 (timed out, 3000ms)`

打印汇总统计报告（包含总数、通过数、失败数、成功率与判定结果）：
- 全通：`策略全部检测通过`
- 存在失败：`策略部分成功，请检查失败的日志`
- 存在严重连接错误：`存在错误请检查日志`

- [ ] **Step 2: 编写测试验证日志格式输出**

- [ ] **Step 3: 提交代码**

```bash
git add src/reporter.rs
git commit -m "feat: implement reporter with legacy log format and structured summary table"
```

---

### Task 7: CLI 集成、主程序编排与端到端测试 (`src/main.rs`)

**Files:**
- Modify: `src/main.rs`
- Create: `tests/cli_test.rs`

- [ ] **Step 1: 使用 clap 实现 CLI 参数支持**

支持参数：
`-c, --config <FILE>`（默认 `ips.yml`）
`-l, --log <FILE>`（默认 `ssh.log`）
`--concurrency <NUM>`（可选覆盖配置文件的并发数）

- [ ] **Step 2: 编排主流程**

1. 解析命令行参数。
2. 加载 `ips.yml`。
3. 启动 `runner` 并实时通过 `reporter` 输出日志与存盘。
4. 打印最终 Summary。
5. 退出返回码（全部成功返回 0，存在失败返回 1）。

- [ ] **Step 3: 运行完整端到端测试与本地验证**

Run: `cargo test`
Expected: 全测项通过。

- [ ] **Step 4: 提交代码**

```bash
git add src/main.rs tests/cli_test.rs
git commit -m "feat: integrate CLI arguments and runner orchestration"
```

---

### Task 8: 完善 README 文档与收尾清理

**Files:**
- Modify: `README.md`
- Clean: 标记/整理旧版 Go 代码或在文档中说明双实现并存/Rust 重构版本特性。

- [ ] **Step 1: 更新 README.md 说明 Rust 重构版用法、优势与原理解析**
- [ ] **Step 2: 运行 cargo build --release 验证二进制构建无误**
- [ ] **Step 3: 提交代码**

```bash
git add README.md
git commit -m "docs: update README for Rust refactored version"
```
