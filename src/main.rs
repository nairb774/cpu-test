use std::env;
use std::mem;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use log::*;
use rand::Rng;

const ITERATION_COUNT: usize = 1 << 20;
const ORDER: Ordering = Ordering::Relaxed;

type AtomicType = AtomicU32;
type IndexType = u32;

struct Slot {
    atomic: AtomicType,
    me: IndexType,
}

fn run_rand(iteration_counter: &AtomicUsize, mem: &Vec<Slot>, finish: &Instant) {
    let mut r = rand::thread_rng();
    while Instant::now() < *finish {
        iteration_counter.fetch_add(1, ORDER);
        for _ in 0..ITERATION_COUNT {
            let idx = r.gen_range(0..mem.len());
            mem[idx].atomic.fetch_add(1, ORDER);
            assert_eq!(mem[idx].me, idx as IndexType);
        }
    }
}

fn parse_env<F>(name: &str, default: &str) -> Result<F, F::Err>
where
    F: std::str::FromStr,
{
    let value = env::var(name).unwrap_or_else(|_| default.to_string());
    info!("{name}={value}");
    value.parse()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let block_size: usize = parse_env("BLOCK_SIZE", "1")?;
    let thread_count: usize = parse_env("THREAD_COUNT", "1")?;
    let validation_seconds: u64 = parse_env("VALIDATION_SECONDS", "60")?;

    let mut mem = Vec::with_capacity((block_size << 30) / mem::size_of::<Slot>());
    for idx in 0..mem.capacity() {
        mem.push(Slot {
            atomic: AtomicType::new(0),
            me: idx as IndexType,
        });
    }

    info!("Starting...");
    let iteration_counter = AtomicUsize::new(0);
    loop {
        let finish = Instant::now()
            .checked_add(Duration::from_secs(validation_seconds))
            .unwrap();
        thread::scope(|scope| {
            for _ in 0..thread_count {
                scope.spawn(|| {
                    run_rand(&iteration_counter, &mem, &finish);
                });
            }
        });

        let total: usize = mem
            .iter()
            .map(|slot| slot.atomic.load(Ordering::Acquire) as usize)
            .sum();
        let iter_count = iteration_counter.load(Ordering::Acquire);
        info!("{iter_count} iterations.");
        assert_eq!(total, iter_count * ITERATION_COUNT);
    }
}
