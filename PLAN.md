# Plan: Revert Zap Branding → OpenWarp

Branding-only revert of commit `a00ae6c`. No structural/code-cleanup changes reversed.

**Decisions:**
- Binary: `zap-oss` → `openwarp-oss`
- Config dir: `~/.zap` → `~/.openwarp`
- Drive feature: "Zap Drive" → "OpenWarp Drive"
- Bundle ID: `dev.zap.Zap` → `dev.openwarp.OpenWarp`

## Steps

### 1. App metadata & binary name
- `app/Cargo.toml`: author → "OpenWarp Team", default-run/bin name → "openwarp-oss"
- `app/src/bin/zap_oss.rs` → rename to `openwarp_oss.rs`, update AppId, plist strings, URL scheme, copyright
- `app/build.rs`: default app_name/publisher → "OpenWarp"

### 2. Core runtime identity
- `crates/warp_core/src/channel/state.rs`: AppId → `("dev","openwarp","OpenWarp")`, URL scheme → "openwarp"
- `crates/warp_core/src/paths.rs`: `.zap` → `.openwarp`, name mapping "Zap" → "OpenWarp"

### 3. Desktop file
- Rename `app/channels/oss/dev.zap.Zap.desktop` → `dev.openwarp.OpenWarp.desktop`
- Update all fields inside (Name, Exec, Icon, StartupWMClass, MimeType)

### 4. i18n files (en, zh-CN, ja)
- `app/i18n/en/warp.ftl`: replace all "Zap" → "OpenWarp", "Zap Drive" → "OpenWarp Drive", "Get Zapping" → "Get Warping"
- `app/i18n/zh-CN/warp.ftl`: same pattern
- `app/i18n/ja/warp.ftl`: same pattern

### 5. Packaging scripts
- `script/macos/bundle`: oss block vars, DMG volname
- `script/linux/bundle`: oss block vars
- `script/windows/bundle.ps1`: oss block vars, AUMID, InnoSetup app ID
- `script/windows/windows-installer.iss`: app name references

### 6. Linux package metadata
- `resources/linux/debian/app/control.template`: maintainer, description
- `resources/linux/rpm/app/warp.spec.template`: summary, vendor, packager, description

### 7. Platform-specific storage/secrets
- `crates/warpui_extras/src/secure_storage/linux.rs`: fallback key → "openwarp-..."
- `crates/warpui_extras/src/user_preferences/registry_backed.rs`: `Software\Zap\` → `Software\OpenWarp\`
- `crates/warp_ssh_manager/src/secrets.rs`: `zap.ssh` → `openwarp.ssh`

### 8. Remote server
- `crates/remote_server/src/setup.rs`: `.zap` → `.openwarp`, tarball name
- `crates/remote_server/src/install_remote_server.sh`: all `zap` refs → `openwarp`

### 9. Workspace view modal
- Rename `app/src/workspace/view/zap_launch_modal/` → `openwarp_launch_modal/`
- Update Rust type names: `ZapLaunchModal` → `OpenWarpLaunchModal`, etc.
- Update references in `app/src/workspace/view.rs` and `app/src/workspace/mod.rs`

### 10. Verify
- `cargo check --bin openwarp-oss`
- `cargo check --bin generate_settings_schema`
- Grep for remaining "Zap" branding to catch stragglers

### Commit strategy
Commit after each major group (steps 1-3, step 4, steps 5-6, steps 7-8, step 9, step 10).
