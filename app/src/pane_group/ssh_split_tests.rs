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
