
use platform::{time::Instant, rng::*};

fn main() {

    let runs = 3;
    let times = 100_000;
    let range = 21;

    for _ in 0..runs {

        {
            let mut rng = RapidRng::with_entropy();

            let mut sum: u64 = 0;

            let then = Instant::now();

            for _ in 0..times {
                sum += rng.gen_range(0..range);
            }

            let elapsed = then.elapsed();

            println!("RapidRng {:?} | {:?}", sum as f64 / times as f64, elapsed);
        }

        {
            let mut rng = TimeRng::with_entropy();

            let mut sum: u64 = 0;

            let then = Instant::now();

            for _ in 0..times {
                sum += rng.gen_range(0..range);
            }

            let elapsed = then.elapsed();

            println!("TimeRng {:?} | {:?}", sum as f64 / times as f64, elapsed);
        }

        {
            let mut sum: u64 = 0;

            let then = Instant::now();

            for _ in 0..times {
                sum += entropy() % range;
            }

            let elapsed = then.elapsed();

            println!("entropy {:?} | {:?}", sum as f64 / times as f64, elapsed);
        }

    }
}