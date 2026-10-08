use bulk_telnet_ssh::config::{normalize_host_port, Config};

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
    assert_eq!(cfg.auth.password, "secret");
    assert_eq!(cfg.auth.port, 22);
    assert_eq!(cfg.auth.concurrency, 16);
    let resolved_ips = cfg.resolved_ips().unwrap();
    assert_eq!(resolved_ips.len(), 2);
    assert_eq!(resolved_ips[0], ("10.0.0.1".to_string(), 22));
    assert_eq!(resolved_ips[1], ("10.0.0.2".to_string(), 2222));

    let resolved_targets = cfg.resolved_targets().unwrap();
    assert_eq!(resolved_targets.len(), 1);
    assert_eq!(resolved_targets[0], ("192.168.1.1".to_string(), 80));
}

#[test]
fn test_parse_existing_ips_yml() {
    let cfg = Config::load_from_file("ips.yml").unwrap();
    assert_eq!(cfg.auth.user, "root");
    assert_eq!(cfg.auth.port, 2222);
    assert_eq!(cfg.auth.concurrency, 29);
    assert_eq!(cfg.ips.len(), 2);
    assert_eq!(cfg.target.len(), 3);
}
