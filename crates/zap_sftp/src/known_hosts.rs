//! known_hosts 主机密钥校验
//!
//! 按 OpenSSH 的 known_hosts 格式核对服务器主机密钥:支持哈希主机名(`|1|salt|hash`)、
//! `[host]:port`、`*`/`?` 通配与 `!` 否定、`@revoked` 标记。`@cert-authority` 行不支持,跳过。
//! 不用 libssh2 自带的 knownhost 检查:它遇到无法解析的行会整份文件读取失败,
//! 且不区分密钥类型,会把"类型不同"误报成"密钥不匹配"。

use std::path::{Path, PathBuf};

use base64::Engine;
use hmac::{Hmac, Mac};
use sha1::Sha1;

/// OpenSSH 默认的 host key 算法顺序(libssh2 方法名)
const DEFAULT_HOST_KEY_METHODS: [&str; 7] = [
    "ssh-ed25519",
    "ecdsa-sha2-nistp256",
    "ecdsa-sha2-nistp384",
    "ecdsa-sha2-nistp521",
    "rsa-sha2-512",
    "rsa-sha2-256",
    "ssh-rsa",
];

/// known_hosts 中与目标主机匹配的一条记录
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownKey {
    pub key_type: String,
    pub key: Vec<u8>,
    pub revoked: bool,
}

/// 默认读取的 known_hosts 文件:用户级 + 系统级
pub fn default_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Some(home) = dirs::home_dir() {
        files.push(home.join(".ssh").join("known_hosts"));
    }
    files.push(PathBuf::from("/etc/ssh/ssh_known_hosts"));
    files
}

/// known_hosts 里的主机名写法:22 端口为 `host`,其他端口为 `[host]:port`
pub fn host_entry_name(host: &str, port: u16) -> String {
    if port == 22 {
        host.to_string()
    } else {
        format!("[{host}]:{port}")
    }
}

/// 从若干 known_hosts 文件中收集目标主机的所有记录;不存在或不可读的文件跳过
pub fn lookup(files: &[PathBuf], host: &str, port: u16) -> Vec<KnownKey> {
    let name = host_entry_name(host, port);
    files
        .iter()
        .filter_map(|f| read_file(f))
        .flat_map(|text| {
            text.lines()
                .filter_map(|line| parse_line(line, &name))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn read_file(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// 解析一行 known_hosts;主机不匹配、注释、无法解析的行返回 None
pub fn parse_line(line: &str, name: &str) -> Option<KnownKey> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let mut fields = line.split_whitespace();
    let mut hosts = fields.next()?;
    let mut revoked = false;
    if let Some(marker) = hosts.strip_prefix('@') {
        if marker != "revoked" {
            return None;
        }
        revoked = true;
        hosts = fields.next()?;
    }
    let key_type = fields.next()?;
    let key = base64::engine::general_purpose::STANDARD
        .decode(fields.next()?)
        .ok()?;
    hosts_match(hosts, name).then(|| KnownKey {
        key_type: key_type.to_string(),
        key,
        revoked,
    })
}

fn hosts_match(patterns: &str, name: &str) -> bool {
    if let Some(hashed) = patterns.strip_prefix("|1|") {
        return hashed_match(hashed, name);
    }
    let mut matched = false;
    for pattern in patterns.split(',') {
        if let Some(negated) = pattern.strip_prefix('!') {
            if glob_match(negated, name) {
                return false;
            }
        } else if glob_match(pattern, name) {
            matched = true;
        }
    }
    matched
}

/// 哈希主机名:`|1|base64(salt)|base64(HMAC-SHA1(salt, name))`
fn hashed_match(hashed: &str, name: &str) -> bool {
    let Some((salt, hash)) = hashed.split_once('|') else {
        return false;
    };
    let engine = &base64::engine::general_purpose::STANDARD;
    let (Ok(salt), Ok(hash)) = (engine.decode(salt), engine.decode(hash)) else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha1>::new_from_slice(&salt) else {
        return false;
    };
    mac.update(name.as_bytes());
    mac.verify_slice(&hash).is_ok()
}

/// `*` / `?` 通配,大小写不敏感(与 OpenSSH 一致)
fn glob_match(pattern: &str, name: &str) -> bool {
    fn matches(p: &[u8], n: &[u8]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (Some(b'*'), _) => matches(&p[1..], n) || (!n.is_empty() && matches(p, &n[1..])),
            (Some(b'?'), Some(_)) => matches(&p[1..], &n[1..]),
            (Some(a), Some(b)) if a.eq_ignore_ascii_case(b) => matches(&p[1..], &n[1..]),
            _ => false,
        }
    }
    matches(pattern.as_bytes(), name.as_bytes())
}

/// libssh2 方法名对应的 known_hosts 密钥类型
fn key_type_of_method(method: &str) -> &str {
    if method.starts_with("rsa-sha2-") {
        "ssh-rsa"
    } else {
        method
    }
}

/// host key 算法偏好:known_hosts 里已有的类型排前面(与 OpenSSH 行为一致),
/// 避免服务器用另一种类型的密钥应答而被误判为密钥不匹配
pub fn host_key_prefs(known: &[KnownKey]) -> String {
    let is_known = |method: &str| {
        known
            .iter()
            .any(|k| !k.revoked && k.key_type == key_type_of_method(method))
    };
    let (mut prefs, rest): (Vec<&str>, Vec<&str>) =
        DEFAULT_HOST_KEY_METHODS.iter().partition(|m| is_known(m));
    prefs.extend(rest);
    prefs.join(",")
}

/// 核对服务器出示的主机密钥;Err 为给用户看的原因
pub fn verify(known: &[KnownKey], host: &str, port: u16, presented: &[u8]) -> Result<(), String> {
    let name = host_entry_name(host, port);
    if known.iter().any(|k| k.revoked && k.key == presented) {
        return Err(format!("the host key for {name} is marked @revoked in known_hosts"));
    }
    if known.iter().any(|k| !k.revoked && k.key == presented) {
        return Ok(());
    }
    if known.iter().any(|k| !k.revoked) {
        Err(format!(
            "the host key for {name} does not match known_hosts. Someone may be \
             impersonating the server. If you expected the key to change, remove the \
             old entry with `ssh-keygen -R '{name}'` and connect once with ssh."
        ))
    } else {
        Err(format!(
            "{name} is not in known_hosts. Connect once with ssh in a terminal to \
             verify and save its host key, then retry."
        ))
    }
}
