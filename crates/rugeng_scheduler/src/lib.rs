//! Rugeng Scheduler: Work-stealing thread pool, DAG paralel sistem çalıştırıcısı ve erişim denetimi.
//!
//! Özellikler:
//! - Work-stealing thread pool (`crossbeam-deque` tabanlı kilit-özgür yerel kuyruklar).
//! - `AccessSet` tabanlı çakışma matrisi ve otomatik DAG katmanlama (stages).
//! - Bağımsız sistemler ve paralel veri dilimleme (data-parallel chunks) eşzamanlı çalıştırılır.
//! - Çakışan sistemler sıralı aşamalar (sequential stages) halinde çalıştırılır.
//! - Worker başına thread-local arena desteği (contention sıfırlama).

pub mod access;
pub mod dag;
pub mod pool;
pub mod thread_arena;

pub use access::{Access, SystemAccess};
pub use dag::{ScheduleBatch, SystemDAG, SystemNode};
pub use pool::{JobHandle, ThreadPool};
pub use thread_arena::ThreadArenaPool;
