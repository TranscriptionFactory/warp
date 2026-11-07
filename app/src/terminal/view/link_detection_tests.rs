#[cfg(feature = "local_fs")]
mod link_session_location {
    use warp_core::HostId;

    use super::super::LinkSessionLocation;

    #[test]
    fn connected_remote_server_session_is_remote() {
        let host_id = HostId::new("host".to_string());
        assert_eq!(
            LinkSessionLocation::new(Some(false), Some(host_id.clone()), true),
            LinkSessionLocation::Remote(host_id)
        );
    }

    #[test]
    fn remote_session_without_daemon_is_unavailable() {
        assert_eq!(
            LinkSessionLocation::new(Some(false), None, false),
            LinkSessionLocation::RemoteUnavailable
        );
    }

    #[test]
    fn remote_session_with_ssh_host_is_reached_over_scp() {
        assert_eq!(
            LinkSessionLocation::new(Some(false), None, true),
            LinkSessionLocation::RemoteOverScp
        );
    }

    #[test]
    fn local_session_ignores_an_ssh_host() {
        assert_eq!(
            LinkSessionLocation::new(Some(true), None, true),
            LinkSessionLocation::Local
        );
    }

    #[test]
    fn local_or_unknown_session_is_local() {
        assert_eq!(
            LinkSessionLocation::new(Some(true), None, false),
            LinkSessionLocation::Local
        );
        assert_eq!(
            LinkSessionLocation::new(None, None, false),
            LinkSessionLocation::Local
        );
    }
}
