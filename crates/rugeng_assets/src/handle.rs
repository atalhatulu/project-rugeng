//! Asset Handle Sistemi ve Referans Sayımı (Asset Handle System).
//!
//! Benzersiz kimlik (ID), referans sayımı (Arc), yükleme durumu (LoadingState)
//! ve tip güvenli handle sarmalayıcısı sağlar.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

static NEXT_HANDLE_ID: AtomicU64 = AtomicU64::new(1);

/// Varlık yükleme durumu
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadingState {
    Unloaded,
    Loading,
    Loaded,
    Failed,
}

/// Tip güvenli varlık tanıtıcısı (Asset Handle).
///
/// İçerisinde `Arc` tabanlı dahili sayaç bulundurur. Handle kopyalandıkça
/// referans sayısı artar; tüm handle'lar bırakıldığında referans sayısı 0'a iner.
pub struct Handle<T> {
    id: u64,
    ref_count: Arc<AtomicU32>,
    _marker: PhantomData<T>,
}

impl<T> Handle<T> {
    pub fn new() -> Self {
        let id = NEXT_HANDLE_ID.fetch_add(1, Ordering::Relaxed);
        Self {
            id,
            ref_count: Arc::new(AtomicU32::new(1)),
            _marker: PhantomData,
        }
    }

    pub fn from_id(id: u64) -> Self {
        Self {
            id,
            ref_count: Arc::new(AtomicU32::new(1)),
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Aktif referans sayısını döndürür.
    #[inline]
    pub fn ref_count(&self) -> u32 {
        self.ref_count.load(Ordering::Acquire)
    }

    /// Yeni bir zayıf kopya yerine geçerli bir handle üretir ve sayacı artırır.
    pub fn clone_handle(&self) -> Self {
        self.ref_count.fetch_add(1, Ordering::Release);
        Self {
            id: self.id,
            ref_count: Arc::clone(&self.ref_count),
            _marker: PhantomData,
        }
    }
}

impl<T> Default for Handle<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        self.clone_handle()
    }
}

impl<T> Drop for Handle<T> {
    fn drop(&mut self) {
        self.ref_count.fetch_sub(1, Ordering::Release);
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Handle<T> {}

impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handle")
            .field("id", &self.id)
            .field("ref_count", &self.ref_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyAsset;

    #[test]
    fn test_handle_ref_counting() {
        let handle: Handle<DummyAsset> = Handle::new();
        assert_eq!(handle.ref_count(), 1);

        let handle2 = handle.clone();
        assert_eq!(handle.ref_count(), 2);
        assert_eq!(handle2.ref_count(), 2);
        assert_eq!(handle, handle2);

        {
            let _handle3 = handle2.clone();
            assert_eq!(handle.ref_count(), 3);
        }

        assert_eq!(handle.ref_count(), 2);
    }
}
