# Stage 11 Planı: Gölge Haritası Entegrasyonu (DirectionalShadowMap + PCF 3x3)

## 1. Amaç ve Kapsam
Project-Ember 3D render hattına yönlü ışık gölgelendirmesi (Directional Shadow Mapping) eklemek.
- Işık uzayı matrisi (`light_space_matrix`) hesaplaması ve sahne AABB kapsama doğrulaması.
- Gölge derinlik geçişi için özel gölge pipeline'ı (`ShadowPipeline3D` - sadece derinlik yazan, renksiz shader).
- `mesh_3d.frag` shader'ına gölge haritası dokusu bağlama (Binding 2: `sampler2DShadow` veya `sampler2D`), PCF (3x3 Percentage-Closer Filtering) ve bias formülü entegrasyonu.
- Peter-panning ve shadow acne'yi önleyen eğim duyarlı bias (`slope scale bias`).
- Editör Denetçisinde (`Inspector`) gölge kontrolleri (ışık açısı, bias çarpanı).
- `demo_3d`'de gölge veren nesneler ve gölge alan zemin ile dogfooding.

## 2. Mimari Değişiklikler
1. `crates/rugeng_render/src/shadow.rs`:
   - `ShadowPipeline3D` yapısı (`mesh_3d.vert` ile sadece vertex dönüşümü ve derinlik eki `D32_SFLOAT`).
   - `DirectionalShadowMap` yardımcı metodları (layout geçişi `transition_to_shader_read`, `transition_to_depth_attachment`).
2. `crates/rugeng_render/shaders/mesh_3d.frag`:
   - `layout(set = 0, binding = 2) uniform sampler2D u_shadow_map;`
   - `in vec4 in_light_space_pos;`
   - PCF 3x3 örnekleme fonksiyonu ve gölge çarpanı (`shadow_factor`).
3. `crates/rugeng_render/shaders/mesh_3d.vert`:
   - Push constant içine `mat4 light_space_matrix` veya ayrı matris parametresi aktarımı.
   - `out vec4 out_light_space_pos;`
4. `crates/rugeng_render/src/pipeline_3d.rs`:
   - Descriptor set layout'a Binding 2 eklenmesi.
   - Shadow sampler oluşturulması (Vulkan `VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_BORDER`, border color float opaque white `1.0`).
   - `draw_mesh_pbr_viewport_ext` ve `draw_mesh` fonksiyonlarının ışık matrisi ve gölge haritası ile uyumlu hale getirilmesi.
5. `crates/rugeng_editor/src/ui/inspector.rs`:
   - Directional light ayarları (Pitch, Yaw, Bias, Intensity).
6. `games/demo_3d/src/main.rs`:
   - Render döngüsüne `Shadow Pass` eklenmesi; önce gölge haritasına çizim, ardından ana sahne çizimi.

## 3. Test ve Doğrulama
- `test_light_space_matrix_ortho_bounds`: Işık uzayı matrisi sahne AABB'sini kapsıyor.
- `test_shadow_bias_no_peter_panning`: Sentetik düzlem testi ile bias sınırları.
- `./start.sh check`: Tüm testler yeşil, 0 clippy uyarısı, fmt temiz.
- 2D Benchmark: Bullet hell ve 50k sprite süreleri etkilenmedi.
