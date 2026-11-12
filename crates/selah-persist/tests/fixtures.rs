//! Old show files must always open. One fixture per format version.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use selah_core::CURRENT_SCHEMA_VERSION;
use selah_persist::load;

fn fixture(version: u32) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/shows")
        .join(format!("v{version}"))
}

#[test]
fn every_historical_format_opens_and_upgrades() {
    for version in 1..=CURRENT_SCHEMA_VERSION {
        let loaded = load(&fixture(version)).unwrap_or_else(|e| panic!("v{version}: {e}"));
        assert_eq!(
            loaded.show.schema_version, CURRENT_SCHEMA_VERSION,
            "v{version}"
        );
        assert_eq!(loaded.show.meta.title, "Fixture service", "v{version}");
        assert_eq!(loaded.show.all_cues().len(), 2, "v{version}");
    }
}
