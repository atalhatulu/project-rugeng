//! ECS Registry & Scheduler Access Model:
//!
//! Bileşen havuzları `UnsafeCell` ile tutulur.
//! Sistemler çalıştırmadan önce `AccessSet` bildirir (Read/Write set).
//! Scheduler, çakışan sistemleri sıralı, bağımsız sistemleri ise paralel iş parçacıklarına dağıtır.
//! Sıcak döngü içinde `RefCell` veya dinamik `dyn` kontrolü yoktur; saf monomorfik referanslarla çalışır.

use std::any::{Any, TypeId};
use std::cell::UnsafeCell;
use std::collections::{HashMap, HashSet};

use crate::sparse_set::{SparseSet, SparseTable};
use crate::types::{Entity, ENTITY_GEN_MASK, MAX_ENTITY_INDEX};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessType {
    Read,
    Write,
}

#[derive(Debug, Default, Clone)]
pub struct AccessSet {
    reads: HashSet<TypeId>,
    writes: HashSet<TypeId>,
}

impl AccessSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_read<T: 'static>(&mut self) {
        let type_id = TypeId::of::<T>();
        assert!(
            !self.writes.contains(&type_id),
            "Ayni sistem icinde ayni bilesene hem Read hem Write erisimi talep edilemez"
        );
        self.reads.insert(type_id);
    }

    pub fn add_write<T: 'static>(&mut self) {
        let type_id = TypeId::of::<T>();
        assert!(
            !self.writes.contains(&type_id),
            "Ayni sistem icinde ayni bilesene cift Write erisimi talep edilemez"
        );
        assert!(
            !self.reads.contains(&type_id),
            "Ayni sistem icinde ayni bilesene hem Read hem Write erisimi talep edilemez"
        );
        self.writes.insert(type_id);
    }

    /// İki sistemin aynı anda paralel çalışıp çalışamayacağını denetler.
    /// Eğer iki sistemden en az biri aynı TypeId için Write istiyorsa çakışma (data race) vardır.
    pub fn conflicts_with(&self, other: &AccessSet) -> bool {
        // Benim yazdığım şeye karşı taraf okuyor veya yazıyor mu?
        for write in &self.writes {
            if other.reads.contains(write) || other.writes.contains(write) {
                return true;
            }
        }
        // Karşı tarafın yazdığı şeyi ben okuyor muyum?
        for other_write in &other.writes {
            if self.reads.contains(other_write) {
                return true;
            }
        }
        false
    }
}

pub trait AnyStorage: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn remove_entity(&self, entity: Entity) -> bool;
    fn clear_all(&self);
}

pub struct StorageCell<T: 'static, S: SparseTable> {
    cell: UnsafeCell<SparseSet<T, S>>,
}

// Scheduler erişim garantisi verdiğinde StorageCell multi-thread güvenlidir.
//
// GÜVENLİK KURALI (denetim R4): Aşağıdaki `storage()/storage_mut()` kaçış
// kapıları YALNIZ Scheduler tarafından zamanlanmış sistemlerden çağrılabilir
// (AccessSet çakışma denetimi sayesinde). Scheduler dışında, testte veya ham
// thread'lerden doğrudan çağırmak veri yarışına (UB) yol açar.
unsafe impl<T: Send + 'static, S: SparseTable> Send for StorageCell<T, S> {}
unsafe impl<T: Sync + 'static, S: SparseTable> Sync for StorageCell<T, S> {}

impl<T: 'static + Send + Sync, S: SparseTable> Default for StorageCell<T, S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static + Send + Sync, S: SparseTable> StorageCell<T, S> {
    pub fn new() -> Self {
        Self {
            cell: UnsafeCell::new(SparseSet::new()),
        }
    }

    /// # Safety
    /// Scheduler, bu türe eşzamanlı hiçbir `Write` erişimi olmadığını garanti etmelidir.
    #[inline(always)]
    pub unsafe fn get(&self) -> &SparseSet<T, S> {
        &*self.cell.get()
    }

    /// # Safety
    /// Scheduler, bu türe münhasır (exclusive) erişim olduğunu ve başka hiçbir
    /// iş parçacığının bu türe erişmediğini garanti etmelidir.
    #[inline(always)]
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_mut(&self) -> &mut SparseSet<T, S> {
        &mut *self.cell.get()
    }
}

impl<T: 'static + Send + Sync, S: SparseTable> AnyStorage for StorageCell<T, S> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn remove_entity(&self, entity: Entity) -> bool {
        // SAFETY: Entity silme sırasında Registry mutably borçlanmalıdır.
        unsafe { (*self.cell.get()).remove(entity).is_some() }
    }

    fn clear_all(&self) {
        unsafe { (*self.cell.get()).clear() }
    }
}

pub struct Registry<S: SparseTable = crate::sparse_set::FlatSparseTable> {
    generations: Vec<u32>,
    free_list: Vec<u32>,
    pools: HashMap<TypeId, Box<dyn AnyStorage>>,
    _marker: std::marker::PhantomData<S>,
}

unsafe impl<S: SparseTable> Send for Registry<S> {}
unsafe impl<S: SparseTable> Sync for Registry<S> {}

impl<S: SparseTable> Default for Registry<S> {
    fn default() -> Self {
        Self::with_sparse_table()
    }
}

impl Registry<crate::sparse_set::FlatSparseTable> {
    pub fn new() -> Self {
        Self::with_sparse_table()
    }
}

impl<S: SparseTable> Registry<S> {
    pub fn with_sparse_table() -> Self {
        Self {
            generations: Vec::with_capacity(1024),
            free_list: Vec::with_capacity(256),
            pools: HashMap::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn create(&mut self) -> Entity {
        let (index, generation) = if let Some(free_idx) = self.free_list.pop() {
            let gen = self.generations[free_idx as usize];
            (free_idx, gen)
        } else {
            let idx = self.generations.len() as u32;
            assert!(idx <= MAX_ENTITY_INDEX, "Maksimum Entity kapasitesi asildi");
            self.generations.push(0);
            (idx, 0)
        };

        Entity::new(index, generation)
    }

    #[inline(always)]
    pub fn is_alive(&self, entity: Entity) -> bool {
        let idx = entity.index() as usize;
        if idx < self.generations.len() {
            self.generations[idx] == entity.generation()
        } else {
            false
        }
    }

    pub fn destroy(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }

        let idx = entity.index() as usize;

        for pool in self.pools.values() {
            pool.remove_entity(entity);
        }

        self.generations[idx] = (self.generations[idx] + 1) & ENTITY_GEN_MASK;
        self.free_list.push(idx as u32);

        true
    }

    /// Tüm bileşen havuzlarını ve canlı entity'leri temizler.
    pub fn clear(&mut self) {
        for pool in self.pools.values() {
            pool.clear_all();
        }
        for gen in &mut self.generations {
            *gen = (*gen + 1) & ENTITY_GEN_MASK;
        }
        self.free_list.clear();
        for idx in 0..self.generations.len() {
            self.free_list.push(idx as u32);
        }
    }

    pub fn register_component<T: 'static + Send + Sync>(&mut self) {
        let type_id = TypeId::of::<T>();
        self.pools.entry(type_id).or_insert_with(|| {
            let cell: Box<StorageCell<T, S>> = Box::default();
            cell
        });
    }

    /// Doğrudan havuz hücresine erişim.
    #[inline]
    fn get_cell<T: 'static + Send + Sync>(&self) -> &StorageCell<T, S> {
        let type_id = TypeId::of::<T>();
        let storage = self
            .pools
            .get(&type_id)
            .expect("Bilesen havuzu kayitli degil");
        storage
            .as_any()
            .downcast_ref::<StorageCell<T, S>>()
            .unwrap()
    }

    /// # Safety
    /// Bu bileşen türü üzerinde eşzamanlı `Write` borçlanması olmamalıdır.
    #[inline(always)]
    pub unsafe fn storage<T: 'static + Send + Sync>(&self) -> &SparseSet<T, S> {
        self.get_cell::<T>().get()
    }

    /// # Safety
    /// Bu bileşen türü üzerinde başka hiçbir eşzamanlı `Read` veya `Write` borçlanması olmamalıdır.
    #[inline(always)]
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn storage_mut<T: 'static + Send + Sync>(&self) -> &mut SparseSet<T, S> {
        self.get_cell::<T>().get_mut()
    }

    /// Entity'ye bileşen ekler
    #[inline]
    pub fn emplace<T: 'static + Send + Sync>(&mut self, entity: Entity, component: T) {
        assert!(
            self.is_alive(entity),
            "Gecersiz entity uzerine bilesen eklenemez"
        );
        let type_id = TypeId::of::<T>();
        self.pools.entry(type_id).or_insert_with(|| {
            let cell: Box<StorageCell<T, S>> = Box::default();
            cell
        });
        let cell = self.get_cell::<T>();
        // SAFETY: `&mut self` olduğundan emplace esnasında eşzamanlı erişim yoktur.
        unsafe {
            cell.get_mut().insert(entity, component);
        }
    }

    /// Entity canlıysa bileşen ekler ve true döner; ölü veya geçersizse paniklemeden false döner.
    #[inline]
    pub fn emplace_checked<T: 'static + Send + Sync>(
        &mut self,
        entity: Entity,
        component: T,
    ) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        let type_id = TypeId::of::<T>();
        self.pools.entry(type_id).or_insert_with(|| {
            let cell: Box<StorageCell<T, S>> = Box::default();
            cell
        });
        let cell = self.get_cell::<T>();
        // SAFETY: `&mut self` olduğundan emplace esnasında eşzamanlı erişim yoktur.
        unsafe {
            cell.get_mut().insert(entity, component);
        }
        true
    }

    /// Güvenli bileşen okuma. `&self` borçlanması ile bileşene erişir.
    #[inline]
    pub fn get<T: 'static + Send + Sync>(&self, entity: Entity) -> Option<&T> {
        if !self.is_alive(entity) {
            return None;
        }
        let type_id = TypeId::of::<T>();
        let cell = self
            .pools
            .get(&type_id)?
            .as_any()
            .downcast_ref::<StorageCell<T, S>>()?;
        // SAFETY: &self borçlanması geçerli olduğu sürece eşzamanlı &mut borçlanma yoktur.
        unsafe { cell.get().get(entity) }
    }

    /// Güvenli bileşen yazma. `&mut self` münhasır borçlanması ile tek bir bileşeni mutably döndürür.
    #[inline]
    pub fn get_mut<T: 'static + Send + Sync>(&mut self, entity: Entity) -> Option<&mut T> {
        if !self.is_alive(entity) {
            return None;
        }
        let type_id = TypeId::of::<T>();
        let cell = self
            .pools
            .get_mut(&type_id)?
            .as_any_mut()
            .downcast_mut::<StorageCell<T, S>>()?;
        // SAFETY: `&mut self` borçlanması Registry'ye ve hücreye münhasır erişim sağlar.
        unsafe { cell.get_mut().get_mut(entity) }
    }

    /// Güvenli bileşen çıkarma. `&mut self` ile bileşeni kaldırıp döndürür.
    #[inline]
    pub fn remove<T: 'static + Send + Sync>(&mut self, entity: Entity) -> Option<T> {
        if !self.is_alive(entity) {
            return None;
        }
        let type_id = TypeId::of::<T>();
        let cell = self
            .pools
            .get_mut(&type_id)?
            .as_any_mut()
            .downcast_mut::<StorageCell<T, S>>()?;
        unsafe { cell.get_mut().remove(entity) }
    }

    /// Entity'nin belirtilen bileşene sahip olup olmadığını kontrol eder.
    #[inline]
    pub fn has<T: 'static + Send + Sync>(&self, entity: Entity) -> bool {
        self.get::<T>(entity).is_some()
    }

    /// Belirtilen bileşene sahip tüm aktif Entity'lerin listesini güvenli bir şekilde döndürür.
    #[inline]
    pub fn entities<T: 'static + Send + Sync>(&self) -> Vec<Entity> {
        let type_id = TypeId::of::<T>();
        if let Some(storage) = self.pools.get(&type_id) {
            if let Some(cell) = storage.as_any().downcast_ref::<StorageCell<T, S>>() {
                // SAFETY: &self borçlanması geçerli olduğu sürece güvenli okuma yapılır.
                return unsafe { cell.get().entities().to_vec() };
            }
        }
        Vec::new()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.generations.len() - self.free_list.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Tek bileşenli güvenli mut view
    #[inline]
    pub fn view1_mut<A: 'static + Send + Sync, F>(&mut self, mut f: F)
    where
        F: FnMut(Entity, &mut A),
    {
        self.register_component::<A>();
        let pool_a = unsafe { self.storage_mut::<A>() };
        let (entities, components) = pool_a.split_entities_and_components_mut();
        for (idx, &entity) in entities.iter().enumerate() {
            f(entity, &mut components[idx]);
        }
    }

    /// Tek bileşenli güvenli ref view
    #[inline]
    pub fn view1_ref<A: 'static + Send + Sync, F>(&self, mut f: F)
    where
        F: FnMut(Entity, &A),
    {
        let Some(cell_a) = self.pools.get(&TypeId::of::<A>()) else {
            return;
        };
        let pool_a = unsafe {
            cell_a
                .as_any()
                .downcast_ref::<StorageCell<A, S>>()
                .unwrap()
                .get()
        };
        for (idx, &entity) in pool_a.entities().iter().enumerate() {
            f(entity, &pool_a.components()[idx]);
        }
    }

    /// İki bileşenli güvenli view iterasyonu (A mut, B ref).
    /// Soundness gereği `&mut self` ister; böylece aynı anda iki sistemin veya closure içinden
    /// çakışan başka bir view'ın çağrılması derleme zamanında engellenir.
    #[inline]
    pub fn view2_mut_ref<A: 'static + Send + Sync, B: 'static + Send + Sync, F>(&mut self, mut f: F)
    where
        F: FnMut(Entity, &mut A, &B),
    {
        assert!(
            TypeId::of::<A>() != TypeId::of::<B>(),
            "A ve B ayni tip olamaz"
        );
        self.register_component::<A>();
        self.register_component::<B>();

        // SAFETY: `&mut self` borçlanması altında A ve B tipleri ayrık olduğundan
        // aliasing veya data race oluşamaz.
        let pool_a = unsafe { self.storage_mut::<A>() };
        let pool_b = unsafe { self.storage::<B>() };

        if pool_a.len() <= pool_b.len() {
            let (entities, components) = pool_a.split_entities_and_components_mut();
            for (idx, &entity) in entities.iter().enumerate() {
                if let Some(b) = pool_b.get(entity) {
                    f(entity, &mut components[idx], b);
                }
            }
        } else {
            for &entity in pool_b.entities() {
                if let Some(a) = pool_a.get_mut(entity) {
                    let b = pool_b.get(entity).unwrap();
                    f(entity, a, b);
                }
            }
        }
    }

    /// 3 Bileşenli view iterasyonu (A mut, B ref, C ref)
    /// Soundness gereği `&mut self` ister.
    #[inline]
    pub fn view3_mut_ref_ref<
        A: 'static + Send + Sync,
        B: 'static + Send + Sync,
        C: 'static + Send + Sync,
        F,
    >(
        &mut self,
        mut f: F,
    ) where
        F: FnMut(Entity, &mut A, &B, &C),
    {
        assert!(
            TypeId::of::<A>() != TypeId::of::<B>()
                && TypeId::of::<A>() != TypeId::of::<C>()
                && TypeId::of::<B>() != TypeId::of::<C>(),
            "View bilesen tipleri benzersiz olmalidir"
        );
        self.register_component::<A>();
        self.register_component::<B>();
        self.register_component::<C>();

        // SAFETY: `&mut self` altında tiplerin ayrıklığı yukarıda kanıtlanmıştır.
        let pool_a = unsafe { self.storage_mut::<A>() };
        let pool_b = unsafe { self.storage::<B>() };
        let pool_c = unsafe { self.storage::<C>() };

        let len_a = pool_a.len();
        let len_b = pool_b.len();
        let len_c = pool_c.len();

        if len_a <= len_b && len_a <= len_c {
            let (entities, components) = pool_a.split_entities_and_components_mut();
            for (idx, &entity) in entities.iter().enumerate() {
                if let (Some(b), Some(c)) = (pool_b.get(entity), pool_c.get(entity)) {
                    f(entity, &mut components[idx], b, c);
                }
            }
        } else if len_b <= len_c {
            for &entity in pool_b.entities() {
                if let (Some(a), Some(c)) = (pool_a.get_mut(entity), pool_c.get(entity)) {
                    let b = pool_b.get(entity).unwrap();
                    f(entity, a, b, c);
                }
            }
        } else {
            for &entity in pool_c.entities() {
                if let (Some(a), Some(b)) = (pool_a.get_mut(entity), pool_b.get(entity)) {
                    let c = pool_c.get(entity).unwrap();
                    f(entity, a, b, c);
                }
            }
        }
    }

    /// 2 Bileşenli salt-okunur (immutable) view iterasyonu (A ref, B ref)
    /// Render toplama gibi sistemlerde yazma izni talep etmeyerek scheduler çakışmalarını önler.
    #[inline]
    pub fn view2_ref_ref<A: 'static + Send + Sync, B: 'static + Send + Sync, F>(&self, mut f: F)
    where
        F: FnMut(Entity, &A, &B),
    {
        let Some(cell_a) = self.pools.get(&TypeId::of::<A>()) else {
            return;
        };
        let Some(cell_b) = self.pools.get(&TypeId::of::<B>()) else {
            return;
        };
        let pool_a = unsafe {
            cell_a
                .as_any()
                .downcast_ref::<StorageCell<A, S>>()
                .unwrap()
                .get()
        };
        let pool_b = unsafe {
            cell_b
                .as_any()
                .downcast_ref::<StorageCell<B, S>>()
                .unwrap()
                .get()
        };

        if pool_a.len() <= pool_b.len() {
            let entities = pool_a.entities();
            let components = pool_a.components();
            for (idx, &entity) in entities.iter().enumerate() {
                if let Some(b) = pool_b.get(entity) {
                    f(entity, &components[idx], b);
                }
            }
        } else {
            let entities = pool_b.entities();
            let components = pool_b.components();
            for (idx, &entity) in entities.iter().enumerate() {
                if let Some(a) = pool_a.get(entity) {
                    f(entity, a, &components[idx]);
                }
            }
        }
    }

    /// 3 Bileşenli salt-okunur (immutable) view iterasyonu (A ref, B ref, C ref)
    #[inline]
    pub fn view3_ref_ref_ref<
        A: 'static + Send + Sync,
        B: 'static + Send + Sync,
        C: 'static + Send + Sync,
        F,
    >(
        &self,
        mut f: F,
    ) where
        F: FnMut(Entity, &A, &B, &C),
    {
        // SAFETY: Üç bileşen de sadece okunur (&), veri yarışması oluşamaz.
        let pool_a = unsafe { self.storage::<A>() };
        let pool_b = unsafe { self.storage::<B>() };
        let pool_c = unsafe { self.storage::<C>() };

        let len_a = pool_a.len();
        let len_b = pool_b.len();
        let len_c = pool_c.len();

        if len_a <= len_b && len_a <= len_c {
            let entities = pool_a.entities();
            let components = pool_a.components();
            for (idx, &entity) in entities.iter().enumerate() {
                if let (Some(b), Some(c)) = (pool_b.get(entity), pool_c.get(entity)) {
                    f(entity, &components[idx], b, c);
                }
            }
        } else if len_b <= len_c {
            let entities = pool_b.entities();
            let components = pool_b.components();
            for (idx, &entity) in entities.iter().enumerate() {
                if let (Some(a), Some(c)) = (pool_a.get(entity), pool_c.get(entity)) {
                    f(entity, a, &components[idx], c);
                }
            }
        } else {
            let entities = pool_c.entities();
            let components = pool_c.components();
            for (idx, &entity) in entities.iter().enumerate() {
                if let (Some(a), Some(b)) = (pool_a.get(entity), pool_b.get(entity)) {
                    f(entity, a, b, &components[idx]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Pos(f32, f32);

    #[derive(Debug, PartialEq)]
    struct Vel(f32, f32);

    #[test]
    fn test_access_conflict() {
        let mut sys_a = AccessSet::new();
        sys_a.add_write::<Pos>();
        sys_a.add_read::<Vel>();

        let mut sys_b = AccessSet::new();
        sys_b.add_read::<Pos>(); // Pos okuyor, ama sys_a yazıyor -> Çakışır

        assert!(sys_a.conflicts_with(&sys_b));

        let mut sys_c = AccessSet::new();
        sys_c.add_read::<Vel>(); // Vel sadece okunuyor -> Çakışmaz

        assert!(!sys_a.conflicts_with(&sys_c));
    }

    #[test]
    #[should_panic(expected = "cift Write erisimi talep edilemez")]
    fn test_duplicate_write_rejected() {
        let mut sys = AccessSet::new();
        sys.add_write::<Pos>();
        sys.add_write::<Pos>(); // Panic olmalı
    }

    #[test]
    #[should_panic(expected = "hem Read hem Write erisimi talep edilemez")]
    fn test_read_write_same_system_rejected() {
        let mut sys = AccessSet::new();
        sys.add_write::<Pos>();
        sys.add_read::<Pos>(); // Panic olmalı
    }

    #[test]
    fn test_registry_unsafe_cell_views() {
        let mut reg = Registry::new();
        reg.register_component::<Pos>();
        reg.register_component::<Vel>();

        let e1 = reg.create();
        let e2 = reg.create();

        reg.emplace(e1, Pos(0.0, 0.0));
        reg.emplace(e1, Vel(1.0, 2.0));

        reg.emplace(e2, Pos(10.0, 20.0));

        reg.view2_mut_ref::<Pos, Vel, _>(|_e, pos, vel| {
            pos.0 += vel.0;
            pos.1 += vel.1;
        });

        unsafe {
            let pos_pool = reg.storage::<Pos>();
            assert_eq!(pos_pool.get(e1), Some(&Pos(1.0, 2.0)));
            assert_eq!(pos_pool.get(e2), Some(&Pos(10.0, 20.0)));
        }
    }

    #[test]
    fn test_registry_safe_accessors() {
        let mut reg = Registry::new();
        let e = reg.create();

        assert!(!reg.has::<Pos>(e));
        assert_eq!(reg.get::<Pos>(e), None);

        reg.emplace(e, Pos(10.0, 20.0));
        assert!(reg.has::<Pos>(e));
        assert_eq!(reg.get::<Pos>(e), Some(&Pos(10.0, 20.0)));
        assert_eq!(reg.entities::<Pos>(), vec![e]);

        if let Some(pos) = reg.get_mut::<Pos>(e) {
            pos.0 = 50.0;
        }
        assert_eq!(reg.get::<Pos>(e), Some(&Pos(50.0, 20.0)));

        let removed = reg.remove::<Pos>(e);
        assert_eq!(removed, Some(Pos(50.0, 20.0)));
        assert!(!reg.has::<Pos>(e));
        assert_eq!(reg.get::<Pos>(e), None);
        assert!(reg.entities::<Pos>().is_empty());
    }

    #[test]
    fn test_registry_clear() {
        let mut reg = Registry::new();
        let e1 = reg.create();
        let e2 = reg.create();
        reg.emplace(e1, Pos(1.0, 2.0));
        reg.emplace(e2, Pos(3.0, 4.0));
        assert_eq!(reg.len(), 2);
        assert!(reg.is_alive(e1));
        assert!(reg.is_alive(e2));

        reg.clear();
        assert_eq!(reg.len(), 0);
        assert!(!reg.is_alive(e1));
        assert!(!reg.is_alive(e2));
        assert!(reg.entities::<Pos>().is_empty());

        let e3 = reg.create();
        assert_eq!(e3.index(), e2.index());
        assert_eq!(e3.generation(), 1);
        reg.emplace(e3, Pos(5.0, 6.0));
        assert_eq!(reg.get::<Pos>(e3), Some(&Pos(5.0, 6.0)));
    }

    #[test]
    fn test_registry_emplace_checked() {
        let mut reg = Registry::new();
        let e1 = reg.create();
        assert!(reg.emplace_checked(e1, Pos(10.0, 20.0)));
        assert_eq!(reg.get::<Pos>(e1), Some(&Pos(10.0, 20.0)));

        reg.destroy(e1);
        // Destroyed entity must return false without panicking
        assert!(!reg.emplace_checked(e1, Pos(30.0, 40.0)));
    }
}
