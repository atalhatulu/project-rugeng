//! Sparse-Set Veri Yapıları: Düz (Flat) ve Sayfalı (Paged) Tablolar.
//!
//! Ekleme, arama ve swap-and-pop ile silme O(1)'dir.
//! FlatSparseTable: Düz ardışık `Vec<u32>`. Basit, tek seviyeli, yüksek önbellek yerelliği.
//! PagedSparseTable: 1024 elemanlık 2 kademeli dinamik sayfalar. Seyrek id dağılımlarında bellek tasarrufu.

use crate::types::{Entity, ENTITY_INDEX_BITS};

pub const SPARSE_PAGE_SIZE: usize = 1024;
pub const SPARSE_PAGE_COUNT: usize = (1 << ENTITY_INDEX_BITS) / SPARSE_PAGE_SIZE; // 1024 sayfa
pub const SPARSE_NULL_INDEX: u32 = u32::MAX;

/// Sparse indeks tablosu arayüzü
pub trait SparseTable: Default + Send + Sync + 'static {
    fn get(&self, entity_index: u32) -> u32;
    fn set(&mut self, entity_index: u32, dense_index: u32);
    fn clear_entry(&mut self, entity_index: u32);
    fn allocated_bytes(&self) -> usize;
}

/// Düz Sparse Tablosu (Flat): `Vec<u32>`
#[derive(Default)]
pub struct FlatSparseTable {
    indices: Vec<u32>,
}

impl SparseTable for FlatSparseTable {
    #[inline(always)]
    fn get(&self, entity_index: u32) -> u32 {
        let idx = entity_index as usize;
        if idx < self.indices.len() {
            self.indices[idx]
        } else {
            SPARSE_NULL_INDEX
        }
    }

    #[inline(always)]
    fn set(&mut self, entity_index: u32, dense_index: u32) {
        let idx = entity_index as usize;
        if idx >= self.indices.len() {
            self.indices.resize(idx + 1, SPARSE_NULL_INDEX);
        }
        self.indices[idx] = dense_index;
    }

    #[inline(always)]
    fn clear_entry(&mut self, entity_index: u32) {
        let idx = entity_index as usize;
        if idx < self.indices.len() {
            self.indices[idx] = SPARSE_NULL_INDEX;
        }
    }

    #[inline]
    fn allocated_bytes(&self) -> usize {
        self.indices.capacity() * std::mem::size_of::<u32>()
    }
}

/// 2 Kademeli Sayfalı Sparse Tablosu (Paged)
pub struct PagedSparseTable {
    pages: Box<[Option<Box<[u32; SPARSE_PAGE_SIZE]>>]>,
}

impl Default for PagedSparseTable {
    fn default() -> Self {
        let mut pages_vec = Vec::with_capacity(SPARSE_PAGE_COUNT);
        for _ in 0..SPARSE_PAGE_COUNT {
            pages_vec.push(None);
        }
        Self {
            pages: pages_vec.into_boxed_slice(),
        }
    }
}

impl SparseTable for PagedSparseTable {
    #[inline(always)]
    fn get(&self, entity_index: u32) -> u32 {
        let page_idx = (entity_index as usize) / SPARSE_PAGE_SIZE;
        let offset = (entity_index as usize) % SPARSE_PAGE_SIZE;

        if let Some(page) = self.pages.get(page_idx).and_then(|p| p.as_ref()) {
            page[offset]
        } else {
            SPARSE_NULL_INDEX
        }
    }

    #[inline(always)]
    fn set(&mut self, entity_index: u32, dense_index: u32) {
        let page_idx = (entity_index as usize) / SPARSE_PAGE_SIZE;
        let offset = (entity_index as usize) % SPARSE_PAGE_SIZE;

        debug_assert!(page_idx < SPARSE_PAGE_COUNT, "Entity index sinir disi");

        if self.pages[page_idx].is_none() {
            let mut page = Box::new([SPARSE_NULL_INDEX; SPARSE_PAGE_SIZE]);
            page[offset] = dense_index;
            self.pages[page_idx] = Some(page);
        } else {
            // SAFETY: Sayfanın var olduğu doğrulanmıştır.
            unsafe {
                let page = self
                    .pages
                    .get_unchecked_mut(page_idx)
                    .as_mut()
                    .unwrap_unchecked();
                page[offset] = dense_index;
            }
        }
    }

    #[inline(always)]
    fn clear_entry(&mut self, entity_index: u32) {
        let page_idx = (entity_index as usize) / SPARSE_PAGE_SIZE;
        let offset = (entity_index as usize) % SPARSE_PAGE_SIZE;

        if let Some(page) = self.pages.get_mut(page_idx).and_then(|p| p.as_mut()) {
            page[offset] = SPARSE_NULL_INDEX;
        }
    }

    #[inline]
    fn allocated_bytes(&self) -> usize {
        let table_overhead =
            SPARSE_PAGE_COUNT * std::mem::size_of::<Option<Box<[u32; SPARSE_PAGE_SIZE]>>>();
        let mut pages_bytes = 0;
        for p in self.pages.iter() {
            if p.is_some() {
                pages_bytes += SPARSE_PAGE_SIZE * std::mem::size_of::<u32>();
            }
        }
        table_overhead + pages_bytes
    }
}

/// Jenerik SparseSet: Tablo tipine göre (Flat veya Paged) çalışabilir.
pub struct SparseSet<T, S: SparseTable = FlatSparseTable> {
    sparse: S,
    dense_entities: Vec<Entity>,
    dense_components: Vec<T>,
}

pub type FlatSparseSet<T> = SparseSet<T, FlatSparseTable>;
pub type PagedSparseSet<T> = SparseSet<T, PagedSparseTable>;

impl<T, S: SparseTable> Default for SparseSet<T, S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, S: SparseTable> SparseSet<T, S> {
    pub fn new() -> Self {
        Self {
            sparse: S::default(),
            dense_entities: Vec::new(),
            dense_components: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            sparse: S::default(),
            dense_entities: Vec::with_capacity(capacity),
            dense_components: Vec::with_capacity(capacity),
        }
    }

    #[inline(always)]
    pub fn contains(&self, entity: Entity) -> bool {
        let dense_idx = self.sparse.get(entity.index());
        if dense_idx != SPARSE_NULL_INDEX && (dense_idx as usize) < self.dense_entities.len() {
            self.dense_entities[dense_idx as usize] == entity
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn get_dense_index(&self, entity: Entity) -> Option<usize> {
        let dense_idx = self.sparse.get(entity.index());
        if dense_idx != SPARSE_NULL_INDEX
            && (dense_idx as usize) < self.dense_entities.len()
            && self.dense_entities[dense_idx as usize] == entity
        {
            return Some(dense_idx as usize);
        }
        None
    }

    #[inline]
    pub fn insert(&mut self, entity: Entity, component: T) -> &mut T {
        let index = entity.index();
        let dense_idx = self.sparse.get(index);

        if dense_idx != SPARSE_NULL_INDEX
            && (dense_idx as usize) < self.dense_entities.len()
            && self.dense_entities[dense_idx as usize].index() == index
        {
            let idx = dense_idx as usize;
            self.dense_entities[idx] = entity;
            self.dense_components[idx] = component;
            return &mut self.dense_components[idx];
        }

        let new_dense_idx = self.dense_entities.len() as u32;
        self.sparse.set(index, new_dense_idx);
        self.dense_entities.push(entity);
        self.dense_components.push(component);

        self.dense_components.last_mut().unwrap()
    }

    #[inline]
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let index = entity.index();
        let dense_idx = self.sparse.get(index);

        if dense_idx == SPARSE_NULL_INDEX
            || (dense_idx as usize) >= self.dense_entities.len()
            || self.dense_entities[dense_idx as usize] != entity
        {
            return None;
        }

        let dense_idx = dense_idx as usize;
        let last_idx = self.dense_entities.len() - 1;

        if dense_idx != last_idx {
            let last_entity = self.dense_entities[last_idx];
            self.dense_entities[dense_idx] = last_entity;
            self.dense_components.swap(dense_idx, last_idx);

            self.sparse.set(last_entity.index(), dense_idx as u32);
        }

        self.dense_entities.pop();
        let removed_component = self.dense_components.pop();
        self.sparse.clear_entry(index);

        removed_component
    }

    #[inline(always)]
    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.get_dense_index(entity)
            .map(|idx| &self.dense_components[idx])
    }

    #[inline(always)]
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.get_dense_index(entity)
            .map(move |idx| &mut self.dense_components[idx])
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.dense_entities.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.dense_entities.is_empty()
    }

    #[inline(always)]
    pub fn entities(&self) -> &[Entity] {
        &self.dense_entities
    }

    #[inline(always)]
    pub fn components(&self) -> &[T] {
        &self.dense_components
    }

    #[inline(always)]
    pub fn components_mut(&mut self) -> &mut [T] {
        &mut self.dense_components
    }

    #[inline(always)]
    pub fn split_entities_and_components_mut(&mut self) -> (&[Entity], &mut [T]) {
        (&self.dense_entities, &mut self.dense_components)
    }

    #[inline]
    pub fn sparse_allocated_bytes(&self) -> usize {
        self.sparse.allocated_bytes()
    }

    pub fn clear(&mut self) {
        for e in &self.dense_entities {
            self.sparse.clear_entry(e.index());
        }
        self.dense_entities.clear();
        self.dense_components.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_set_generation_reuse_no_zombies() {
        let mut set = FlatSparseSet::<String>::new();
        let e_gen0 = Entity::new(10, 0);
        let e_gen1 = Entity::new(10, 1);

        set.insert(e_gen0, "First Gen".to_string());
        assert_eq!(set.len(), 1);
        assert_eq!(set.entities().len(), 1);
        assert_eq!(set.get(e_gen0), Some(&"First Gen".to_string()));

        // Insert new generation for same slot without explicit removal
        set.insert(e_gen1, "Second Gen".to_string());
        assert_eq!(set.len(), 1, "Must not create a zombie entry!");
        assert_eq!(set.entities().len(), 1);
        assert_eq!(set.get(e_gen0), None, "Old generation must be invalidated");
        assert_eq!(set.get(e_gen1), Some(&"Second Gen".to_string()));
    }
}
