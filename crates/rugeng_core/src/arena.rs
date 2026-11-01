//! Frame Arena: 64MB sanal bellek tabanlı O(1) lock-free bump allocator.
//!
//! Kare sonunda `reset(&mut self)` ile offset sıfırlanır.
//! `alloc(&self)` atomik CAS ile çalıştığı için iş parçacıkları (job workers)
//! aynı arenadan kilit olmadan (lock-free) eşzamanlı bellek tahsis edebilir.

use std::alloc::Layout;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const DEFAULT_ARENA_CAPACITY: usize = 64 * 1024 * 1024; // 64 MB

pub struct FrameArena {
    memory: NonNull<u8>,
    capacity: usize,
    offset: AtomicUsize,
}

// Atomik CAS tabanlı tahsis sayesinde FrameArena hem Send hem Sync'tir.
unsafe impl Send for FrameArena {}
unsafe impl Sync for FrameArena {}

impl FrameArena {
    /// Belirtilen kapasitede (varsayılan 64MB) anonim sanal bellek ayırır.
    pub fn new(capacity: usize) -> Result<Self, &'static str> {
        let actual_cap = if capacity == 0 {
            DEFAULT_ARENA_CAPACITY
        } else {
            capacity
        };

        #[cfg(unix)]
        let ptr = {
            // SAFETY: MAP_PRIVATE | MAP_ANONYMOUS ile sıfırlanmış sanal sayfa tahsisi yapılır.
            let raw = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    actual_cap,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                    -1,
                    0,
                )
            };
            if raw == libc::MAP_FAILED {
                return Err("mmap ile bellek tahsisi basarisiz oldu");
            }
            raw as *mut u8
        };

        #[cfg(windows)]
        let ptr = {
            extern "system" {
                fn VirtualAlloc(
                    lpAddress: *mut libc::c_void,
                    dwSize: usize,
                    flAllocationType: u32,
                    flProtect: u32,
                ) -> *mut libc::c_void;
            }
            const MEM_COMMIT: u32 = 0x00001000;
            const MEM_RESERVE: u32 = 0x00002000;
            const PAGE_READWRITE: u32 = 0x04;

            // SAFETY: Windows VirtualAlloc çağrısı
            let raw = unsafe {
                VirtualAlloc(
                    std::ptr::null_mut(),
                    actual_cap,
                    MEM_COMMIT | MEM_RESERVE,
                    PAGE_READWRITE,
                )
            };
            if raw.is_null() {
                return Err("VirtualAlloc ile bellek tahsisi basarisiz oldu");
            }
            raw as *mut u8
        };

        let non_null = NonNull::new(ptr).ok_or("Bellek adresi gecersiz")?;
        Ok(Self {
            memory: non_null,
            capacity: actual_cap,
            offset: AtomicUsize::new(0),
        })
    }

    /// Ham bayt tahsisi (lock-free atomik CAS)
    #[inline]
    pub fn alloc_raw(&self, layout: Layout) -> Option<NonNull<u8>> {
        let align = layout.align();
        let size = layout.size();

        let mut current = self.offset.load(Ordering::Relaxed);
        loop {
            let current_ptr = unsafe { self.memory.as_ptr().add(current) };
            let addr = current_ptr as usize;
            let aligned_addr = (addr + align - 1) & !(align - 1);
            let padding = aligned_addr - addr;
            let next = current + padding + size;

            if next > self.capacity {
                return None; // Arena bitti
            }

            match self.offset.compare_exchange_weak(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    // SAFETY: target_ptr, mmap ile ayrılmış aralıkta ve hizalama kuralına uygundur.
                    let target_ptr = unsafe { self.memory.as_ptr().add(current + padding) };
                    return Some(unsafe { NonNull::new_unchecked(target_ptr) });
                }
                Err(actual) => {
                    current = actual;
                }
            }
        }
    }

    /// Arena üzerinde nesne tahsis eder.
    ///
    /// **Kısıt:** Arena Drop çalıştırmaz; `Drop` implement eden tipler sessiz
    /// bellek sızıntısına yol açar (debug build'te assert ile yakalanır).
    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn alloc<T>(&self, val: T) -> Option<&mut T> {
        debug_assert!(
            !std::mem::needs_drop::<T>(),
            "FrameArena Drop tipini desteklemez: {} sessizce sızmazdı; Drop'ı olmayan (Copy) tip tahsis edin",
            std::any::type_name::<T>()
        );
        let layout = Layout::new::<T>();
        let ptr = self.alloc_raw(layout)?;

        // SAFETY: Tahsis edilen işaretçi T için geçerli hizalama ve boyuta sahiptir.
        unsafe {
            let typed_ptr = ptr.as_ptr() as *mut T;
            typed_ptr.write(val);
            Some(&mut *typed_ptr)
        }
    }

    /// Copy tiplerinden oluşan dilim (slice) kopyalar.
    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_slice_copy<T: Copy>(&self, src: &[T]) -> Option<&mut [T]> {
        if src.is_empty() {
            return Some(&mut []);
        }
        let layout = Layout::array::<T>(src.len()).ok()?;
        let ptr = self.alloc_raw(layout)?;

        // SAFETY: Kaynak ile hedef bellek çakışmaz ve aynı boyuttadır.
        unsafe {
            let typed_ptr = ptr.as_ptr() as *mut T;
            std::ptr::copy_nonoverlapping(src.as_ptr(), typed_ptr, src.len());
            Some(std::slice::from_raw_parts_mut(typed_ptr, src.len()))
        }
    }

    /// Kare sonunda tüm geçici tahsisleri O(1) maliyetle sıfırlar.
    ///
    /// Not: `&mut self` gerektirdiği için derleyici zamanında tüm
    /// borçlanmaların (ve iş parçacıklarının) sonlanmış olması garanti edilir.
    #[inline]
    pub fn reset(&mut self) {
        self.offset.store(0, Ordering::Release);
    }

    #[inline]
    pub fn get_marker(&self) -> usize {
        self.offset.load(Ordering::Acquire)
    }

    /// # Safety
    /// Marker geçerli bir geçmiş offset olmalıdır ve sonrasında yapılan tahsisler kullanılmamalıdır.
    #[inline]
    pub unsafe fn reset_to_marker(&self, marker: usize) {
        debug_assert!(
            marker <= self.offset.load(Ordering::Relaxed),
            "Gecersiz marker"
        );
        self.offset.store(marker, Ordering::Release);
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn allocated_bytes(&self) -> usize {
        self.offset.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn remaining_bytes(&self) -> usize {
        self.capacity
            .saturating_sub(self.offset.load(Ordering::Relaxed))
    }
}

impl Drop for FrameArena {
    fn drop(&mut self) {
        #[cfg(unix)]
        // SAFETY: mmap ile ayrılmış bellek munmap ile serbest bırakılır.
        unsafe {
            libc::munmap(self.memory.as_ptr() as *mut libc::c_void, self.capacity);
        }

        #[cfg(windows)]
        // SAFETY: VirtualFree çağrısı
        unsafe {
            extern "system" {
                fn VirtualFree(
                    lpAddress: *mut libc::c_void,
                    dwSize: usize,
                    dwFreeType: u32,
                ) -> libc::c_int;
            }
            const MEM_RELEASE: u32 = 0x00008000;
            VirtualFree(self.memory.as_ptr() as *mut libc::c_void, 0, MEM_RELEASE);
        }
    }
}

pub struct ScopedScratch<'a> {
    arena: &'a mut FrameArena,
    marker: usize,
}

impl<'a> ScopedScratch<'a> {
    #[inline]
    pub fn new(arena: &'a mut FrameArena) -> Self {
        let marker = arena.get_marker();
        Self { arena, marker }
    }

    #[inline]
    pub fn arena(&self) -> &FrameArena {
        self.arena
    }

    #[inline]
    pub fn alloc<T>(&self, val: T) -> Option<&mut T> {
        self.arena.alloc(val)
    }

    #[inline]
    pub fn alloc_slice_copy<T: Copy>(&self, src: &[T]) -> Option<&mut [T]> {
        self.arena.alloc_slice_copy(src)
    }
}

impl<'a> Drop for ScopedScratch<'a> {
    fn drop(&mut self) {
        // SAFETY: ScopedScratch sonlandığında marker'a dönülür.
        unsafe {
            self.arena.reset_to_marker(self.marker);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concurrent_allocations() {
        use std::sync::Arc;
        let arena = Arc::new(FrameArena::new(10 * 1024 * 1024).unwrap());
        let mut handles = vec![];

        for t in 0..4 {
            let a = Arc::clone(&arena);
            handles.push(std::thread::spawn(move || {
                for i in 0..1000 {
                    let val = a.alloc((t as usize, i as usize)).unwrap();
                    assert_eq!(*val, (t as usize, i as usize));
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert!(arena.allocated_bytes() >= 4000 * std::mem::size_of::<(usize, usize)>());
    }

    #[test]
    fn test_scoped_scratch_marker_reset() {
        let mut arena = FrameArena::new(1024 * 1024).unwrap();
        let initial_offset = arena.get_marker();
        assert_eq!(initial_offset, 0);

        {
            let scratch = ScopedScratch::new(&mut arena);
            let val1 = scratch.alloc(12345u64).unwrap();
            let val2 = scratch.alloc(67890u64).unwrap();
            assert_eq!(*val1, 12345u64);
            assert_eq!(*val2, 67890u64);
            assert_eq!(*val1 + *val2, 80235u64);
            assert!(scratch.arena().allocated_bytes() > 0);
        }

        // After ScopedScratch is dropped, marker must be reset to initial_offset
        assert_eq!(arena.get_marker(), initial_offset);
    }
}
