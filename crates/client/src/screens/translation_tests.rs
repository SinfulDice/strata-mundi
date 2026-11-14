#![allow(clippy::unwrap_used)]
//! Every interface text has its translations.

/// Interface texts chosen in code rather than in screen files.
const CODE_KEYS: &[&str] = &[
    super::inventory::TAB_ALL_KEY,
    "ui-editor-sample",
    "ui-error-incompatible",
    "ui-error-unavailable",
    "ui-error-broken",
    "ui-reload-failed",
    "ui-reload-blocks-changed",
    "ui-update-available",
    "ui-version",
    "ui-scale-times",
    "ui-world-detail",
    "ui-world-too-old",
    "ui-world-too-new",
    "ui-world-broken",
    "ui-worlds-empty",
    "ui-mode-creative",
    "ui-mode-survival",
    "ui-new-world-mode",
    "ui-new-world-default-name",
    "ui-delete-question",
    "ui-play-time-hours",
    "ui-play-time-minutes",
    "ui-on",
    "ui-off",
    "ui-scale-auto",
    "ui-key-erase",
    "ui-key-shift",
    "ui-key-space",
    "ui-key-done",
    "ui-gen-type",
    "ui-gen-tab-general",
    "ui-gen-tab-layers",
    "ui-gen-tab-biome",
    "ui-gen-tab-code",
    "ui-gen-biome-size",
    "ui-gen-sea-level",
    "ui-gen-caves",
    "ui-gen-ores",
    "ui-gen-percent",
    "ui-gen-default",
    "ui-gen-thickness",
    "ui-gen-block",
    "ui-gen-layer-thickness",
    "ui-gen-code-applied",
    "ui-gen-preview-off",
    "ui-gen-preview-busy",
    "ui-gen-preview-legend",
    "ui-gen-preview-failed",
    "ui-gen-copied",
    "ui-gen-no-clipboard",
];

mod tests {
    use crate::input::translation_keys as key_translation_keys;
    use crate::ui::UiAssets;
    use std::path::Path;
    use strata_core::content::Content;
    use strata_core::pack::PackSet;

    #[test]
    fn every_interface_text_is_translated() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let mut errors = Vec::new();
        let packs = PackSet::discover(&assets.join("packs"), &mut errors);
        let ui = UiAssets::load(&assets.join("ui"), &packs, &mut errors).unwrap();
        assert!(errors.is_empty(), "{errors:#?}");
        let (content, _) = Content::load(&packs, &PackSet::default());
        let mut keys = ui.translation_keys();
        keys.extend(super::CODE_KEYS.iter().map(|k| (*k).to_owned()));
        keys.extend(key_translation_keys());
        keys.extend(super::super::options::translation_keys());
        keys.extend(crate::debug_overlay::translation_keys());
        keys.extend(super::super::console::translation_keys());
        let missing = content.translations.missing(&keys);
        assert!(missing.is_empty(), "missing translations: {missing:#?}");
    }
}
