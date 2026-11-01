//! Work-Stealing ThreadPool: crossbeam-deque tabanlı, düşük gecikmeli iş havuzu.

use crossbeam_deque::{Injector, Steal, Stealer, Worker};
use parking_lot::{Condvar, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

type Job = Box<dyn FnOnce() + Send + 'static>;

struct PoolInner {
    injector: Injector<Job>,
    stealers: Vec<Stealer<Job>>,
    shutdown: AtomicBool,
    active_jobs: AtomicUsize,
    completion_mutex: Mutex<()>,
    completion_cvar: Condvar,
    sleep_mutex: Mutex<()>,
    sleep_cvar: Condvar,
}

pub struct ThreadPool {
    inner: Arc<PoolInner>,
    workers: Vec<JoinHandle<()>>,
    worker_count: usize,
}

impl ThreadPool {
    /// Sistem çekirdek sayısına veya belirtilen sayıya göre thread havuzu başlatır.
    pub fn new(num_threads: usize) -> Self {
        let thread_count = if num_threads == 0 {
            std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(4)
        } else {
            num_threads
        };

        let injector = Injector::new();
        let mut workers_deques = Vec::with_capacity(thread_count);
        let mut stealers = Vec::with_capacity(thread_count);

        for _ in 0..thread_count {
            let w = Worker::new_fifo();
            stealers.push(w.stealer());
            workers_deques.push(w);
        }

        let inner = Arc::new(PoolInner {
            injector,
            stealers,
            shutdown: AtomicBool::new(false),
            active_jobs: AtomicUsize::new(0),
            completion_mutex: Mutex::new(()),
            completion_cvar: Condvar::new(),
            sleep_mutex: Mutex::new(()),
            sleep_cvar: Condvar::new(),
        });

        let mut workers = Vec::with_capacity(thread_count);
        for (id, worker_deque) in workers_deques.into_iter().enumerate() {
            let inner_clone = Arc::clone(&inner);
            let handle = thread::Builder::new()
                .name(format!("rugeng-worker-{id}"))
                .spawn(move || {
                    Self::worker_loop(id, worker_deque, inner_clone);
                })
                .expect("Worker thread baslatilamadi");
            workers.push(handle);
        }

        Self {
            inner,
            workers,
            worker_count: thread_count,
        }
    }

    pub fn worker_count(&self) -> usize {
        self.worker_count
    }

    /// Yeni bir görevi global enjektöre ekler.
    pub fn spawn<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.inner.active_jobs.fetch_add(1, Ordering::SeqCst);
        self.inner.injector.push(Box::new(f));
        self.inner.sleep_cvar.notify_one();
    }

    /// Sonuç değeri döndüren asenkron bir görevi havuza gönderir ve bir `JobHandle` döndürür.
    pub fn spawn_with_handle<F, T>(&self, f: F) -> JobHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let result = Arc::new(Mutex::new(None));
        let done = Arc::new(AtomicBool::new(false));

        let res_clone = Arc::clone(&result);
        let done_clone = Arc::clone(&done);

        self.spawn(move || {
            // R1: Panikleyen görev sonucu yazamaz; `done` yine de set edilir ki
            // join() sonsuza dek bloklamasın (sonuç yoksa join panikler).
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
                Ok(val) => *res_clone.lock() = Some(val),
                Err(_) => eprintln!("[Pool] Job gorevi panikledi; sonucu bos kaliyor."),
            }
            done_clone.store(true, Ordering::Release);
        });

        JobHandle { result, done }
    }

    /// Dilimlenmiş görevleri (data parallelism) worker'lara dağıtır ve hepsi bitene kadar senkron bekler.
    pub fn scope<'env, F, R>(&'env self, f: F) -> R
    where
        F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> R,
    {
        let scope = Scope {
            pool: self,
            active_tasks: Arc::new(AtomicUsize::new(0)),
            notify: Arc::new((Mutex::new(()), Condvar::new())),
            _marker: std::marker::PhantomData,
        };

        let result = f(&scope);

        // Scope içindeki tüm görevlerin tamamlanmasını bekle
        scope.wait();

        result
    }

    /// Gönderilmiş tüm bekleyen işlerin tamamlanmasını bekler.
    pub fn wait_idle(&self) {
        let mut lock = self.inner.completion_mutex.lock();
        while self.inner.active_jobs.load(Ordering::SeqCst) > 0 {
            self.inner.completion_cvar.wait(&mut lock);
        }
    }

    fn worker_loop(id: usize, local: Worker<Job>, inner: Arc<PoolInner>) {
        loop {
            if inner.shutdown.load(Ordering::Relaxed)
                && local.is_empty()
                && inner.injector.is_empty()
            {
                break;
            }

            // 1. Yerel kuyruktan al
            let mut job = local.pop();

            // 2. Bulunamadıysa global enjektörden çal/çek
            if job.is_none() {
                job = loop {
                    match inner.injector.steal_batch_and_pop(&local) {
                        Steal::Success(j) => break Some(j),
                        Steal::Empty => break None,
                        Steal::Retry => {}
                    }
                };
            }

            // 3. Hala bulunamadıysa diğer worker'lardan iş çal (work stealing)
            if job.is_none() {
                let num_stealers = inner.stealers.len();
                for i in 0..num_stealers {
                    let target = (id + i + 1) % num_stealers;
                    job = loop {
                        match inner.stealers[target].steal() {
                            Steal::Success(j) => break Some(j),
                            Steal::Empty => break None,
                            Steal::Retry => {}
                        }
                    };
                    if job.is_some() {
                        break;
                    }
                }
            }

            // 4. İş bulunduysa çalıştır, bulunamadıysa uyu
            if let Some(j) = job {
                // R1: Görev paniklese bile worker thread'i ölmez ve sayaç düşümü kaçınılmazdır.
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(j)).is_err() {
                    eprintln!("[Pool] Gorev panikledi; worker saglam bir sekilde devam ediyor.");
                }
                let remaining = inner.active_jobs.fetch_sub(1, Ordering::SeqCst);
                if remaining == 1 {
                    let _lock = inner.completion_mutex.lock();
                    inner.completion_cvar.notify_all();
                }
            } else {
                if inner.shutdown.load(Ordering::Relaxed) {
                    break;
                }
                let mut guard = inner.sleep_mutex.lock();
                if inner.injector.is_empty() && local.is_empty() {
                    inner
                        .sleep_cvar
                        .wait_for(&mut guard, std::time::Duration::from_micros(250));
                }
            }
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.inner.shutdown.store(true, Ordering::SeqCst);
        self.inner.sleep_cvar.notify_all();

        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

pub struct JobHandle<T> {
    result: Arc<Mutex<Option<T>>>,
    done: Arc<AtomicBool>,
}

impl<T> JobHandle<T> {
    pub fn is_finished(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    pub fn join(self) -> T {
        while !self.is_finished() {
            std::thread::yield_now();
        }
        self.result.lock().take().expect("Job sonucu bulunamadi")
    }
}

pub struct Scope<'scope, 'env: 'scope> {
    pool: &'env ThreadPool,
    active_tasks: Arc<AtomicUsize>,
    notify: Arc<(Mutex<()>, Condvar)>,
    _marker: std::marker::PhantomData<&'scope mut &'env ()>,
}

impl<'scope, 'env: 'scope> Scope<'scope, 'env> {
    /// Scope içinde dinamik ömürlü bir görevi havuza gönderir.
    pub fn spawn<F>(&'scope self, f: F)
    where
        F: FnOnce() + Send + 'scope,
    {
        self.active_tasks.fetch_add(1, Ordering::SeqCst);
        self.pool.inner.active_jobs.fetch_add(1, Ordering::SeqCst);

        let active_tasks = Arc::clone(&self.active_tasks);
        let notify = Arc::clone(&self.notify);

        let raw_closure: Box<dyn FnOnce() + Send + 'scope> = Box::new(move || {
            // R1: f() paniklerse active_tasks sayacı düşmez kalırdı; panik yutulur,
            // sayaç ve bildirim her koşulda çalışır.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            let remaining = active_tasks.fetch_sub(1, Ordering::SeqCst);
            if remaining == 1 {
                let _lock = notify.0.lock();
                notify.1.notify_all();
            }
        });

        // SAFETY: Scope::wait çağrısı tamamlanmadan Scope düşmeyeceğinden 'scope referansları güvenlidir.
        let static_closure: Box<dyn FnOnce() + Send + 'static> =
            unsafe { std::mem::transmute(raw_closure) };

        self.pool.inner.injector.push(static_closure);
        self.pool.inner.sleep_cvar.notify_one();
    }

    /// Verilen eleman sayısını worker sayısına göre dengeli parçalara (chunks) bölüp paralel çalıştırır.
    pub fn for_each_chunk<T, F>(&'scope self, items: &'scope mut [T], min_chunk_size: usize, f: F)
    where
        T: Send,
        F: Fn(&mut [T]) + Send + Sync + 'scope,
    {
        let len = items.len();
        if len == 0 {
            return;
        }

        let worker_count = self.pool.worker_count();
        let chunk_size = (len / worker_count).max(min_chunk_size).max(1);

        let f_ref = &f;
        let chunks_iter = items.chunks_mut(chunk_size);

        for chunk in chunks_iter {
            self.spawn(move || {
                f_ref(chunk);
            });
        }
    }

    /// Belirtilen aralığı worker sayısına göre dengeli dilimlere (range) bölüp paralel çalıştırır.
    pub fn for_each_range<F>(&'scope self, total_len: usize, min_chunk_size: usize, f: F)
    where
        F: Fn(usize, usize) + Send + Sync + 'scope,
    {
        if total_len == 0 {
            return;
        }

        let worker_count = self.pool.worker_count();
        let chunk_size = (total_len / worker_count).max(min_chunk_size).max(1);

        let f_ref = &f;
        let mut start = 0;
        while start < total_len {
            let end = (start + chunk_size).min(total_len);
            self.spawn(move || {
                f_ref(start, end);
            });
            start = end;
        }
    }

    fn wait(&self) {
        let mut lock = self.notify.0.lock();
        while self.active_tasks.load(Ordering::SeqCst) > 0 {
            self.notify.1.wait(&mut lock);
        }
    }
}

impl Drop for Scope<'_, '_> {
    fn drop(&mut self) {
        // R2: Kullanıcı closure'ı paniklerse `scope.wait()` atlanır; Drop, 'env
        // verilerine referans taşıyan görevler bitmeden Scope'un bırakılmasını
        // engeller (aks halde worker sonradan ölü veriyi çalıştırıp UB yapardı).
        self.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_pool_basic_execution() {
        let pool = ThreadPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));

        for _ in 0..100 {
            let c = Arc::clone(&counter);
            pool.spawn(move || {
                c.fetch_add(1, Ordering::Relaxed);
            });
        }

        pool.wait_idle();
        assert_eq!(counter.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn test_thread_pool_scope_and_chunks() {
        let pool = ThreadPool::new(4);
        let mut data: Vec<u32> = (0..10_000).collect();

        pool.scope(|s| {
            s.for_each_chunk(&mut data, 256, |chunk| {
                for item in chunk {
                    *item *= 2;
                }
            });
        });

        for (i, val) in data.iter().enumerate() {
            assert_eq!(*val, (i as u32) * 2);
        }
    }

    #[test]
    fn test_thread_pool_job_handle() {
        let pool = ThreadPool::new(2);
        let handle = pool.spawn_with_handle(|| 21 * 2);
        let result = handle.join();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_panicking_task_does_not_break_pool() {
        // R1: Panikleyen görev worker'ı öldürmemeli, sayaçları bozmamalı,
        // wait_idle() kilitlenmemeli ve havuz panikten sonra da çalışmalı.
        let pool = ThreadPool::new(2);
        let counter = Arc::new(AtomicUsize::new(0));

        for _ in 0..8 {
            let c = Arc::clone(&counter);
            pool.spawn(move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        let panicker = {
            let c = Arc::clone(&counter);
            move || {
                c.fetch_add(1, Ordering::SeqCst);
                panic!("test panigi");
            }
        };
        pool.spawn(panicker);
        for _ in 0..8 {
            let c = Arc::clone(&counter);
            pool.spawn(move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }

        pool.wait_idle();
        assert_eq!(counter.load(Ordering::SeqCst), 17);

        // Havuz hâlâ işlevsel olmalı (panikleyen worker geri kazanıldı).
        let h = pool.spawn_with_handle(|| 7 * 6);
        assert_eq!(h.join(), 42);
    }

    #[test]
    fn test_scope_with_panicking_task_completes() {
        // R1: Scope içindeki panik active_tasks sayacını düşürmeye devam etmeli;
        // aksi halde Scope::wait() sonsuza dek bloklardı.
        let pool = ThreadPool::new(2);
        let counter = Arc::new(AtomicUsize::new(0));

        pool.scope(|s| {
            let c = Arc::clone(&counter);
            s.spawn(move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
            s.spawn(|| panic!("scope ici panik"));
        });

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_scope_closure_panic_drains_via_drop() {
        // R2: Kapanış closure'ı paniklerse Drop::wait devreye girmeli; 'env
        // verisine referanslı görev, veri serbest bırakılmadan tamamlanmalı.
        let pool = ThreadPool::new(2);
        let mut data: Vec<u32> = (0..64).collect();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pool.scope(|s| {
                let d = &data;
                s.spawn(move || {
                    let _sum: u32 = d.iter().sum();
                });
                panic!("kapanis panigi");
            });
        }));
        assert!(result.is_err(), "kapanis panigi iletildi");

        // Görev veriye referansını sürdürürken veri hâlâ geçerli olmalı.
        assert_eq!(data.len(), 64);
        data[0] = 99;
        pool.wait_idle();
    }

    #[test]
    fn test_job_handle_panicking_task_joins_with_panic() {
        // Panikleyen job'ın join()'i bloklamamalı; net bir panikle sonuçsuzluğu bildirmeli.
        let pool = ThreadPool::new(1);
        let handle = pool.spawn_with_handle(|| -> u32 { panic!("gorev patladi") });

        let joined = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || handle.join()));
        assert!(joined.is_err(), "panikleyen gorevin join'i paniklemeli");

        // Havuz bozulmadan devam etmeli.
        let h2 = pool.spawn_with_handle(|| 123u32);
        assert_eq!(h2.join(), 123);
    }
}
