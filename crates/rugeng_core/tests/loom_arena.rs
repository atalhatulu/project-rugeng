//! Loom eşzamanlılık testi: FrameArena atomik CAS bump mantığının tüm olası
//! thread interleaving durumlarında yarışma ve çakışma (overlap) üretmediğini doğrular.

#![allow(unexpected_cfgs)]

#[cfg(loom)]
mod loom_tests {
    use loom::sync::atomic::{AtomicUsize, Ordering};
    use loom::sync::Arc;
    use loom::thread;

    struct LoomArenaModel {
        capacity: usize,
        offset: AtomicUsize,
    }

    impl LoomArenaModel {
        fn new(capacity: usize) -> Self {
            Self {
                capacity,
                offset: AtomicUsize::new(0),
            }
        }

        fn alloc_range(&self, size: usize, align: usize) -> Option<(usize, usize)> {
            let mut current = self.offset.load(Ordering::Relaxed);
            loop {
                let aligned = (current + align - 1) & !(align - 1);
                let next = aligned + size;

                if next > self.capacity {
                    return None;
                }

                match self.offset.compare_exchange_weak(
                    current,
                    next,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return Some((aligned, next)),
                    Err(actual) => current = actual,
                }
            }
        }
    }

    #[test]
    fn test_concurrent_arena_alloc_no_overlap() {
        loom::model(|| {
            let arena = Arc::new(LoomArenaModel::new(64));

            let a1 = Arc::clone(&arena);
            let h1 = thread::spawn(move || a1.alloc_range(16, 8));

            let a2 = Arc::clone(&arena);
            let h2 = thread::spawn(move || a2.alloc_range(16, 8));

            let r1 = h1.join().unwrap();
            let r2 = h2.join().unwrap();

            if let (Some((start1, end1)), Some((start2, end2))) = (r1, r2) {
                // İki tahsis asla çakışmamalıdır
                assert!(
                    end1 <= start2 || end2 <= start1,
                    "Arena bellek bolgeleri cakisti!"
                );
            }
        });
    }
}

#[cfg(not(loom))]
mod std_concurrency_tests {
    use rugeng_core::arena::FrameArena;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_concurrent_frame_arena_no_overlap() {
        let arena = Arc::new(FrameArena::new(1024 * 1024).unwrap());
        let thread_count = 8;
        let iters = 500;
        let mut handles = Vec::new();

        for t in 0..thread_count {
            let a = Arc::clone(&arena);
            handles.push(thread::spawn(move || {
                let mut allocations = Vec::new();
                for i in 0..iters {
                    let val = a
                        .alloc((t as usize, i as usize))
                        .expect("allocation failed");
                    allocations.push(val as *mut (usize, usize) as usize);
                }
                allocations
            }));
        }

        let mut all_addrs = Vec::new();
        for h in handles {
            let addrs = h.join().unwrap();
            all_addrs.extend(addrs);
        }

        // Verify all allocation addresses are strictly non-overlapping
        all_addrs.sort_unstable();
        for i in 1..all_addrs.len() {
            assert!(
                all_addrs[i] >= all_addrs[i - 1] + std::mem::size_of::<(usize, usize)>(),
                "Memory overlap detected between concurrent allocations!"
            );
        }
    }
}
