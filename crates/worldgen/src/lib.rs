//! Deterministic world generation: the same seed always yields the same world,
//! whatever the order in which columns are generated.

/// World seed as typed by the player, hashed into 64 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Seed(pub u64);

impl Seed {
    /// Numeric text is used as-is (like Minecraft); any other text is hashed with FNV-1a,
    /// which is stable across platforms and Rust versions, unlike `std`'s hasher.
    pub fn from_text(text: &str) -> Self {
        let trimmed = text.trim();
        if let Ok(value) = trimmed.parse::<i64>() {
            return Self(value as u64);
        }
        Self(fnv1a(trimmed.as_bytes()))
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes.iter().fold(OFFSET, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(PRIME)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_seed_is_kept() {
        assert_eq!(Seed::from_text("42"), Seed(42));
        assert_eq!(Seed::from_text("-1"), Seed(u64::MAX));
    }

    #[test]
    fn text_seed_is_stable() {
        assert_eq!(Seed::from_text("strata"), Seed::from_text(" strata "));
        assert_eq!(Seed::from_text(""), Seed(0xcbf2_9ce4_8422_2325));
        assert_ne!(Seed::from_text("strata"), Seed::from_text("mundi"));
    }
}
