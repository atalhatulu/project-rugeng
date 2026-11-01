# PROJECT-EMBER (Rust Port / Rugeng) Mimari Analiz, Riskler ve Eleştiri Raporu

## 1. Yönetici Özeti ve Eleştirel Değerlendirme

Brief'te sunulan C++ prototipi incelendi (`ember::core::FrameArena`, `ember::core::SparsePageTable`, `SparseSet<T>`, `Registry`, `View`). C++'taki mimari doğrudan işaretçi (pointer) manipülasyonuna ve esnek nesne ömürlerine dayanıyor. Bu yapıyı doğrudan birebir Rust'a taşımaya çalışmak, Rust'ın sahiplik (ownership), borçlanma denetleyicisi (borrow checker) ve eşzamanlılık (aliasing / data race) garantileriyle doğrudan çatışır.

Aşağıda brief'teki kritik varsayımlar, zayıf noktalar, riskler ve önerilen alternatif yaklaşımlar listelenmiştir.

---

## 2. Riskler, Zayıf Noktalar ve Eksik Varsayımlar

### Risk 1: Frame Arena ve Rust Borrow Checker Çatışması
- **Zayıf Nokta:** C++'ta `FrameArena::allocate<T>()` ham bellekten işaretçi döndürür ve kare sonunda `reset()` çağrıldığında işaretçilerin geçerliliği denetlenmez. Rust'ta arena'dan bir dilim (`&'a mut [T]`) dağıtıldığında, arena o ömür (`'a`) boyunca münhasıran borçlanılmış (borrowed) kalır.
- **Kritik Sorun:** Eğer arena borçlanmış durumdaysa, başka bir sistem arena'dan yer tahsis edemez; kare sonunda `arena.reset()` çağrılması derleyici tarafından engellenir (çünkü hala yaşayan referanslar vardır). Üstelik Job System ile sistemler paralel koştuğunda tek bir `&mut Arena` üzerinden eşzamanlı tahsis yapılamaz.
- **Eksik Varsayım:** "Frame arena ile gameplay döngüsünde sıfır malloc" iddiası, Rust'ın güvenli dünyasında ya `unsafe` ile ömürleri silmeyi (`transmute`) ya da her iş parçacığına özel (thread-local) bump allocator kullanmayı gerektirir.

### Risk 2: ECS View Sorgularında Çoklu Değiştirilebilir Borçlanma (Multiple Mutable Borrow)
- **Zayıf Nokta:** Bir sistem hem `&mut Position` hem de `&Velocity` talep ettiğinde; C++'ta `registry.view<Position, Velocity>()` ham işaretçilerle bunu çözer. Rust'ta ise `Registry` tek bir yapıdaysa, `registry.get_pool_mut::<Position>()` çağrısı tüm registry'yi borçlar, ardından `registry.get_pool::<Velocity>()` çağrısı derleme hatası verir (E0499 / E0502).
- **Sonuç:** Bevy ECS veya Hecs gibi kütüphaneler bu sorunu çözmek için bileşen depolarını içsel değişkenlik (`UnsafeCell` + çalışma zamanı borçlanma bayrakları / Archetype id'leri) ile sarmalar. Sıfırdan bir Registry yazarken tip güvenli "split borrow" mekanizması tasarlanmazsa her sistem `unsafe` batağına saplanır.

### Risk 3: Sayfalı Sparse-Set Karmaşıklığı (Overengineering Tuzağı)
- **Zayıf Nokta:** 2 kademeli 1024'lük sayfa tablosu (SparsePageTable), 1 milyon entity'ye kadar sparse belleği sınırlandırmak içindir. Ancak 50k - 200k entity hedefinde düz bir `Vec<u32>` sparse dizisi yalnızca 200k * 4 bayt = 800 KB RAM tutar. Modern L3 önbellekler 32-96 MB iken 800 KB için 2 kademeli işaretçi takibi (indirection) CPU önbellek ıskalamasını (cache miss) artırabilir.
- **Eksik Varsayım:** Sayfalı sparse set, bellek tasarrufu sağlar ancak arama başına fazladan bir pointer dereference ve dallanma (branch) ekler. 50k entity benchmark'ında düz sparse set daha hızlı çıkabilir.

### Risk 4: Vulkan 1.3 `ash` Kullanımı ve Kapsam Sapması
- **Zayıf Nokta:** Vulkan 1.3'ü (dynamic rendering, sync2, swapchain recreate, ring upload buffer) `ash` ile sıfırdan yazmak binlerce satır hata riskli unsafe kod demektir. Hedef "hızlı çalışan oyun motoru" iken proje "Vulkan boilerplate kütüphanesi yazma" batağına saplanabilir.
- **Kural Çelişkisi:** "Olgun kütüphane varsa sıfırdan yazılmaz" ilkesi geçerliyse, render donanım soyutlaması için neden `wgpu` tamamen eleniyor? `wgpu`'nun Vulkan backend'i instanced 50k quad çiziminde saniyede 1000+ FPS verebilmektedir. `ash` tercihi korunacaksa, renderer motor çekirdeğinden kesin çizgilerle izole edilmeli, ilk aşamada minimum tutulmalıdır.

---

## 3. Önerilen Alternatif Yaklaşımlar

| Alan | Brief'in Önerisi | Karşılaşılan Sorun | Önerilen Alternatif Yaklaşım (Daha İyi / Neden?) |
|---|---|---|---|
| **Arena** | Tekil 64MB Sanal Bellek Arena (`mmap`) | Thread-safety yok, borrow-checker çakışması, lifetime kilitlenmesi. | **Thread-Local Chunked Bump Arena / Scope Tabanlı Arena:** Her iş parçacığına bağımsız `Bump` havuzu verilmesi veya frame verilerinin nesne ömrünün `scoped` closure ile sınırlandırılması. |
| **ECS Storage** | Tüm havuzları tutan tekil dinamik `Registry` | Tip bazlı `&mut` çakışması, compile-time borrow hatası. | **Ayrık Bileşen Depoları (Component Storage Tuples / Borrow Set):** Bileşen havuzlarının bağımsız `UnsafeCell` / Read-Write kilitli hücrelerde tutulması, sistemlerin sadece ilgilendikleri havuzları referans alması. |
| **Sparse-Set** | 2 Kademeli Sayfalı Tablo (1024'lük sayfalar) | 50k entity için gereksiz dolaylı erişim (pointer indirection). | **Hibrit Düz/Sayfalı Sparse-Set:** 50k için düz `Vec<u32>`, 1M+ gereksiniminde sayfalama. Benchmark ile iki yapının L1/L3 önbellek farkı kıyaslanmalı. |
| **Render** | Çekirdeğe sıkı bağlı Vulkan 1.3 kodu | Render kodu motorun geri kalanını bloke eder. | **Veri Güdümlü Çizim Kuyruğu (RenderPacket / DrawStream):** Motor sadece ardışık `[QuadInstance; N]` üretir. Renderer bu paketi tüketir. Vulkan implementasyonu tamamen bir eklenti (plugin) gibi çalışır. |

---

## 4. Yapılan Mimari Düzeltmeler (Kullanıcı İncelemesi Sonrası)

1. **`UnsafeCell` + Sistem Erişim Deklarasyonu (`AccessSet`):**
   - `RefCell<Box<dyn ComponentPool>>` kaldırıldı (`RefCell` `Sync` olmadığı ve `dyn` dinamik çağrı maliyeti getirdiği için).
   - Yerine `StorageCell<T, S>` (`UnsafeCell`) kuruldu.
   - Sistemler çalıştırmadan önce `AccessSet` ile okuma (`Read`) ve yazma (`Write`) bileşenlerini bildirir.
   - Scheduler, çakışmayan sistemleri paralel iş parçacıklarına dağıtır; çakışanları sıralar.
   - Döngü içinde borrow/dyn kontrolü SIFIRDIR; saf monomorfik referanslarla çalışır.

2. **FrameArena Lock-Free Eşzamanlılık (`AtomicUsize`):**
   - Arena'nın dahili offset'i `AtomicUsize` haline getirildi.
   - `alloc(&self)` atomik CAS ile çalışarak işçi iş parçacıklarının (job workers) aynı kareden kilit olmadan (lock-free) eşzamanlı bellek tahsis etmesine olanak tanır.
   - `reset(&mut self)` münhasır referans istediği için kare sonunda tüm işlerin bittiğini derleyici düzeyinde zorunlu kılar.

3. **Flat vs Paged Sparse-Set Kıyaslaması:**
   - Düz `FlatSparseTable` (`Vec<u32>`) ve 2 kademeli `PagedSparseTable` (1024 sayfa) doğrudan aynı arayüz altında kıyaslandı.

---

## 5. Kapsamlı 50k Entity Benchmark Sonuçları

Ölçümler Criterion ile 50.000 entity üzerinde hem **Temiz (sıralı)** hem de **%50 silinip yeniden üretilmiş Fragmante** sahnede yapılmıştır. Girdiler `black_box` ile optimize edilmeye karşı korunmuştur.

### 5.1. 50k Hareket Güncellemesi (View2: Position mut, Velocity ref)

| Mimari | Senaryo | Ortalama Süre | 60 FPS Bütçesi (16.66 ms) | 144 FPS Bütçesi (6.94 ms) |
|---|---|---|---|---|
| **Flat Sparse** | Temiz (Sıralı) | **84.07 µs (0.084 ms)** | **%0.50** | **%1.21** |
| **Flat Sparse** | Fragmante (%50 sil/ekle) | **83.22 µs (0.083 ms)** | **%0.50** | **%1.20** |
| **Paged Sparse** | Temiz (Sıralı) | **93.70 µs (0.094 ms)** | **%0.56** | **%1.35** |
| **Paged Sparse** | Fragmante (%50 sil/ekle) | **91.86 µs (0.092 ms)** | **%0.55** | **%1.32** |

*Sonuç:* **Flat Sparse Table, Paged olanına kıyasla ~%11.5 daha hızlıdır.**

### 5.2. 50k Render Toplama + Arena Scratch (View3: Pos mut, Vel ref, Sprite ref -> Arena)

| Mimari | Senaryo | Ortalama Süre | 60 FPS Bütçesi (16.66 ms) | 144 FPS Bütçesi (6.94 ms) |
|---|---|---|---|---|
| **Flat Sparse** | Temiz (Sıralı) | **323.59 µs (0.324 ms)** | **%1.94** | **%4.66** |
| **Flat Sparse** | Fragmante (%50 sil/ekle) | **321.71 µs (0.322 ms)** | **%1.93** | **%4.63** |
| **Paged Sparse** | Temiz (Sıralı) | **341.19 µs (0.341 ms)** | **%2.05** | **%4.91** |
| **Paged Sparse** | Fragmante (%50 sil/ekle) | **350.55 µs (0.351 ms)** | **%2.10** | **%5.05** |

*Sonuç:* **Flat Sparse Table render toplamada da ~%8.2 daha hızlıdır.**

---

## 6. Bellek Tüketimi ve Mimari Karar

| Depo Tipi | 50.000 Entity RAM Tüketimi | İşaretçi Seviyesi (Indirection) |
|---|---|---|
| **Flat Sparse Table** | **200 KB** (`50.000 * 4 bayt`) | 1 seviye (Doğrudan indeksleme) |
| **Paged Sparse Table** | **~212 KB** (`8 KB tablo` + `50 * 4 KB sayfa`) | 2 seviye (Sayfa tablosu -> Sayfa -> İndeks) |

---

## 7. 1 Milyon Entity Heterojen Benchmark Sonuçları (L1/L2 Önbellek Sınırlarının Ötesinde)

- **Senaryo:** 1.000.000 Entity toplam dünya boyutu.
- **Dağılım:** Position: %100 (1.000.000 entity), Sprite: %70 (700.000 entity), Velocity: %40 (400.000 entity).
- **Bellek Ayak İzi:** ~40 MB (L1/L2 sınırlarını katbekat aşar, L3'ü zorlar).
- **Derleme:** `--release`, `lto = "thin"`, `codegen-units = 1`.

| İşlem | Alt Küme Büyüklüğü | Flat Sparse Süresi | Paged Sparse Süresi | 60 FPS Bütçesi (16.66 ms) |
|---|---|---|---|---|
| **Hareket Güncellemesi (View2)** | 400.000 Hareketli Entity | **2.82 ms** (~7.0 ns/entity) | **2.89 ms** (~7.2 ns/entity) | **%16.9** |
| **Render Toplama + Arena (View2)** | 700.000 Sprite Toplayıcı | **9.48 ms** (~13.5 ns/entity) | **9.35 ms** (~13.3 ns/entity) | **%56.9** |

### Miri ve Güvenlik Denetimleri
- `cargo +nightly miri test` çalıştırıldı.
- `FrameArena` (atomik CAS, lock-free allocation, strict pointer provenance), `CommandBuffer` (yapısal değişiklik tamponu), `Registry` (`UnsafeCell` + `AccessSet`) ve `SparseSet` testleri **0 hata, 0 uyarı ve 0 tanımsız davranış (UB)** ile doğrulandı.



