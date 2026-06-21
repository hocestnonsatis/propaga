use propaga_core::{Domain, DomainView};

/// Dense bitset domain over a fixed inclusive value range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitsetDomain {
    offset: i32,
    bits: Vec<u64>,
}

impl BitsetDomain {
    /// Creates a full bitset domain over the inclusive range `[min, max]`.
    #[must_use]
    pub fn new(min: i32, max: i32) -> Self {
        assert!(min <= max, "bitset domain requires min <= max");
        let len = (max - min + 1) as usize;
        let words = len.div_ceil(64);
        let mut bits = vec![0_u64; words];
        fill_range(&mut bits, 0, len);
        Self { offset: min, bits }
    }

    /// Creates a singleton domain.
    #[must_use]
    pub fn fix(value: i32) -> Self {
        Self {
            offset: value,
            bits: vec![1],
        }
    }

    /// Creates a bitset from an explicit value list inside `[min, max]`.
    #[must_use]
    pub fn from_values(min: i32, max: i32, values: &[i32]) -> Self {
        let mut domain = Self::empty_range(min, max);
        for &value in values {
            if value < min || value > max {
                continue;
            }
            let index = (value - min) as usize;
            let word = index / 64;
            let bit = index % 64;
            domain.bits[word] |= 1_u64 << bit;
        }
        domain.normalize();
        domain
    }

    /// Returns the inclusive lower bound of the value span.
    #[must_use]
    pub const fn offset(&self) -> i32 {
        self.offset
    }

    /// Returns the inclusive upper bound of the value span.
    #[must_use]
    pub fn upper_bound(&self) -> i32 {
        self.offset + self.span_len() as i32 - 1
    }

    /// Removes values strictly below `bound`.
    #[must_use]
    pub fn remove_below(&self, bound: i32) -> Self {
        if self.is_empty() {
            return self.clone();
        }
        let mut next = self.clone();
        let cutoff = (bound - next.offset).max(0) as usize;
        for word in next.bits.iter_mut().take(cutoff.div_ceil(64)) {
            *word = 0;
        }
        if cutoff > 0 {
            let word_index = cutoff / 64;
            let bit_index = cutoff % 64;
            if word_index < next.bits.len() {
                next.bits[word_index] &= !((1_u64 << bit_index) - 1);
            }
        }
        next.normalize();
        next
    }

    /// Removes values strictly above `bound`.
    #[must_use]
    pub fn remove_above(&self, bound: i32) -> Self {
        if self.is_empty() {
            return self.clone();
        }
        let mut next = self.clone();
        let last_index = (bound - next.offset + 1).max(0) as usize;
        for (index, word) in next.bits.iter_mut().enumerate() {
            let base = index * 64;
            if base >= last_index {
                *word = 0;
                continue;
            }
            if base + 64 > last_index {
                let keep = last_index - base;
                let mask = if keep >= 64 {
                    u64::MAX
                } else {
                    (1_u64 << keep) - 1
                };
                *word &= mask;
            }
        }
        next.normalize();
        next
    }

    /// Removes `value` from the domain.
    #[must_use]
    pub fn remove(&self, value: i32) -> Self {
        if !self.contains(value) {
            return self.clone();
        }
        let mut next = self.clone();
        let index = (value - next.offset) as usize;
        let word = index / 64;
        let bit = index % 64;
        next.bits[word] &= !(1_u64 << bit);
        next.normalize();
        next
    }

    /// Returns an iterator over values present in the domain.
    pub fn values(&self) -> impl Iterator<Item = i32> + '_ {
        let offset = self.offset;
        self.bits.iter().enumerate().flat_map(move |(word_index, &word)| {
            let base = word_index * 64;
            (0..64).filter_map(move |bit| {
                if word & (1_u64 << bit) != 0 {
                    Some(offset + (base + bit) as i32)
                } else {
                    None
                }
            })
        })
    }

    fn empty_range(min: i32, max: i32) -> Self {
        let len = (max - min + 1) as usize;
        Self {
            offset: min,
            bits: vec![0_u64; len.div_ceil(64)],
        }
    }

    fn span_len(&self) -> usize {
        self.bits.len() * 64
    }

    fn normalize(&mut self) {
        while let Some(&last) = self.bits.last() {
            if last == 0 {
                self.bits.pop();
            } else {
                break;
            }
        }
    }
}

impl DomainView for BitsetDomain {
    type Value = i32;

    fn is_empty(&self) -> bool {
        self.bits.iter().all(|word| *word == 0)
    }

    fn is_fixed(&self) -> bool {
        self.size() == 1
    }

    fn size(&self) -> usize {
        self.bits.iter().map(|word| word.count_ones() as usize).sum()
    }

    fn min(&self) -> Option<Self::Value> {
        self.values().next()
    }

    fn max(&self) -> Option<Self::Value> {
        for (word_index, word) in self.bits.iter().enumerate().rev() {
            if *word == 0 {
                continue;
            }
            let bit = 63 - word.leading_zeros();
            return Some(self.offset + (word_index * 64 + bit as usize) as i32);
        }
        None
    }

    fn contains(&self, value: Self::Value) -> bool {
        let index = value - self.offset;
        if index < 0 {
            return false;
        }
        let index = index as usize;
        let word = index / 64;
        let bit = index % 64;
        self.bits.get(word).is_some_and(|bits| bits & (1_u64 << bit) != 0)
    }
}

impl Domain for BitsetDomain {}

fn fill_range(bits: &mut [u64], start: usize, end: usize) {
    for index in start..end {
        let word = index / 64;
        let bit = index % 64;
        bits[word] |= 1_u64 << bit;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_interior_value() {
        let domain = BitsetDomain::new(1, 5).remove(3);
        assert_eq!(domain.size(), 4);
        assert!(!domain.contains(3));
    }

    #[test]
    fn values_iterator() {
        let domain = BitsetDomain::new(2, 4);
        let values: Vec<_> = domain.values().collect();
        assert_eq!(values, vec![2, 3, 4]);
    }
}
