# Stage 13 Uygulama Planı: İskelet Animasyonu (glTF Animation -> Animation3D)

## 1. Hedef ve Kapsam
glTF 2.0 modellerinde bulunan iskelet (skin/joint) ve animasyon kanallarını (translation, rotation, scale) ayrıştırıp, ECS içinde `Animation3DComponent` ve `Skeleton` ile canlandırmak; CPU/GPU skinning matris paletini oluşturup vertex skinning'i shader ve pipeline hattına entegre etmek.

## 2. Mimari Bileşenler

### A. `rugeng_core::animation_3d`
- `Joint`: name, parent_index, local_transform (pos, rot, scale), inverse_bind_matrix.
- `Skeleton`: joints vektörü, `update_global_transforms()`, `compute_skinning_palette() -> [Mat4; MAX_JOINTS]`.
- `AnimationChannel3D`: target_joint, target_property (Translation, Rotation, Scale), timestamps, values, interpolation (Linear, Step).
- `AnimationClip3D`: name, duration, channels, `sample(time, &mut Skeleton)`.
- `Animation3DComponent`: skeleton, clips, current_clip, elapsed_time, playback_speed, is_playing, current_palette: `[Mat4; 32]`.

### B. `rugeng_assets::gltf` Genişletmesi
- glTF Document'ından skins ve animations ayrıştırma:
  - `Skin`: inverseBindMatrices accessor'ı okuma, joints node indeksleri.
  - `Animation`: Sampler input (timestamps) ve output (vec3 translation/scale, vec4 quaternion rotation).
  - Meş primitiflerinden `JOINTS_0` (u16x4 veya u8x4 -> [u16; 4]) ve `WEIGHTS_0` (vec4 f32) okuma.

### C. `rugeng_render` Skinning Pipeline ve Vertex3D
- `Vertex3D`:
  - Mevcut 48 bayt: pos(12), normal(12), uv(8), color(16).
  - Skinning için ek alanlar veya CPU skinning / GPU skinning mimarisi.
  - GpuMesh3D üzerinde dinamik skinning buffer veya vertex shader'da 32 kemik matris paleti (Uniform buffer veya Descriptor binding 3 SSBO/UBO).
  - Minimum overhead ve geriye dönük 2D/3D uyumu için `skin_indices` & `skin_weights` desteği.

## 3. Test ve Doğrulama
1. `test_skeleton_bone_hierarchy_world_transform`: Hiyerarşik ebeveyn-çocuk kemik dönüşüm çarpımı testi.
2. `test_animation_keyframe_interpolation_linear`: Zaman damgaları arasında yumuşak doğrusal (lerp / slerp) enterpolasyon testi.
3. `test_skinning_palette_matrix_correctness`: Bind pose matris çarpımlarının kimlik (identity) matris vermesi testi.
4. `test_gltf_animation_and_skin_parsing`: glTF dosyasından iskelet ve animasyon kanallarının doğru okunması.
