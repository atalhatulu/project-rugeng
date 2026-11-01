use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use glam::Vec2;
use rugeng_core::{FlatSparseTable, FrameArena, PagedSparseTable, Registry, SparseTable};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub pos: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Velocity {
    pub vel: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprite {
    pub color: [f32; 4],
    pub tex_id: u32,
    pub size: Vec2,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct QuadInstance {
    pub pos: Vec2,
    pub size: Vec2,
    pub color: [f32; 4],
    pub tex_id: u32,
}

/// 1 Milyon Entity'lik Heterojen ve Dağınık Dünya Kurulumu.
///
/// - Position: %100 (1.000.000 entity)
/// - Sprite: %70 (700.000 entity)
/// - Velocity: %40 (400.000 entity)
///
/// Çalışma kümesi ~40 MB olup L1/L2 önbellek sınırlarını aşar.
fn setup_1m_heterogeneous_world<S: SparseTable>() -> Registry<S> {
    let mut reg = Registry::with_sparse_table();
    reg.register_component::<Position>();
    reg.register_component::<Velocity>();
    reg.register_component::<Sprite>();

    // Deterministik basit LCG rastgele sayı üreteci
    let mut state: u64 = 0x123456789ABCDEF;
    let mut next_rand = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 32) as u32
    };

    const TOTAL_ENTITIES: usize = 1_000_000;

    for i in 0..TOTAL_ENTITIES {
        let e = reg.create();
        let fi = i as f32;

        // Her entity'nin pozisyonu vardır (%100)
        reg.emplace(
            e,
            Position {
                pos: Vec2::new(fi * 0.1, fi * 0.2),
            },
        );

        // %40 olasılıkla Velocity
        if next_rand() % 100 < 40 {
            reg.emplace(
                e,
                Velocity {
                    vel: Vec2::new(1.0, 2.0),
                },
            );
        }

        // %70 olasılıkla Sprite
        if next_rand() % 100 < 70 {
            reg.emplace(
                e,
                Sprite {
                    color: [1.0, 0.5, 0.2, 1.0],
                    tex_id: (i % 16) as u32,
                    size: Vec2::new(16.0, 16.0),
                },
            );
        }
    }

    reg
}

fn bench_1m_movement(c: &mut Criterion) {
    let mut group = c.benchmark_group("movement_1m_heterogeneous");
    group.sample_size(20); // 1M entity için örnek boyutunu optimize et

    println!("1M Heterogeneous World (Flat) hazirlaniyor...");
    let mut flat_world = setup_1m_heterogeneous_world::<FlatSparseTable>();

    println!("1M Heterogeneous World (Paged) hazirlaniyor...");
    let mut paged_world = setup_1m_heterogeneous_world::<PagedSparseTable>();

    let dt = black_box(1.0 / 60.0);

    // Flat: 400k hareketli entity iterasyonu (Pos mut, Vel ref)
    group.bench_function(BenchmarkId::new("flat", "1m_entities_400k_moving"), |b| {
        b.iter(|| {
            flat_world.view2_mut_ref::<Position, Velocity, _>(|_entity, pos, vel| {
                pos.pos += vel.vel * dt;
            });
            black_box(&mut flat_world);
        });
    });

    // Paged: 400k hareketli entity iterasyonu (Pos mut, Vel ref)
    group.bench_function(BenchmarkId::new("paged", "1m_entities_400k_moving"), |b| {
        b.iter(|| {
            paged_world.view2_mut_ref::<Position, Velocity, _>(|_entity, pos, vel| {
                pos.pos += vel.vel * dt;
            });
            black_box(&mut paged_world);
        });
    });

    group.finish();
}

fn bench_1m_render_gather(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_gather_1m_heterogeneous");
    group.sample_size(20);

    let flat_world = setup_1m_heterogeneous_world::<FlatSparseTable>();
    let paged_world = setup_1m_heterogeneous_world::<PagedSparseTable>();

    let mut arena = FrameArena::new(64 * 1024 * 1024).expect("Arena tahsis edilemedi");

    // Flat: 700k Sprite toplayıcı (Pos ref, Sprite ref -> Arena Scratch)
    group.bench_function(BenchmarkId::new("flat", "1m_entities_700k_sprites"), |b| {
        b.iter(|| {
            arena.reset();
            let mut instances = Vec::with_capacity(700_000);
            flat_world.view2_ref_ref::<Position, Sprite, _>(|_e, pos, sprite| {
                instances.push(QuadInstance {
                    pos: pos.pos,
                    size: sprite.size,
                    color: sprite.color,
                    tex_id: sprite.tex_id,
                });
            });
            let slice = arena.alloc_slice_copy(&instances).unwrap();
            black_box(slice);
        });
    });

    // Paged: 700k Sprite toplayıcı (Pos ref, Sprite ref -> Arena Scratch)
    group.bench_function(BenchmarkId::new("paged", "1m_entities_700k_sprites"), |b| {
        b.iter(|| {
            arena.reset();
            let mut instances = Vec::with_capacity(700_000);
            paged_world.view2_ref_ref::<Position, Sprite, _>(|_e, pos, sprite| {
                instances.push(QuadInstance {
                    pos: pos.pos,
                    size: sprite.size,
                    color: sprite.color,
                    tex_id: sprite.tex_id,
                });
            });
            let slice = arena.alloc_slice_copy(&instances).unwrap();
            black_box(slice);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_1m_movement, bench_1m_render_gather);
criterion_main!(benches);
