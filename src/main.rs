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

fn main() {
    env_logger::init();
    info!("Starting...");

    const BLOCK: usize = 16 << 30;
    const THREAD_COUNT: usize = 32;

    let mut mem = Vec::with_capacity(BLOCK / mem::size_of::<Slot>());
    for idx in 0..mem.capacity() {
        mem.push(Slot {
            atomic: AtomicType::new(0),
            me: idx as IndexType,
        });
    }

    let iteration_counter = AtomicUsize::new(0);
    loop {
        let finish = Instant::now()
            .checked_add(Duration::from_secs(5 * 60))
            .unwrap();
        thread::scope(|scope| {
            for _ in 0..THREAD_COUNT {
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
