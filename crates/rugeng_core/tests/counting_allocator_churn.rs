use rugeng_core::{CommandBuffer, Registry};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingAllocator;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static A: CountingAllocator = CountingAllocator;

#[derive(Clone, Copy, PartialEq, Debug)]
struct Transform {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Velocity {
    vx: f32,
    vy: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Health {
    hp: i32,
}

#[test]
fn test_gameplay_loop_churn_10k_zero_malloc() {
    let mut reg = Registry::new();
    reg.register_component::<Transform>();
    reg.register_component::<Velocity>();
    reg.register_component::<Health>();

    let mut cmd = CommandBuffer::new();

    const ENTITY_COUNT: usize = 10_000;

    // 1. Warmup Karesi: Kapasiteleri önceden ısıt
    let mut initial_entities = Vec::with_capacity(ENTITY_COUNT);
    for i in 0..ENTITY_COUNT {
        let e = reg.create();
        cmd.emplace(
            e,
            Transform {
                x: i as f32,
                y: i as f32,
            },
        );
        cmd.emplace(e, Velocity { vx: 1.0, vy: 1.0 });
        if i % 2 == 0 {
            cmd.emplace(e, Health { hp: 100 });
        }
        initial_entities.push(e);
    }
    cmd.apply(&mut reg);

    // Warmup silme ve ekleme churn'ü (steady state kapasitelerini önceden açar)
    let mut warmup_destroy = Vec::with_capacity(1000);
    for _ in 0..2 {
        warmup_destroy.clear();
        reg.view1_ref::<Transform, _>(|e, _| {
            if warmup_destroy.len() < 1000 {
                warmup_destroy.push(e);
            }
        });
        for &e in &warmup_destroy {
            cmd.destroy(e);
        }
        for i in 0..1000 {
            let new_e = reg.create();
            cmd.emplace(new_e, Transform { x: 0.0, y: 0.0 });
            cmd.emplace(new_e, Velocity { vx: 0.5, vy: -0.5 });
            if i % 3 == 0 {
                cmd.emplace(new_e, Health { hp: 50 });
            }
        }
        cmd.apply(&mut reg);
    }

    let mut to_destroy = Vec::with_capacity(1000);

    // 2. Steady-State Ölçümü: 10 ardışık kare boyunca yoğun churn (Spawn, Despawn, Ekle, Çıkar)
    let allocs_before = ALLOC_COUNT.load(Ordering::SeqCst);

    for frame in 0..10 {
        let b = ALLOC_COUNT.load(Ordering::SeqCst);
        // Her kare 1.000 entity topla ve CommandBuffer ile sil
        to_destroy.clear();
        reg.view1_ref::<Transform, _>(|e, _| {
            if to_destroy.len() < 1000 {
                to_destroy.push(e);
            }
        });

        for &e in &to_destroy {
            cmd.destroy(e);
        }

        // 1.000 yeni entity oluştur ve bileşen ekle
        for i in 0..1000 {
            let new_e = reg.create();
            cmd.emplace(
                new_e,
                Transform {
                    x: (frame * 1000 + i) as f32,
                    y: 0.0,
                },
            );
            cmd.emplace(new_e, Velocity { vx: 0.5, vy: -0.5 });
            if i % 3 == 0 {
                cmd.emplace(new_e, Health { hp: 50 });
            }
        }
        cmd.apply(&mut reg);
        let a = ALLOC_COUNT.load(Ordering::SeqCst);
        if a > b {
            println!("Frame {} de {} malloc oldu!", frame, a - b);
        }
    }

    let allocs_after = ALLOC_COUNT.load(Ordering::SeqCst);
    let diff = allocs_after - allocs_before;

    println!("\n========================================================");
    println!(" [COUNTING ALLOCATOR RAPORU - 10K ENTITY CHURN] ");
    println!(" Steady-State Toplam Kare: 10");
    println!(" Kare Başına Spawn/Despawn: 1.000 Entity (Toplam 10.000 Churn)");
    println!(" Steady-State Malloc Sayısı: {}", diff);
    println!("========================================================\n");

    assert_eq!(
        diff, 0,
        "Gameplay döngüsünde steady-state'te 0 malloc olmalidir!"
    );
}
