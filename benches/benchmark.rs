use std::collections::VecDeque;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::prelude::*;

fn fragment_memory() {
    let mut rng = SmallRng::seed_from_u64(42);
    let h = 200000;
    let allocs = (0..h)
        .into_iter()
        .map(|_| rust_allocator::alloc(rng.gen_range(1..100)))
        .collect::<Vec<*mut u8>>();
    for i in (0..h).step_by(2) {
        if !allocs[i].is_null() {
            rust_allocator::dealloc(allocs[i]);
        }
    }
}

fn alloc_benchmark_small_inp(c: &mut Criterion) {
    // fragment_memory();
    let mut rng1 = SmallRng::seed_from_u64(0xcafebabe);
    // let mut rng1 = Uniform
    let mut rng2 = SmallRng::seed_from_u64(0xaaaaaaaa);
    //
    let mut dq = VecDeque::new();
    c.bench_function("alloc after some random fragmentation", |b| {
        b.iter(|| {
            let mem = rust_allocator::alloc(black_box(rng1.gen_range(100..=2000)));
            // // FIX: problem when we uncomment this? why?
            // let v = rng2.gen_range(0..=1);
            dq.push_back(mem);
            if dq.len() > 1000 {
                rust_allocator::dealloc(*dq.front().unwrap());
                dq.pop_front();
            }
        })
    });
}

criterion_group!(benches, alloc_benchmark_small_inp);
criterion_main!(benches);
