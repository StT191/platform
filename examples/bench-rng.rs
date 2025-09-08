
use platform::{time::*, rng::*};

fn main() {

    let runs = 3;
    let times = 100_000;
    let exp = 0.5;

    let dashes = "-".repeat(42);

    println!("{dashes}\ntimes: {times}, runs: {runs}");

    for _ in 0..runs {

        println!("{dashes}");

        {
            let mut rng = RapidRng::default();

            let mut sum: f64 = 0.0;

            let then = Instant::now();

            for _ in 0..times {
                sum += rng.random::<f64>();
            }

            let elapsed = then.elapsed();

            println!("RapidRng\t{:+.6?}   ~   {:?}", exp - sum / times as f64, elapsed);
        }

        {
            let mut rng = RapidTimeRng::default();

            let mut sum: f64 = 0.0;

            let then = Instant::now();

            for _ in 0..times {
                sum += rng.random::<f64>();
            }

            let elapsed = then.elapsed();

            println!("RapidTimeRng\t{:+.6?}   ~   {:?}", exp - sum / times as f64, elapsed);
        }

        {
            let mut rng = EntropyRng::default();

            let mut sum: f64 = 0.0;

            let then = Instant::now();

            for _ in 0..times {
                sum += rng.random::<f64>();
            }

            let elapsed = then.elapsed();

            println!("EntropyRng\t{:+.6?}   ~   {:?}", exp - sum / times as f64, elapsed);
        }
    }

    println!("{dashes}");
}