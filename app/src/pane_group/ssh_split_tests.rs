use super::*;
use crate::terminal::ssh::util::InteractiveSshCommand;

const REMOTE: SessionType = SessionType::WarpifiedRemote { host_id: None };
const LOCAL: SessionType = SessionType::Local;

fn info(command: &str, parsed: bool) -> SubshellInitializationInfo {
    SubshellInitializationInfo {
        spawning_command: command.to_string(),
        was_triggered_by_rc_file_snippet: false,
        env_var_collection_name: None,
        ssh_connection_info: parsed.then(InteractiveSshCommand::default),
    }
}

fn inherited(
    session_type: &SessionType,
    subshell_info: Option<&SubshellInitializationInfo>,
    legacy: Option<&str>,
    spawning: Option<&SessionType>,
) -> Option<String> {
    inherited_ssh_command(
        session_type,
        resolve_ssh_spawning_command(subshell_info, legacy),
        spawning,
    )
}

#[test]
fn plain_ssh_is_inherited() {
    assert_eq!(
        inherited(&REMOTE, Some(&info("ssh host", true)), None, Some(&LOCAL)),
        Some("ssh host".to_string())
    );
}

#[test]
fn ssh_options_are_kept_verbatim() {
    assert_eq!(
        inherited(
            &REMOTE,
            Some(&info("  ssh -J jump -p 2222 host \n", true)),
            None,
            Some(&LOCAL)
        ),
        Some("ssh -J jump -p 2222 host".to_string())
    );
}

#[test]
fn local_session_is_not_inherited() {
    assert_eq!(
        inherited(&LOCAL, Some(&info("ssh host", true)), None, Some(&LOCAL)),
        None
    );
}

#[test]
fn wrapped_ssh_is_not_inherited() {
    assert_eq!(
        inherited(
            &REMOTE,
            Some(&info("sshpass -p x ssh host", true)),
            None,
            Some(&LOCAL)
        ),
        None
    );
}

#[test]
fn missing_ssh_connection_info_is_not_inherited() {
    assert_eq!(
        inherited(&REMOTE, Some(&info("ssh host", false)), None, Some(&LOCAL)),
        None
    );
    assert_eq!(inherited(&REMOTE, None, None, Some(&LOCAL)), None);
}

#[test]
fn nested_or_unknown_spawning_session_is_not_inherited() {
    assert_eq!(
        inherited(&REMOTE, Some(&info("ssh host", true)), None, Some(&REMOTE)),
        None
    );
    assert_eq!(
        inherited(&REMOTE, Some(&info("ssh host", true)), None, None),
        None
    );
}

#[test]
fn legacy_plain_ssh_is_inherited() {
    assert_eq!(
        inherited(&REMOTE, None, Some("ssh host"), Some(&LOCAL)),
        Some("ssh host".to_string())
    );
    assert_eq!(
        inherited(
            &REMOTE,
            None,
            Some("ssh -J jump -p 2222 host"),
            Some(&LOCAL)
        ),
        Some("ssh -J jump -p 2222 host".to_string())
    );
}

#[test]
fn legacy_wrapped_or_non_interactive_ssh_is_not_inherited() {
    assert_eq!(
        inherited(&REMOTE, None, Some("sshpass -p x ssh host"), Some(&LOCAL)),
        None
    );
    assert_eq!(
        inherited(&REMOTE, None, Some("ssh host ls"), Some(&LOCAL)),
        None
    );
}

#[test]
fn legacy_nested_ssh_is_not_inherited() {
    assert_eq!(
        inherited(&REMOTE, None, Some("ssh host"), Some(&REMOTE)),
        None
    );
}

#[test]
fn subshell_info_takes_precedence_over_legacy_command() {
    assert_eq!(
        resolve_ssh_spawning_command(Some(&info("ssh a", false)), Some("ssh b")),
        None
    );
}

fn pending(path: &str) -> PendingRemoteCd {
    PendingRemoteCd {
        remote_host: "me@box".to_string(),
        path: path.to_string(),
    }
}

#[test]
fn remote_cd_matches_host() {
    assert_eq!(
        remote_cd_command(&pending("/srv/app"), "me@box", ShellType::Bash, true),
        Some("cd '/srv/app'".to_string())
    );
}

#[test]
fn remote_cd_quotes_path() {
    let cmd = remote_cd_command(&pending("/srv/it's here"), "me@box", ShellType::Bash, true);
    assert_eq!(cmd, Some(r#"cd '/srv/it'"'"'s here'"#.to_string()));
}

#[test]
fn remote_cd_skips_host_mismatch_and_dirty_input() {
    assert_eq!(
        remote_cd_command(&pending("/srv"), "me@other", ShellType::Bash, true),
        None
    );
    assert_eq!(
        remote_cd_command(&pending("/srv"), "me@box", ShellType::Bash, false),
        None
    );
}
