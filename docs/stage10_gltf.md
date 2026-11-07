# Stage 10 Raporu: glTF 2.0 Yükleyici ve PBR Materyal Hattı Entegrasyonu

**Tarih:** 7 Kasım 2026  
**Platform:** Linux (CachyOS x86_64) | AMD Radeon RX 5500 XT (RADV Vulkan 1.4)  
**Rust Versiyonu:** rustc 1.97.1  

---

## 1. Kapsam ve Amaç

Stage 10 kapsamında Project Rugeng motoruna endüstri standardı glTF 2.0 (özellikle GLB binary formatı) model yükleme ve PBR (Physically-Based Rendering) materyal eşleme desteği entegre edilmiştir.

Bu kapsamda:
1. `rugeng_assets::gltf` modülü oluşturularak glTF 2.0 dosya formatındaki geometri (vertex pozisyonları, normaller, doku koordinatları/UV, tepe rengi) ve alt ağaç (submesh) yapısı ayrıştırılmıştır.
2. PBR materyal tanımları (baseColorFactor, metallicFactor, roughnessFactor, baseColorTexture, metallicRoughnessTexture) motorun varlık sistemine aktarılmıştır.
3. `rugeng_render` içerisindeki `Mesh3D` ve `pipeline_3d.rs` PBR gölgelendiricisine (`mesh_3d.frag`) 2. doku bağlama noktası (descriptor binding 1: `u_metallic_roughness`) eklenmiş, glTF 2.0 standardına uygun şekilde Yeşil kanal Roughness, Mavi kanal Metallic olacak şekilde örneklenmiştir.
4. `rugeng_editor` sahne ağacına glTF import mekanizması (`import_gltf_scene` ve üst menü `[M] glTF 3D Model (GLB)`) eklenmiştir.
5. `games/demo_3d` örnek 1-üçgenlik PBR GLB modeli (`assets/models/sample.glb`) ile dogfood edilmiştir.

---

## 2. Doğrulama ve Test Tablosu

| Bileşen / Test | Hedef | Sonuç | Durum |
| :--- | :--- | :--- | :--- |
| `test_gltf_parse_minimal_glb` | 1 üçgen GLB modelinden 3 vertex ve 1 submesh çıkarma | 3 tepe noktası, pozisyon ve normaller eksiksiz ayrıştırıldı | Başarılı (Yeşil) |
| `test_gltf_material_mapping` | PBR faktörleri ve doku haritalama | BaseColor: (0.8, 0.2, 0.2), metallic: 0.7, roughness: 0.3 doğrulandı | Başarılı (Yeşil) |
| `test_import_gltf_scene_and_pbr_mapping` | Editör sahnesine glTF hiyerarşisi aktarımı | Mesh3D bileşeni, materyal pbr_factors ve Transform3D ile sahnede oluşturuldu | Başarılı (Yeşil) |
| `cargo test --workspace` | Tüm testlerin kırılmadan geçmesi | 157 testin tamamı başarılı (önceki 153'ten +4 artış) | Başarılı (Yeşil) |
| `cargo clippy --workspace --all-targets` | Sıfır uyarı | 0 hata, 0 uyarı | Başarılı (Yeşil) |
| `cargo fmt --check` | Kod biçimlendirme doğrulaması | Temiz | Başarılı (Yeşil) |
| 2D Benchmark Regresyon Denetimi | 50k sprite < 5ms, bullet hell < 8.33ms | 50k sprite: 1.01ms, bullet hell p99: 3.88ms (2D hattı tamamen korundu) | Başarılı (Yeşil) |

---

## 3. Mimari ve Uygulama Detayları

- **`rugeng_assets::gltf`:**
  - `GltfModel`, `GltfSubmesh`, `GltfMaterial`, `GltfTexture`, `GltfVertex` veri yapıları tanımlandı.
  - `parse_glb(&[u8])` ve `from_file(path)` fonksiyonları ile buffer ve dosya üzerinden ayrıştırma sağlandı.
  - Doku eksik olduğunda veya gömülü doku bulunmadığında 1x1 magenta/beyaz placeholder doku üretimi eklendi.
- **`rugeng_render` Shader & Pipeline:**
  - `mesh_3d.frag` içine `layout(set = 0, binding = 1) uniform sampler2D u_metallic_roughness;` eklendi.
  - Push constant `pbr_params.z` alanı glTF materyali metalik-pürüzlülük dokusu içeriyorsa doku örneklemeyi açan bayrak (`has_mr_texture`) olarak kullanıldı.
  - Push constant boyutu 208 bayt olarak korundu.
  - SPIR-V derlemesi `glslangValidator` ile yapılarak ikili dosya güncellendi.
- **Dogfooding:**
  - `games/demo_3d/src/main.rs` içinde GLB model yüklenip PBR materyal ile küpün yanına yerleştirilerek çalışma zamanı doğrulaması yapıldı.

---

## 4. Bilinen Eksikler ve Sonraki Aşama (Stage 11)

- GLB modelleri için şimdilik tek alt ağaç (submesh 0) varsayılan çizim hedeflenmektedir; karmaşık sahne düğüm hiyerarşisi (Node Graph transform inheritance) Stage 13'te ele alınacaktır.
- Sırada **Stage 11: Gölge Haritası Entegrasyonu (DirectionalShadowMap + PCF 3x3 filtering)** bulunmaktadır.
