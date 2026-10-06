use crate::terminal::model::session::SessionType;
use crate::terminal::model::terminal_model::SubshellInitializationInfo;

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

#[cfg(test)]
#[path = "ssh_split_tests.rs"]
mod tests;
