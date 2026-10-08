use crate::probe::{bash_tcp, direct_tcp, ProbeResult};
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

    // 首先尝试 SSH direct-tcpip 协议通道探测（最高效，无远程依赖）
    let status = match direct_tcp::probe_direct_tcp(
        client,
        target_host,
        target_port,
        Duration::from_secs(timeout_secs),
    ) {
        Ok(st) => st,
        Err(_) => {
            // 降级为远程执行 bash /dev/tcp
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
