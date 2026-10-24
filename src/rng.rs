#![allow(clippy::should_implement_trait)]

use rand::rand_core::utils::fill_bytes_via_next_word;

// export
pub use rand::*;
pub use rapidrand::RapidRng;

type Res<T> = Result<T, !>;


// best effort for the simplest source of entropy on every platform
// the entropy rises with every call at unpredictable times
#[inline]
pub fn entropy() -> u64 {
    use std::hash::{BuildHasher, RandomState};
    RandomState::new().hash_one(crate::time::SystemTime::now())
}


// convenience method to instatiate a seedable Rng with entropy
pub trait WithEntropy: Sized {
    fn with_entropy() -> Self;
    #[inline] fn reseed_with_entropy(&mut self) { *self = Self::with_entropy() }
}

impl<R: SeedableRng> WithEntropy for R {
    #[inline] fn with_entropy() -> Self { Self::seed_from_u64(entropy()) }
}


// simple Rng wrapper around entropy
#[derive(Copy, Clone, Default, Debug)]
pub struct EntropyRng;

impl EntropyRng {
    #[inline] pub fn next(&mut self) -> u64 { entropy() }
}

impl TryRng for EntropyRng {
    type Error = !;
    #[inline] fn try_next_u64(&mut self) -> Res<u64> { Ok(self.next()) }
    #[inline] fn try_next_u32(&mut self) -> Res<u32> { Ok((self.next() >> 32) as u32) }
    #[inline] fn try_fill_bytes(&mut self, buffer: &mut [u8]) -> Res<()> { fill_bytes_via_next_word(buffer, || Ok(self.next())) }
}


// time-hash-based rng, works similar to EntropyRng internally
use crate::rapidhash::RapidHasher;
use std::hash::*;

#[derive(Copy, Clone, Default)]
pub struct RapidTimeRng { pub hasher: RapidHasher<'static> }

impl RapidTimeRng {

    #[inline] pub fn new(seed: u64) -> Self {
        Self { hasher: RapidHasher::new(seed) }
    }

    #[inline] pub fn next(&mut self) -> u64 {
        crate::time::Instant::now().hash(&mut self.hasher);
        self.hasher.finish()
    }
}

impl TryRng for RapidTimeRng {
    type Error = !;
    #[inline] fn try_next_u64(&mut self) -> Res<u64> { Ok(self.next()) }
    #[inline] fn try_next_u32(&mut self) -> Res<u32> { Ok((self.next() >> 32) as u32) }
    #[inline] fn try_fill_bytes(&mut self, buffer: &mut [u8]) -> Res<()> { fill_bytes_via_next_word(buffer, || Ok(self.next())) }
}

impl SeedableRng for RapidTimeRng {
    type Seed = [u8; 8];
    #[inline] fn from_seed(seed: [u8; 8]) -> Self { Self::new(u64::from_le_bytes(seed)) }
}



#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn rapid_rng_ranges() {

        let mut rng = RapidRng::with_entropy();

        let num: u32 = rng.random_range(1..2);
        assert_eq!(num, 1);

        let num: i32 = rng.random_range(-3..-2);
        assert_eq!(num, -3);
    }

    #[test]
    fn entropy_rng_ranges() {

        let mut rng = EntropyRng;

        let num: u32 = rng.random_range(1..2);
        assert_eq!(num, 1);

        let num: i32 = rng.random_range(-3..-2);
        assert_eq!(num, -3);
    }

    #[test]
    fn rapid_time_rng_ranges() {

        let mut rng = RapidTimeRng::with_entropy();

        let num: u32 = rng.random_range(1..2);
        assert_eq!(num, 1);

        let num: i32 = rng.random_range(-3..-2);
        assert_eq!(num, -3);
    }
}