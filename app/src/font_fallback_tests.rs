use super::*;
use warpui::AssetProvider as _;

#[test]
fn nerd_font_icons_use_the_bundled_nerd_font() {
    // yazi and superfile emit icons from both the BMP and the supplementary Private Use Areas.
    for icon in ['\u{E5FF}', '\u{F07B}', '\u{F0219}'] {
        let family = native_fallback_font_fn(icon).expect("icon should have a fallback family");
        assert_eq!(family.name, "Hack Nerd Font Mono");
    }
}

#[test]
fn ordinary_text_is_left_to_the_system_cascade() {
    for ch in ['a', 'é', '中', '🙂'] {
        assert!(native_fallback_font_fn(ch).is_none());
    }
}

#[test]
fn nerd_font_resolves_to_an_embedded_asset() {
    let location = &HACK_NERD_FONT_MONO.font_urls[0];
    let AssetSource::Bundled { path } = fallback_font_source(location) else {
        panic!("nerd font should load from bundled assets, not the network");
    };
    assert!(crate::ASSETS.get(path).is_ok(), "{path} is not embedded");
}
