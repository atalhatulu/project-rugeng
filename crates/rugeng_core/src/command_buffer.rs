//! Komut Tamponu (CommandBuffer): Sıfır Tahsisli (Zero-Allocation) Yapısal Değişiklik Kuyruğu.
//!
//! Paralel sistemler veya oyun döngüsü çalışırken doğrudan Entity spawn/destroy veya bileşen
//! ekleme/çıkarma işlemleri yapılamaz (veri yarışması ve bellek geçersizliği).
//!
//! Bu modül, heap üzerinde `Box<dyn FnOnce>` tahsisi yapmaz.
//! Bellek ardışık bayt tamponunda (`Vec<u8>`) tutulur ve kareler arasında kapasiteyi koruyarak
//! gameplay döngüsünde 0 malloc garantisi verir.

use crate::registry::Registry;
use crate::sparse_set::{FlatSparseTable, SparseTable};
use crate::types::Entity;
use std::mem::{align_of, size_of};

type ApplyFn<S> = unsafe fn(*mut u8, &mut Registry<S>);
type DropFn = unsafe fn(*mut u8);

enum FastCommand<S: SparseTable> {
    Destroy(Entity),
    Custom {
        apply: ApplyFn<S>,
        drop: DropFn,
        offset: usize,
    },
}

#[repr(align(16))]
#[derive(Copy, Clone)]
#[allow(dead_code)]
struct Block16([u8; 16]);

struct AlignedBuffer {
    storage: Vec<Block16>,
    len: usize,
}

impl AlignedBuffer {
    fn with_capacity(capacity_bytes: usize) -> Self {
        let blocks = capacity_bytes.div_ceil(16);
        Self {
            storage: Vec::with_capacity(blocks),
            len: 0,
        }
    }

    #[inline(always)]
    fn as_mut_ptr(&mut self) -> *mut u8 {
        self.storage.as_mut_ptr() as *mut u8
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.len
    }

    fn reserve(&mut self, additional_bytes: usize) {
        let needed_bytes = self.len + additional_bytes;
        let needed_blocks = needed_bytes.div_ceil(16);
        if needed_blocks > self.storage.capacity() {
            self.storage.reserve(needed_blocks - self.storage.len());
        }
    }

    fn resize(&mut self, new_len: usize) {
        let needed_blocks = new_len.div_ceil(16);
        if needed_blocks > self.storage.len() {
            self.storage.resize(needed_blocks, Block16([0; 16]));
        }
        self.len = new_len;
    }

    fn clear(&mut self) {
        self.len = 0;
    }
}

pub struct CommandBuffer<S: SparseTable = FlatSparseTable> {
    buffer: AlignedBuffer,
    entries: Vec<FastCommand<S>>,
}

impl<S: SparseTable> Default for CommandBuffer<S> {
    fn default() -> Self {
        Self::with_sparse_table()
    }
}

impl<S: SparseTable> Drop for CommandBuffer<S> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl CommandBuffer<FlatSparseTable> {
    pub fn new() -> Self {
        Self {
            buffer: AlignedBuffer::with_capacity(16 * 1024), // 16 KB başlangıç tamponu
            entries: Vec::with_capacity(512),
        }
    }
}

impl<S: SparseTable> CommandBuffer<S> {
    pub fn with_sparse_table() -> Self {
        Self {
            buffer: AlignedBuffer::with_capacity(16 * 1024),
            entries: Vec::with_capacity(512),
        }
    }

    /// Entity silme komutu kaydeder (0 bayt veri tamponu, anında kuyruk)
    #[inline]
    pub fn destroy(&mut self, entity: Entity) {
        self.entries.push(FastCommand::Destroy(entity));
    }

    /// Entity'ye bileşen ekleme komutu kaydeder.
    /// Veriyi doğrudan ardışık 16-bayt hizalı tampona kopyalar; Box/malloc yapmaz.
    pub fn emplace<T: 'static + Send + Sync>(&mut self, entity: Entity, component: T) {
        type Payload<T> = (Entity, T);

        let align = align_of::<Payload<T>>().max(1);
        assert!(
            align <= 16,
            "Component alignment exceeds maximum buffer alignment of 16"
        );
        let size = size_of::<Payload<T>>();
        let max_needed = align + size;
        self.buffer.reserve(max_needed);

        let current_len = self.buffer.len();
        let misalign = current_len % align;
        let padding = if misalign == 0 { 0 } else { align - misalign };
        let offset = current_len + padding;

        self.buffer.resize(offset + size);

        unsafe {
            let ptr = self.buffer.as_mut_ptr().add(offset) as *mut Payload<T>;
            debug_assert_eq!(ptr as usize % align, 0, "Pointer must be aligned");
            std::ptr::write(ptr, (entity, component));
        }

        unsafe fn apply_emplace<S: SparseTable, T: 'static + Send + Sync>(
            ptr: *mut u8,
            reg: &mut Registry<S>,
        ) {
            // SAFETY: ptr geçerli bir (Entity, T) içerir ve sahipliği reg'e aktarılır.
            let (entity, component) = std::ptr::read(ptr as *mut Payload<T>);
            if reg.is_alive(entity) {
                reg.emplace(entity, component);
            } else {
                drop(component);
            }
        }

        unsafe fn drop_emplace<T>(ptr: *mut u8) {
            // SAFETY: Komut uygulanmadan CommandBuffer düşürülürse çağrılır.
            std::ptr::drop_in_place(ptr as *mut Payload<T>);
        }

        self.entries.push(FastCommand::Custom {
            apply: apply_emplace::<S, T>,
            drop: drop_emplace::<T>,
            offset,
        });
    }

    /// Özel bir işlev/fonksiyon çalıştırma komutu (yine ardışık tamponda tutulur, Box yapmaz)
    pub fn defer<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Registry<S>) + Send + Sync + 'static,
    {
        let align = align_of::<F>().max(1);
        assert!(
            align <= 16,
            "Closure alignment exceeds maximum buffer alignment of 16"
        );
        let size = size_of::<F>();
        let max_needed = align + size;
        self.buffer.reserve(max_needed);

        let current_len = self.buffer.len();
        let misalign = current_len % align;
        let padding = if misalign == 0 { 0 } else { align - misalign };
        let offset = current_len + padding;

        self.buffer.resize(offset + size);

        unsafe {
            let ptr = self.buffer.as_mut_ptr().add(offset) as *mut F;
            debug_assert_eq!(ptr as usize % align, 0, "Pointer must be aligned");
            std::ptr::write(ptr, f);
        }

        unsafe fn apply_fn<S: SparseTable, F: FnOnce(&mut Registry<S>) + 'static>(
            ptr: *mut u8,
            reg: &mut Registry<S>,
        ) {
            let func = std::ptr::read(ptr as *mut F);
            func(reg);
        }

        unsafe fn drop_fn<F>(ptr: *mut u8) {
            std::ptr::drop_in_place(ptr as *mut F);
        }

        self.entries.push(FastCommand::Custom {
            apply: apply_fn::<S, F>,
            drop: drop_fn::<F>,
            offset,
        });
    }

    /// Tampondaki tüm yapısal değişiklikleri Registry'ye münhasır (&mut) erişimle uygular.
    /// Uygulama sonrası tampon kapasitesi sıfırlanmaz, sonraki karede 0 malloc için korunur.
    /// Panik/unwind durumunda uygulanmamış komutlar temizlenir ve çift-drop önlenir.
    pub fn apply(&mut self, registry: &mut Registry<S>) {
        struct ApplyGuard<'a, S: SparseTable> {
            buffer: &'a mut AlignedBuffer,
            entries: &'a mut Vec<FastCommand<S>>,
            idx: usize,
        }
        impl<'a, S: SparseTable> Drop for ApplyGuard<'a, S> {
            fn drop(&mut self) {
                if self.idx < self.entries.len() {
                    for entry in self.entries.drain(self.idx..) {
                        if let FastCommand::Custom { drop, offset, .. } = entry {
                            unsafe {
                                let ptr = self.buffer.as_mut_ptr().add(offset);
                                drop(ptr);
                            }
                        }
                    }
                }
                self.entries.clear();
                self.buffer.clear();
            }
        }

        let mut guard = ApplyGuard {
            buffer: &mut self.buffer,
            entries: &mut self.entries,
            idx: 0,
        };

        while guard.idx < guard.entries.len() {
            let i = guard.idx;
            guard.idx += 1;
            match guard.entries[i] {
                FastCommand::Destroy(entity) => {
                    registry.destroy(entity);
                }
                FastCommand::Custom { apply, offset, .. } => unsafe {
                    let ptr = guard.buffer.as_mut_ptr().add(offset);
                    apply(ptr, registry);
                },
            }
        }
    }

    /// Tampondaki tüm komutları uygulamadan temizler ve içindeki verileri güvenle drop eder.
    pub fn clear(&mut self) {
        for entry in self.entries.drain(..) {
            if let FastCommand::Custom { drop, offset, .. } = entry {
                unsafe {
                    let ptr = self.buffer.as_mut_ptr().add(offset);
                    drop(ptr);
                }
            }
        }
        self.buffer.clear();
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Score(u32);

    #[derive(Debug)]
    struct DropDetector(std::sync::Arc<std::sync::atomic::AtomicBool>);

    impl Drop for DropDetector {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[test]
    fn test_command_buffer_zero_alloc_apply() {
        let mut reg = Registry::new();
        let e1 = reg.create();
        let e2 = reg.create();

        let mut cmd = CommandBuffer::new();
        cmd.emplace(e1, Score(100));
        cmd.destroy(e2);

        assert!(reg.is_alive(e2));

        cmd.apply(&mut reg);

        assert!(!reg.is_alive(e2));
        unsafe {
            let score_pool = reg.storage::<Score>();
            assert_eq!(score_pool.get(e1), Some(&Score(100)));
        }

        // İkinci karede tekrar kullanım kapasiteyi korur
        let e3 = reg.create();
        cmd.emplace(e3, Score(250));
        cmd.apply(&mut reg);

        unsafe {
            let score_pool = reg.storage::<Score>();
            assert_eq!(score_pool.get(e3), Some(&Score(250)));
        }
    }

    #[test]
    fn test_command_buffer_drop_safety() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let dropped = Arc::new(AtomicBool::new(false));
        {
            let mut cmd = CommandBuffer::new();
            let mut reg = Registry::new();
            let e = reg.create();
            cmd.emplace(e, DropDetector(dropped.clone()));
            // cmd burada apply çağrılmadan drop oluyor!
        }
        assert!(
            dropped.load(Ordering::SeqCst),
            "Unapplied command must drop its component safely!"
        );
    }

    #[test]
    fn test_command_buffer_dead_entity_emplace_safety() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let dropped = Arc::new(AtomicBool::new(false));
        let mut reg = Registry::new();
        let e = reg.create();
        let mut cmd = CommandBuffer::new();

        cmd.emplace(e, DropDetector(dropped.clone()));
        // Destroy the entity before apply!
        reg.destroy(e);

        // Apply should safely drop the component instead of panicking on dead entity assert
        cmd.apply(&mut reg);

        assert!(
            dropped.load(Ordering::SeqCst),
            "Component on dead entity must be safely dropped!"
        );
    }
}
