//! Text codes of generation settings, to share a world type: `S1-` then the settings in
//! postcard with a checksum, in Crockford base32 (no look-alike letters, any case).

use super::{FlatLayer, FlatSettings, WorldgenSettings};
use crate::id::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const VERSION: u32 = 1;
/// Longer texts are refused before decoding (64 long layers stay well below).
pub const MAX_CODE_LENGTH: usize = 4096;
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// The pack's namespace is left out of codes to keep them short.
const BASE_NAMESPACE: &str = "strata";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CodeError {
    #[error("this code is damaged or is not a world code")]
    Malformed,
    #[error("this code comes from a newer version of the game (format {0})")]
    FutureVersion(u32),
}

impl CodeError {
    /// Fluent message explaining the error to the player.
    pub fn message_id(&self) -> &'static str {
        match self {
            Self::Malformed => "world-code-malformed",
            Self::FutureVersion(_) => "world-code-future",
        }
    }
}

/// Field order is the format: never change it, add a `CodeV2` instead.
#[derive(Serialize, Deserialize)]
struct CodeV1 {
    world_type: String,
    biome_size: u16,
    sea_level: Option<i32>,
    caves: u16,
    ores: u16,
    structures: u16,
    biome: Option<String>,
    flat: Option<FlatV1>,
}

/// Layers `(block, thickness)`, caves, features.
type FlatV1 = (Vec<(String, u16)>, bool, bool);

impl WorldgenSettings {
    pub fn to_code(&self) -> String {
        let code = CodeV1 {
            world_type: short(&self.world_type),
            biome_size: self.biome_size,
            sea_level: self.sea_level,
            caves: self.caves,
            ores: self.ores,
            structures: self.structures,
            biome: self.biome.as_ref().map(short),
            flat: self.flat.as_ref().map(|flat| {
                let layers = flat.layers.iter().map(|l| (short(&l.block), l.thickness));
                (layers.collect(), flat.caves, flat.features)
            }),
        };
        // Serializing plain integers and strings into memory cannot fail.
        let mut bytes = postcard::to_stdvec(&code).unwrap_or_default();
        bytes.extend_from_slice(&checksum(&bytes));
        format!("S{VERSION}-{}", encode(&bytes))
    }

    /// Spaces, dashes and case are ignored; the values read are bounded.
    pub fn from_code(text: &str) -> Result<Self, CodeError> {
        if text.len() > MAX_CODE_LENGTH {
            return Err(CodeError::Malformed);
        }
        let text = text.trim().to_ascii_uppercase();
        let (version, body) = text
            .strip_prefix('S')
            .and_then(|rest| rest.split_once('-'))
            .ok_or(CodeError::Malformed)?;
        let version: u32 = version.parse().map_err(|_| CodeError::Malformed)?;
        if version > VERSION {
            return Err(CodeError::FutureVersion(version));
        }
        if version != VERSION {
            return Err(CodeError::Malformed);
        }
        let bytes = decode(body).ok_or(CodeError::Malformed)?;
        let (payload, sum) = bytes.split_last_chunk::<2>().ok_or(CodeError::Malformed)?;
        if checksum(payload) != *sum {
            return Err(CodeError::Malformed);
        }
        let code: CodeV1 = postcard::from_bytes(payload).map_err(|_| CodeError::Malformed)?;
        settings(code).map(Self::bounded)
    }
}

fn settings(code: CodeV1) -> Result<WorldgenSettings, CodeError> {
    let flat = code
        .flat
        .map(|(layers, caves, features)| {
            let layers = layers
                .iter()
                .map(|(block, thickness)| {
                    long(block).map(|block| FlatLayer {
                        block,
                        thickness: *thickness,
                    })
                })
                .collect::<Result<_, _>>()?;
            Ok(FlatSettings {
                layers,
                caves,
                features,
            })
        })
        .transpose()?;
    Ok(WorldgenSettings {
        world_type: long(&code.world_type)?,
        biome_size: code.biome_size,
        sea_level: code.sea_level,
        caves: code.caves,
        ores: code.ores,
        structures: code.structures,
        biome: code.biome.as_deref().map(long).transpose()?,
        flat,
    })
}

fn short(id: &Identifier) -> String {
    if id.namespace() == BASE_NAMESPACE {
        id.name().to_owned()
    } else {
        id.to_string()
    }
}

fn long(text: &str) -> Result<Identifier, CodeError> {
    let parsed = if text.contains(':') {
        text.parse()
    } else {
        Identifier::new(BASE_NAMESPACE, text)
    };
    parsed.map_err(|_| CodeError::Malformed)
}

fn checksum(bytes: &[u8]) -> [u8; 2] {
    let hash = crate::hash::fnv1a(bytes);
    [(hash >> 8) as u8, hash as u8]
}

fn encode(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 8 / 5 + 1);
    let (mut buffer, mut bits) = (0u32, 0);
    for &byte in bytes {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            text.push(char::from(ALPHABET[((buffer >> bits) & 31) as usize]));
        }
    }
    if bits > 0 {
        text.push(char::from(ALPHABET[((buffer << (5 - bits)) & 31) as usize]));
    }
    text
}

fn decode(text: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(text.len() * 5 / 8);
    let (mut buffer, mut bits) = (0u32, 0);
    for c in text.bytes().filter(|c| !matches!(c, b'-' | b' ')) {
        let c = match c {
            b'O' => b'0',
            b'I' | b'L' => b'1',
            other => other,
        };
        let value = ALPHABET.iter().position(|&a| a == c)? as u32;
        buffer = ((buffer << 5) | value) & 0xFFFF;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            bytes.push((buffer >> bits) as u8);
        }
    }
    // Padding bits must be zero, or a character was added or changed at the end.
    (buffer & ((1 << bits) - 1) == 0).then_some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat() -> WorldgenSettings {
        WorldgenSettings {
            world_type: Identifier::builtin("strata", "flat"),
            biome_size: 150,
            sea_level: Some(64),
            caves: 0,
            ores: 250,
            structures: 40,
            biome: Some(Identifier::builtin("other", "meadow")),
            flat: Some(FlatSettings {
                layers: vec![
                    FlatLayer {
                        block: Identifier::builtin("strata", "bedrock"),
                        thickness: 1,
                    },
                    FlatLayer {
                        block: Identifier::builtin("strata", "stone"),
                        thickness: 60,
                    },
                ],
                caves: true,
                features: false,
            }),
        }
    }

    #[test]
    fn codes_round_trip_exactly() {
        for settings in [WorldgenSettings::default(), flat()] {
            let code = settings.to_code();
            assert!(code.starts_with("S1-"), "{code}");
            assert_eq!(WorldgenSettings::from_code(&code), Ok(settings.clone()));
            let typed = format!("  {}  ", code.to_lowercase());
            assert_eq!(WorldgenSettings::from_code(&typed), Ok(settings));
        }
        assert!(WorldgenSettings::default().to_code().len() < 40);
    }

    #[test]
    fn damaged_codes_are_errors() {
        let code = flat().to_code();
        for bad in ["", "S1-", "S1", "X1-ABC", "S1-U!", "Sx-00", "S0-00"] {
            assert_eq!(
                WorldgenSettings::from_code(bad),
                Err(CodeError::Malformed),
                "{bad}"
            );
        }
        let truncated = &code[..code.len() - 3];
        assert_eq!(
            WorldgenSettings::from_code(truncated),
            Err(CodeError::Malformed)
        );
        for index in 3..code.len() {
            let mut damaged = code.clone().into_bytes();
            damaged[index] = if damaged[index] == b'Z' { b'Y' } else { b'Z' };
            let damaged = String::from_utf8(damaged).unwrap();
            assert!(WorldgenSettings::from_code(&damaged).is_err(), "{damaged}");
        }
    }

    #[test]
    fn overlong_texts_are_refused() {
        let long = format!("S1-{}", "0".repeat(MAX_CODE_LENGTH));
        assert_eq!(
            WorldgenSettings::from_code(&long),
            Err(CodeError::Malformed)
        );
    }

    #[test]
    fn future_versions_are_recognised() {
        assert_eq!(
            WorldgenSettings::from_code("S7-ABCDEF"),
            Err(CodeError::FutureVersion(7))
        );
    }

    #[test]
    fn decoded_values_are_bounded() {
        let settings = WorldgenSettings {
            caves: 4000,
            ..WorldgenSettings::default()
        };
        let decoded = WorldgenSettings::from_code(&settings.to_code()).unwrap();
        assert_eq!(decoded.caves, 400);
    }
}
