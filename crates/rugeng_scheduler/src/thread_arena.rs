//! Worker Başına Thread-Local FrameArena Havuzu (Contention Sıfırlama).
//!
//! Ortak atomik `FrameArena` üzerinde çok sayıda thread eşzamanlı CAS yaptığında
//! önbellek hattı sıçraması (cache-line bouncing) ve çekişme (contention) oluşabilir.
//! Bu yapı, her worker için bağımsız bir 16MB/32MB `FrameArena` tutarak çekişmeyi tamamen 0'a indirir.

use rugeng_core::FrameArena;

pub struct ThreadArenaPool {
    arenas: Vec<FrameArena>,
}

impl ThreadArenaPool {
    /// Her worker için belirtilen kapasitede bağımsız bir arena tahsis eder.
    pub fn new(worker_count: usize, capacity_per_worker: usize) -> Result<Self, &'static str> {
        let mut arenas = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            arenas.push(FrameArena::new(capacity_per_worker)?);
        }
        Ok(Self { arenas })
    }

    /// Belirtilen worker indeksine ait arenayı döner.
    #[inline(always)]
    pub fn get(&self, worker_index: usize) -> &FrameArena {
        &self.arenas[worker_index % self.arenas.len()]
    }

    /// Kare sonunda tüm worker arena'larını O(1) maliyetle sıfırlar.
    pub fn reset_all(&mut self) {
        for arena in &mut self.arenas {
            arena.reset();
        }
    }

    pub fn total_allocated_bytes(&self) -> usize {
        self.arenas.iter().map(|a| a.allocated_bytes()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_arena_pool_independent_allocs() {
        let mut pool = ThreadArenaPool::new(4, 1024 * 1024).expect("Pool olusturulamadi");

        // Her worker bağımsız tahsis yapar
        for i in 0..4 {
            let arena = pool.get(i);
            let ptr = arena.alloc_slice_copy(&[i as u32; 100]).unwrap();
            assert_eq!(ptr[0], i as u32);
        }

        assert!(pool.total_allocated_bytes() > 0);

        pool.reset_all();
        assert_eq!(pool.total_allocated_bytes(), 0);
    }
}
