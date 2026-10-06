use crate::terminal::model::session::{shell_quote_arg, SessionType};
use crate::terminal::model::terminal_model::SubshellInitializationInfo;
use crate::terminal::shell::ShellType;

/// Returns the command to run in a new pane split from a warpified SSH session so that the new
/// pane lands on the same remote host, or `None` if the split should stay a plain local shell.
///
/// Only a plain `ssh ...` command typed in a local session qualifies. Wrappers (`sshpass`,
/// `gcloud ...`) and nested SSH hops are rejected because replaying them is not safe.
pub fn inherited_ssh_command(
    session_type: &SessionType,
    subshell_info: Option<&SubshellInitializationInfo>,
    spawning_session_type: Option<&SessionType>,
) -> Option<String> {
    if !matches!(session_type, SessionType::WarpifiedRemote { .. }) {
        return None;
    }
    let info = subshell_info?;
    info.ssh_connection_info.as_ref()?;
    if !matches!(spawning_session_type, Some(SessionType::Local)) {
        return None;
    }
    let command = info.spawning_command.trim();
    (command.split_whitespace().next() == Some("ssh")).then(|| command.to_string())
}

/// Working directory of the original pane's remote session, to restore in a pane that
/// inherited its ssh connection.
#[derive(Clone, Debug)]
pub struct PendingRemoteCd {
    /// `user@hostname` of the remote session the path was read from.
    pub remote_host: String,
    /// Raw remote path. It is never a local path.
    pub path: String,
}

/// Returns the `cd` command to run once the inherited ssh session has bootstrapped. Only
/// fires if the bootstrapped session is the host the path came from and the input is untouched.
pub fn remote_cd_command(
    pending: &PendingRemoteCd,
    session_host: &str,
    shell_type: ShellType,
    input_is_empty: bool,
) -> Option<String> {
    (input_is_empty && pending.remote_host == session_host)
        .then(|| format!("cd {}", shell_quote_arg(&pending.path, shell_type)))
}

#[cfg(test)]
#[path = "ssh_split_tests.rs"]
mod tests;
