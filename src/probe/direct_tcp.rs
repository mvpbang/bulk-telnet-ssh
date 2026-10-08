use crate::probe::ProbeStatus;
use crate::ssh::SshClient;
use std::time::Duration;

/// 使用 SSH 2.0 协议标准的 direct-tcpip 通道探测远端到目标的连通性
/// 若返回 Err(()) 则代表被远端 sshd 限制（如 AllowTcpForwarding no）或其它原因，需要降级到 bash
pub fn probe_direct_tcp(
    client: &SshClient,
    target_host: &str,
    target_port: u16,
    timeout: Duration,
) -> Result<ProbeStatus, ()> {
    client.with_session(|sess| {
        sess.set_timeout(timeout.as_millis() as u32);

        match sess.channel_direct_tcpip(target_host, target_port, None) {
            Ok(mut channel) => {
                let _ = channel.close();
                let _ = channel.wait_close();
                Ok(ProbeStatus::Pong)
            }
            Err(err) => {
                let code = err.code();
                let msg = err.message().to_lowercase();

                // 远端配置了 AllowTcpForwarding no，sshd 会返回 administratively prohibited
                if msg.contains("prohibited")
                    || msg.contains("administratively")
                    || code == ssh2::ErrorCode::Session(-18) /* LIBSSH2_ERROR_CHANNEL_REQUEST_DENIED */
                {
                    return Err(());
                }

                // 连接被目标拒绝（端口未监听）
                if msg.contains("refused") {
                    return Ok(ProbeStatus::Refused);
                }

                // 连接超时（防火墙丢包或策略阻断）
                if msg.contains("timed out")
                    || msg.contains("timeout")
                    || code == ssh2::ErrorCode::Session(-30) /* LIBSSH2_ERROR_TIMEOUT */
                {
                    return Ok(ProbeStatus::TimedOut);
                }

                // 不可达 / 无路由
                if msg.contains("no route") || msg.contains("unreachable") {
                    return Ok(ProbeStatus::NoRoute);
                }

                // 其他情况，若无法明确判断，尝试降级到 bash 验证，避免误报
                Err(())
            }
        }
    })
}
