//! Project Ember — Sahne Serileştirme Sistemi (Scene Serialization).
//!
//! ECS varlık hiyerarşisini, bileşenlerini ve sahneleri insan tarafından okunabilir
//! ve düzenlenebilir `.scene` (JSON) formatına kaydeder ve geri yükler.

use crate::animation::Animation2DComponent;
use crate::components::Text2DComponent;
use crate::tilemap::Tilemap2DComponent;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneEntity {
    pub id: u32,
    pub name: String,
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub rotation: f32,
    pub color: [u8; 4],
    pub texture: Option<String>,
    pub has_collider: bool,
    #[serde(default = "default_true")]
    pub is_dynamic: bool,
    #[serde(default)]
    pub is_sensor: bool,
    pub velocity: Option<[f32; 2]>,
    pub script: Option<String>,
    #[serde(default)]
    pub parent_id: Option<u32>,
    #[serde(default)]
    pub text: Option<Text2DComponent>,
    #[serde(default)]
    pub animation: Option<Animation2DComponent>,
    #[serde(default)]
    pub tilemap: Option<Tilemap2DComponent>,
    #[serde(default)]
    pub transform_3d: Option<crate::transform_3d::Transform3D>,
    #[serde(default)]
    pub mesh_3d: Option<crate::components::Mesh3DComponent>,
    #[serde(default)]
    pub z_index: i32,
    /// Parçacık yayıcı ayarları. Çalışma zamanı durumu (canlı parçacıklar,
    /// emission sayacı) serileştirilmez; yalnızca konfigürasyon saklanır.
    #[serde(default)]
    pub particle: Option<crate::particle::ParticleEmitterComponent>,
    #[serde(default)]
    pub animation_3d: Option<crate::animation_3d::Animation3DComponent>,
    #[serde(default)]
    pub point_light: Option<crate::components::PointLight>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneData {
    pub name: String,
    pub version: u32,
    pub entities: Vec<SceneEntity>,
}

impl SceneData {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: 1,
            entities: Vec::new(),
        }
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("JSON serilestirme hatasi: {e}"))
    }

    pub fn from_json(json_str: &str) -> Result<Self, String> {
        serde_json::from_str(json_str).map_err(|e| format!("JSON cozme hatasi: {e}"))
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let json_str = self.to_json()?;
        if let Some(parent) = path.as_ref().parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path.as_ref(), json_str).map_err(|e| format!("Sahne dosyasina yazilamadi: {e}"))
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let json_str = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Sahne dosyasi okunamadi: {e}"))?;
        Self::from_json(&json_str)
    }
}
