use settings::{
    macros::define_settings_group, RespectUserSyncSetting, SupportedPlatforms, SyncToCloud,
};

define_settings_group!(SshSettings,
    settings: [
        enable_legacy_ssh_wrapper: EnableSshWrapper {
            type: bool,
            default: true,
            supported_platforms: SupportedPlatforms::ALL,
            sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
            private: false,
            storage_key: "EnableSSHWrapper",
            toml_path: "warpify.ssh.enable_legacy_ssh_wrapper",
            description: "Whether the legacy SSH wrapper is enabled for SSH sessions.",
        },
        enable_ssh_auto_discovery: EnableSshAutoDiscovery {
            type: bool,
            default: true,
            supported_platforms: SupportedPlatforms::ALL,
            sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
            private: false,
            storage_key: "EnableSSHAutoDiscovery",
            toml_path: "warpify.ssh.enable_ssh_auto_discovery",
            description: "Whether to auto-discover SSH hosts from ~/.ssh/config.",
        },
        split_inherits_ssh: SplitInheritsSshSetting {
            type: bool,
            default: true,
            supported_platforms: SupportedPlatforms::ALL,
            sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
            private: false,
            storage_key: "SplitInheritsSsh",
            toml_path: "warpify.ssh.split_inherits_ssh",
            description: "Whether splitting a pane from an SSH session opens the new pane on the same host.",
        },
    ]
);
