//! Update check at launch: asks GitHub for the latest release on a background thread,
//! so the game never waits for the network. Can be disabled in the settings; offline
//! or blocked, it simply finds nothing.

use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use ureq::tls::{RootCerts, TlsConfig};

const LATEST_RELEASE: &str = "https://api.github.com/repos/SinfulDice/strata-mundi/releases/latest";
const TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

/// Filled with the newer version's number, if one is found.
#[derive(Clone, Default)]
pub struct UpdateCheck(Arc<Mutex<Option<String>>>);

impl UpdateCheck {
    pub fn start() -> Self {
        let check = Self::default();
        let found = check.clone();
        let spawned = std::thread::Builder::new()
            .name("update-check".into())
            .spawn(move || match latest() {
                Ok(tag) if is_newer(&tag, env!("CARGO_PKG_VERSION")) => {
                    tracing::info!(%tag, "a newer version is available");
                    if let Ok(mut slot) = found.0.lock() {
                        *slot = Some(tag.trim_start_matches('v').to_owned());
                    }
                }
                Ok(_) => tracing::info!("the game is up to date"),
                Err(error) => tracing::info!(%error, "update check unavailable"),
            });
        if let Err(error) = spawned {
            tracing::warn!(%error, "update check not started");
        }
        check
    }

    pub fn newer_version(&self) -> Option<String> {
        self.0.try_lock().ok().and_then(|slot| slot.clone())
    }
}

fn latest() -> Result<String, ureq::Error> {
    // The system's certificates, so the check also works behind a company proxy.
    let tls = TlsConfig::builder()
        .root_certs(RootCerts::PlatformVerifier)
        .build();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .tls_config(tls)
        .build()
        .into();
    let release: Release = agent
        .get(LATEST_RELEASE)
        .header(
            "User-Agent",
            concat!("strata-mundi/", env!("CARGO_PKG_VERSION")),
        )
        .call()?
        .body_mut()
        .read_json()?;
    Ok(release.tag_name)
}

/// `v0.2.0` against `0.1.3`: compares the dotted numbers, ignoring a leading `v`.
fn is_newer(tag: &str, current: &str) -> bool {
    let numbers = |text: &str| -> Vec<u64> {
        text.trim_start_matches('v')
            .split(['.', '-'])
            .map_while(|part| part.parse().ok())
            .collect()
    };
    numbers(tag) > numbers(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_numerically() {
        assert!(is_newer("v0.10.0", "0.9.3"));
        assert!(is_newer("v1.0.0", "0.99.0"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
        assert!(!is_newer("nightly", "0.1.0"));
    }
}
