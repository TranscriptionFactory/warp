use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

#[cfg(feature = "local_fs")]
use std::collections::HashMap;
#[cfg(feature = "local_fs")]
use std::sync::Arc;

#[cfg(feature = "local_fs")]
use crate::remote_server::client::RemoteServerClient;
#[cfg(feature = "local_fs")]
use warp_core::SessionId;

#[cfg(test)]
#[path = "git_tests.rs"]
mod tests;

/// Target a git command runs against: a local working copy (subprocess) or a
/// remote SSH session (commands shipped over the `remote_server` channel).
///
/// This is the single transport-agnostic chokepoint for the code-review diff
/// engine. All diff/parse/format logic above it is string-based and identical
/// for both transports. The `Remote` arm only exists on desktop builds where
/// the remote client is available (`local_fs`, i.e. `not(target_family =
/// "wasm")`); the `Local` arm exists everywhere.
#[derive(Clone)]
pub enum GitExecTarget {
    Local {
        repo_path: PathBuf,
    },
    #[cfg(feature = "local_fs")]
    Remote {
        client: Arc<RemoteServerClient>,
        session_id: SessionId,
        /// Remote repository root, rendered as a path string for the remote shell.
        repo_path: String,
        /// User-facing label for the host (e.g. `user@host`), used to word
        /// errors about host-local tooling such as `gh`.
        host: String,
    },
}

impl GitExecTarget {
    /// Convenience constructor for a local target.
    pub fn local(repo_path: PathBuf) -> Self {
        Self::Local { repo_path }
    }

    /// The local repository path, or `None` for a remote target. Used by the
    /// few diff operations that touch the filesystem directly (e.g. counting
    /// lines in untracked files) rather than shelling out to git.
    pub fn local_repo_path(&self) -> Option<&Path> {
        match self {
            Self::Local { repo_path } => Some(repo_path.as_path()),
            #[cfg(feature = "local_fs")]
            Self::Remote { .. } => None,
        }
    }

    /// A stable path used to identify the repository this target points at.
    /// For local targets this is the working-copy path; for remote targets it
    /// is the remote root rendered as a `PathBuf` (used only for equality
    /// checks, never the local filesystem).
    pub fn identity_path(&self) -> PathBuf {
        match self {
            Self::Local { repo_path } => repo_path.clone(),
            #[cfg(feature = "local_fs")]
            Self::Remote { repo_path, .. } => PathBuf::from(repo_path),
        }
    }

    /// Whether this target executes on a remote SSH host.
    pub fn is_remote(&self) -> bool {
        match self {
            Self::Local { .. } => false,
            #[cfg(feature = "local_fs")]
            Self::Remote { .. } => true,
        }
    }

    /// The remote host label (e.g. `user@host`), or `None` for local targets.
    pub fn remote_host(&self) -> Option<&str> {
        match self {
            Self::Local { .. } => None,
            #[cfg(feature = "local_fs")]
            Self::Remote { host, .. } => Some(host),
        }
    }

    /// Runs `git <args>` against this target and returns stdout as a (lossy)
    /// String. Identical contract to [`run_git_command`]: exit code 0 is ok,
    /// exit code 1 with non-empty stdout is ok (diff "differences found"),
    /// anything else is an error.
    pub async fn run_git(&self, args: &[&str]) -> Result<String> {
        self.run_git_with_env(args, None).await
    }

    /// Like [`Self::run_git`] but forwards `path_env` on the local arm so hooks
    /// can find user-installed binaries (e.g. the LFS `pre-push` hook). `path_env`
    /// exists because macOS GUI launches inherit launchd's minimal `PATH`; it is
    /// ignored on the remote arm, where the remote shell's own `PATH` applies.
    pub async fn run_git_with_env(&self, args: &[&str], path_env: Option<&str>) -> Result<String> {
        match self {
            Self::Local { repo_path } => run_git_command_with_env(repo_path, args, path_env).await,
            #[cfg(feature = "local_fs")]
            Self::Remote {
                client,
                session_id,
                repo_path,
                ..
            } => run_git_remote(client, *session_id, repo_path, args).await,
        }
    }

    /// Runs a foreign CLI against the repo at this target and returns stdout.
    ///
    /// Local: a subprocess in the repo directory; `path_env` overrides the
    /// child's `PATH` so a Homebrew-installed binary is findable from a GUI
    /// launch. Remote: one POSIX-quoted command over the session, run with the
    /// repo root as the working directory; `path_env` is ignored.
    pub async fn run_program(
        &self,
        program: &str,
        args: &[&str],
        path_env: Option<&str>,
    ) -> Result<String> {
        match self {
            Self::Local { repo_path } => {
                run_local_program(repo_path, program, args, path_env).await
            }
            #[cfg(feature = "local_fs")]
            Self::Remote {
                client,
                session_id,
                repo_path,
                ..
            } => run_remote_program(client, *session_id, repo_path, program, args).await,
        }
    }
}

/// Appends each arg to `cmd`, shell-quoted, separated by a space.
#[cfg(feature = "local_fs")]
fn push_shell_quoted_args(cmd: &mut String, args: &[&str]) {
    for arg in args {
        cmd.push(' ');
        cmd.push_str(&shell_quote(arg));
    }
}

/// Builds the shell command string for a remote invocation of an arbitrary
/// program: `<program> <quoted args...>`. Shared by the git and foreign-CLI
/// (e.g. `gh`) remote arms so quoting rules cannot drift between them.
#[cfg(feature = "local_fs")]
fn build_remote_command(program: &str, args: &[&str]) -> String {
    let mut cmd = String::from(program);
    push_shell_quoted_args(&mut cmd, args);
    cmd
}

/// Builds the shell command string for a remote git invocation:
/// `git -c diff.autoRefreshIndex=false <quoted args...>`. Each arg is
/// POSIX-shell-quoted so paths / branch names with spaces or special characters
/// survive transport to the remote shell intact.
#[cfg(feature = "local_fs")]
fn build_remote_git_command(args: &[&str]) -> String {
    let mut cmd = build_remote_command("git", &["-c", "diff.autoRefreshIndex=false"]);
    push_shell_quoted_args(&mut cmd, args);
    cmd
}

/// Maps a remote `RunCommandSuccess` payload to the same `Result<String>`
/// contract as the local subprocess path. Extracted as a pure function so the
/// exit-code / stdout rules can be unit-tested against canned bytes (binary
/// diff, `-z` null-delimited status) without a live client.
///
/// `label` names the program in the error text (e.g. `Git`, `gh`) so the
/// failure classifier can tell a missing remote `gh` from a git failure.
#[cfg(feature = "local_fs")]
fn map_remote_output(
    label: &str,
    stdout: &[u8],
    stderr: &[u8],
    exit_code: Option<i32>,
) -> Result<String> {
    let stdout = String::from_utf8_lossy(stdout).to_string();
    // Mirror the local arm: exit 0 => ok; exit 1 with non-empty stdout => ok
    // (git diff "differences found"); anything else (including death by signal,
    // where exit_code is None) => error.
    if exit_code == Some(0) || (exit_code == Some(1) && !stdout.is_empty()) {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(stderr);
        Err(anyhow!("{label} command failed: {stderr}, {stdout}"))
    }
}

/// Runs a fully-built command string on the remote host via
/// [`RemoteServerClient::run_command`]. `working_directory` replaces a local
/// `cd`; `env` is passed through as the command's environment. `label` names
/// the program in error text (see [`map_remote_output`]).
#[cfg(feature = "local_fs")]
async fn run_remote_command(
    client: &RemoteServerClient,
    session_id: SessionId,
    working_directory: &str,
    command: String,
    env: HashMap<String, String>,
    label: &str,
) -> Result<String> {
    use crate::remote_server::proto::run_command_response;

    log::debug!("[GIT OPERATION] git.rs run_remote_command {command}");

    let response = client
        .run_command(session_id, command, Some(working_directory.to_string()), env)
        .await
        .map_err(|e| anyhow!("Failed to execute remote {label} command: {e}"))?;

    match response.result {
        Some(run_command_response::Result::Success(success)) => {
            map_remote_output(label, &success.stdout, &success.stderr, success.exit_code)
        }
        Some(run_command_response::Result::Error(err)) => {
            Err(anyhow!("Remote {label} command error: {}", err.message))
        }
        None => Err(anyhow!("Remote {label} command returned an empty response")),
    }
}

/// Runs a git command on the remote host. `GIT_OPTIONAL_LOCKS=0` is passed
/// through the environment map to match the local arm.
#[cfg(feature = "local_fs")]
async fn run_git_remote(
    client: &RemoteServerClient,
    session_id: SessionId,
    repo_path: &str,
    args: &[&str],
) -> Result<String> {
    let mut env = HashMap::new();
    env.insert("GIT_OPTIONAL_LOCKS".to_string(), "0".to_string());
    run_remote_command(
        client,
        session_id,
        repo_path,
        build_remote_git_command(args),
        env,
        "Git",
    )
    .await
}

/// Runs a foreign CLI (e.g. `gh`) on the remote host. Failures are mapped with
/// the same exit-code rules as the git arm; `user_facing_git_error` recognises
/// the host-local `gh` failure wording.
#[cfg(feature = "local_fs")]
async fn run_remote_program(
    client: &RemoteServerClient,
    session_id: SessionId,
    repo_path: &str,
    program: &str,
    args: &[&str],
) -> Result<String> {
    run_remote_command(
        client,
        session_id,
        repo_path,
        build_remote_command(program, args),
        HashMap::new(),
        program,
    )
    .await
}

/// Runs a foreign CLI as a local subprocess in `repo_path`. `path_env`, when
/// `Some`, replaces the child's `PATH` so GUI-launched sessions can find
/// binaries installed outside launchd's minimal `PATH` (e.g. Homebrew's `gh`).
#[cfg(feature = "local_fs")]
async fn run_local_program(
    repo_path: &Path,
    program: &str,
    args: &[&str],
    path_env: Option<&str>,
) -> Result<String> {
    use command::r#async::Command;
    use command::Stdio;

    log::debug!("[GIT OPERATION] git.rs run_local_program {program} {}", args.join(" "));

    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(repo_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("HOMEBREW_NO_AUTO_UPDATE", "1")
        .kill_on_drop(true);
    if let Some(path_env) = path_env {
        cmd.env("PATH", path_env);
    }
    let output = cmd
        .output()
        .await
        .map_err(|e| anyhow!("Failed to execute {program} command: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(anyhow!("{program} command failed: {stderr}"))
    }
}

#[cfg(not(feature = "local_fs"))]
async fn run_local_program(
    _repo_path: &Path,
    _program: &str,
    _args: &[&str],
    _path_env: Option<&str>,
) -> Result<String> {
    Err(anyhow!("Not supported on wasm"))
}

/// POSIX single-quote escaping for one shell argument. An argument made up
/// entirely of "safe" characters is passed through unquoted; everything else
/// (spaces, `--`-prefixed args containing specials, unicode, etc.) is wrapped
/// in single quotes with embedded `'` replaced by the `'\''` sequence.
#[cfg(feature = "local_fs")]
fn shell_quote(arg: &str) -> String {
    fn is_safe(b: u8) -> bool {
        matches!(b,
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9'
            | b'-' | b'_' | b'/' | b'.' | b',' | b'=' | b':' | b'@' | b'+' | b'%')
    }
    if !arg.is_empty() && arg.bytes().all(is_safe) {
        return arg.to_string();
    }
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('\'');
    for ch in arg.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

/// Runs a git command and returns the output as a string.
/// Thin wrapper over [`run_git_command_with_env`] with no `PATH` override.
#[cfg(feature = "local_fs")]
pub async fn run_git_command(repo_path: &Path, args: &[&str]) -> Result<String> {
    run_git_command_with_env(repo_path, args, None).await
}

/// Like [`run_git_command`] but sets `PATH` on the child when `path_env` is
/// `Some`. Used by callers whose hooks need user-installed binaries (e.g.
/// the LFS `pre-push` hook → `git-lfs`). See `specs/APP-4188/TECH.md`.
#[cfg(feature = "local_fs")]
pub async fn run_git_command_with_env(
    repo_path: &Path,
    args: &[&str],
    path_env: Option<&str>,
) -> Result<String> {
    use command::r#async::Command;
    use command::Stdio;

    log::debug!(
        "[GIT OPERATION] git.rs run_git_command git {}",
        args.join(" ")
    );
    let mut cmd = Command::new("git");
    cmd.arg("-c")
        .arg("diff.autoRefreshIndex=false")
        .args(args)
        .current_dir(repo_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_OPTIONAL_LOCKS", "0")
        .kill_on_drop(true);
    if let Some(path_env) = path_env {
        cmd.env("PATH", path_env);
    }
    let output = cmd
        .output()
        .await
        .map_err(|e| anyhow!("Failed to execute git command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Handle git diff specific behavior:
    // - Exit code 0: no differences
    // - Exit code 1: differences found (this is normal for diff commands)
    // - Exit code > 1: actual error
    if output.status.success() || (output.status.code() == Some(1) && !stdout.is_empty()) {
        Ok(stdout)
    } else {
        Err(anyhow!("Git command failed: {}, {}", stderr, stdout))
    }
}

#[cfg(not(feature = "local_fs"))]
pub async fn run_git_command(_repo_path: &Path, _args: &[&str]) -> Result<String> {
    Err(anyhow!("Not supported on wasm"))
}

#[cfg(not(feature = "local_fs"))]
pub async fn run_git_command_with_env(
    _repo_path: &Path,
    _args: &[&str],
    _path_env: Option<&str>,
) -> Result<String> {
    Err(anyhow!("Not supported on wasm"))
}

/// Returns the set of local branch names for the repo at `repo_path`.
/// Uses a synchronous subprocess call — suitable for call sites in
/// synchronous view handlers where the result is needed immediately.
/// Returns an empty set on any failure (not a git repo, git not found, etc.).
#[cfg(feature = "local_fs")]
pub fn list_local_branches_sync(repo_path: &Path) -> HashSet<String> {
    let output = command::blocking::Command::new("git")
        .args(["branch", "--list", "--format=%(refname:short)"])
        .current_dir(repo_path)
        .stdout(command::Stdio::piped())
        .stderr(command::Stdio::null())
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect(),
        _ => HashSet::new(),
    }
}

#[cfg(not(feature = "local_fs"))]
pub fn list_local_branches_sync(_repo_path: &Path) -> HashSet<String> {
    HashSet::new()
}

/// Fetches the current git branch.
/// In detached HEAD state this returns the literal string "HEAD".
pub async fn detect_current_branch(target: &GitExecTarget) -> Result<String> {
    log::debug!("[GIT OPERATION] git.rs detect_current_branch git rev-parse --abbrev-ref HEAD");
    let result = target.run_git(&["rev-parse", "--abbrev-ref", "HEAD"]).await;

    if result.is_err() {
        log::debug!("[GIT OPERATION] git.rs detect_current_branch git branch --show-current");
        target.run_git(&["branch", "--show-current"]).await
    } else {
        result
    }
    .map(|branch_name| branch_name.trim().to_owned())
}

/// Like [`detect_current_branch`], but in detached HEAD state returns the short
/// commit SHA instead of the literal "HEAD".
/// (Matches the shell command `git symbolic-ref --short HEAD || git rev-parse --short HEAD`.)
#[cfg(feature = "local_fs")]
pub async fn detect_current_branch_display(repo_path: &Path) -> Result<String> {
    let branch = detect_current_branch(&GitExecTarget::local(repo_path.to_path_buf())).await?;
    if branch == "HEAD" {
        run_git_command(repo_path, &["rev-parse", "--short", "HEAD"])
            .await
            .map(|sha| sha.trim().to_owned())
    } else {
        Ok(branch)
    }
}

#[cfg(not(feature = "local_fs"))]
pub async fn detect_current_branch_display(_repo_path: &Path) -> Result<String> {
    Err(anyhow!("Not supported without local_fs"))
}

/// Detects the main branch using git-branchless style heuristics.
pub async fn detect_main_branch(target: &GitExecTarget) -> Result<String> {
    // First try to get the default branch from origin
    log::debug!(
        "[GIT OPERATION] git.rs detect_main_branch git symbolic-ref refs/remotes/origin/HEAD"
    );
    match target.run_git(&["symbolic-ref", "refs/remotes/origin/HEAD"]).await {
        Ok(output) => {
            let branch_ref = output.trim();
            if let Some(branch_name) = branch_ref.strip_prefix("refs/remotes/") {
                return Ok(branch_name.to_string());
            }
        }
        Err(_) => {
            // If remote fetch fails, fall back to candidates.
        }
    }

    // Fallback: try common main branch names in order of preference.
    let candidates = ["origin/main", "origin/master", "main", "master", "develop"];

    for candidate in candidates {
        log::debug!(
            "[GIT OPERATION] git.rs detect_main_branch git rev-parse --verify {candidate}^{{}}"
        );
        let result = target
            .run_git(&["rev-parse", "--verify", &format!("{candidate}^{{}}")])
            .await;

        if result.is_ok() {
            return Ok(candidate.to_string());
        }
    }

    // Final fallback if all else fails.
    log::debug!("[GIT OPERATION] git.rs detect_main_branch git branch --show-current");
    target.run_git(&["branch", "--show-current"]).await
}

/// Returns the SHA where `HEAD` forked from any other ref. Use
/// `<fork>..HEAD` for "commits unique to this branch".
pub async fn detect_fork_point(
    target: &GitExecTarget,
    current_branch_name: Option<&str>,
) -> Result<Option<String>> {
    // Exclude `<current>` and `origin/<current>` so the branch isn't
    // subtracted from itself.
    let current = current_branch_name
        .map(str::trim)
        .filter(|branch| !branch.is_empty() && *branch != "HEAD");

    let branch_exclude = current.map(|c| format!("--exclude={c}"));
    let remote_exclude = current.map(|c| format!("--exclude=origin/{c}"));

    let mut args: Vec<&str> = vec!["rev-list", "HEAD", "--not"];
    args.extend(branch_exclude.as_deref());
    args.push("--branches");
    args.extend(remote_exclude.as_deref());
    args.push("--remotes");

    let unique = match target.run_git(&args).await {
        Ok(out) => out,
        Err(e) => {
            log::debug!("detect_fork_point: rev-list failed: {e}");
            return Ok(None);
        }
    };

    // Last non-empty line = oldest unique commit; its parent = fork point.
    // No unique commits means HEAD is fully shared, so fork = HEAD.
    let rev = match unique.lines().rfind(|l| !l.trim().is_empty()) {
        Some(sha) => format!("{}^", sha.trim()),
        None => "HEAD".to_string(),
    };
    Ok(target
        .run_git(&["rev-parse", &rev])
        .await
        .ok()
        .map(|s| s.trim().to_string()))
}

/// Git summary for a repo: current branch + uncommitted diff stats.
#[derive(Debug, Clone)]
#[cfg(feature = "local_fs")]
pub struct RepoGitSummary {
    pub branch: String,
    pub lines_added: u32,
    pub lines_removed: u32,
}

/// Runs git commands in `repo_root` to get current branch + diff stats.
/// Returns None if not a git repo or git is unavailable.
#[cfg(feature = "local_fs")]
pub async fn get_repo_git_summary(repo_root: &Path) -> Option<RepoGitSummary> {
    use crate::context_chips::display_chip::GitLineChanges;

    let branch = {
        log::debug!("[GIT OPERATION] git.rs get_repo_git_summary git symbolic-ref --short HEAD");
        let result = run_git_command(repo_root, &["symbolic-ref", "--short", "HEAD"]).await;
        match result {
            Ok(output) => Some(output.trim().to_string()),
            Err(_) => {
                // Fallback to rev-parse for detached HEAD
                log::debug!(
                    "[GIT OPERATION] git.rs get_repo_git_summary git rev-parse --short HEAD"
                );
                run_git_command(repo_root, &["rev-parse", "--short", "HEAD"])
                    .await
                    .ok()
                    .map(|o| o.trim().to_string())
            }
        }
    };

    // Tracked file changes (git diff --shortstat HEAD doesn't include untracked files).
    log::debug!("[GIT OPERATION] git.rs get_repo_git_summary git diff --shortstat HEAD");
    let stats = run_git_command(repo_root, &["diff", "--shortstat", "HEAD"])
        .await
        .ok()
        .and_then(|o| GitLineChanges::parse_from_git_output(&o));

    let mut lines_added = stats.as_ref().map_or(0, |s| s.lines_added);
    let lines_removed = stats.as_ref().map_or(0, |s| s.lines_removed);

    // Also count lines in untracked files to match what the git diff chip shows.
    log::debug!(
        "[GIT OPERATION] git.rs get_repo_git_summary git ls-files --others --exclude-standard"
    );
    if let Ok(untracked_output) =
        run_git_command(repo_root, &["ls-files", "--others", "--exclude-standard"]).await
    {
        for file_name in untracked_output.lines() {
            if file_name.is_empty() {
                continue;
            }
            lines_added += count_lines_if_text_file(&repo_root.join(file_name));
        }
    }

    let branch = branch?;
    Some(RepoGitSummary {
        branch,
        lines_added,
        lines_removed,
    })
}

/// Short summary of a commit: hash and subject line.
#[derive(Debug, Clone)]
pub struct Commit {
    pub hash: String,
    pub subject: String,
    pub files_changed: usize,
    pub additions: usize,
    pub deletions: usize,
}

/// A single changed file with per-file addition/deletion counts.
#[derive(Debug, Clone)]
pub struct FileChangeEntry {
    pub path: String,
    pub additions: usize,
    pub deletions: usize,
}

/// Returns per-file change entries. When `include_unstaged` is true, returns all
/// uncommitted changes (staged + unstaged + untracked) vs HEAD; otherwise only staged changes.
#[cfg(feature = "local_fs")]
pub async fn get_file_change_entries(
    target: &GitExecTarget,
    include_unstaged: bool,
) -> Result<Vec<FileChangeEntry>> {
    let args: &[&str] = if include_unstaged {
        &["diff", "--numstat", "HEAD"]
    } else {
        &["diff", "--cached", "--numstat"]
    };
    let output = target.run_git(args).await.unwrap_or_default();
    let mut entries = Vec::new();
    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            entries.push(FileChangeEntry {
                path: parts[2].to_string(),
                additions: parts[0].parse().unwrap_or(0),
                deletions: parts[1].parse().unwrap_or(0),
            });
        }
    }

    // Also include untracked files when showing all changes.
    if include_unstaged {
        if let Ok(untracked) = target
            .run_git(&["ls-files", "--others", "--exclude-standard"])
            .await
        {
            for file_name in untracked.lines() {
                if file_name.is_empty() {
                    continue;
                }
                // Local targets count the file off disk; remote targets can't,
                // so they ask git (see `count_untracked_lines_via_git`).
                let additions = match target.local_repo_path() {
                    Some(local_root) => {
                        count_lines_if_text_file(&local_root.join(file_name)) as usize
                    }
                    None => count_untracked_lines_via_git(target, file_name).await,
                };
                entries.push(FileChangeEntry {
                    path: file_name.to_string(),
                    additions,
                    deletions: 0,
                });
            }
        }
    }

    Ok(entries)
}

#[cfg(not(feature = "local_fs"))]
pub async fn get_file_change_entries(
    _target: &GitExecTarget,
    _include_unstaged: bool,
) -> Result<Vec<FileChangeEntry>> {
    Err(anyhow!("Not supported on wasm"))
}

/// Line count git reports for one untracked file, for targets whose working
/// copy is not readable locally.
///
/// `git diff --no-index --numstat -- /dev/null <path>` emits
/// `<added>\t0\t<path>` for text (`added` is the line count) and `-\t-\t<path>`
/// for binary. Differences yield exit code 1, which [`GitExecTarget::run_git`]
/// accepts.
#[cfg(feature = "local_fs")]
pub async fn count_untracked_lines_via_git(target: &GitExecTarget, rel_path: &str) -> usize {
    let output = target
        .run_git(&["diff", "--no-index", "--numstat", "--", "/dev/null", rel_path])
        .await
        .unwrap_or_default();
    output
        .lines()
        .next()
        .and_then(|line| line.split('\t').next())
        .and_then(|added| added.parse::<usize>().ok())
        .unwrap_or(0)
}

/// Unpushed commits: `<upstream>..HEAD`, or `<fork_point>..HEAD` if no upstream.
pub async fn get_unpushed_commits(
    target: &GitExecTarget,
    current_branch_name: Option<&str>,
    upstream_ref: Option<&str>,
) -> Result<Vec<Commit>> {
    let output = if let Some(upstream_ref) = upstream_ref.map(str::trim).filter(|s| !s.is_empty()) {
        let range = format!("{upstream_ref}..HEAD");
        target
            .run_git(&["log", &range, "--format=COMMIT:%H\t%s", "--numstat"])
            .await?
    } else {
        // No upstream — fall back to the fork-point commit so we show
        // exactly the commits unique to this branch
        let fork_point = detect_fork_point(target, current_branch_name)
            .await
            .ok()
            .flatten();

        let range = match fork_point {
            Some(sha) => format!("{sha}..HEAD"),
            None => "HEAD".to_string(),
        };

        target
            .run_git(&["log", &range, "--format=COMMIT:%H\t%s", "--numstat"])
            .await
            .inspect_err(|e| log::warn!("Fallback unpushed-commits log failed: {e}"))
            .unwrap_or_default()
    };
    parse_commit_log(&output)
}

fn parse_commit_log(output: &str) -> Result<Vec<Commit>> {
    let mut commits = Vec::new();
    let mut current: Option<Commit> = None;

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("COMMIT:") {
            if let Some(commit) = current.take() {
                commits.push(commit);
            }
            let parts: Vec<&str> = rest.splitn(2, '\t').collect();
            if parts.len() == 2 {
                current = Some(Commit {
                    hash: parts[0].to_string(),
                    subject: parts[1].to_string(),
                    files_changed: 0,
                    additions: 0,
                    deletions: 0,
                });
            }
        } else if !line.is_empty() {
            // numstat line: "<additions>\t<deletions>\t<path>"
            if let Some(ref mut commit) = current {
                let parts: Vec<&str> = line.splitn(3, '\t').collect();
                if parts.len() == 3 {
                    commit.additions += parts[0].parse::<usize>().unwrap_or(0);
                    commit.deletions += parts[1].parse::<usize>().unwrap_or(0);
                    commit.files_changed += 1;
                }
            }
        }
    }

    if let Some(commit) = current {
        commits.push(commit);
    }

    Ok(commits)
}

/// Returns the list of files changed in a specific commit, with per-file stats.
#[cfg(feature = "local_fs")]
pub async fn get_commit_files(target: &GitExecTarget, hash: &str) -> Result<Vec<FileChangeEntry>> {
    let output = target
        .run_git(&[
            "diff-tree",
            "--root",
            "--no-commit-id",
            "-r",
            "--numstat",
            hash,
        ])
        .await?;

    let mut entries = Vec::new();
    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() == 3 {
            entries.push(FileChangeEntry {
                path: parts[2].to_string(),
                additions: parts[0].parse().unwrap_or(0),
                deletions: parts[1].parse().unwrap_or(0),
            });
        }
    }

    Ok(entries)
}

#[cfg(not(feature = "local_fs"))]
pub async fn get_commit_files(
    _target: &GitExecTarget,
    _hash: &str,
) -> Result<Vec<FileChangeEntry>> {
    Err(anyhow!("Not supported on wasm"))
}

/// Maximum number of characters of diff content to send to AI for commit
/// message / PR title / PR description generation.
#[cfg(feature = "local_fs")]
const MAX_DIFF_CHARS_FOR_AI: usize = 16_000;

/// Per-file cap for untracked-file content we synthesise into the diff sent
/// to AI. Keeps any one new file from dominating the budget.
#[cfg(feature = "local_fs")]
const MAX_UNTRACKED_FILE_BYTES: usize = 4_000;

/// Number of leading bytes examined when classifying an untracked file as
/// binary, mirroring the heuristic in `count_lines_if_text_file`.
#[cfg(feature = "local_fs")]
const BINARY_CHECK_BYTES: usize = 1_024;

/// Maximum number of bytes in a PR title passed to `gh pr create`. GitHub's
/// hard limit is 256; we cap short of that to leave headroom for an
/// ellipsis marker. Measured in bytes because it's fed to
/// [`truncate_on_char_boundary`], which slices on byte offsets.
#[cfg(feature = "local_fs")]
const MAX_PR_TITLE_BYTES: usize = 200;

/// Maximum PR body accepted for `gh pr create --body`. On the remote arm the
/// body is part of one shell command string, bounded by the host's
/// `MAX_ARG_STRLEN` (128 KiB on Linux); 64 KiB leaves headroom for the
/// surrounding command without truncating silently.
#[cfg(feature = "local_fs")]
const MAX_PR_BODY_BYTES: usize = 64 * 1024;

/// Returns a prefix of `s` whose length is at most `byte_cap` and which ends
/// on a UTF-8 char boundary. Plain `&s[..byte_cap]` panics when the cut
/// point lands inside a multi-byte code point, which is reachable in diffs
/// and source files containing non-ASCII text.
#[cfg(feature = "local_fs")]
fn truncate_on_char_boundary(s: &str, byte_cap: usize) -> &str {
    if s.len() <= byte_cap {
        return s;
    }
    let mut cut = byte_cap;
    while cut > 0 && !s.is_char_boundary(cut) {
        cut -= 1;
    }
    &s[..cut]
}

/// Returns the diff for commit message generation, truncated to avoid token
/// limits. When `include_unstaged` is true, diffs against HEAD (all
/// uncommitted changes) and also appends untracked files as synthetic diff
/// hunks so the LLM has full context even when the commit consists entirely
/// of new files. When `include_unstaged` is false, diffs only staged changes.
#[cfg(feature = "local_fs")]
pub async fn get_diff_for_commit_message(
    target: &GitExecTarget,
    include_unstaged: bool,
) -> Result<String> {
    let mut diff = if !include_unstaged {
        target.run_git(&["diff", "--cached"]).await?
    } else if target.run_git(&["rev-parse", "--verify", "HEAD"]).await.is_ok() {
        target.run_git(&["diff", "HEAD"]).await?
    } else {
        // No HEAD before the first commit. Include staged changes plus
        // unstaged edits to staged files; untracked files are added below.
        let mut diff = target.run_git(&["diff", "--cached"]).await?;
        diff.push_str(&target.run_git(&["diff"]).await?);
        diff
    };

    // `git diff HEAD` only shows changes to already-tracked files. New files that
    // haven't been staged yet are invisible to it, so we synthesise diff hunks for
    // them here — mirroring the logic in `get_file_change_entries`.
    if include_unstaged {
        if let Ok(untracked) = target
            .run_git(&["ls-files", "--others", "--exclude-standard", "-z"])
            .await
        {
            // `-z` separates paths with NUL bytes and disables C-style
            // quoting, so paths containing spaces or non-ASCII characters
            // round-trip intact.
            for file_name_bytes in untracked.as_bytes().split(|b| *b == 0) {
                if file_name_bytes.is_empty() {
                    continue;
                }
                let Ok(file_name) = std::str::from_utf8(file_name_bytes) else {
                    continue;
                };
                if let Some(hunk) = synthesize_untracked_hunk(target, file_name).await {
                    diff.push_str(&hunk);
                }
            }
        }
    }

    if diff.len() <= MAX_DIFF_CHARS_FOR_AI {
        Ok(diff)
    } else {
        Ok(format!(
            "{}\n... (diff truncated)",
            truncate_on_char_boundary(&diff, MAX_DIFF_CHARS_FOR_AI)
        ))
    }
}

/// Builds a synthetic `diff --git` hunk for one untracked file, so the commit
/// message prompt sees new files that `git diff HEAD` omits. Local targets read
/// the file off disk (bounded, with the same binary heuristic as
/// [`count_lines_if_text_file`]); remote targets ask git via
/// `diff --no-index -- /dev/null <path>` instead of touching the local FS.
#[cfg(feature = "local_fs")]
async fn synthesize_untracked_hunk(target: &GitExecTarget, file_name: &str) -> Option<String> {
    let Some(local_root) = target.local_repo_path() else {
        let file_diff = target
            .run_git(&["diff", "--no-index", "--", "/dev/null", file_name])
            .await
            .ok()?;
        if file_diff.trim().is_empty() || file_diff.contains("Binary files ") {
            return None;
        }
        let truncated = truncate_on_char_boundary(&file_diff, MAX_UNTRACKED_FILE_BYTES);
        let mut hunk = truncated.to_string();
        if truncated.len() < file_diff.len() {
            hunk.push_str("\n... (truncated)");
        }
        return Some(hunk);
    };

    // Cap the read to cover both the binary-check window and the
    // synthesised-hunk budget.
    let read_cap = BINARY_CHECK_BYTES.max(MAX_UNTRACKED_FILE_BYTES);
    // Async + bounded so a large untracked file doesn't block the executor or
    // balloon memory.
    let file = tokio::fs::File::open(local_root.join(file_name)).await.ok()?;
    let mut bytes = Vec::with_capacity(read_cap);
    use tokio::io::AsyncReadExt as _;
    file.take(read_cap as u64).read_to_end(&mut bytes).await.ok()?;
    let check_len = bytes.len().min(BINARY_CHECK_BYTES);
    if warp_util::file_type::is_buffer_binary(&bytes[..check_len]) {
        return None;
    }
    let content = std::str::from_utf8(&bytes).ok()?;
    let content = truncate_on_char_boundary(content, MAX_UNTRACKED_FILE_BYTES);
    let line_count = content.lines().count();
    let mut hunk = format!(
        "diff --git a/{file_name} b/{file_name}\nnew file mode 100644\n\
         --- /dev/null\n+++ b/{file_name}\n@@ -0,0 +1,{line_count} @@\n"
    );
    for line in content.lines() {
        hunk.push('+');
        hunk.push_str(line);
        hunk.push('\n');
    }
    Some(hunk)
}

#[cfg(not(feature = "local_fs"))]
pub async fn get_diff_for_commit_message(
    _target: &GitExecTarget,
    _include_unstaged: bool,
) -> Result<String> {
    Err(anyhow!("Not supported on wasm"))
}

/// Commits changes. If `include_unstaged` is true, stages all changes first via `git add -A`.
/// `path_env` is forwarded so commit hooks can find tools on the user's `PATH`
/// (local targets only).
#[cfg(feature = "local_fs")]
pub async fn run_commit(
    target: &GitExecTarget,
    message: &str,
    include_unstaged: bool,
    path_env: Option<&str>,
) -> Result<String> {
    if include_unstaged {
        target.run_git_with_env(&["add", "-A"], path_env).await?;
    }
    target
        .run_git_with_env(&["commit", "-m", message], path_env)
        .await
}

#[cfg(not(feature = "local_fs"))]
pub async fn run_commit(
    _target: &GitExecTarget,
    _message: &str,
    _include_unstaged: bool,
    _path_env: Option<&str>,
) -> Result<String> {
    Err(anyhow!("Not supported on wasm"))
}

/// Per-file stats for what would land in a PR: default branch vs
/// `origin/<current>` (or HEAD when unpushed).
#[cfg(feature = "local_fs")]
pub async fn get_branch_diff_entries(target: &GitExecTarget) -> Result<Vec<FileChangeEntry>> {
    let base = detect_main_branch(target).await?;
    let base = base.trim();
    let current = detect_current_branch(target).await?;
    let remote_ref = format!("origin/{current}");

    // Use the remote ref if it exists, otherwise fall back to HEAD.
    let end_ref = if target
        .run_git(&["rev-parse", "--verify", &remote_ref])
        .await
        .is_ok()
    {
        remote_ref
    } else {
        "HEAD".to_string()
    };

    let range = format!("{base}..{end_ref}");
    let output = target.run_git(&["diff", "--numstat", &range]).await?;
    let mut entries = Vec::new();
    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            entries.push(FileChangeEntry {
                path: parts[2].to_string(),
                additions: parts[0].parse().unwrap_or(0),
                deletions: parts[1].parse().unwrap_or(0),
            });
        }
    }
    Ok(entries)
}

#[cfg(not(feature = "local_fs"))]
pub async fn get_branch_diff_entries(_target: &GitExecTarget) -> Result<Vec<FileChangeEntry>> {
    Err(anyhow!("Not supported on wasm"))
}

/// Pushes the given branch to origin, setting upstream tracking if not already configured.
/// `path_env` is forwarded so the LFS `pre-push` hook can find `git-lfs`
/// (local targets only).
#[cfg(feature = "local_fs")]
pub async fn run_push(
    target: &GitExecTarget,
    branch: &str,
    path_env: Option<&str>,
) -> Result<String> {
    target
        .run_git_with_env(&["push", "--set-upstream", "origin", branch], path_env)
        .await
}

#[cfg(not(feature = "local_fs"))]
pub async fn run_push(
    _target: &GitExecTarget,
    _branch: &str,
    _path_env: Option<&str>,
) -> Result<String> {
    Err(anyhow!("Not supported on wasm"))
}

// ── gh CLI helpers ───────────────────────────────────────────────────────────

/// PR information returned by `gh pr view`.
#[derive(Debug, Clone)]
pub struct PrInfo {
    pub number: u64,
    pub url: String,
}

/// Looks up the PR for the current branch via `gh pr view`.
/// Returns `Ok(None)` if there is simply no PR for this branch.
/// Returns `Err` for real failures (auth, network, gh not installed).
#[cfg(feature = "local_fs")]
pub async fn get_pr_for_branch(
    target: &GitExecTarget,
    path_env: Option<&str>,
) -> Result<Option<PrInfo>> {
    match target
        .run_program("gh", &["pr", "view", "--json", "number,url"], path_env)
        .await
    {
        Ok(stdout) => {
            let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
                .map_err(|e| anyhow!("Failed to parse gh output: {e}"))?;
            let number = parsed["number"]
                .as_u64()
                .ok_or_else(|| anyhow!("Missing 'number' in gh output"))?;
            let url = parsed["url"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing 'url' in gh output"))?
                .to_string();
            Ok(Some(PrInfo { number, url }))
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("no pull requests found") {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

#[cfg(not(feature = "local_fs"))]
pub async fn get_pr_for_branch(
    _target: &GitExecTarget,
    _path_env: Option<&str>,
) -> Result<Option<PrInfo>> {
    Err(anyhow!("Not supported on wasm"))
}

/// PR-ready diff (default branch vs `origin/<current>` or HEAD),
/// truncated for AI token limits.
#[cfg(feature = "local_fs")]
pub async fn get_diff_for_pr(target: &GitExecTarget) -> Result<String> {
    let base = detect_main_branch(target).await?;
    let base = base.trim();
    let current = detect_current_branch(target).await?;
    let remote_ref = format!("origin/{current}");

    let end_ref = if target
        .run_git(&["rev-parse", "--verify", &remote_ref])
        .await
        .is_ok()
    {
        remote_ref
    } else {
        "HEAD".to_string()
    };

    let range = format!("{base}..{end_ref}");
    let mut diff = target.run_git(&["diff", &range]).await?;
    if diff.len() > MAX_DIFF_CHARS_FOR_AI {
        diff = format!(
            "{}\n... (diff truncated)",
            truncate_on_char_boundary(&diff, MAX_DIFF_CHARS_FOR_AI)
        );
    }
    Ok(diff)
}

#[cfg(not(feature = "local_fs"))]
pub async fn get_diff_for_pr(_target: &GitExecTarget) -> Result<String> {
    Err(anyhow!("Not supported on wasm"))
}

/// Commit subject lines on the current branch since the default branch.
#[cfg(feature = "local_fs")]
pub async fn get_branch_commit_messages(target: &GitExecTarget) -> Result<Vec<String>> {
    let base = detect_main_branch(target).await?;
    let base = base.trim();
    let range = format!("{base}..HEAD");
    let output = target.run_git(&["log", &range, "--format=%s"]).await?;
    Ok(output
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect())
}

#[cfg(not(feature = "local_fs"))]
pub async fn get_branch_commit_messages(_target: &GitExecTarget) -> Result<Vec<String>> {
    Err(anyhow!("Not supported on wasm"))
}

/// Creates a PR for the current branch (must already be pushed). Falls back
/// to `--fill` when title/body are `None`. Always targets the detected
/// default branch.
#[cfg(feature = "local_fs")]
pub async fn create_pr(
    target: &GitExecTarget,
    title: Option<&str>,
    body: Option<&str>,
    path_env: Option<&str>,
) -> Result<PrInfo> {
    if let Some(body) = body {
        // The body travels as one shell-quoted argument on the remote arm; the
        // host's `MAX_ARG_STRLEN` (128 KiB on Linux) bounds how much fits, and
        // `--body` costs nothing extra locally. Reject anything that could not
        // survive that pipe instead of letting the host truncate it silently.
        if body.len() > MAX_PR_BODY_BYTES {
            return Err(anyhow!(
                "PR body is too large ({} bytes, limit {MAX_PR_BODY_BYTES})",
                body.len()
            ));
        }
    }
    let base = detect_main_branch(target).await?;
    let base = base.trim();
    let base = base.strip_prefix("origin/").unwrap_or(base);
    let sanitized_title;
    let args: Vec<&str> = match (title, body) {
        (Some(t), Some(b)) => {
            sanitized_title = sanitize_pr_title(t);
            vec![
                "pr",
                "create",
                "--base",
                base,
                "--title",
                &sanitized_title,
                "--body",
                b,
            ]
        }
        _ => vec!["pr", "create", "--base", base, "--fill"],
    };
    let stdout = target.run_program("gh", &args, path_env).await?;
    // `gh pr create` prints the PR URL on success.
    let url = stdout.trim().to_string();
    // Extract PR number from the URL (e.g. https://github.com/owner/repo/pull/123)
    let number = url
        .rsplit('/')
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| anyhow!("Could not parse PR number from URL: {url}"))?;
    Ok(PrInfo { number, url })
}

/// Trims an AI-generated PR title to a single line and caps its length.
#[cfg(feature = "local_fs")]
fn sanitize_pr_title(raw: &str) -> String {
    let first_line = raw.lines().next().unwrap_or("").trim();
    truncate_on_char_boundary(first_line, MAX_PR_TITLE_BYTES).to_string()
}

#[cfg(not(feature = "local_fs"))]
pub async fn create_pr(
    _target: &GitExecTarget,
    _title: Option<&str>,
    _body: Option<&str>,
    _path_env: Option<&str>,
) -> Result<PrInfo> {
    Err(anyhow!("Not supported on wasm"))
}

/// Shell probe for an in-progress git operation, used when the working copy is
/// remote. Resolves the git dir (so worktrees and submodules are handled) and
/// tests the same marker files the local arm stats, in one round trip. Always
/// exits 0 with `blocked` or `ok` on stdout.
#[cfg(feature = "local_fs")]
const GIT_OPERATION_PROBE: &str = "gd=$(git rev-parse --git-dir 2>/dev/null) || exit 0; \
for f in MERGE_HEAD CHERRY_PICK_HEAD REVERT_HEAD rebase-merge rebase-apply index.lock; do \
if [ -e \"$gd/$f\" ]; then echo blocked; exit 0; fi; done; echo ok";

/// Probes a remote target for an in-progress git operation (merge, rebase,
/// cherry-pick, revert) or a stale `index.lock`, mirroring the local
/// filesystem check in `DiffStateModel::is_git_operation_blocked`. Returns
/// `false` for local targets and on any probe failure — a transport hiccup
/// must not block the user's writes.
#[cfg(feature = "local_fs")]
pub async fn probe_remote_git_operation_blocked(target: &GitExecTarget) -> bool {
    if !target.is_remote() {
        return false;
    }
    match target.run_program("sh", &["-c", GIT_OPERATION_PROBE], None).await {
        Ok(stdout) => stdout.trim() == "blocked",
        Err(e) => {
            log::warn!("Remote git-operation probe failed: {e}");
            false
        }
    }
}

/// Counts newlines in a file, returning 0 for binary or oversized files.
#[cfg(feature = "local_fs")]
fn count_lines_if_text_file(path: &Path) -> u32 {
    const MAX_FILE_SIZE: u64 = 20_000_000;
    const BINARY_CHECK_SIZE: usize = 1024;

    let Ok(metadata) = std::fs::metadata(path) else {
        return 0;
    };
    if metadata.len() > MAX_FILE_SIZE || !metadata.is_file() {
        return 0;
    }
    let Ok(content) = std::fs::read(path) else {
        return 0;
    };
    let check_len = content.len().min(BINARY_CHECK_SIZE);
    if warp_util::file_type::is_buffer_binary(&content[..check_len]) {
        return 0;
    }
    bytecount::count(&content, b'\n') as u32
}
