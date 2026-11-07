# Stage 11 Raporu: Gölge Haritası Entegrasyonu (DirectionalShadowMap + PCF 3x3)

**Tarih:** 7 Kasım 2026  
**Platform:** Linux (CachyOS x86_64) | AMD Radeon RX 5500 XT (RADV Vulkan 1.4)  
**Rust Versiyonu:** rustc 1.97.1  

---

## 1. Kapsam ve Amaç

Stage 11 kapsamında Project Rugeng motorunun 3D grafik hattına yönlü ışık gölgelendirmesi (Directional Shadow Mapping) entegre edilmiştir.

Bu kapsamda:
1. `crates/rugeng_render/src/shadow.rs` modülü ve `ShadowPipeline3D` oluşturuldu. Sadece derinlik yazan (`D32_SFLOAT`) renksiz grafik hattı, `depth_bias_constant_factor: 1.5`, `depth_bias_slope_factor: 2.5` donanımsal eğim duyarlılığı ve `FrontFace::CLOCKWISE` ile yapılandırıldı.
2. `DirectionalShadowMap` yardımcı metodları eklenerek Vulkan 1.3 `Synchronization2` ile `DEPTH_STENCIL_ATTACHMENT_OPTIMAL` ve `SHADER_READ_ONLY_OPTIMAL` durumları arasında güvenli geçişler (`pipeline_barrier2`) sağlandı.
3. `crates/rugeng_render/shaders/mesh_3d.vert` ve `mesh_3d.frag` gölgelendiricileri güncellendi:
   - Push Constants 256 bayta genişletilerek `mat4 light_space` matrisi eklendi.
   - Descriptor set layout'a Binding 2 olarak `u_shadow_map` (sampler2D) bağlandı.
   - 3x3 Percentage-Closer Filtering (PCF) ve dinamik eğim duyarlı bias (`slope-scaled bias`: `base_bias * (1.0 - cos_theta)`) uygulandı.
4. `crates/rugeng_editor/src/ui/inspector.rs` içerisine yönlü ışık ve gölge parametreleri (Direction, Res: 2048x2048, PCF 3x3, Slope Bias: 0.0025, D32_SFLOAT) bilgi kartı eklendi.
5. `games/demo_3d` içerisinde çoklu geçiş (Multi-pass: Shadow Pass -> Main Pass) mimarisi kuruldu, dönen küpler, küreler ve GLB modeli gölge veren nesne, geniş zemin ise gölge alıcı olarak çalıştırıldı.

---

## 2. Doğrulama ve Test Tablosu

| Bileşen / Test | Hedef | Sonuç | Durum |
| :--- | :--- | :--- | :--- |
| `test_light_space_matrix_ortho_bounds` | Işık uzayı ortografik matrisinin sahne AABB'sini kapsama testi | Tüm 8 AABB tepe noktası Vulkan NDC [-1, 1] ve Z [0, 1] sınırlarında kaldı | Başarılı (Yeşil) |
| `test_shadow_bias_no_peter_panning` | Sentetik düzlemde peter-panning ve shadow acne sınır denetimi | Eğim açısına göre bias aralığı [0.0005, 0.0025] sınırlarında korundu | Başarılı (Yeşil) |
| `demo_3d` Çalışma Zamanı Doğrulaması | 60 kare kesintisiz gölgeli render | 60/60 kare tamamlandı, 0 validation hatası | Başarılı (Yeşil) |
| `cargo test --workspace` | Tüm çalışma alanı birim/entegrasyon testleri | 160 testin tamamı başarılı (158'den +2 artış) | Başarılı (Yeşil) |
| `cargo clippy --workspace --all-targets` | Sıfır uyarı | 0 hata, 0 uyarı | Başarılı (Yeşil) |
| `cargo fmt --check` | Kod biçimlendirme doğrulaması | Temiz | Başarılı (Yeşil) |
| 2D Benchmark Regresyon Denetimi | 50.000 Sprite Toplam Süre < 5.0 ms | 50.000 Sprite: **0.94 ms** (p99: 4.77 ms) — 2D hattı sıfır regresyonla korundu | Başarılı (Yeşil) |

---

## 3. Mimari ve Uygulama Detayları

- **Sync2 Barrier Senkronizasyonu:**
  Gölge haritası geçişi öncesinde `UNDEFINED` -> `DEPTH_STENCIL_ATTACHMENT_OPTIMAL` bariyeri işletilir. Çizim tamamlandığında `cmd_end_rendering` ardından `DEPTH_STENCIL_ATTACHMENT_OPTIMAL` -> `SHADER_READ_ONLY_OPTIMAL` geçişi yapılarak ana sahnede `FRAGMENT_SHADER` okuması güvence altına alınır.
- **PCF Filtreleme Formülü:**
  ```glsl
  for (int x = -1; x <= 1; ++x) {
      for (int y = -1; y <= 1; ++y) {
          float pcf_depth = texture(u_shadow_map, proj_coords.xy + vec2(x, y) * texel_size).r;
          shadow += (current_depth - bias > pcf_depth) ? 0.0 : 1.0;
      }
  }
  shadow /= 9.0;
  ```
- **Performans:**
  2048x2048 çözünürlükteki D32_SFLOAT gölge haritası donanımsal derinlik bias'ı ve PCF ile modern GPU üzerinde ek yük oluşturmadan sabit 60+ FPS ile çalışmaktadır.

---

## 4. Sonraki Aşama (Stage 12)

- **Stage 12: 3D Fizik ve Seçim (3D Collider + Hareketli Gövde Çözücüsü + Ray Picking)**:
  - `AABB3D`, `OBB`, `Capsule`, `resolve_aabb3d_collision`, `resolve_sphere3d_collision`.
  - 3D spatial acceleration structure.
  - Editör 3D Viewport Ray Picking derinleştirmesi.
