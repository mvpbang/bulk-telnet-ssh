use crate::probe::ProbeStatus;
use crate::ssh::SshClient;
use std::io::Read;

pub fn parse_bash_output(exit_status: i32, output: &str) -> ProbeStatus {
    let out_lower = output.to_lowercase();
    if exit_status == 0 {
        ProbeStatus::Pong
    } else if out_lower.contains("refused") {
        ProbeStatus::Refused
    } else if exit_status == 124
        || exit_status == 137
        || out_lower.contains("timed out")
        || out_lower.contains("killed")
    {
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
    client.with_session(|sess| {
        let mut channel = match sess.channel_session() {
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
        let _ = channel.close();
        let _ = channel.wait_close();
        let exit_status = channel.exit_status().unwrap_or(1);

        parse_bash_output(exit_status, &output)
    })
}
