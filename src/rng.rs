
// export
pub use rand::{*, Rng as RngTrait};
pub use rapidhash::rng::RapidRng;


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
    #[inline]
    fn reseed_with_entropy(&mut self) { *self = Self::with_entropy() }
}

impl<Rng: SeedableRng> WithEntropy for Rng {
    #[inline]
    fn with_entropy() -> Self { Self::seed_from_u64(entropy()) }
}


// simple Rng wrapper around entropy
#[derive(Copy, Clone, Default, Debug)]
pub struct EntropyRng;

impl EntropyRng {
    #[inline]
    pub fn next(&mut self) -> u64 { entropy() }
}

impl RngCore for EntropyRng {

    #[inline]
    fn next_u64(&mut self) -> u64 { self.next() }

    #[inline]
    fn next_u32(&mut self) -> u32 { self.next() as u32 }

    #[inline]
    fn fill_bytes(&mut self, buffer: &mut [u8]) {

        let (chunks, remainder) = buffer.as_chunks_mut::<8>();

        for chunk in chunks {
            *chunk = self.next().to_le_bytes();
        }

        if !remainder.is_empty() {
            let random = self.next().to_le_bytes();
            remainder.copy_from_slice(&random[0..remainder.len()]);
        }
    }
}


// time-hash-based rng, works similar to EntropyRng internally
use crate::rapidhash::RapidHasher;
use std::hash::*;

#[derive(Copy, Clone, Default)]
pub struct RapidTimeRng { pub hasher: RapidHasher }

impl RapidTimeRng {

    #[inline]
    pub fn new(seed: u64) -> Self {
        Self { hasher: RapidHasher::new(seed) }
    }

    #[inline]
    pub fn next(&mut self) -> u64 {
        crate::time::Instant::now().hash(&mut self.hasher);
        self.hasher.finish()
    }
}

impl RngCore for RapidTimeRng {

    #[inline]
    fn next_u64(&mut self) -> u64 { self.next() }

    #[inline]
    fn next_u32(&mut self) -> u32 { self.next() as u32 }

    #[inline]
    fn fill_bytes(&mut self, buffer: &mut [u8]) {

        let (chunks, remainder) = buffer.as_chunks_mut::<8>();

        for chunk in chunks {
            *chunk = self.next().to_le_bytes();
        }

        if !remainder.is_empty() {
            let random = self.next().to_le_bytes();
            remainder.copy_from_slice(&random[0..remainder.len()]);
        }
    }
}

impl SeedableRng for RapidTimeRng {
    type Seed = [u8; 8];

    #[inline]
    fn from_seed(seed: [u8; 8]) -> Self { Self::new(u64::from_le_bytes(seed)) }
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