//! DAG Tabanlı Sistem Planlayıcısı (SystemDAG & ScheduleBatch).
//!
//! Kaydedilen sistemlerin `SystemAccess` bildirimlerini analiz ederek, çakışmayan sistemleri
//! aynı paralel aşamaya (stage) gruplar. Çakışan sistemler ise sıralı aşamalarda çalıştırılır.

use crate::access::SystemAccess;
use crate::pool::ThreadPool;
use rugeng_core::Registry;

pub type SystemFn = Box<dyn Fn(&Registry, &ThreadPool) + Send + Sync + 'static>;

pub struct SystemNode {
    pub name: String,
    pub access: SystemAccess,
    pub run_fn: SystemFn,
}

pub struct ScheduleBatch {
    pub stage_index: usize,
    pub system_indices: Vec<usize>,
}

pub struct SystemDAG {
    systems: Vec<SystemNode>,
    stages: Vec<ScheduleBatch>,
    dirty: bool,
}

impl Default for SystemDAG {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemDAG {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            stages: Vec::new(),
            dirty: false,
        }
    }

    /// Yeni bir sistemi isim, erişim kümesi ve çalıştırma fonksiyonu ile ekler.
    pub fn add_system<F>(&mut self, name: impl Into<String>, access: SystemAccess, f: F)
    where
        F: Fn(&Registry, &ThreadPool) + Send + Sync + 'static,
    {
        self.systems.push(SystemNode {
            name: name.into(),
            access,
            run_fn: Box::new(f),
        });
        self.dirty = true;
    }

    /// Çakışma matrisine göre sistemleri DAG katmanlarına (stages) ayırır.
    pub fn build_schedule(&mut self) {
        if !self.dirty && !self.stages.is_empty() {
            return;
        }

        self.stages.clear();

        for (idx, sys) in self.systems.iter().enumerate() {
            // idx'ten önce gelen ve bu sistemle çakışan sistemlerin bulunduğu en son stage indeksini bul.
            // Bu sistem, o çakışan sistemlerden SONRAKİ bir stage'e yerleştirilmelidir (sıralı bağımlılık / RAW-WAR koruma).
            let mut min_stage_idx = 0;
            for prev_idx in 0..idx {
                if sys.access.conflicts_with(&self.systems[prev_idx].access) {
                    // prev_idx hangi stage'de?
                    if let Some(stage_idx) = self
                        .stages
                        .iter()
                        .position(|st| st.system_indices.contains(&prev_idx))
                    {
                        min_stage_idx = min_stage_idx.max(stage_idx + 1);
                    }
                }
            }

            // min_stage_idx'ten başlayarak bu sistemle çakışmayan ilk stage'e yerleştir
            let mut placed = false;
            for stage in self.stages.iter_mut().skip(min_stage_idx) {
                let has_conflict = stage
                    .system_indices
                    .iter()
                    .any(|&other_idx| sys.access.conflicts_with(&self.systems[other_idx].access));

                if !has_conflict {
                    stage.system_indices.push(idx);
                    placed = true;
                    break;
                }
            }

            if !placed {
                // Çakışmayan uygun bir stage bulunamadıysa yeni bir stage aç
                self.stages.push(ScheduleBatch {
                    stage_index: self.stages.len(),
                    system_indices: vec![idx],
                });
            }
        }

        self.dirty = false;
    }

    pub fn stages(&self) -> &[ScheduleBatch] {
        &self.stages
    }

    pub fn system_count(&self) -> usize {
        self.systems.len()
    }

    /// Planlanan tüm sistemleri ThreadPool üzerinde aşama aşama (stage by stage) paralel çalıştırır.
    pub fn run_all(&mut self, registry: &Registry, pool: &ThreadPool) {
        if self.dirty {
            self.build_schedule();
        }

        for stage in &self.stages {
            if stage.system_indices.len() == 1 {
                // Tek sistem varsa gereksiz thread geçişi yapmadan doğrudan çalıştır
                let idx = stage.system_indices[0];
                (self.systems[idx].run_fn)(registry, pool);
            } else {
                // Aynı aşamadaki çakışmayan sistemleri paralel çalıştır
                pool.scope(|scope| {
                    for &idx in &stage.system_indices {
                        let sys = &self.systems[idx];
                        scope.spawn(move || {
                            (sys.run_fn)(registry, pool);
                        });
                    }
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct Pos;
    struct Vel;
    struct RenderData;

    #[test]
    fn test_dag_schedule_batching() {
        let mut dag = SystemDAG::new();

        // System 1: Pos'a yazıyor
        dag.add_system(
            "Movement",
            SystemAccess::new().with_write::<Pos>().with_read::<Vel>(),
            |_reg, _pool| {},
        );

        // System 2: RenderData'ya yazıyor, Pos'u okuyor (System 1 ile çakışır çünkü System 1 Pos'a yazıyor)
        dag.add_system(
            "RenderPrepare",
            SystemAccess::new()
                .with_write::<RenderData>()
                .with_read::<Pos>(),
            |_reg, _pool| {},
        );

        // System 3: Vel'i okuyor (System 1 Vel'i sadece okuyor, bu da okuyor; çakışmaz! Ama System 1 Pos'a yazıyor, sys 3 Pos'a dokunmuyor)
        dag.add_system(
            "VelocityCheck",
            SystemAccess::new().with_read::<Vel>(),
            |_reg, _pool| {},
        );

        dag.build_schedule();

        // System 1 ve System 3 çakışmaz -> Aynı Stage 0'a yerleşmeli!
        // System 2 ise Stage 1'e yerleşmeli!
        assert_eq!(dag.stages().len(), 2);
        assert_eq!(dag.stages()[0].system_indices.len(), 2); // Sys 1 ve Sys 3
        assert_eq!(dag.stages()[1].system_indices.len(), 1); // Sys 2
    }

    #[test]
    fn test_dag_execution() {
        let mut dag = SystemDAG::new();
        let pool = ThreadPool::new(4);
        let registry = Registry::new();

        let execution_order = Arc::new(AtomicUsize::new(0));

        let c1 = Arc::clone(&execution_order);
        dag.add_system(
            "Sys1",
            SystemAccess::new().with_write::<Pos>(),
            move |_reg, _pool| {
                c1.fetch_add(1, Ordering::SeqCst);
            },
        );

        let c2 = Arc::clone(&execution_order);
        dag.add_system(
            "Sys2",
            SystemAccess::new().with_write::<Vel>(),
            move |_reg, _pool| {
                c2.fetch_add(10, Ordering::SeqCst);
            },
        );

        dag.run_all(&registry, &pool);

        assert_eq!(execution_order.load(Ordering::SeqCst), 11);
    }

    #[test]
    fn test_dag_three_system_chain_preserves_order() {
        // System A: Write Pos (Stage 0)
        // System B: Read Pos (System A ile çakışır -> Stage 1)
        // System C: Write Pos (System B ile çakışır -> Stage 2)
        // Eğer yanlış algoritmada C Stage 0'a yerleşirse B'den önce çalışırdı ve data race/sıra bozulması olurdu!
        let mut dag = SystemDAG::new();

        dag.add_system(
            "SysA_WritePos",
            SystemAccess::new().with_write::<Pos>(),
            |_reg, _pool| {},
        );
        dag.add_system(
            "SysB_ReadPos",
            SystemAccess::new().with_read::<Pos>(),
            |_reg, _pool| {},
        );
        dag.add_system(
            "SysC_WritePos",
            SystemAccess::new().with_write::<Pos>(),
            |_reg, _pool| {},
        );

        dag.build_schedule();

        assert_eq!(dag.stages().len(), 3);
        assert_eq!(dag.stages()[0].system_indices, vec![0]);
        assert_eq!(dag.stages()[1].system_indices, vec![1]);
        assert_eq!(dag.stages()[2].system_indices, vec![2]);
    }
}
