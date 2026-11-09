# Stage 12 Planı: 3D Fizik, Çarpışma Çözücüsü ve Seçim (Collision 3D + Spatial Hash 3D + Ray Picking)

## 1. Amaç ve Kapsam
Project-Ember `rugeng_physics` kütüphanesine 3D collider, çarpışma algılama ve itki/penetrasyon çözümü eklemek; editör ve demo üzerinde 3D ray picking ile dogfooding yapmak.

### Kapsam:
1. `crates/rugeng_physics/src/collision_3d.rs`:
   - `OBB3D` (Oriented Bounding Box: center, half_extents, rotation/axes).
   - `Capsule3D` (segment start/end veya center, half_height, radius).
   - `resolve_aabb3d_collision(a_min, a_max, b_min, b_max)` -> Minimum Translation Vector (MTV) ve temas normali.
   - `resolve_sphere3d_collision(center_a, radius_a, center_b, radius_b)` -> İtki ve ayrıştırma vektörü.
   - `CollisionManifold3D` (normal, penetration, contact_point).
2. `crates/rugeng_physics/src/spatial_hash_3d.rs`:
   - `SpatialHash3D`: Geniş hücre boyutlu 3D geniş faz (broadphase) hızlandırma yapısı.
   - Yarıçap duyarlı ekleme (radius / half_extents kapsayan tüm 3D hücrelere ekleme ve çoklu sorgu tekilleştirme).
3. `crates/rugeng_editor/src/input.rs`:
   - Viewport 3D tıklamasında sahnedeki tüm 3D varlıkları `ray_intersect_aabb_3d` ve `ray_intersect_sphere` ile test edip en yakın mesafedekini seçme (`nearest_t`).
4. `games/demo_3d`:
   - Sahneye hareketli veya çarpışan 3D küp/küre ekleyerek `resolve_aabb3d_collision` ve `resolve_sphere3d_collision` ile dogfood etme.
5. Benchmark & Testler:
   - `test_resolve_aabb3d_penetration_minimum_translation`
   - `test_sphere_sphere_3d_response`
   - `test_ray_pick_nearest_object`
   - `test_spatial_hash_3d_cell_boundary`
   - 10k 3D nesne broadphase benchmark'ı (`benches/spatial_hash_3d_bench.rs` veya Criterion).

## 2. Riskler ve Önlemler
- **2D Fizik Bozulmayacak:** 2D spatial hash ve slope solver testleri (20 test) tamamen korunacak.
- **Determinizm ve Miri:** Tüm matematik `glam::Vec3` ve safe Rust üzerinde kayan nokta hassasiyetine dikkat edilerek yazılacak.
