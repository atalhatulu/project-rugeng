//! Project-Ember (Rugeng) — Fizik Motoru Benchmark (Aşama 6: G2 Karar Raporu İçin)
//!
//! Spatial Hash Grid vs Naive O(N^2) Çarpışma Taraması
//! ve AABB Penetrasyon Çözümleme Hız Testleri.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use glam::Vec2;
use rugeng_physics::{resolve_aabb_collision_with_velocity, test_aabb_overlap, SpatialHashGrid};

fn bench_spatial_hash_vs_naive(c: &mut Criterion) {
    let mut group = c.benchmark_group("Collision Detection (5,000 Entities)");
    let n = 5000;

    // 5000 nesne üret (1000x1000 alanda rastgele dağıtılmış)
    let mut positions = Vec::with_capacity(n);
    let mut lcg = 0x12345678u64;
    for _ in 0..n {
        lcg = lcg.wrapping_mul(6364136223846793005).wrapping_add(1);
        let x = (lcg >> 32) as f32 % 1000.0;
        lcg = lcg.wrapping_mul(6364136223846793005).wrapping_add(1);
        let y = (lcg >> 32) as f32 % 1000.0;
        positions.push(Vec2::new(x, y));
    }

    // 1. Spatial Hash Grid Insert + Query
    group.bench_function("Rugeng Dahili Spatial Hash (Insert 5k + Query)", |b| {
        let mut grid = SpatialHashGrid::new(64.0, 4096, n);
        b.iter(|| {
            grid.clear();
            for (id, pos) in positions.iter().enumerate() {
                grid.insert(id as u32, pos.x, pos.y, 6.0);
            }
            let mut hit_count = 0usize;
            // 100 rastgele sorgu yap
            for pos in positions.iter().take(100) {
                grid.query_radius(pos.x, pos.y, 16.0, |_id, _x, _y, _r| {
                    hit_count += 1;
                });
            }
            black_box(hit_count);
        });
    });

    // 2. Naive O(N^2) (100 sorgu x 5000 nesne = 500.000 test)
    group.bench_function("Naive Brute Force O(N) Query (100 queries)", |b| {
        b.iter(|| {
            let mut hit_count = 0usize;
            for query in positions.iter().take(100) {
                for pos in &positions {
                    if query.distance_squared(*pos) <= 16.0 * 16.0 {
                        hit_count += 1;
                    }
                }
            }
            black_box(hit_count);
        });
    });

    group.finish();
}

fn bench_aabb_resolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("AABB Solver (10,000 Iterations)");
    let size = Vec2::new(32.0, 32.0);

    group.bench_function("test_aabb_overlap (10k)", |b| {
        let pos_a = Vec2::new(100.0, 100.0);
        let pos_b = Vec2::new(110.0, 105.0);
        b.iter(|| {
            for _ in 0..10_000 {
                let m = test_aabb_overlap(black_box(pos_a), size, black_box(pos_b), size);
                black_box(m);
            }
        });
    });

    group.bench_function("resolve_aabb_collision_with_velocity (10k)", |b| {
        b.iter(|| {
            for _ in 0..10_000 {
                let mut pos_a = black_box(Vec2::new(100.0, 100.0));
                let mut vel_a = black_box(Vec2::new(50.0, 0.0));
                let mut pos_b = black_box(Vec2::new(110.0, 105.0));
                let mut vel_b = black_box(Vec2::new(-20.0, 0.0));
                let resolved = resolve_aabb_collision_with_velocity(
                    &mut pos_a,
                    Some(&mut vel_a),
                    size,
                    true,
                    &mut pos_b,
                    Some(&mut vel_b),
                    size,
                    false,
                );
                black_box(resolved);
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_spatial_hash_vs_naive, bench_aabb_resolution);
criterion_main!(benches);
