use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::prelude::*;

fn _fragment_memory() {
    let mut rng = SmallRng::seed_from_u64(42);
    let mut rng1 = SmallRng::seed_from_u64(0xcafebabe);
    let h = 200000;
    (0..h).into_iter().for_each(|_| {
        let m = rust_allocator::alloc(rng.gen_range(1..100));
        if rng1.gen_range(1..=100) > 80 {
            rust_allocator::dealloc(m);
        }
    });
}

fn alloc_benchmark_small_inp(c: &mut Criterion) {
    // std::env::set_var("MIN_EXPANSION_WORDSIZE", "1048576");

    _fragment_memory();
    let mut rng1 = SmallRng::seed_from_u64(0xcafebabe);
    let mut rng2 = SmallRng::seed_from_u64(0xaaaaaaaa);

    c.bench_function("alloc after some random fragmentation", |b| {
        b.iter(|| {
            let mem = rust_allocator::alloc(black_box(rng1.gen_range(1..=4096)));
            let v = rng2.gen_range(0..=1);
            if v == 1 {
                rust_allocator::dealloc(mem);
            }
        })
    });
}

criterion_group!(benches, alloc_benchmark_small_inp);
criterion_main!(benches);
