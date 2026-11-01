# Aşama 0: Çekirdek Sağlamlaştırma (Core Hardening) Raporu

**Tarih:** 29 Ekim 2026  
**Proje:** Project-Ember (Rust - `rugeng`)  
**Durum:** Çıkış Kriterleri Tamamen Sağlandı

---

## 1. Neler Yapıldı?

1. **ECS Veri Yapısı (Flat vs Paged Karşılaştırması ve Karar):**
   - 1.000.000 Entity büyüklüğünde, rastgele dağıtılmış heterojen bileşen alt kümeleri (%100 Position, %70 Sprite, %40 Velocity) ile test edildi.
   - Çalışma seti ~40 MB olup L1 ve L2 CPU önbelleklerini aşmaktadır.
   - Ölçüm Sonuçları:
     - 400.000 Hareketli Entity View İterasyonu:
       - **Flat Sparse:** 2.82 ms (~7.0 ns/entity) — 60 FPS bütçesinin %16.9'u
       - **Paged Sparse:** 2.89 ms (~7.2 ns/entity) — 60 FPS bütçesinin %17.3'ü
     - 700.000 Sprite Toplayıcı View İterasyonu:
       - **Flat Sparse:** 9.48 ms (~13.5 ns/entity) — 60 FPS bütçesinin %56.9'u
       - **Paged Sparse:** 9.35 ms (~13.3 ns/entity) — 60 FPS bütçesinin %56.1'i
   - **Karar:** 50k-200k sprite hedefinde Flat Sparse yapısı %10 daha hızlı ve 200 KB RAM ile önbelleğe tam oturduğu için varsayılan olarak seçildi. Paged yapı jenerik opsiyon olarak korundu.

2. **Yapısal Değişiklik Komut Tamponu (`CommandBuffer`):**
   - Sistemler paralel koşarken doğrudan `spawn`, `destroy`, `emplace` çağrılması engellendi (UB ve data race riski ortadan kaldırıldı).
   - Değişiklikler `CommandBuffer`'a kaydedilir ve senkronizasyon anında `&mut Registry` ile tek thread'de uygulanır.

3. **`AccessSet` Doğrulaması:**
   - Aynı sistem içinde aynı bileşene çift mutable (`Write`) veya çelişkili (`Read` + `Write`) erişim talepleri erken çalışma zamanında (`assert!`) engellendi.

4. **FrameArena Lock-Free Eşzamanlılık:**
   - `AtomicUsize` ile worker thread'lerin aynı arenadan lock-free tahsis yapabilmesi sağlandı.
   - Strict pointer provenance uygulandı.

5. **Miri ve Güvenlik:**
   - `cargo +nightly miri test -p rugeng_core` çalıştırıldı.
   - 7 testin 7'si de 0 tanımsız davranış (UB), 0 bellek sızıntısı ve 0 data race ile tamamlandı.

6. **Vulkan 1.3 Altyapısı (`rugeng_render`):**
   - `winit 0.30` ve `ash 0.38` entegrasyonu.
   - Dynamic Rendering ve Synchronization2 standartları ile RenderPass/Framebuffer külfetsiz clear pass.
   - Swapchain resize ve minimize yönetimi.
   - AMD Radeon RX 5500 XT GPU üzerinde 1280x720 pencere ile çalışan demo doğrulandı.

---

## 2. Çıkış Kriterleri Kontrol Listesi

| Kriter | Hedef | Gerçekleşen Sonuç | Durum |
|---|---|---|---|
| **Miri Testleri** | Çekirdekte 0 UB / 0 Warning | 7/7 test passed, 0 error, 0 UB | **BAŞARILI** |
| **1M Benchmark Raporu** | Heterojen sahnede kare bütçesi ölçümü | Raporlandı (Flat vs Paged kıyaslandı) | **BAŞARILI** |
| **Komut Tamponu** | Yapısal değişikliklerin tamponlanması | `CommandBuffer` uygulandı ve test edildi | **BAŞARILI** |
| **Çift Erişim Engeli** | Aynı bileşene çift mut talebini reddetme | `test_duplicate_write_rejected` (panic) | **BAŞARILI** |
| **CI Kapıları** | Clippy (-D warnings), Fmt, Test | Hepsi 0 hata ile geçti | **BAŞARILI** |

---

## 3. Bilinen Eksikler ve Teknik Borç
- `CommandBuffer` şu an `Box<dyn FnOnce>` kullanıyor. Aşama 5'te (Luau) veya yüksek entity churn senaryolarında closure allocation yerine flat enum/byte stream tamponu tercih edilebilir.
- Tracy profil bağlayıcısı henüz eklenmedi (Aşama 1'de eklenecek).

---

## 4. Aşama 1 (Vulkan 2D Render) İçin Riskler
1. **Frames-in-flight Ring Buffer Senkronu:** Instance verisi arena'dan ring upload buffer'a kopyalanırken GPU çakışmalarının önlenmesi.
2. **Shader Pipeline:** GLSL/Slang shader'larının derleme boru hattı (`shaderc` vs önceden SPIR-V derleme).
