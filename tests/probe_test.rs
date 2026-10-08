use bulk_telnet_ssh::probe::{parse_bash_output, ProbeStatus};

#[test]
fn test_parse_bash_output_mapping() {
    assert_eq!(parse_bash_output(0, ""), ProbeStatus::Pong);
    assert_eq!(
        parse_bash_output(1, "bash: connect: Connection refused"),
        ProbeStatus::Refused
    );
    assert_eq!(parse_bash_output(124, "timed out"), ProbeStatus::TimedOut);
    assert_eq!(parse_bash_output(137, "Killed"), ProbeStatus::TimedOut);
    assert_eq!(
        parse_bash_output(1, "bash: connect: No route to host"),
        ProbeStatus::NoRoute
    );
    assert_eq!(
        parse_bash_output(127, "bash: line 1: /dev/tcp/1.1.1.1/80: No such file or directory"),
        ProbeStatus::Error("bash: line 1: /dev/tcp/1.1.1.1/80: No such file or directory".to_string())
    );
}

#[test]
fn test_probe_status_format_compatibility() {
    let s1 = ProbeStatus::Pong;
    assert_eq!(s1.legacy_keyword(), "pong");
    assert!(s1.is_success());

    let s2 = ProbeStatus::Refused;
    assert_eq!(s2.legacy_keyword(), "refused");
    assert!(!s2.is_success());

    let s3 = ProbeStatus::TimedOut;
    assert_eq!(s3.legacy_keyword(), "timed out");
    assert!(!s3.is_success());

    let s4 = ProbeStatus::NoRoute;
    assert_eq!(s4.legacy_keyword(), "no route");
    assert!(!s4.is_success());
}
