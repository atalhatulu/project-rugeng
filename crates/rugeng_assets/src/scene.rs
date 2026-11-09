//! Versiyonlu Sahne Serileştirme ve Eski Sürüm Uyumluluğu (Versioned Scene).
//!
//! Versiyon 1 (Legacy) ve Versiyon 2 (Modern) şemalarını destekler,
//! eski sürüm dosyalarını otomatik olarak geçerli formata dönüştürür.

use rugeng_core::scene::{SceneData, SceneEntity};
use serde::{Deserialize, Serialize};

/// Versiyon 1 Şeması (Eski format)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneDataV1 {
    pub name: String,
    pub version: u32, // == 1
    pub entities: Vec<SceneEntity>,
}

/// Versiyon 2 Şeması (Ortam ayarları ve metadata eklenmiş modern format)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneEnvironment {
    pub ambient_color: [f32; 4],
    pub gravity: [f32; 2],
    pub author: String,
}

impl Default for SceneEnvironment {
    fn default() -> Self {
        Self {
            ambient_color: [0.05, 0.06, 0.08, 1.0],
            gravity: [0.0, 980.0],
            author: "Rugeng".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VersionedScene {
    pub name: String,
    pub version: u32,
    pub environment: SceneEnvironment,
    pub entities: Vec<SceneEntity>,
}

impl VersionedScene {
    pub const CURRENT_VERSION: u32 = 2;

    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: Self::CURRENT_VERSION,
            environment: SceneEnvironment::default(),
            entities: Vec::new(),
        }
    }

    /// JSON dizesinden çözer; versiyon 1 ise şeffaf şekilde versiyon 2'ye yükseltir.
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct VersionProbe {
            #[serde(default = "default_version_one")]
            version: u32,
        }
        fn default_version_one() -> u32 {
            1
        }

        let probe: VersionProbe = serde_json::from_str(json_str)
            .map_err(|e| format!("Sahne JSON ayrıştırılamadı: {e}"))?;

        match probe.version {
            1 => {
                let v1: SceneDataV1 = serde_json::from_str(json_str)
                    .map_err(|e| format!("V1 sahne formatı çözülemedi: {e}"))?;
                Ok(Self {
                    name: v1.name,
                    version: Self::CURRENT_VERSION,
                    environment: SceneEnvironment::default(),
                    entities: v1.entities,
                })
            }
            2 => {
                let v2: VersionedScene = serde_json::from_str(json_str)
                    .map_err(|e| format!("V2 sahne formatı çözülemedi: {e}"))?;
                Ok(v2)
            }
            v => Err(format!("Desteklenmeyen sahne sürümü: {v}")),
        }
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("JSON serileştirme hatası: {e}"))
    }

    /// İkili (Binary) formata kodlar. Başında "RUGENG_SCENE_BIN\0" sihirli baytı bulunur.
    pub fn to_binary(&self) -> Result<Vec<u8>, String> {
        let json_bytes =
            serde_json::to_vec(self).map_err(|e| format!("Binary sahne paketleme hatası: {e}"))?;
        let mut out = Vec::with_capacity(16 + json_bytes.len());
        out.extend_from_slice(b"RUGENG_SCN_BIN\0");
        out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&json_bytes);
        Ok(out)
    }

    /// İkili (Binary) formattan çözer.
    pub fn from_binary(bytes: &[u8]) -> Result<Self, String> {
        const MAGIC: &[u8] = b"RUGENG_SCN_BIN\0";
        if bytes.len() < 19 || &bytes[..15] != MAGIC {
            return Err("Gecersiz ikili sahne sihirli basligi".to_string());
        }
        let len_bytes = [bytes[15], bytes[16], bytes[17], bytes[18]];
        let len = u32::from_le_bytes(len_bytes) as usize;
        if bytes.len() < 19 + len {
            return Err("Eksik veya bozuk ikili sahne verisi".to_string());
        }
        let json_slice = &bytes[19..19 + len];
        let json_str = std::str::from_utf8(json_slice)
            .map_err(|e| format!("Gecersiz UTF-8 sahne verisi: {e}"))?;
        Self::from_json(json_str)
    }

    /// Otomatik tespit: Dosya baytları ikili ise binary, değilse JSON olarak ayrıştırır.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.starts_with(b"RUGENG_SCN_BIN\0") {
            Self::from_binary(bytes)
        } else {
            let json_str = std::str::from_utf8(bytes)
                .map_err(|e| format!("Sahne verisi UTF-8 olarak okunamadi: {e}"))?;
            Self::from_json(json_str)
        }
    }

    /// RugengCore `SceneData` yapısına dönüştürür
    pub fn to_core_scene(&self) -> SceneData {
        SceneData {
            name: self.name.clone(),
            version: self.version,
            entities: self.entities.clone(),
        }
    }

    /// RugengCore `SceneData` yapısından oluşturur
    pub fn from_core_scene(core: &SceneData) -> Self {
        Self {
            name: core.name.clone(),
            version: Self::CURRENT_VERSION,
            environment: SceneEnvironment::default(),
            entities: core.entities.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v1_to_v2_migration() {
        let v1_json = r#"{
            "name": "OldLegacyLevel",
            "version": 1,
            "entities": [
                {
                    "id": 10,
                    "name": "LegacyPlayer",
                    "pos": [100.0, 200.0],
                    "size": [32.0, 32.0],
                    "rotation": 0.0,
                    "color": [255, 255, 255, 255],
                    "texture": "player.png",
                    "has_collider": true,
                    "is_dynamic": true
                }
            ]
        }"#;

        let scene = VersionedScene::from_json(v1_json).expect("V1 sahnede yukseltme basarisiz");
        assert_eq!(scene.name, "OldLegacyLevel");
        assert_eq!(scene.version, 2);
        assert_eq!(scene.entities.len(), 1);
        assert_eq!(scene.entities[0].name, "LegacyPlayer");
        assert_eq!(scene.environment, SceneEnvironment::default());
    }

    #[test]
    fn test_v2_roundtrip() {
        let mut scene = VersionedScene::new("V2Test");
        scene.environment.author = "TestAuthor".to_string();
        let json = scene.to_json().unwrap();
        let decoded = VersionedScene::from_json(&json).unwrap();
        assert_eq!(scene, decoded);
    }

    #[test]
    fn test_v2_binary_roundtrip() {
        let mut scene = VersionedScene::new("V2BinaryTest");
        scene.environment.author = "BinaryAuthor".to_string();
        let bin = scene
            .to_binary()
            .expect("Binary serilestirme basarili olmali");
        let decoded = VersionedScene::from_bytes(&bin).expect("Binary cozumleme basarili olmali");
        assert_eq!(scene, decoded);
    }
}
