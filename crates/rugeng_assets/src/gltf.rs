//! glTF 2.0 / GLB Model ve PBR Materyal Ayrıştırıcısı.
//!
//! Bu modül, standart glTF 2.0 ve binary GLB dosyalarından
//! geometrileri (Pozisyon, Normal, UV, Renk) ve Cook-Torrance PBR
//! materyal parametrelerini (BaseColor, Metallic-Roughness) ayrıştırır.

use std::path::Path;

/// glTF Köşe Verisi
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GltfVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl Default for GltfVertex {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            color: [1.0, 1.0, 1.0, 1.0],
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

/// Çözümlenmiş Doku Verisi (Gömülü veya ayrık)
#[derive(Clone, Debug)]
pub struct GltfTexture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA8
}

/// Tam glTF Modeli
#[derive(Clone, Debug, Default)]
pub struct GltfModel {
    pub name: String,
    pub submeshes: Vec<GltfSubmesh>,
    pub materials: Vec<GltfMaterial>,
    pub textures: Vec<GltfTexture>,
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

                let mut vertices = Vec::with_capacity(positions.len());
                for (i, &pos) in positions.iter().enumerate() {
                    vertices.push(GltfVertex {
                        position: pos,
                        normal: normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
                        uv: uvs.get(i).copied().unwrap_or([0.0, 0.0]),
                        color: colors.get(i).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]),
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
}
