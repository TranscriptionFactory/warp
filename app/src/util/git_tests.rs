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

/// Regression tests for the local arm of the target-taking helpers. These pin
/// the observable contract the Code Review panel depends on: untracked files are
/// counted, staged/unstaged selection is honored, and commit/push mutate the
/// local repository — i.e. the remote plumbing did not change local behavior.
#[cfg(feature = "local_fs")]
mod local_target {
    use super::super::{get_file_change_entries, run_commit, run_push, GitExecTarget};
    use super::{git, init_repo};

    #[tokio::test]
    async fn file_change_entries_count_untracked_and_respect_staged_only() {
        let (_dir, repo) = init_repo().await;
        let target = GitExecTarget::local(repo.clone());

        std::fs::write(repo.join("tracked.txt"), "a\nb\nc\n").expect("write tracked");
        std::fs::write(repo.join("untracked.txt"), "one\ntwo\n").expect("write untracked");
        git(&repo, &["add", "tracked.txt"]).await;

        let all = get_file_change_entries(&target, true)
            .await
            .expect("all changes");
        let all: Vec<(&str, usize)> = all
            .iter()
            .map(|entry| (entry.path.as_str(), entry.additions))
            .collect();
        assert!(all.contains(&("tracked.txt", 3)), "got {all:?}");
        assert!(
            all.contains(&("untracked.txt", 2)),
            "untracked line count missing from {all:?}"
        );

        let staged_only = get_file_change_entries(&target, false)
            .await
            .expect("staged changes");
        assert_eq!(
            staged_only.iter().map(|e| e.path.as_str()).collect::<Vec<_>>(),
            vec!["tracked.txt"],
            "staged-only view must exclude the untracked file"
        );
    }

    #[tokio::test]
    async fn commit_and_push_run_against_the_local_working_copy() {
        let (_dir, repo) = init_repo().await;
        let target = GitExecTarget::local(repo.clone());

        std::fs::write(repo.join("note.txt"), "hello\n").expect("write note");
        run_commit(&target, "add note", true, None)
            .await
            .expect("commit should succeed");
        assert_eq!(git(&repo, &["log", "-1", "--format=%s"]).await, "add note");

        git(&repo, &["init", "--bare", "origin.git"]).await;
        git(&repo, &["remote", "add", "origin", "origin.git"]).await;
        git(&repo, &["checkout", "-b", "feature"]).await;
        std::fs::write(repo.join("note.txt"), "hello again\n").expect("edit note");
        run_commit(&target, "edit note", true, None)
            .await
            .expect("commit should succeed");
        run_push(&target, "feature", None)
            .await
            .expect("push should succeed");

        let upstream = git(
            &repo,
            &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
        )
        .await;
        assert_eq!(upstream, "origin/feature");
    }
}

/// Tests for the remote git-execution path (shell quoting + output mapping).
/// These cover the chokepoint logic without a live `RemoteServerClient`.
#[cfg(feature = "local_fs")]
mod remote_exec {
    use super::super::{
        build_remote_command, build_remote_git_command, map_remote_output, shell_quote,
    };

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
    fn build_remote_command_prefixes_program_and_quotes_args() {
        assert_eq!(build_remote_command("gh", &["pr", "view"]), "gh pr view");
        assert_eq!(
            build_remote_command("gh", &["pr", "create", "--body", "hi there"]),
            "gh pr create --body 'hi there'"
        );
        // A shell that would run the command must receive each argument intact:
        // quotes, newlines, `$`, and unicode all survive the quoting.
        let body = "Fixed `foo`'s \"bar\"\ncosts $5 — vérifié ✓";
        let cmd = build_remote_command("gh", &["pr", "create", "--body", body]);
        assert_eq!(
            cmd,
            "gh pr create --body 'Fixed `foo`'\\''s \"bar\"\ncosts $5 — vérifié ✓'"
        );
        // Have a shell echo every argument NUL-delimited: each argument the
        // host would receive must come back byte-for-byte, including the
        // embedded newline in the body.
        let argv = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("printf '%s\\0' {cmd}"))
            .output()
            .expect("failed to run sh");
        let stdout = String::from_utf8(argv.stdout).expect("sh output not utf-8");
        let args: Vec<&str> = stdout.split_terminator('\0').collect();
        assert_eq!(args, vec!["gh", "pr", "create", "--body", body]);
    }

    #[test]
    fn map_remote_output_matches_local_exit_code_rules() {
        // exit 0 => ok with stdout
        assert_eq!(
            map_remote_output("Git", b"out", b"", Some(0)).unwrap(),
            "out"
        );
        // exit 0 empty => ok empty
        assert_eq!(map_remote_output("Git", b"", b"", Some(0)).unwrap(), "");
        // exit 1 with stdout => ok (git diff "differences found")
        assert_eq!(
            map_remote_output("Git", b"diff", b"", Some(1)).unwrap(),
            "diff"
        );
        // exit 1 empty stdout => error
        assert!(map_remote_output("Git", b"", b"err", Some(1)).is_err());
        // exit 2 => error even with stdout
        assert!(map_remote_output("Git", b"x", b"fatal", Some(2)).is_err());
        // killed by signal (None exit code) => error
        assert!(map_remote_output("Git", b"x", b"", None).is_err());
        // the label names the program so the failure classifier can word it
        let err = map_remote_output("gh", b"", b"gh: not found", Some(127)).unwrap_err();
        assert!(err.to_string().contains("gh command failed"));
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn map_remote_output_preserves_binary_and_null_payloads() {
        // Binary-diff marker passes through unchanged so the binary-file
        // detection above the chokepoint behaves identically.
        let binary = b"Binary files a/img.png and b/img.png differ\n";
        let out = map_remote_output("Git", binary, b"", Some(1)).unwrap();
        assert!(out.contains("Binary files ") && out.contains(" differ"));

        // `-z` NUL-delimited status payload round-trips byte-for-byte so the
        // downstream null-split parsing sees identical input.
        let status = b"1 .M N... 100644 100644 100644 aaa bbb file one.txt\0";
        let out = map_remote_output("Git", status, b"", Some(0)).unwrap();
        assert_eq!(out.as_bytes(), status);
    }
}

/// Proto-level proof that the write/PR paths emit the exact command a remote
/// host would run. Uses a mock server over a duplex stream (the pattern from
/// `crates/remote_server/src/client_tests.rs`) instead of a live host.
#[cfg(feature = "local_fs")]
mod remote_commands {
    use std::sync::{Arc, Mutex};

    use async_compat::CompatExt;
    use futures::io::{AsyncRead, AsyncWrite};
    use remote_server::client::RemoteServerClient;
    use remote_server::proto::{
        client_message, run_command_response, server_message, RunCommandResponse,
        RunCommandSuccess, ServerMessage,
    };
    use remote_server::protocol;
    use warp_core::SessionId;
    use warpui::r#async::executor;

    use super::super::{create_pr, get_pr_for_branch, run_commit, run_push, GitExecTarget};

    /// Serves a duplex stream: records every command, always answers exit 0 with
    /// `stdout`.
    async fn serve(
        mut reader: impl AsyncRead + Unpin,
        mut writer: impl AsyncWrite + Unpin,
        stdout: String,
        commands: Arc<Mutex<Vec<String>>>,
    ) {
        loop {
            let msg = match protocol::read_client_message(&mut reader).await {
                Ok(msg) => msg,
                Err(protocol::ProtocolError::UnexpectedEof) => break,
                Err(e) => panic!("mock server error: {e}"),
            };
            let command = match &msg.message {
                Some(client_message::Message::RunCommand(req)) => req.command.clone(),
                other => panic!("expected RunCommand, got {other:?}"),
            };
            commands.lock().expect("command log poisoned").push(command);
            let response = ServerMessage {
                request_id: msg.request_id.clone(),
                message: Some(server_message::Message::RunCommandResponse(
                    RunCommandResponse {
                        result: Some(run_command_response::Result::Success(RunCommandSuccess {
                            stdout: stdout.clone().into_bytes(),
                            stderr: Vec::new(),
                            exit_code: Some(0),
                        })),
                    },
                )),
            };
            protocol::write_server_message(&mut writer, &response)
                .await
                .expect("failed to write response");
        }
    }

    /// Builds a remote target backed by a mock server that answers every command
    /// with `stdout` and exit 0. Returns the target, the recorded commands, and
    /// the background executor that must outlive the test body.
    fn mock_remote_target(
        stdout: &str,
    ) -> (
        GitExecTarget,
        Arc<Mutex<Vec<String>>>,
        executor::Background,
    ) {
        let commands: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let (client_stream, server_stream) = tokio::io::duplex(4096);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let (client_read, client_write) = tokio::io::split(client_stream);

        let server_commands = commands.clone();
        let server_stdout = stdout.to_string();
        tokio::spawn(async move {
            serve(
                server_read.compat(),
                server_write.compat(),
                server_stdout,
                server_commands,
            )
            .await;
        });

        let executor = executor::Background::default();
        let (client, _events) =
            RemoteServerClient::new(client_read.compat(), client_write.compat(), &executor);
        let target = GitExecTarget::Remote {
            client: Arc::new(client),
            session_id: SessionId::from(7u64),
            repo_path: "/srv/repo".to_string(),
            host: "dev@box".to_string(),
        };
        (target, commands, executor)
    }

    #[tokio::test]
    async fn commit_runs_on_the_remote_host() {
        let (target, commands, _executor) = mock_remote_target("");
        run_commit(&target, "fix: on remote", true, Some("/local/bin"))
            .await
            .expect("commit should succeed");

        let commands = commands.lock().expect("command log poisoned").clone();
        assert_eq!(
            commands,
            vec![
                "git -c diff.autoRefreshIndex=false add -A".to_string(),
                "git -c diff.autoRefreshIndex=false commit -m 'fix: on remote'".to_string(),
            ],
            "commit must stage and commit on the host, and must not forward the local PATH"
        );
    }

    #[tokio::test]
    async fn push_targets_origin_on_the_remote_host() {
        let (target, commands, _executor) = mock_remote_target("");
        run_push(&target, "feature/x", None)
            .await
            .expect("push should succeed");

        let commands = commands.lock().expect("command log poisoned").clone();
        assert_eq!(
            commands,
            vec![
                "git -c diff.autoRefreshIndex=false push --set-upstream origin feature/x"
                    .to_string()
            ]
        );
    }

    #[tokio::test]
    async fn pr_lookup_asks_gh_on_the_remote_host() {
        let (target, commands, _executor) =
            mock_remote_target(r#"{"number":42,"url":"https://example.com/pr/42"}"#);
        let pr = get_pr_for_branch(&target, Some("/local/bin"))
            .await
            .expect("lookup should succeed")
            .expect("expected PR info");
        assert_eq!(pr.number, 42);
        assert_eq!(pr.url, "https://example.com/pr/42");

        let commands = commands.lock().expect("command log poisoned").clone();
        assert_eq!(commands, vec!["gh pr view --json number,url".to_string()]);
    }

    #[tokio::test]
    async fn create_pr_quotes_title_and_body_for_the_host_shell() {
        let (target, commands, _executor) =
            mock_remote_target("https://github.com/acme/repo/pull/7\n");
        let pr = create_pr(
            &target,
            Some("Fix the thing"),
            Some("Body with 'quotes', \"double\", $VAR and a\nnewline"),
            None,
        )
        .await
        .expect("create_pr should succeed");
        assert_eq!(pr.number, 7);

        let commands = commands.lock().expect("command log poisoned").clone();
        // Branch detection runs first on the host; the last command is the
        // `gh pr create` whose quoting matters.
        assert_eq!(
            commands.last().map(String::as_str),
            Some(
                "gh pr create --base main --title 'Fix the thing' --body \
                 'Body with '\\''quotes'\\'', \"double\", $VAR and a\nnewline'"
            ),
            "got commands: {commands:?}"
        );
    }
}
