# Stage 10 — glTF 2.0 Yükleyici ve PBR Materyal Hattı Uygulama Planı

**Tarih:** 5 Kasım 2026  
**Aşama:** Katman B — Stage 10  
**Hedef:** Editörde ve 3D sahnede gerçek glTF / GLB modellerinin PBR (Cook-Torrance) materyal ve doku setleriyle hatasız render edilmesi.

---

## 1. Mimari Tasarım ve Bileşenler

### A. Varlık Katmanı (`rugeng_assets::gltf`)
- `gltf` 1.4 crate bağımlılığı eklenmesi (`rugeng_assets/Cargo.toml`).
- `GltfMesh`, `GltfSubmesh`, `GltfMaterial`, `GltfModel` veri yapıları:
  - Pozisyon (vec3), Normal (vec3), UV (vec2), Renk (vec4).
  - PBR Materyal Parametreleri:
    - `base_color_factor: [f32; 4]`
    - `roughness_factor: f32`
    - `metallic_factor: f32`
    - `base_color_texture: Option<String>` veya gömülü görüntü verisi
    - `metallic_roughness_texture: Option<String>` veya gömülü görüntü verisi
  - `parse_glb(bytes: &[u8]) -> Result<GltfModel, String>`: Gömülü binary GLB çözümü (minimum 1 üçgen ve çoklu primitive/submesh desteği).
  - `load_gltf_from_file(path: impl AsRef<Path>) -> Result<GltfModel, String>`: glTF JSON ve GLB ayrıştırma.

### B. Render ve Geometri Katmanı (`rugeng_render::mesh` & `pipeline_3d`)
- `Mesh3D` genişletmesi:
  - `material_index: Option<usize>`
  - `pbr_params: Option<[f32; 4]>` (roughness, metallic)
  - `submeshes: Vec<Submesh3D>` desteği.
- `mesh_3d.frag` PBR Shader güncellemesi:
  - Binding 0: `u_texture` (Albedo / Base Color)
  - Binding 1: `u_metallic_roughness` (G=roughness, B=metallic glTF 2.0 PBR standardı)
  - Binding 2: `u_normal_map` (Opsiyonel / Rezerve)
  - Binding 3: `u_emissive_map` (Opsiyonel / Rezerve)
  - Descriptor Set Layout genişletmesi (Bindings 0..4) ve fallback 1x1 beyaz/metalik placeholder dokular.
  - PushConstants pbr_flags / pbr_params ile dokulu ve dokusuz modeller arasında otomatik geçiş.

### C. Editör Sahne Entegrasyonu (`rugeng_editor::scene_ops` & `ui`)
- `import_gltf_scene(path: &str)`: Sahneye otomatik glTF kök düğümü ve submesh parçalarını `Transform3D` ve `Mesh3DComponent` kartları ile ekleme.
- `demo_3d` oyununa örnek GLB modeli entegrasyonu (`assets/models/sample.glb`).

---

## 2. Doğrulama ve Test Planı

1. `test_gltf_parse_minimal_glb`: Sentetik minimal 1-üçgen GLB ayrıştırma (3 vertex, 1 submesh).
2. `test_gltf_material_mapping`: Albedo factor, metallic-roughness factor ve doku bağlama testi.
3. `test_gltf_missing_texture_placeholder`: Eksik doku durumunda 1x1 fallback mekanizması testi.
4. `start.sh check` ve 2D regresyon kontrolü:
   - 153+ testin tamamı yeşil.
   - `render_bench`: 50k sprite < 5 ms.
   - `bullet_hell`: 20k mermi p99 < 8.33 ms.
