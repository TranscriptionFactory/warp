use std::path::Path;

use command::r#async::Command;
use command::Stdio;
use tempfile::TempDir;

use super::{detect_current_branch, detect_current_branch_display, GitExecTarget};

/// Helper: run a git command inside the given repo directory.
async fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("failed to run git");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// Creates a temp git repo with one commit and returns `(dir_handle, repo_path)`.
async fn init_repo() -> (TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let path = dir.path().to_path_buf();

    git(&path, &["init", "-b", "main"]).await;
    git(&path, &["config", "user.email", "test@test.com"]).await;
    git(&path, &["config", "user.name", "Test"]).await;
    git(&path, &["commit", "--allow-empty", "-m", "initial"]).await;

    (dir, path)
}

#[tokio::test]
async fn on_normal_branch_returns_branch_name() {
    let (_dir, repo) = init_repo().await;
    git(&repo, &["checkout", "-b", "feature-xyz"]).await;

    assert_eq!(
        detect_current_branch(&GitExecTarget::local(repo.clone()))
            .await
            .unwrap(),
        "feature-xyz"
    );
    assert_eq!(
        detect_current_branch_display(&repo).await.unwrap(),
        "feature-xyz"
    );
}

#[tokio::test]
async fn detached_head_raw_returns_head() {
    let (_dir, repo) = init_repo().await;
    git(&repo, &["checkout", "--detach", "HEAD"]).await;

    assert_eq!(
        detect_current_branch(&GitExecTarget::local(repo.clone()))
            .await
            .unwrap(),
        "HEAD"
    );
}

#[tokio::test]
async fn detached_head_display_returns_short_sha() {
    let (_dir, repo) = init_repo().await;
    let full_sha = git(&repo, &["rev-parse", "HEAD"]).await;
    git(&repo, &["checkout", "--detach", "HEAD"]).await;

    let result = detect_current_branch_display(&repo).await.unwrap();

    assert_ne!(
        result, "HEAD",
        "display variant should not return literal HEAD"
    );
    assert!(
        full_sha.starts_with(&result),
        "expected {full_sha} to start with {result}"
    );
}

#[tokio::test]
async fn detached_tag_display_returns_short_sha() {
    let (_dir, repo) = init_repo().await;
    git(&repo, &["tag", "v1.0"]).await;
    git(&repo, &["checkout", "v1.0"]).await;

    let full_sha = git(&repo, &["rev-parse", "HEAD"]).await;
    let result = detect_current_branch_display(&repo).await.unwrap();

    assert_ne!(result, "HEAD");
    assert!(
        full_sha.starts_with(&result),
        "expected {full_sha} to start with {result}"
    );
}

/// Tests for the remote git-execution path (shell quoting + output mapping).
/// These cover the chokepoint logic without a live `RemoteServerClient`.
#[cfg(feature = "local_fs")]
mod remote_exec {
    use super::super::{build_remote_git_command, map_remote_git_output, shell_quote};

    /// Round-trips `arg` through a POSIX shell: quote it, then have `sh` echo it
    /// back verbatim. Verifies the remote shell would receive exactly the same
    /// argv git would have gotten locally.
    fn sh_roundtrip(arg: &str) -> String {
        let quoted = shell_quote(arg);
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("printf %s {quoted}"))
            .output()
            .expect("failed to run sh");
        String::from_utf8(output.stdout).expect("sh output not utf-8")
    }

    #[test]
    fn shell_quote_roundtrips_through_posix_shell() {
        for arg in [
            "simple",
            "with space.txt",
            "a/b/c.rs",
            "--cached",
            "--",
            "feature/my-branch",
            "weird'quote",
            "ünîcödé.txt",
            "tab\tinside",
            "dollar$var",
            "semi;colon && rm -rf /",
            "back`tick`",
            "new\nline",
            "glob*?[chars]",
        ] {
            assert_eq!(sh_roundtrip(arg), arg, "round-trip failed for {arg:?}");
        }
    }

    #[test]
    fn shell_quote_passes_safe_args_unquoted() {
        assert_eq!(shell_quote("rev-parse"), "rev-parse");
        assert_eq!(shell_quote("HEAD"), "HEAD");
        assert_eq!(shell_quote("origin/main"), "origin/main");
        assert_eq!(shell_quote("a.b_c-1"), "a.b_c-1");
    }

    #[test]
    fn shell_quote_quotes_empty_arg() {
        assert_eq!(shell_quote(""), "''");
        assert_eq!(sh_roundtrip(""), "");
    }

    #[test]
    fn build_remote_git_command_quotes_each_arg() {
        let cmd = build_remote_git_command(&["diff", "--", "a b.txt"]);
        assert_eq!(cmd, "git -c diff.autoRefreshIndex=false diff -- 'a b.txt'");
    }

    #[test]
    fn map_remote_output_matches_local_exit_code_rules() {
        // exit 0 => ok with stdout
        assert_eq!(map_remote_git_output(b"out", b"", Some(0)).unwrap(), "out");
        // exit 0 empty => ok empty
        assert_eq!(map_remote_git_output(b"", b"", Some(0)).unwrap(), "");
        // exit 1 with stdout => ok (git diff "differences found")
        assert_eq!(
            map_remote_git_output(b"diff", b"", Some(1)).unwrap(),
            "diff"
        );
        // exit 1 empty stdout => error
        assert!(map_remote_git_output(b"", b"err", Some(1)).is_err());
        // exit 2 => error even with stdout
        assert!(map_remote_git_output(b"x", b"fatal", Some(2)).is_err());
        // killed by signal (None exit code) => error
        assert!(map_remote_git_output(b"x", b"", None).is_err());
    }

    #[test]
    fn map_remote_output_preserves_binary_and_null_payloads() {
        // Binary-diff marker passes through unchanged so the binary-file
        // detection above the chokepoint behaves identically.
        let binary = b"Binary files a/img.png and b/img.png differ\n";
        let out = map_remote_git_output(binary, b"", Some(1)).unwrap();
        assert!(out.contains("Binary files ") && out.contains(" differ"));

        // `-z` NUL-delimited status payload round-trips byte-for-byte so the
        // downstream null-split parsing sees identical input.
        let status = b"1 .M N... 100644 100644 100644 aaa bbb file one.txt\0";
        let out = map_remote_git_output(status, b"", Some(0)).unwrap();
        assert_eq!(out.as_bytes(), status);
    }
}
