use super::*;
use crate::terminal::ssh::util::InteractiveSshCommand;

const REMOTE: SessionType = SessionType::WarpifiedRemote { host_id: None };

fn info(command: &str, parsed: bool) -> SubshellInitializationInfo {
    SubshellInitializationInfo {
        spawning_command: command.to_string(),
        was_triggered_by_rc_file_snippet: false,
        env_var_collection_name: None,
        ssh_connection_info: parsed.then(InteractiveSshCommand::default),
    }
}

#[test]
fn plain_ssh_is_inherited() {
    assert_eq!(
        inherited_ssh_command(
            &REMOTE,
            Some(&info("ssh host", true)),
            Some(&SessionType::Local)
        ),
        Some("ssh host".to_string())
    );
}

#[test]
fn ssh_options_are_kept_verbatim() {
    assert_eq!(
        inherited_ssh_command(
            &REMOTE,
            Some(&info("  ssh -J jump -p 2222 host \n", true)),
            Some(&SessionType::Local)
        ),
        Some("ssh -J jump -p 2222 host".to_string())
    );
}

#[test]
fn local_session_is_not_inherited() {
    assert_eq!(
        inherited_ssh_command(
            &SessionType::Local,
            Some(&info("ssh host", true)),
            Some(&SessionType::Local)
        ),
        None
    );
}

#[test]
fn wrapped_ssh_is_not_inherited() {
    assert_eq!(
        inherited_ssh_command(
            &REMOTE,
            Some(&info("sshpass -p x ssh host", true)),
            Some(&SessionType::Local)
        ),
        None
    );
}

#[test]
fn missing_ssh_connection_info_is_not_inherited() {
    assert_eq!(
        inherited_ssh_command(
            &REMOTE,
            Some(&info("ssh host", false)),
            Some(&SessionType::Local)
        ),
        None
    );
    assert_eq!(
        inherited_ssh_command(&REMOTE, None, Some(&SessionType::Local)),
        None
    );
}

#[test]
fn nested_or_unknown_spawning_session_is_not_inherited() {
    assert_eq!(
        inherited_ssh_command(&REMOTE, Some(&info("ssh host", true)), Some(&REMOTE)),
        None
    );
    assert_eq!(
        inherited_ssh_command(&REMOTE, Some(&info("ssh host", true)), None),
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
