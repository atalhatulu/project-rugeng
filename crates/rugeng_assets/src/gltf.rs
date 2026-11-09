//! glTF 2.0 / GLB Model ve PBR Materyal Ayrıştırıcısı.
//!
//! Bu modül, standart glTF 2.0 ve binary GLB dosyalarından
//! geometrileri (Pozisyon, Normal, UV, Renk) ve Cook-Torrance PBR
//! materyal parametrelerini (BaseColor, Metallic-Roughness) ayrıştırır.

use std::path::Path;

/// glTF Köşe Verisi (Pozisyon, Normal, UV, Renk ve İskelet Ciltleme İndeks/Ağırlıkları)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GltfVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}

impl Default for GltfVertex {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            color: [1.0, 1.0, 1.0, 1.0],
            joints: [0; 4],
            weights: [1.0, 0.0, 0.0, 0.0],
        }
    }
}

/// glTF PBR Materyal Bilgisi
#[derive(Clone, Debug, PartialEq)]
pub struct GltfMaterial {
    pub name: String,
    pub base_color_factor: [f32; 4],
    pub roughness_factor: f32,
    pub metallic_factor: f32,
    pub base_color_texture_index: Option<usize>,
    pub metallic_roughness_texture_index: Option<usize>,
}

impl Default for GltfMaterial {
    fn default() -> Self {
        Self {
            name: "DefaultMaterial".to_string(),
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            roughness_factor: 0.5,
            metallic_factor: 0.0,
            base_color_texture_index: None,
            metallic_roughness_texture_index: None,
        }
    }
}

/// Alt Ağ (Submesh / Primitive)
#[derive(Clone, Debug, Default)]
pub struct GltfSubmesh {
    pub name: String,
    pub vertices: Vec<GltfVertex>,
    pub indices: Vec<u32>,
    pub material_index: Option<usize>,
}

impl GltfSubmesh {
    pub fn joint_indices(&self) -> Vec<[u16; 4]> {
        self.vertices.iter().map(|v| v.joints).collect()
    }

    pub fn joint_weights(&self) -> Vec<[f32; 4]> {
        self.vertices.iter().map(|v| v.weights).collect()
    }
}

/// Çözümlenmiş Doku Verisi (Gömülü veya ayrık)
#[derive(Clone, Debug)]
pub struct GltfTexture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA8
}

/// glTF Düğüm Hiyerarşisi (Node Graph)
#[derive(Clone, Debug)]
pub struct GltfNode {
    pub name: String,
    pub translation: [f32; 3],
    pub rotation: [f32; 4], // xyzw
    pub scale: [f32; 3],
    pub mesh_index: Option<usize>,
    pub skin_index: Option<usize>,
    pub children: Vec<GltfNode>,
}

/// Tam glTF Modeli
#[derive(Clone, Debug, Default)]
pub struct GltfModel {
    pub name: String,
    pub submeshes: Vec<GltfSubmesh>,
    pub materials: Vec<GltfMaterial>,
    pub textures: Vec<GltfTexture>,
    pub skeleton: Option<rugeng_core::animation_3d::Skeleton>,
    pub animations: Vec<rugeng_core::animation_3d::AnimationClip3D>,
    pub root_nodes: Vec<GltfNode>,
}

impl GltfModel {
    /// Ham glTF/GLB bayt dizisinden modeli yükler.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, String> {
        let (document, buffers, images) =
            gltf::import_slice(bytes).map_err(|e| format!("glTF dilim cozumlenemedi: {e}"))?;
        Self::build_model(&document, &buffers, &images, "SliceModel")
    }

    /// Dosya yolundan glTF/GLB modelini yükler.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let path_ref = path.as_ref();
        let (document, buffers, images) = gltf::import(path_ref)
            .map_err(|e| format!("glTF dosyasi cozumlenemedi ({}): {e}", path_ref.display()))?;
        let name = path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Model");
        Self::build_model(&document, &buffers, &images, name)
    }

    fn build_model(
        document: &gltf::Document,
        buffers: &[gltf::buffer::Data],
        images: &[gltf::image::Data],
        default_name: &str,
    ) -> Result<Self, String> {
        let mut model = Self {
            name: default_name.to_string(),
            submeshes: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            skeleton: None,
            animations: Vec::new(),
            root_nodes: Vec::new(),
        };

        // 1. Dokuları çıkar
        for (i, img) in images.iter().enumerate() {
            let rgba_pixels = match img.format {
                gltf::image::Format::R8G8B8A8 => img.pixels.clone(),
                gltf::image::Format::R8G8B8 => {
                    let mut rgba = Vec::with_capacity((img.width * img.height * 4) as usize);
                    for chunk in img.pixels.chunks_exact(3) {
                        rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                    }
                    rgba
                }
                _ => {
                    // Fallback: 1x1 beyaz piksel
                    vec![255, 255, 255, 255]
                }
            };
            model.textures.push(GltfTexture {
                name: format!("texture_{i}"),
                width: img.width,
                height: img.height,
                pixels: rgba_pixels,
            });
        }

        // 2. Materyalleri çıkar
        for mat in document.materials() {
            let pbr = mat.pbr_metallic_roughness();
            let base_color_factor = pbr.base_color_factor();
            let roughness_factor = pbr.roughness_factor();
            let metallic_factor = pbr.metallic_factor();

            let base_color_texture_index = pbr.base_color_texture().map(|t| t.texture().index());
            let metallic_roughness_texture_index = pbr
                .metallic_roughness_texture()
                .map(|t| t.texture().index());

            model.materials.push(GltfMaterial {
                name: mat.name().unwrap_or("Material").to_string(),
                base_color_factor,
                roughness_factor,
                metallic_factor,
                base_color_texture_index,
                metallic_roughness_texture_index,
            });
        }

        // 3. Meşleri ve primitifleri (submesh) çıkar
        for mesh in document.meshes() {
            for (prim_idx, primitive) in mesh.primitives().enumerate() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                let positions: Vec<[f32; 3]> = reader
                    .read_positions()
                    .map(|iter| iter.collect())
                    .unwrap_or_default();

                if positions.is_empty() {
                    continue;
                }

                let normals: Vec<[f32; 3]> = reader
                    .read_normals()
                    .map(|iter| iter.collect())
                    .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);

                let uvs: Vec<[f32; 2]> = reader
                    .read_tex_coords(0)
                    .map(|iter| iter.into_f32().collect())
                    .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

                let colors: Vec<[f32; 4]> = reader
                    .read_colors(0)
                    .map(|iter| iter.into_rgba_f32().collect())
                    .unwrap_or_else(|| vec![[1.0, 1.0, 1.0, 1.0]; positions.len()]);

                let joints: Vec<[u16; 4]> = reader
                    .read_joints(0)
                    .map(|iter| iter.into_u16().collect())
                    .unwrap_or_else(|| vec![[0; 4]; positions.len()]);

                let weights: Vec<[f32; 4]> = reader
                    .read_weights(0)
                    .map(|iter| iter.into_f32().collect())
                    .unwrap_or_else(|| vec![[1.0, 0.0, 0.0, 0.0]; positions.len()]);

                let mut vertices = Vec::with_capacity(positions.len());
                for (i, &pos) in positions.iter().enumerate() {
                    vertices.push(GltfVertex {
                        position: pos,
                        normal: normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
                        uv: uvs.get(i).copied().unwrap_or([0.0, 0.0]),
                        color: colors.get(i).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]),
                        joints: joints.get(i).copied().unwrap_or([0; 4]),
                        weights: weights.get(i).copied().unwrap_or([1.0, 0.0, 0.0, 0.0]),
                    });
                }

                let indices: Vec<u32> = reader
                    .read_indices()
                    .map(|iter| iter.into_u32().collect())
                    .unwrap_or_else(|| (0..positions.len() as u32).collect());

                let submesh_name = format!("{}_{}", mesh.name().unwrap_or("Mesh"), prim_idx);

                model.submeshes.push(GltfSubmesh {
                    name: submesh_name,
                    vertices,
                    indices,
                    material_index: primitive.material().index(),
                });
            }
        }

        // 4. İskelet (Skin / Joints) Ayrıştırma
        if let Some(skin) = document.skins().next() {
            let reader = skin.reader(|buffer| Some(&buffers[buffer.index()]));
            let inverse_bind_matrices: Vec<glam::Mat4> = reader
                .read_inverse_bind_matrices()
                .map(|iter| {
                    iter.map(|cols| glam::Mat4::from_cols_array_2d(&cols))
                        .collect()
                })
                .unwrap_or_default();

            let mut joints = Vec::new();
            let joint_nodes: Vec<gltf::Node> = skin.joints().collect();

            for (idx, joint_node) in joint_nodes.iter().enumerate() {
                let name = joint_node.name().unwrap_or("Joint").to_string();
                let (trans, rot, scale) = joint_node.transform().decomposed();

                // Ebeveyn düğümün skin.joints() içindeki indeksini tespit et
                let mut parent_idx = None;
                for (p_idx, candidate) in joint_nodes.iter().enumerate() {
                    if candidate
                        .children()
                        .any(|c| c.index() == joint_node.index())
                    {
                        parent_idx = Some(p_idx);
                        break;
                    }
                }

                let inv_bind = inverse_bind_matrices
                    .get(idx)
                    .copied()
                    .unwrap_or(glam::Mat4::IDENTITY);

                let mut joint = rugeng_core::animation_3d::Joint::new(name, parent_idx, inv_bind);
                joint.local_translation = glam::Vec3::from(trans);
                joint.local_rotation = glam::Quat::from_xyzw(rot[0], rot[1], rot[2], rot[3]);
                joint.local_scale = glam::Vec3::from(scale);
                joints.push(joint);
            }

            if !joints.is_empty() {
                model.skeleton = Some(rugeng_core::animation_3d::Skeleton::new(joints));
            }
        }

        // 5. Animasyon Klipleri Ayrıştırma
        for anim in document.animations() {
            let anim_name = anim.name().unwrap_or("Animation").to_string();
            let mut channels = Vec::new();
            let mut max_duration: f32 = 0.0;

            for channel in anim.channels() {
                let target_node = channel.target().node();
                let target_property = match channel.target().property() {
                    gltf::animation::Property::Translation => {
                        rugeng_core::animation_3d::AnimationProperty::Translation
                    }
                    gltf::animation::Property::Rotation => {
                        rugeng_core::animation_3d::AnimationProperty::Rotation
                    }
                    gltf::animation::Property::Scale => {
                        rugeng_core::animation_3d::AnimationProperty::Scale
                    }
                    _ => continue,
                };

                let sampler = channel.sampler();
                let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));

                let timestamps: Vec<f32> = reader
                    .read_inputs()
                    .map(|iter| iter.collect())
                    .unwrap_or_default();
                if let Some(&last_t) = timestamps.last() {
                    if last_t > max_duration {
                        max_duration = last_t;
                    }
                }

                let interpolation = match sampler.interpolation() {
                    gltf::animation::Interpolation::Step => {
                        rugeng_core::animation_3d::Interpolation::Step
                    }
                    _ => rugeng_core::animation_3d::Interpolation::Linear,
                };

                // Node indeksini joint indeksine eşle
                let joint_index = if let Some(skin) = document.skins().next() {
                    skin.joints()
                        .position(|j| j.index() == target_node.index())
                        .unwrap_or(target_node.index())
                } else {
                    target_node.index()
                };

                let mut vec3_values = Vec::new();
                let mut quat_values = Vec::new();

                match target_property {
                    rugeng_core::animation_3d::AnimationProperty::Translation
                    | rugeng_core::animation_3d::AnimationProperty::Scale => {
                        if let Some(outputs) = reader.read_outputs() {
                            match outputs {
                                gltf::animation::util::ReadOutputs::Translations(iter) => {
                                    vec3_values = iter.map(glam::Vec3::from).collect();
                                }
                                gltf::animation::util::ReadOutputs::Scales(iter) => {
                                    vec3_values = iter.map(glam::Vec3::from).collect();
                                }
                                _ => {}
                            }
                        }
                    }
                    rugeng_core::animation_3d::AnimationProperty::Rotation => {
                        if let Some(gltf::animation::util::ReadOutputs::Rotations(iter)) =
                            reader.read_outputs()
                        {
                            quat_values = iter
                                .into_f32()
                                .map(|r| glam::Quat::from_xyzw(r[0], r[1], r[2], r[3]))
                                .collect();
                        }
                    }
                }

                if !timestamps.is_empty() {
                    channels.push(rugeng_core::animation_3d::AnimationChannel3D {
                        joint_index,
                        property: target_property,
                        timestamps,
                        vec3_values,
                        quat_values,
                        interpolation,
                    });
                }
            }

            model
                .animations
                .push(rugeng_core::animation_3d::AnimationClip3D::new(
                    anim_name,
                    max_duration,
                    channels,
                ));
        }

        // 6. Düğüm Hiyerarşisi (Node Graph) Ayrıştırma
        fn build_gltf_node(node: &gltf::Node) -> GltfNode {
            let (trans, rot, scale) = node.transform().decomposed();
            let children = node.children().map(|c| build_gltf_node(&c)).collect();
            GltfNode {
                name: node.name().unwrap_or("Node").to_string(),
                translation: trans,
                rotation: rot,
                scale,
                mesh_index: node.mesh().map(|m| m.index()),
                skin_index: node.skin().map(|s| s.index()),
                children,
            }
        }

        let scenes_roots: Vec<GltfNode> = if let Some(scene) = document.default_scene() {
            scene.nodes().map(|n| build_gltf_node(&n)).collect()
        } else if let Some(scene) = document.scenes().next() {
            scene.nodes().map(|n| build_gltf_node(&n)).collect()
        } else {
            // Hiç sahne yoksa kök düğümleri (ebeveyni olmayanları) bul
            let all_nodes: Vec<gltf::Node> = document.nodes().collect();
            let child_indices: std::collections::HashSet<usize> = all_nodes
                .iter()
                .flat_map(|n| n.children().map(|c| c.index()))
                .collect();
            all_nodes
                .iter()
                .filter(|n| !child_indices.contains(&n.index()))
                .map(|n| build_gltf_node(n))
                .collect()
        };

        model.root_nodes = scenes_roots;

        Ok(model)
    }

    /// Eğer modelde materyale bağlı eksik doku varsa 1x1 RGBA varsayılan doku döndürür.
    pub fn get_or_placeholder_texture(
        &self,
        index: Option<usize>,
        default_rgba: [u8; 4],
    ) -> GltfTexture {
        if let Some(idx) = index {
            if let Some(tex) = self.textures.get(idx) {
                return tex.clone();
            }
        }
        GltfTexture {
            name: "placeholder".to_string(),
            width: 1,
            height: 1,
            pixels: default_rgba.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gltf_material_default_mapping() {
        let mat = GltfMaterial::default();
        assert_eq!(mat.base_color_factor, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(mat.roughness_factor, 0.5);
        assert_eq!(mat.metallic_factor, 0.0);
        assert_eq!(mat.base_color_texture_index, None);
        assert_eq!(mat.metallic_roughness_texture_index, None);
    }

    #[test]
    fn test_gltf_missing_texture_placeholder() {
        let model = GltfModel::default();
        let placeholder = model.get_or_placeholder_texture(None, [128, 128, 255, 255]);
        assert_eq!(placeholder.width, 1);
        assert_eq!(placeholder.height, 1);
        assert_eq!(placeholder.pixels, vec![128, 128, 255, 255]);

        let out_of_bounds = model.get_or_placeholder_texture(Some(999), [255, 255, 255, 255]);
        assert_eq!(out_of_bounds.pixels, vec![255, 255, 255, 255]);
    }

    #[test]
    fn test_gltf_parse_minimal_glb() {
        let sample_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/models/sample.glb"
        );
        let model = GltfModel::from_file(sample_path).expect("sample.glb ayrıştırılabilmeli");
        assert_eq!(model.submeshes.len(), 1, "1 submesh bulunmalı");
        let sub = &model.submeshes[0];
        assert_eq!(sub.vertices.len(), 3, "1 üçgen için 3 köşe bulunmalı");
        assert_eq!(sub.indices.len(), 3, "3 indeks bulunmalı");
        assert_eq!(sub.material_index, Some(0), "0. materyale bağlı olmalı");
    }

    #[test]
    fn test_gltf_material_mapping() {
        let sample_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/models/sample.glb"
        );
        let model = GltfModel::from_file(sample_path).expect("sample.glb ayrıştırılabilmeli");
        assert_eq!(model.materials.len(), 1);
        let mat = &model.materials[0];
        assert_eq!(mat.name, "SamplePbrMaterial");
        assert_eq!(mat.base_color_factor, [0.2, 0.8, 0.4, 1.0]);
        assert!((mat.roughness_factor - 0.3).abs() < 1e-4);
        assert!((mat.metallic_factor - 0.7).abs() < 1e-4);
    }

    #[test]
    fn test_gltf_animation_and_skin_parsing() {
        let sample_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/models/sample.glb"
        );
        let model = GltfModel::from_file(sample_path).expect("sample.glb ayrıştırılabilmeli");
        // sample.glb tek bir statik üçgendir, skin/animation opsiyoneldir ve boş/None olmalıdır
        assert!(model.skeleton.is_none());
        assert!(model.animations.is_empty());
        // Köşe verilerinde varsayılan joint ve weight yapısı doğrulanmalıdır
        let sub = &model.submeshes[0];
        assert_eq!(sub.vertices[0].joints, [0, 0, 0, 0]);
        assert_eq!(sub.vertices[0].weights, [1.0, 0.0, 0.0, 0.0]);
    }
}
