//! Hiyerarşik Sahne Grafı (Parent - Child Transform Mimarisi)

use crate::types::Entity;
use glam::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parent {
    pub entity: Entity,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Children {
    pub list: Vec<Entity>,
}

impl Children {
    pub fn new() -> Self {
        Self { list: Vec::new() }
    }

    pub fn add(&mut self, child: Entity) {
        if !self.list.contains(&child) {
            self.list.push(child);
        }
    }

    pub fn remove(&mut self, child: Entity) {
        self.list.retain(|&e| e != child);
    }
}

/// Yerel pozisyon ve açıyı ebeveynin dünya koordinatlarına göre dönüştürür.
pub fn compute_world_transform(
    local_pos: Vec2,
    local_rot: f32,
    parent_pos: Vec2,
    parent_rot: f32,
) -> (Vec2, f32) {
    let cos_a = parent_rot.cos();
    let sin_a = parent_rot.sin();
    let rotated_offset = Vec2::new(
        local_pos.x * cos_a - local_pos.y * sin_a,
        local_pos.x * sin_a + local_pos.y * cos_a,
    );
    (parent_pos + rotated_offset, parent_rot + local_rot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchy_world_transform() {
        let parent_pos = Vec2::new(100.0, 100.0);
        let parent_rot = 0.0;
        let local_pos = Vec2::new(20.0, 0.0);
        let local_rot = 0.5;

        let (w_pos, w_rot) = compute_world_transform(local_pos, local_rot, parent_pos, parent_rot);
        assert!((w_pos.x - 120.0).abs() < 1e-4);
        assert!((w_pos.y - 100.0).abs() < 1e-4);
        assert!((w_rot - 0.5).abs() < 1e-4);

        // 90 derece dönüş testi (PI / 2)
        let parent_rot_90 = std::f32::consts::FRAC_PI_2;
        let (w_pos_rot, _) =
            compute_world_transform(local_pos, local_rot, parent_pos, parent_rot_90);
        assert!((w_pos_rot.x - 100.0).abs() < 1e-3);
        assert!((w_pos_rot.y - 120.0).abs() < 1e-3);
    }
}
