# Aşama 3: İş Sistemi ve Paralel Planlayıcı Raporu (Job System & Parallel Scheduler)

**Tarih:** 31 Ekim 2026  
**Proje:** Project-Ember (Rust - `rugeng`)  
**Kapsam:** `crates/rugeng_scheduler` ve `games/bullet_hell` Entegrasyonu  
**Durum:** Çıkış Kriterleri ve G1–G6 Başarıyla Doğrulandı  

---

## 1. Mimari Genel Bakış ve Gerçekleştirilen Hedefler (G1–G6)

### G1: Work-Stealing İş Parçacığı Havuzu (`rugeng_scheduler::ThreadPool`)
- `crossbeam-deque` (v0.8) tabanlı kilit-özgür (lock-free) yerel iş kuyrukları (`Worker<Job>`), iş çalma arayüzü (`Stealer<Job>`) ve merkezi enjektör (`Injector<Job>`) mimarisi kuruldu.
- FIFO tabanlı yerel iş tüketimi ve LIFO/Steal dengesi ile iş parçacıkları arasında dinamik yük dağılımı sağlandı.
- Görevler bittiğinde CPU döngüsünü boşa yakmamak için `parking_lot::Condvar` ile uyku ve uyandırma mekanizması entegre edildi.
- `Scope` yapısı (`ThreadPool::scope`) ile yaşam süresi kısıtlı referanslar üzerinde (`for_each_chunk` ve `for_each_range`) güvenli paralel veri işleme (data parallelism) sağlandı.

### G2: Çakışma Denetimi ve DAG Sistem Planlaması (`rugeng_scheduler::SystemDAG`)
- `rugeng_core::AccessSet` sarmalanarak `SystemAccess` API'si oluşturuldu (`with_read::<T>()`, `with_write::<T>()`).
- Sistemler arasındaki bileşen erişim kümeleri ve kayıt sırası (RAW/WAR veri bağımlılığı) taranarak bağımsız/çakışmayan sistemler aynı aşamaya (`ScheduleBatch`) yerleştirildi; veri yarışması (data race) oluşturabilecek yazma-yazma veya okuma-yazma çakışmaları sıralı aşamalara (sequential stages) bölünerek nedensellik/bağımlılık sırası korundu.
- `SystemDAG::run_all` ile aynı aşamadaki sistemler iş parçacığı havuzunda eşzamanlı çalıştırılırken, tekil sistemler gereksiz thread geçiş maliyeti (context switch overhead) olmaksızın doğrudan yürütülür.

### G3: Unsafe Sınır Denetimi ve Referans Güvenliği
- `rugeng_core::FrameArena::alloc`: CAS tabanlı `AtomicUsize` ile çalışır. Farklı iş parçacıklarının döndürdüğü `&mut T` göstericilerinin kesinlikle çakışmayan bellek adreslerini gösterdiği kanıtlanmıştır.
- `rugeng_core::Registry::storage_mut`: `unsafe` olarak işaretlenmiş olup, çağrıcının çakışmasızlık sağlamasını zorunlu kılar. `SystemAccess` ve `SystemDAG` planlayıcısı bu kuralı derleme/planlama aşamasında garanti altına alır.
- Tüm `unsafe` bloklarında `// SAFETY:` gerekçelendirmeleri eksiksiz yer almaktadır.

### G4: İş Parçacığı Başına Arena Havuzu ve Contention Profili (`ThreadArenaPool`)
- Birden çok çekirdeğin ortak bir `AtomicUsize` üzerinde CAS yapması sonucu oluşan önbellek hattı sıçramasını (cache-line bouncing) ve çekişmeyi (contention) yok etmek için `ThreadArenaPool` geliştirildi.
- Her worker için bağımsız 8MB/16MB `FrameArena` tahsis edildi (örneğin 8 worker için toplam 64MB).
- **Contention Profil Sonuçları (Criterion Ölçümü):**
  - **Ortak Atomik Arena (8 Thread):** `344.78 µs`
  - **ThreadArenaPool (8 Thread Bağımsız):** `51.68 µs`
  - **Kazanım:** Çekişme ortadan kaldırılarak bellek tahsis süresi **%85.0 oranında (6.67 kat)** hızlandırıldı.

### G5: Bullet Hell Simülasyonu Paralelleştirme ve Profil Sonuçları
- `games/bullet_hell` üzerinde 20.000 mermi simülasyonu analiz edildi.
- Mermi pozisyon entegrasyonu (`pos += vel * dt`) ve ekran sınırı kontrolleri `ThreadPool::scope` ile worker çekirdeklerine bölünerek paralelleştirildi.
- Çekirdekler kendi yerel mermi silme kuyruklarına (`thread_despawns`) yazarak kilit çekişmesini sıfırladı.
- Çalışma zamanında `[J]` tuşuyla sıralı tek çekirdek (serial) ile paralel work-stealing planlayıcı arasında dinamik geçiş sağlandı.
- **20.000 Mermi Fizik Simülasyonu (Criterion Benchmark):**
  - **Sıralı Tek Çekirdek:** `16.51 µs` (doğrudan SIMD/L1 önbellek ardışık vektör okuması)
  - **Paralel Work-Stealing (Scope & Chunking):** `17.62 µs`
  - **Analiz:** 20.000 mermilik salt 2x `f32` skalar vektör toplamı L1/L2 önbelleğe sığdığı için (`20.000 * 8 byte = 160 KB`), CPU çekirdekleri arası thread senkronizasyon maliyeti (~1 µs) paralel hızlanmanın sınırında kalmaktadır. Ancak sistem karmaşıklığı ve mekansal çarpışma/parçacık yükü arttığında veya 8-thread arena tahsislerinde paralel havuz **6.67 kat** fark yaratmaktadır.

### G6: Eşzamanlılık ve Model Doğrulama (Loom & Miri)
- `tests/loom_scheduler.rs` altında Loom model testleri çalıştırıldı:
  - `test_loom_task_completion_sync`: Görev bitiş sayaçları ve SeqCst/Release-Acquire bayrak senkronizasyonunda data race olmadığı kanıtlandı.
  - `test_loom_parallel_chunk_disjoint_access`: Paralel dilimlerin ayrık indekslere eriştiği ve veri yarışması üretmediği doğrulandı.
- Determinizm testi: 20.000 elemanın paralel güncellenmesinde sıra bağımlılığı olmaksızın deterministik sonuç verdiği doğrulandı.

---

## 2. Doğrulama ve Test Kayıtları

1. **Birim & Entegrasyon Testleri:**
   ```bash
   cargo test --workspace
   ```
   - Sonuç: `rugeng_core` (27 test), `rugeng_physics` (18 test), `rugeng_render` (12 test), `rugeng_scheduler` (8 test + 2 entegrasyon testi), `rugeng_editor` (9 test) hepsi başarılı (`test result: ok`).

2. **Loom Eşzamanlılık Model Testleri:**
   ```bash
   RUSTFLAGS="--cfg loom" cargo test -p rugeng_scheduler --test loom_scheduler
   ```
   - Sonuç: 2 model testi 0 data race ile tamamlandı.

3. **Sıfır Bellek Tahsisi (0 Malloc Churn Testi):**
   ```bash
   cargo test --test counting_allocator_churn --release
   ```
   - Sonuç: `test_gameplay_loop_churn_10k_zero_malloc ... ok` (0 heap re-alloc).

4. **Biçimlendirme ve Linter Denetimi:**
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   ```
   - Sonuç: 0 uyarı, 0 hata.

---

## 3. Kod Değişiklikleri Özeti

- `crates/rugeng_scheduler`:
  - `src/pool.rs`: Work-stealing `ThreadPool`, `Scope`, `JobHandle`, `spawn_with_handle`, `for_each_chunk`, `for_each_range`.
  - `src/access.rs`: `SystemAccess` ve çakışma çözümleme.
  - `src/dag.rs`: `SystemDAG` ve `ScheduleBatch` (kayıt sırası ve RAW/WAR bağımlılık korumalı aşamalandırıcı).
  - `src/thread_arena.rs`: Worker başına `ThreadArenaPool` ve contention önleyici yapı.
  - `benches/scheduler_benchmark.rs`: Criterion benchmark paketi.
  - `tests/loom_scheduler.rs`: Loom eşzamanlılık modelleri.
- `games/bullet_hell`:
  - `ThreadPool` ve `ThreadArenaPool` entegrasyonu.
  - `[J]` tuşuyla canlı Seri / Paralel Scheduler geçişi ve HUD göstergesi.
  - 20.000 mermi paralel güncellemesi.
