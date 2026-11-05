//! Project Ember — Prefabrik Varlık Sistemi (Prefab System).
//!
//! Sahne varlıklarını (2D/3D bileşenleri, hiyerarşi ve adlarıyla birlikte)
//! bağımsız, tekrar kullanılabilir şablonlar (.prefab JSON) olarak
//! dışa aktarmayı, içe aktarmayı ve sahneye çoklu örneklemeyi (instantiate) sağlar.

use crate::animation::Animation2DComponent;
use crate::components::{Mesh3DComponent, NodeName, Text2DComponent};
use crate::particle::ParticleEmitterComponent;
use crate::registry::Registry;
use crate::scene::SceneEntity;
use crate::tilemap::Tilemap2DComponent;
use crate::transform_3d::Transform3D;
use crate::types::Entity;
use crate::{
    Children, Collider2D, Parent, ScriptComponent, SpriteColor, SpriteTexture, Transform2D,
    Velocity2D, ZIndex2D,
};
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Tekrar kullanılabilir varlık şablonu (Prefab).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Prefab {
    pub name: String,
    pub version: u32,
    pub root: SceneEntity,
    #[serde(default)]
    pub children: Vec<SceneEntity>,
}

impl Prefab {
    pub const CURRENT_VERSION: u32 = 1;

    /// Verilen Entity ve varsa tüm alt varlıklarını (Children hiyerarşisi)
    /// kapsayan bir `Prefab` nesnesi oluşturur.
    pub fn from_entity(registry: &Registry, root_entity: Entity) -> Result<Self, String> {
        if !registry.is_alive(root_entity) {
            return Err("Prefab oluşturulamadı: Kök varlık canlı değil.".to_string());
        }

        let root_scene_ent = entity_to_scene_entity(registry, root_entity, None)?;
        let name = root_scene_ent.name.clone();

        let mut children_ents = Vec::new();
        let mut queue = Vec::new();

        if let Some(children) = registry.get::<Children>(root_entity) {
            for &child_e in &children.list {
                if registry.is_alive(child_e) {
                    queue.push((child_e, root_entity.index()));
                }
            }
        }

        while let Some((curr_e, parent_idx)) = queue.pop() {
            if let Ok(c_ent) = entity_to_scene_entity(registry, curr_e, Some(parent_idx)) {
                let curr_idx = curr_e.index();
                children_ents.push(c_ent);
                if let Some(children) = registry.get::<Children>(curr_e) {
                    for &next_child in &children.list {
                        if registry.is_alive(next_child) {
                            queue.push((next_child, curr_idx));
                        }
                    }
                }
            }
        }

        Ok(Self {
            name,
            version: Self::CURRENT_VERSION,
            root: root_scene_ent,
            children: children_ents,
        })
    }

    /// Prefab'ı JSON dizgisine dönüştürür.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("Prefab JSON serileştirme hatası: {e}"))
    }

    /// JSON dizgisinden Prefab nesnesini ayrıştırır.
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        serde_json::from_str(json_str).map_err(|e| format!("Prefab JSON ayrıştırma hatası: {e}"))
    }

    /// Prefab'ı diske `.prefab` dosyası olarak kaydeder.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let json_str = self.to_json()?;
        if let Some(parent) = path.as_ref().parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path.as_ref(), json_str)
            .map_err(|e| format!("Prefab dosyasına yazılamadı: {e}"))
    }

    /// Diskten `.prefab` dosyasını okuyup Prefab nesnesi döndürür.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let json_str = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Prefab dosyası okunamadı: {e}"))?;
        Self::from_json(&json_str)
    }

    /// Prefab'ı belirtilen 2D ve 3D öteleme (offset) değerleriyle sahneye örnekler (instantiate).
    /// Kök varlığı (`root`) ve tüm hiyerarşik çocuklarını yeni Entity ID'leri ile oluşturup bağlar.
    /// Oluşturulan kök Entity'yi döndürür.
    pub fn instantiate(
        &self,
        registry: &mut Registry,
        offset_2d: Vec2,
        offset_3d: Vec3,
    ) -> Entity {
        let mut id_map: HashMap<u32, Entity> = HashMap::new();

        // 1. Kök varlığı oluştur
        let root_e = scene_entity_to_registry(registry, &self.root, offset_2d, offset_3d);
        id_map.insert(self.root.id, root_e);

        // 2. Çocukları oluştur
        for c in &self.children {
            let child_e = scene_entity_to_registry(registry, c, offset_2d, offset_3d);
            id_map.insert(c.id, child_e);
        }

        // 3. Ebeveyn / Çocuk (Parent / Children) hiyerarşisini bağla
        for c in &self.children {
            if let Some(pid) = c.parent_id {
                if let (Some(&child_ent), Some(&parent_ent)) = (id_map.get(&c.id), id_map.get(&pid)) {
                    registry.emplace(child_ent, Parent { entity: parent_ent });
                    if let Some(ch) = registry.get_mut::<Children>(parent_ent) {
                        ch.add(child_ent);
                    } else {
                        let mut ch = Children::new();
                        ch.add(child_ent);
                        registry.emplace(parent_ent, ch);
                    }
                }
            }
        }

        root_e
    }
}

/// Registry'deki bir Entity'yi SceneEntity formatına dönüştürür.
fn entity_to_scene_entity(
    registry: &Registry,
    e: Entity,
    parent_id: Option<u32>,
) -> Result<SceneEntity, String> {
    let tr_opt = registry.get::<Transform2D>(e).copied();
    let t3_opt = registry.get::<Transform3D>(e).copied();

    if tr_opt.is_none() && t3_opt.is_none() {
        return Err(format!(
            "Varlık #{}:{} ne 2D ne de 3D Transform içeriyor.",
            e.index(),
            e.generation()
        ));
    }

    let name = registry
        .get::<NodeName>(e)
        .map(|n| n.name.clone())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| {
            if t3_opt.is_some() && tr_opt.is_none() {
                format!("Node3D_{}", e.index())
            } else {
                format!("Node2D_{}", e.index())
            }
        });

    let tr = tr_opt.unwrap_or(Transform2D {
        pos: Vec2::ZERO,
        size: Vec2::ONE,
        rotation: 0.0,
    });

    let color = registry
        .get::<SpriteColor>(e)
        .map(|c| c.color)
        .unwrap_or([255, 255, 255, 255]);
    let texture = registry.get::<SpriteTexture>(e).map(|t| t.name.clone());
    let collider = registry.get::<Collider2D>(e).copied();
    let has_collider = collider.is_some();
    let is_dynamic = collider.map(|c| c.is_dynamic).unwrap_or(true);
    let is_sensor = collider.map(|c| c.is_sensor).unwrap_or(false);
    let velocity = registry.get::<Velocity2D>(e).map(|v| [v.vel.x, v.vel.y]);
    let script = registry.get::<ScriptComponent>(e).map(|s| s.name.clone());
    let text = registry.get::<Text2DComponent>(e).cloned();
    let animation = registry.get::<Animation2DComponent>(e).cloned();
    let tilemap = registry.get::<Tilemap2DComponent>(e).cloned();
    let mesh_3d = registry.get::<Mesh3DComponent>(e).cloned();
    let z_index = registry.get::<ZIndex2D>(e).map(|z| z.z).unwrap_or(0);
    let particle = registry
        .get::<ParticleEmitterComponent>(e)
        .cloned()
        .map(|mut p| {
            p.particles.clear();
            p.emission_timer = 0.0;
            p
        });

    Ok(SceneEntity {
        id: e.index(),
        name,
        pos: [tr.pos.x, tr.pos.y],
        size: [tr.size.x, tr.size.y],
        rotation: tr.rotation,
        color,
        texture,
        has_collider,
        is_dynamic,
        is_sensor,
        velocity,
        script,
        parent_id,
        text,
        animation,
        tilemap,
        transform_3d: t3_opt,
        mesh_3d,
        z_index,
        particle,
    })
}

/// SceneEntity tanımından registry üzerinde yeni bir Entity inşa eder.
fn scene_entity_to_registry(
    registry: &mut Registry,
    item: &SceneEntity,
    offset_2d: Vec2,
    offset_3d: Vec3,
) -> Entity {
    let e = registry.create();

    // NodeName
    if !item.name.is_empty() {
        registry.emplace(e, NodeName::new(item.name.clone()));
    }

    // 2D Transform
    if item.transform_3d.is_none() || (item.pos[0] != 0.0 || item.pos[1] != 0.0) {
        registry.emplace(
            e,
            Transform2D {
                pos: Vec2::new(item.pos[0] + offset_2d.x, item.pos[1] + offset_2d.y),
                size: Vec2::new(item.size[0], item.size[1]),
                rotation: item.rotation,
            },
        );
        registry.emplace(e, SpriteColor { color: item.color });
        if let Some(tex) = &item.texture {
            registry.emplace(e, SpriteTexture { name: tex.clone() });
        }
        if item.has_collider {
            registry.emplace(
                e,
                Collider2D {
                    is_dynamic: item.is_dynamic,
                    is_sensor: item.is_sensor,
                },
            );
        }
        if let Some(v) = item.velocity {
            registry.emplace(
                e,
                Velocity2D {
                    vel: Vec2::new(v[0], v[1]),
                },
            );
        }
        if item.z_index != 0 {
            registry.emplace(e, ZIndex2D::new(item.z_index));
        }
    }

    // 3D Transform
    if let Some(mut t3) = item.transform_3d {
        t3.translation += offset_3d;
        registry.emplace(e, t3);
    }
    if let Some(m3) = &item.mesh_3d {
        registry.emplace(e, m3.clone());
    }

    // Ek bileşenler
    if let Some(scr) = &item.script {
        let comp = match scr.as_str() {
            "RotatorScript" => ScriptComponent::new_rotator(1.5),
            "PatrolScript" => ScriptComponent::new_patrol(0, 80.0, 240.0, 75.0),
            "PulseScript" => ScriptComponent::new_pulse(20.0, 44.0, 3.0),
            _ => ScriptComponent::new_rotator(1.0),
        };
        registry.emplace(e, comp);
    }
    if let Some(txt) = &item.text {
        registry.emplace(e, txt.clone());
    }
    if let Some(anim) = &item.animation {
        registry.emplace(e, anim.clone());
    }
    if let Some(tm) = &item.tilemap {
        registry.emplace(e, tm.clone());
    }
    if let Some(p) = &item.particle {
        registry.emplace(e, p.clone());
    }

    e
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefab_roundtrip_and_instantiate() {
        let mut reg = Registry::new();
        reg.register_component::<Transform2D>();
        reg.register_component::<SpriteColor>();
        reg.register_component::<NodeName>();
        reg.register_component::<Parent>();
        reg.register_component::<Children>();

        let parent = reg.create();
        reg.emplace(
            parent,
            Transform2D {
                pos: Vec2::new(10.0, 20.0),
                size: Vec2::new(32.0, 32.0),
                rotation: 0.0,
            },
        );
        reg.emplace(
            parent,
            SpriteColor {
                color: [255, 0, 0, 255],
            },
        );
        reg.emplace(parent, NodeName::new("PlayerShip".to_string()));

        let child = reg.create();
        reg.emplace(
            child,
            Transform2D {
                pos: Vec2::new(5.0, 5.0),
                size: Vec2::new(8.0, 8.0),
                rotation: 0.0,
            },
        );
        reg.emplace(child, NodeName::new("Cannon".to_string()));
        reg.emplace(child, Parent { entity: parent });

        let mut ch = Children::new();
        ch.add(child);
        reg.emplace(parent, ch);

        // Prefab oluştur
        let prefab = Prefab::from_entity(&reg, parent).expect("Prefab oluşturulamadı");
        assert_eq!(prefab.name, "PlayerShip");
        assert_eq!(prefab.children.len(), 1);
        assert_eq!(prefab.children[0].name, "Cannon");

        // JSON Serialize & Deserialize
        let json = prefab.to_json().expect("to_json fail");
        let loaded = Prefab::from_json(&json).expect("from_json fail");
        assert_eq!(loaded, prefab);

        // Örnekleme (Instantiate)
        let new_root = loaded.instantiate(&mut reg, Vec2::new(50.0, 50.0), Vec3::ZERO);
        assert_ne!(new_root, parent);

        let new_tr = reg.get::<Transform2D>(new_root).unwrap();
        assert_eq!(new_tr.pos, Vec2::new(60.0, 70.0));
        assert_eq!(
            reg.get::<NodeName>(new_root).map(|n| n.name.as_str()),
            Some("PlayerShip")
        );

        let new_children = reg.get::<Children>(new_root).unwrap();
        assert_eq!(new_children.list.len(), 1);
        let new_child = new_children.list[0];
        assert_ne!(new_child, child);
        assert_eq!(
            reg.get::<NodeName>(new_child).map(|n| n.name.as_str()),
            Some("Cannon")
        );
        assert_eq!(
            reg.get::<Parent>(new_child).map(|p| p.entity),
            Some(new_root)
        );
    }
}
