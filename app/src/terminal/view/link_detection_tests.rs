#[cfg(feature = "local_fs")]
mod link_session_location {
    use warp_core::HostId;

    use super::super::LinkSessionLocation;

    #[test]
    fn connected_remote_server_session_is_remote() {
        let host_id = HostId::new("host".to_string());
        assert_eq!(
            LinkSessionLocation::new(Some(false), Some(host_id.clone())),
            LinkSessionLocation::Remote(host_id)
        );
    }

    #[test]
    fn remote_session_without_daemon_is_unavailable() {
        assert_eq!(
            LinkSessionLocation::new(Some(false), None),
            LinkSessionLocation::RemoteUnavailable
        );
    }

    #[test]
    fn local_or_unknown_session_is_local() {
        assert_eq!(
            LinkSessionLocation::new(Some(true), None),
            LinkSessionLocation::Local
        );
        assert_eq!(
            LinkSessionLocation::new(None, None),
            LinkSessionLocation::Local
        );
    }
}
