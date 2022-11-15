use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use rand::{distributions::Uniform, prelude::*};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn fragment_memory() {
    let mut rng = SmallRng::seed_from_u64(42);
    // let rng.gen_range(1..100);
    // let mut rng = ChaCha8Rng::seed_from_u64(2);
    let allocs = (0..2000)
        .into_iter()
        .map(|_| rust_allocator::alloc(rng.gen_range(1..100)))
        .collect::<Vec<*mut u8>>();
    for i in (0..2000).step_by(2) {
        rust_allocator::dealloc(allocs[i]);
    }
}

fn alloc_benchmark_small_inp(c: &mut Criterion) {
    fragment_memory();
    let mut rng = SmallRng::seed_from_u64(0xcafebabe);
    //
    c.bench_function("alloc after some random fragmentation", |b| {
        // b.iter_batched(setup, routine, size)
    });
}

criterion_group!(benches, alloc_benchmark_small_inp);
criterion_main!(benches);
