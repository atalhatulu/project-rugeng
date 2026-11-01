use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rugeng_core::FrameArena;
use rugeng_scheduler::{ThreadArenaPool, ThreadPool};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Position([f32; 2]);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity([f32; 2]);

fn bench_serial_vs_parallel_physics(c: &mut Criterion) {
    let mut group = c.benchmark_group("bullet_physics_20k");
    const BULLET_COUNT: usize = 20_000;
    const DT: f32 = 1.0 / 60.0;

    let pool = ThreadPool::new(0); // system thread count

    // Seri güncelleme (Baseline)
    group.bench_function("serial_movement_20k", |b| {
        let mut positions = vec![Position([100.0, 200.0]); BULLET_COUNT];
        let velocities = vec![Velocity([150.0, -100.0]); BULLET_COUNT];

        b.iter(|| {
            for (pos, vel) in positions.iter_mut().zip(velocities.iter()) {
                pos.0[0] += vel.0[0] * DT;
                pos.0[1] += vel.0[1] * DT;
                black_box(pos);
            }
        });
    });

    // Paralel work-stealing chunking (ThreadPool::scope)
    group.bench_function("parallel_chunked_movement_20k", |b| {
        let mut positions = vec![Position([100.0, 200.0]); BULLET_COUNT];
        let velocities = vec![Velocity([150.0, -100.0]); BULLET_COUNT];

        b.iter(|| {
            pool.scope(|s| {
                let worker_count = pool.worker_count();
                let chunk_size = (BULLET_COUNT / worker_count).max(256);
                let pos_chunks = positions.chunks_mut(chunk_size);
                let vel_chunks = velocities.chunks(chunk_size);

                for (pos_chunk, vel_chunk) in pos_chunks.zip(vel_chunks) {
                    s.spawn(move || {
                        for (pos, vel) in pos_chunk.iter_mut().zip(vel_chunk.iter()) {
                            pos.0[0] += vel.0[0] * DT;
                            pos.0[1] += vel.0[1] * DT;
                            black_box(pos);
                        }
                    });
                }
            });
        });
    });

    group.finish();
}

fn bench_arena_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("arena_contention");
    const THREADS: usize = 8;
    const ALLOCS_PER_THREAD: usize = 1_000;

    // 1. Ortak Atomik FrameArena (Atomic CAS contention)
    group.bench_function("shared_atomic_frame_arena_8t", |b| {
        let mut arena = FrameArena::new(32 * 1024 * 1024).unwrap();
        let pool = ThreadPool::new(THREADS);

        b.iter(|| {
            let arena_ref = &arena;
            pool.scope(|s| {
                for _ in 0..THREADS {
                    s.spawn(move || {
                        for i in 0..ALLOCS_PER_THREAD {
                            let val = arena_ref.alloc(i as u64).unwrap();
                            black_box(val);
                        }
                    });
                }
            });
            arena.reset();
        });
    });

    // 2. Worker Başına ThreadArenaPool (0 Contention)
    group.bench_function("thread_arena_pool_independent_8t", |b| {
        let mut pool_arena = ThreadArenaPool::new(THREADS, 8 * 1024 * 1024).unwrap();
        let pool = ThreadPool::new(THREADS);

        b.iter(|| {
            let pool_ref = &pool_arena;
            pool.scope(|s| {
                for thread_id in 0..THREADS {
                    s.spawn(move || {
                        let arena = pool_ref.get(thread_id);
                        for i in 0..ALLOCS_PER_THREAD {
                            let val = arena.alloc(i as u64).unwrap();
                            black_box(val);
                        }
                    });
                }
            });
            pool_arena.reset_all();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_serial_vs_parallel_physics,
    bench_arena_contention
);
criterion_main!(benches);
