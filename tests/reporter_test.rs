use bulk_telnet_ssh::probe::{ProbeResult, ProbeStatus};
use bulk_telnet_ssh::reporter::{format_legacy_log, Reporter};
use std::fs;

#[test]
fn test_format_legacy_log_contains_keywords() {
    let r1 = ProbeResult {
        src: "10.0.0.1:22".to_string(),
        target: "192.168.1.1:80".to_string(),
        status: ProbeStatus::Pong,
        duration_ms: 10,
    };
    let line1 = format_legacy_log(&r1);
    assert!(line1.contains("成功 10.0.0.1:22 pong 192.168.1.1:80"));

    let r2 = ProbeResult {
        src: "10.0.0.1:22".to_string(),
        target: "192.168.1.1:81".to_string(),
        status: ProbeStatus::Refused,
        duration_ms: 2,
    };
    let line2 = format_legacy_log(&r2);
    assert!(line2.contains("失败 10.0.0.1:22 refused 192.168.1.1:81"));

    let r3 = ProbeResult {
        src: "10.0.0.2:22".to_string(),
        target: "192.168.1.1:80".to_string(),
        status: ProbeStatus::SshLoginFailed("handshake failed".to_string()),
        duration_ms: 0,
    };
    let line3 = format_legacy_log(&r3);
    assert!(line3.contains("login false 10.0.0.2:22, err:handshake failed"));
}

#[test]
fn test_reporter_file_logging() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let log_file = tmp_dir.path().join("test_ssh.log");

    let reporter = Reporter::new(&log_file);
    let r = ProbeResult {
        src: "127.0.0.1:22".to_string(),
        target: "127.0.0.1:8080".to_string(),
        status: ProbeStatus::Pong,
        duration_ms: 5,
    };
    reporter.record(&r);

    let content = fs::read_to_string(&log_file).unwrap();
    assert!(content.contains("成功 127.0.0.1:22 pong 127.0.0.1:8080"));
}
