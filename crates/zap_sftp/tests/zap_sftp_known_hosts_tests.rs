//! zap_sftp::known_hosts 模块单元测试
//!
//! 哈希记录由 `ssh-keygen -H` 生成,对应主机名 `[example.com]:2222`。

use std::io::Write;

use base64::Engine;
use zap_sftp::known_hosts::*;

const ED25519: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIBVhmBnk6F46iw1ftyibKnhjpJwE6jWCGUiylPjK4o7n";
const HASHED: &str = "|1|ygCqYZ0YWO7bJ+BPOGSvdYyzvHI=|Kh2umch2Jz5YWRyFKvvSTHkkbQE=";

fn key_bytes() -> Vec<u8> {
    base64::engine::general_purpose::STANDARD.decode(ED25519).unwrap()
}

fn known(key_type: &str, revoked: bool) -> KnownKey {
    KnownKey { key_type: key_type.to_string(), key: key_bytes(), revoked }
}

#[test]
fn test_host_entry_name() {
    assert_eq!(host_entry_name("example.com", 22), "example.com");
    assert_eq!(host_entry_name("example.com", 2222), "[example.com]:2222");
}

#[test]
fn test_parse_plain_and_patterns() {
    let line = format!("a.com,example.com ssh-ed25519 {ED25519}");
    assert_eq!(parse_line(&line, "example.com"), Some(known("ssh-ed25519", false)));
    assert_eq!(parse_line(&line, "other.com"), None);

    let glob = format!("*.example.com,!bad.example.com ssh-ed25519 {ED25519}");
    assert!(parse_line(&glob, "host.EXAMPLE.com").is_some());
    assert!(parse_line(&glob, "bad.example.com").is_none());
}

#[test]
fn test_parse_hashed() {
    let line = format!("{HASHED} ssh-ed25519 {ED25519}");
    assert!(parse_line(&line, "[example.com]:2222").is_some());
    assert!(parse_line(&line, "example.com").is_none());
}

#[test]
fn test_parse_markers_and_junk() {
    let revoked = format!("@revoked example.com ssh-ed25519 {ED25519}");
    assert_eq!(parse_line(&revoked, "example.com"), Some(known("ssh-ed25519", true)));
    let ca = format!("@cert-authority example.com ssh-ed25519 {ED25519}");
    assert_eq!(parse_line(&ca, "example.com"), None);
    assert_eq!(parse_line("# comment", "example.com"), None);
    assert_eq!(parse_line("example.com ssh-ed25519 !!notbase64", "example.com"), None);
}

#[test]
fn test_lookup_skips_unparseable_lines_and_missing_files() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    writeln!(file, "example.com sk-ssh-ed25519@openssh.com garbage").unwrap();
    writeln!(file, "{HASHED} ssh-ed25519 {ED25519}").unwrap();
    let files = vec![file.path().to_path_buf(), "/nonexistent/known_hosts".into()];
    assert_eq!(lookup(&files, "example.com", 2222), vec![known("ssh-ed25519", false)]);
    assert!(lookup(&files, "example.com", 22).is_empty());
}

#[test]
fn test_host_key_prefs_puts_known_types_first() {
    let default = host_key_prefs(&[]);
    assert!(default.starts_with("ssh-ed25519,"));

    let rsa = host_key_prefs(&[known("ssh-rsa", false)]);
    assert!(rsa.starts_with("rsa-sha2-512,rsa-sha2-256,ssh-rsa,ssh-ed25519"));

    // 被吊销的类型不提前
    assert_eq!(host_key_prefs(&[known("ssh-rsa", true)]), default);
}

#[test]
fn test_verify() {
    let key = key_bytes();
    let other = vec![1, 2, 3];
    assert!(verify(&[known("ssh-ed25519", false)], "h", 22, &key).is_ok());

    let mismatch = verify(&[known("ssh-ed25519", false)], "h", 22, &other).unwrap_err();
    assert!(mismatch.contains("does not match"), "{mismatch}");

    let unknown = verify(&[], "h", 2222, &key).unwrap_err();
    assert!(unknown.contains("[h]:2222 is not in known_hosts"), "{unknown}");

    let revoked = verify(&[known("ssh-ed25519", true)], "h", 22, &key).unwrap_err();
    assert!(revoked.contains("@revoked"), "{revoked}");
}
