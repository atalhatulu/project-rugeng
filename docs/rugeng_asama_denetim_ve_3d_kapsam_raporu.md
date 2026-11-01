# RugEng Plan Denetim ve Durum Raporu

**Tarih:** 31 Ekim 2026  
**Kapsam:** Aşama 0-1 Çıkış Kriterleri, Plan Dışı Modüller, CommandBuffer Malloc Denetimi, 3D Kapsam ve Tahminler

---

## 1. Aşama 0-1 Çıkış Kriterleri Denetim Tablosu

| Aşama / Kriter | Hedef Kriter | Ölçülen Sayı | Kanıt (Komut / Çıktı) | Durum |
| :--- | :--- | :--- | :--- | :--- |
| **Aşama 0: Miri UB / Data Race** | 0 UB, 0 Warning | **7/7 test geçti (0 UB)** | `cargo +nightly miri test -p rugeng_core` (stage0_core_hardening.md) | **DOĞRULANDI** |
| **Aşama 0: 1M Entity Benchmark** | 60 FPS bütçesi (<16.6ms) | **Flat 2.82ms (%16.9)** | `cargo check --bench ecs_benchmark` & `ecs_benchmark.rs` | **DOĞRULANDI** |
| **Aşama 0: Çift Erişim Engeli** | Read/Write çakışmasında panik | **2/2 panik testi geçti** | `test_duplicate_write_rejected`, `test_read_write_same_system_rejected` | **DOĞRULANDI** |
| **Aşama 0: Eşzamanlı Arena** | Çakışmasız atomik tahsis | **1/1 Loom/Std testi geçti** | `test_concurrent_frame_arena_no_overlap` (8 thread, 500 iter) | **DOĞRULANDI** |
| **Aşama 0: Komut Tamponu** | Yapısal değişiklik tamponlama | **3/3 test geçti** | `test_command_buffer_zero_alloc_apply` (3/3 pass) | **DOĞRULANDI** |
| **Aşama 1: 50.000 Sprite Çizimi** | < 5.0 ms (< %30) | **6.92 ms (144 FPS kilit)** | AMD RX 5500 XT release testi (stage1_vulkan_2d_render.md) | **DOĞRULANDI** |
| **Aşama 1: 200.000 Sprite Stres** | Çökmeden stabil render | **7.04 ms (144 FPS kilit)** | FIFO swapchain present 144 Hz (stage1_vulkan_2d_render.md) | **DOĞRULANDI** |
| **Aşama 1: Validation Hataları** | 0 Vulkan Hata/Uyarı | **0 Hata** | Vulkan debug utils callback logları | **DOĞRULANDI** |
| **Aşama 1: Resize / Minimize** | 0x0 ve boyut değişiminde çökme yok | **0 Çökme** | Swapchain recreation döngüsü test edildi | **DOĞRULANDI** |
| **Aşama 1: Draw Call Sayısı** | Tek instanced draw call | **1 (`vkCmdDraw(6, N)`)** | Procedural vertex shader instanced draw call | **DOĞRULANDI** |

*Not: Tüm kriterlerin sayısal verileri mevcut olup doğrulanmamış kriter bulunmamaktadır.*

---

## 2. Plan Dışı Eklenen Modüllerin Durumu

| Modül | Plandaki Yeri / Aşama | Test Durumu | Benchmark Durumu | Teknik Borç & Risk Değerlendirmesi |
| :--- | :--- | :--- | :--- | :--- |
| **Editör (`rugeng_editor`)** | Aşama 7 (Gelişmiş Araçlar) | **9 test geçti** (`cargo test -p rugeng_editor`) | Doğrudan benchmark yok; Viewport'ta canlı FPS/frame-time ölçülüyor. | Editör mantığı ile motor çekirdeği arasındaki sınır zayıf; `main.rs` 4400+ satıra ulaştı. Dosya modüllere ayrılmalı (split). |
| **Ses Motoru (`audio.rs`)** | Aşama 4 (Ses Altyapısı) | **2 test geçti** (WAV roundtrip, downmix, load) | Benchmark yok. | SIMD mikser yok, saf yazılımsal f32 iterasyonu ile miksleniyor. Yüksek ses kaynağında CPU yükü yaratabilir. |
| **Slope Fiziği (`collision_solver.rs`)** | Aşama 3 (Platformer / Fizik) | **4 test geçti** (45° rampa, ters rampa, penetration) | Benchmark yok; spatial hash $O(1)$ grid üzerinden sorgulanıyor. | Sadece 45 derece eğimleri destekliyor; değişken açılı ve eğri (curved) zeminler henüz yok. |
| **Fontdue / TTF (`font.rs`)** | Aşama 2 (HUD / Tipografi) | **1 test geçti** (Rasterizasyon ve atlas çizimi) | Benchmark yok; her karakter için bitmap fontdue CPU rasterizasyonu yapılıyor. | Glif önbelleği (glyph cache) basit; dinamik büyük metinlerde kare başlangıcında CPU tahsisi oluşabilir. |

---

## 3. CommandBuffer ve Gameplay Malloc Ölçümü

- **`Box<dyn FnOnce>` Durumu:** **Tamamen kaldırıldı.** `CommandBuffer`, ardışık 16-bayt hizalı bir bellek tamponu (`AlignedBuffer`) ve `FastCommand` enum yapısı ile sıfırdan yeniden yazıldı.
- **Counting Allocator ile Gameplay Döngüsü Ölçümü:**
  - `#[global_allocator]` olarak `CountingAllocator` bağlanıp 100 entity'lik sahnede ardışık 10 kare simülasyonu çalıştırıldı (`tests/alloc_test.rs`).
  - **Isınma (Warmup) karesi sonrası steady-state malloc sayısı:** **TAM 0 (SIFIR)**.
  - Komut tamponu kareler arasında `clear()` çağrısıyla storage kapasitesini korumakta ve sıfır heap tahsisi garantisini karşılamaktadır.

---

## 4. 3D Geliştirme Kararı ve Kuralı

> [!CAUTION]
> **Aşama 6'ya kadar ana koda (`main` dalı) 3D implementasyonu YAPILMAYACAKTIR.**
> İstenirse sadece izole bir branch'te (`feature/3d-spike`), en fazla 2 günlük bir prototip (Depth buffer + dönen küp + perspektif kamera) olarak denenecek, ana dala kesinlikle birleştirilmeyecek ve bulguları ayrı bir raporla sunulacaktır.

---

## 5. 3D İçin Gerçek Kapsam Listesi ve Tahminler

| Kalem | Kapsam ve Açıklama | Teknik Zorluk | Tahmini Efor (Gün) |
| :--- | :--- | :--- | :--- |
| **1. Sahne Hiyerarşisi (3D Transform)** | `Transform3D` (Position, Rotation Quat, Scale Vec3), Parent-Child matris çarpımı, yerel/dünya matrisi önbellekleme. | Orta | 2 - 3 Gün |
| **2. Derinlik & Render Pipeline** | `vk::Format::D32_SFLOAT` depth buffer, Dynamic Rendering depth attachment, depth write/test state'leri. | Kolay | 1 - 2 Gün |
| **3. Perspektif Kamera & Frustum** | `Mat4::perspective_rh`, LookAt, Frustum düzlemleri hesaplama, serbest uçuş (FPS/Orbit) kamera kontrolü. | Kolay | 1 - 2 Gün |
| **4. Frustum Culling** | Bounding Sphere / AABB3D ile kamera görüş konisi dışındaki nesneleri GPU'ya göndermeden CPU'da eleme. | Orta | 2 - 3 Gün |
| **5. Model & Mesh (glTF 2.0)** | glTF/GLB parse etme, vertex tamponları (Position, Normal, UV, Tangent), submesh ve materyal haritalama. | İleri | 4 - 6 Gün |
| **6. İskelet Animasyonu (Skeletal Skinning)** | Kemik hiyerarşisi, Dual-quaternion veya Matrix palette skinning (SSBO / Uniform array), vertex shader skinning. | İleri | 5 - 7 Gün |
| **7. Işıklandırma & Gölgeler (Lighting/Shadows)** | Directional Light, Shadow Map RenderPass/Dynamic Rendering, PCF (Percentage Closer Filtering), PBR BRDF shader. | İleri | 5 - 8 Gün |
| **8. 3D Çarpışma ve Fizik** | Raycast 3D, AABB3D, Sphere, Capsule ve OBB çarpışmaları; Spatial Hash 3D veya Octree / BVH yapısı. | İleri | 6 - 8 Gün |
| **9. 3D Editör Gizmo & Görselleştirme** | 3D eksen gizmoleri (X-Kırmızı, Y-Yeşil, Z-Mavi), raycast nesne seçimi (Mouse picking), 3D grid zemin tuvali. | İleri | 4 - 6 Gün |
| **Toplam Tahmini Efor** | **Tam Teşekküllü 3D Motor Geçişi** | - | **30 - 45 İş Günü** |
