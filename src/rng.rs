
// export
use std::sync::Once;

pub use rand::{*, Rng as RngTrait};
pub use getrandom;

pub use rapidhash::RapidRng;


#[inline]
pub fn entropy() -> u64 {

    // try with getrandom
    use std::mem::{MaybeUninit, transmute};

    unsafe {
        // SAFETY: array of uninits is valid
        let mut bytes_uninit: [MaybeUninit<u8>; 8] = MaybeUninit::uninit().assume_init();

        match getrandom::fill_uninit(&mut bytes_uninit) {
            Ok(_) => {
                // SAFETY: bytes can be assumed init after getrandom succeeds
                let bytes: [u8; 8] = transmute(bytes_uninit);
                return u64::from_ne_bytes(bytes);
            },
            Err(err) => {
                static REPORT_ERR: Once = Once::new();

                REPORT_ERR.call_once(|| {
                    log::warn!("getrandom failed: {:?}", err);
                });
            },
        }
    }

    // fallback
    use std::hash::{Hash, Hasher, BuildHasher, RandomState};
    use crate::time::Instant;

    let mut hasher = RandomState::new().build_hasher();
    Instant::now().hash(&mut hasher);

    hasher.finish()
}


// convenience method to instatiate a Rng with entropy

pub trait WithEntropy {
    fn with_entropy() -> Self;
}

impl<Rng: SeedableRng> WithEntropy for Rng {
    #[inline]
    fn with_entropy() -> Self { Self::seed_from_u64(entropy()) }
}


// custom time-based rng
use rapidhash::RapidHasher;
use std::hash::*;
use crate::time::Instant;

#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub struct TimeRng { pub hasher: RapidHasher }

impl TimeRng {

    #[inline]
    pub fn new(seed: u64) -> Self {
        Self { hasher: RapidHasher::new(seed) }
    }

    #[inline]
    pub fn next(&mut self) -> u64 {
        Instant::now().hash(&mut self.hasher);
        self.hasher.finish()
    }
}

impl RngCore for TimeRng {

    #[inline]
    fn next_u64(&mut self) -> u64 { self.next() }

    #[inline]
    fn next_u32(&mut self) -> u32 { self.next() as u32 }

    #[inline]
    fn fill_bytes(&mut self, buffer: &mut [u8]) {

        let mut chunks = buffer.array_chunks_mut::<8>();

        for chunk in &mut chunks {
            *chunk = self.next().to_le_bytes();
        }

        let remainder = chunks.into_remainder();

        if !remainder.is_empty() {
            let random = self.next().to_le_bytes();
            for i in 0..remainder.len() {
                remainder[i] = random[i];
            }
        }

    }
}

impl SeedableRng for TimeRng {
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
    fn time_rng_ranges() {

        let mut rng = TimeRng::with_entropy();

        let num: u32 = rng.random_range(1..2);
        assert_eq!(num, 1);

        let num: i32 = rng.random_range(-3..-2);
        assert_eq!(num, -3);
    }
}