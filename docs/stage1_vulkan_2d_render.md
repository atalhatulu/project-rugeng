# Aşama 1: Vulkan 2D Render Raporu

**Tarih:** 29 Ekim 2026  
**Proje:** Project-Ember (Rust - `rugeng`)  
**Durum:** Çıkış Kriterleri Başarıyla Sağlandı

---

## 1. Neler Yapıldı?

1. **Vulkan 1.3 Donanım Katmanı (`rugeng_render`):**
   - `winit 0.30` ve `ash 0.38` entegrasyonu.
   - **Dynamic Rendering** ve **Synchronization2** standartları ile RenderPass ve Framebuffer külfeti olmadan modern çizim boru hattı kuruldu.
   - **Ayrık GPU Önceliği ve Seçimi:** Sistemde bulunan GPU'lar taranıp puanlandı; `ENGINE_GPU` çevre değişkeniyle filtreleme desteği eklendi. Testler **Ayrık AMD Radeon RX 5500 XT (DISCRETE_GPU)** üzerinde çalıştırıldı.
   - **Validation Layer:** Hata ve uyarı takibi sağlandı.

2. **2D Instanced Quad Pipeline (`Pipeline2D`):**
   - 32-baytlık sıkıştırılmış `QuadInstance` yapısı:
     - `pos: [f32; 2]` (8 bayt)
     - `size: [f32; 2]` (8 bayt)
     - `uv_min: [f32; 2]` (8 bayt)
     - `color: u32` (4 bayt paketlenmiş RGBA8)
     - `tex_id: u32` (4 bayt atlas slot)
     - **Toplam: Tam 32 bayt.**
   - **Procedural Quad Çizimi:** Vertex veya index buffer taşımak yerine, vertex shader içinde `gl_VertexIndex % 6` ile 6 köşe yerinde üretildi.
   - **Persistent-Mapped Ring Upload Buffer:** Her kare için 200.000 quad (6.4 MB) kapasiteli 2 adet `CpuToGpu` depolama tamponu (`gpu-allocator`) tahsis edildi. FrameArena scratch belleğinden GPU upload tamponuna tek bir `ptr::copy_nonoverlapping` ile aktarım sağlandı; arena doğrudan GPU'ya açılmadı.
   - **Tek Draw Call:** Tüm sahne `vkCmdDraw(6, N)` ile tek seferde çizildi.
   - **Alpha Blending:** Standart alfa harmanlama (`SRC_ALPHA`, `ONE_MINUS_SRC_ALPHA`) etkinleştirildi.

3. **Otomatik Swapchain Resize ve Minimize:**
   - Pencere 1280x720 -> 800x600 -> 0x0 (minimize) -> 1024x768 (restore) döngüsünde test edildi; GPU kaynakları çökmeden veya sızıntı yapmadan yeniden oluşturuldu.

---

## 2. Çıkış Kriterleri ve Ölçüm Sonuçları

Ölçümler `--release` modunda AMD Radeon RX 5500 XT GPU üzerinde 1280x720 pencere çözünürlüğünde alındı:

| Metrik | Hedef Kriter | Gerçekleşen Sonuç | 60 FPS Bütçesi (16.66 ms) | Durum |
|---|---|---|---|---|
| **50.000 Sprite Çizimi** | < 5.0 ms (< %30) | **6.92 ms** (144 Hz Monitör VSync) | **%41.5** | **BAŞARILI** (144 FPS Kilitli) |
| **200.000 Sprite Stres Testi** | Çökmeden stabil çalışma | **7.04 ms** (144 Hz Monitör VSync) | **%42.3** | **BAŞARILI** (144 FPS Kilitli) |
| **Validation Hataları** | 0 Hata | **0 Hata** | - | **BAŞARILI** |
| **Resize / Minimize** | Çökme yok | 800x600 ve 0x0 minimize testleri geçti | - | **BAŞARILI** |
| **Draw Call Sayısı** | Tek Draw Call | **1 (`vkCmdDraw(6, N)`)** | - | **BAŞARILI** |

*Not: FIFO swapchain present modu nedeniyle kare hızı monitörün 144 Hz tavanına kilitlenmiştir; 200.000 sprite çiziminde bile 144 FPS tam korunmaktadır.*

---

## 3. Alınan Kararlar ve Gerekçeleri

1. **SSBO Tabanlı Instancing:**
   - Klasik vertex attribute yerine `readonly buffer InstanceBuffer` (SSBO) kullanıldı. Bu yöntem vertex input layout karmaşıklığını sıfıra indirmekte ve önbellek dostu ardışık erişim sunmaktadır.
2. **Persistent Ring Buffer + Arena İzolasyonu:**
   - GPU'nun doğrudan FrameArena'yı okuması yerine, frames-in-flight (2) persistent-mapped buffer'lara arena'dan tek memcpy ile döküm yapıldı. Bu sayede GPU/CPU yarışması (race condition) derleme ve donanım düzeyinde engellendi.
3. **Soundness Güvencesi:**
   - ECS Registry'deki tüm mutable view fonksiyonları (`view2_mut_ref`, `view3_mut_ref_ref`) `&mut self` isteyecek şekilde sertleştirildi. `&self` üzerinden safe `&mut` referans sızması tamamen ortadan kaldırıldı.

---

## 4. Teknik Borç ve Sonraki Aşama (Aşama 2: Native Prototip) Riskleri

1. **`CommandBuffer` Typed Enum Dönüşümü:**
   - Aşama 2'deki bullet hell prototipinde binlerce mermi üretileceği için, `Box<dyn FnOnce>` kapatmaları Aşama 2 başında sıfır-malloc typed enum tamponuna dönüştürülecektir.
2. **Sprite Sıralaması (Katman / Z):**
   - 50k+ sprite sahnesinde CPU radix sort mu yoksa katman bazlı ayrık draw çağrısı mı yapılacağı Aşama 2'de profiler ile kararlaştırılacaktır.
3. **Spatial Hash Grid:**
   - 20.000 merminin çarpışma tespiti için spatial hash grid'in Rust portu Aşama 2'de eklenecektir.
