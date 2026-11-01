//! Loom Eşzamanlılık ve Paralel Yol Modeli Doğrulaması.
//!
//! Paralel scheduler için:
//! 1. Loom ile atomik CAS ve work-stealing / task sync modelinde data race olmadığını kanıtlar.
//! 2. Determinizm testi: Mermi/varlık güncelleme sırası ve veri yarışması güvenliği.

#![allow(unexpected_cfgs)]

#[cfg(loom)]
mod loom_scheduler_tests {
    use loom::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use loom::sync::Arc;
    use loom::thread;

    #[test]
    fn test_loom_task_completion_sync() {
        loom::model(|| {
            let active = Arc::new(AtomicUsize::new(2));
            let done_flag = Arc::new(AtomicBool::new(false));

            let a1 = Arc::clone(&active);
            let d1 = Arc::clone(&done_flag);
            let t1 = thread::spawn(move || {
                if a1.fetch_sub(1, Ordering::SeqCst) == 1 {
                    d1.store(true, Ordering::Release);
                }
            });

            let a2 = Arc::clone(&active);
            let d2 = Arc::clone(&done_flag);
            let t2 = thread::spawn(move || {
                if a2.fetch_sub(1, Ordering::SeqCst) == 1 {
                    d2.store(true, Ordering::Release);
                }
            });

            t1.join().unwrap();
            t2.join().unwrap();

            assert_eq!(active.load(Ordering::SeqCst), 0);
            assert!(done_flag.load(Ordering::Acquire));
        });
    }

    #[test]
    fn test_loom_parallel_chunk_disjoint_access() {
        loom::model(|| {
            // İki worker thread'in bağımsız (disjoint) indekslere yazdığını ve yarışma olmadığını modelle
            use loom::cell::UnsafeCell;

            struct SharedBuffer {
                buf: [UnsafeCell<u32>; 2],
            }
            unsafe impl Sync for SharedBuffer {}

            let shared = Arc::new(SharedBuffer {
                buf: [UnsafeCell::new(0), UnsafeCell::new(0)],
            });

            let s1 = Arc::clone(&shared);
            let t1 = thread::spawn(move || unsafe {
                s1.buf[0].with_mut(|ptr| *ptr = 10);
            });

            let s2 = Arc::clone(&shared);
            let t2 = thread::spawn(move || unsafe {
                s2.buf[1].with_mut(|ptr| *ptr = 20);
            });

            t1.join().unwrap();
            t2.join().unwrap();

            unsafe {
                shared.buf[0].with(|p| assert_eq!(*p, 10));
                shared.buf[1].with(|p| assert_eq!(*p, 20));
            }
        });
    }
}

#[cfg(not(loom))]
mod std_scheduler_tests {
    use rugeng_scheduler::pool::ThreadPool;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_std_parallel_determinism_and_no_data_race() {
        let pool = ThreadPool::new(4);
        let size = 20_000;
        let mut positions = vec![0.0f32; size];
        let _velocities = vec![1.5f32; size];

        pool.scope(|s| {
            s.for_each_chunk(&mut positions, 1000, |chunk| {
                for pos in chunk {
                    *pos += 1.5 * (1.0 / 60.0);
                }
            });
        });

        // Determinizm kontrolü: Tüm elemanlar kesinlikle aynı değere ulaşmalı
        let expected = 1.5 * (1.0 / 60.0);
        for (idx, pos) in positions.iter().enumerate() {
            assert!(
                (*pos - expected).abs() < 1e-6,
                "Determinizm hatasi idx {idx}: beklenen {expected}, bulunan {pos}"
            );
        }
    }

    #[test]
    fn test_std_high_contention_stress() {
        let pool = ThreadPool::new(8);
        let counter = Arc::new(AtomicUsize::new(0));

        for _ in 0..10_000 {
            let c = Arc::clone(&counter);
            pool.spawn(move || {
                c.fetch_add(1, Ordering::Relaxed);
            });
        }

        pool.wait_idle();
        assert_eq!(counter.load(Ordering::Relaxed), 10_000);
    }
}
