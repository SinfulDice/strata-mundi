/// Fixed-length array of small unsigned integers packed into `u64` words, never straddling
/// two words. With 0 bits per entry it stores nothing (every entry is 0).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PackedArray {
    bits: u8,
    len: usize,
    words: Vec<u64>,
}

impl PackedArray {
    pub fn new(len: usize, bits: u8) -> Self {
        let words = if bits == 0 {
            0
        } else {
            len.div_ceil(per_word(bits))
        };
        Self {
            bits,
            len,
            words: vec![0; words],
        }
    }

    pub fn bits(&self) -> u8 {
        self.bits
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, index: usize) -> u16 {
        if self.bits == 0 {
            return 0;
        }
        let (word, shift) = self.locate(index);
        ((self.words[word] >> shift) & mask(self.bits)) as u16
    }

    pub fn set(&mut self, index: usize, value: u16) {
        debug_assert!(u64::from(value) <= mask(self.bits));
        if self.bits == 0 {
            return;
        }
        let (word, shift) = self.locate(index);
        let cleared = self.words[word] & !(mask(self.bits) << shift);
        self.words[word] = cleared | (u64::from(value) << shift);
    }

    /// Same values, re-packed with `bits` per entry (must be large enough for every value).
    pub fn resized(&self, bits: u8) -> Self {
        let mut resized = Self::new(self.len, bits);
        if self.bits != 0 {
            for index in 0..self.len {
                resized.set(index, self.get(index));
            }
        }
        resized
    }

    pub fn heap_bytes(&self) -> usize {
        self.words.capacity() * std::mem::size_of::<u64>()
    }

    fn locate(&self, index: usize) -> (usize, u32) {
        debug_assert!(index < self.len);
        let per_word = per_word(self.bits);
        (
            index / per_word,
            ((index % per_word) * self.bits as usize) as u32,
        )
    }
}

fn per_word(bits: u8) -> usize {
    64 / bits as usize
}

fn mask(bits: u8) -> u64 {
    (1u64 << bits) - 1
}

/// Smallest supported width holding `count` distinct values: 0, 1, 2, 4, 8 or 16 bits.
pub fn bits_for(count: usize) -> u8 {
    match count {
        0 | 1 => 0,
        2 => 1,
        3..=4 => 2,
        5..=16 => 4,
        17..=256 => 8,
        _ => 16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_round_trip_at_every_width() {
        for bits in [1, 2, 4, 8, 16] {
            let mut array = PackedArray::new(4096, bits);
            let max = (1u32 << bits) - 1;
            for i in 0..4096 {
                array.set(i, (i as u32 % (max + 1)) as u16);
            }
            for i in 0..4096 {
                assert_eq!(u32::from(array.get(i)), i as u32 % (max + 1), "bits {bits}");
            }
        }
    }

    #[test]
    fn resizing_keeps_values() {
        let mut array = PackedArray::new(100, 2);
        array.set(42, 3);
        let wider = array.resized(8);
        assert_eq!(wider.get(42), 3);
        assert_eq!(wider.get(41), 0);
    }

    #[test]
    fn widths_follow_palette_size() {
        assert_eq!(bits_for(1), 0);
        assert_eq!(bits_for(2), 1);
        assert_eq!(bits_for(5), 4);
        assert_eq!(bits_for(300), 16);
    }
}
