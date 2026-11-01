//! 2D Tilemap (Kareli Harita) Sistemi
//!
//! Izgara tabanlı harita çizimi, karo türleri, katı çarpışma ve serileştirme.

use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TileType {
    Empty = 0,
    Grass = 1,
    Stone = 2,
    Water = 3,
    Lava = 4,
    Metal = 5,
}

impl TileType {
    pub fn default_color(&self) -> [u8; 4] {
        match self {
            TileType::Empty => [0, 0, 0, 0],
            TileType::Grass => [66, 185, 131, 255], // Canlı Godot Yeşili
            TileType::Stone => [120, 130, 145, 255], // Taş Grisi
            TileType::Water => [44, 110, 190, 220], // Mavi
            TileType::Lava => [228, 70, 50, 255],   // Akkor Kırmızı-Turuncu
            TileType::Metal => [180, 185, 200, 255], // Çelik Plaka
        }
    }

    pub fn is_default_solid(&self) -> bool {
        matches!(self, TileType::Stone | TileType::Metal)
    }

    pub fn name(&self) -> &'static str {
        match self {
            TileType::Empty => "Boş",
            TileType::Grass => "Çimen",
            TileType::Stone => "Taş Duvar",
            TileType::Water => "Su",
            TileType::Lava => "Lav",
            TileType::Metal => "Metal Platform",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TileData {
    pub tile_type: TileType,
    pub color: [u8; 4],
    pub solid: bool,
    #[serde(default)]
    pub one_way: bool,
}

mod tile_map_serde {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(
        map: &HashMap<(i32, i32), TileData>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(map.len()))?;
        for (&(x, y), &tile) in map {
            seq.serialize_element(&(x, y, tile))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<(i32, i32), TileData>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let list = Vec::<(i32, i32, TileData)>::deserialize(deserializer)?;
        let mut map = HashMap::with_capacity(list.len());
        for (x, y, tile) in list {
            map.insert((x, y), tile);
        }
        Ok(map)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tilemap2DComponent {
    pub tile_size: f32,
    #[serde(with = "tile_map_serde")]
    pub tiles: HashMap<(i32, i32), TileData>,
}

impl Default for Tilemap2DComponent {
    fn default() -> Self {
        Self {
            tile_size: 32.0,
            tiles: HashMap::new(),
        }
    }
}

impl Tilemap2DComponent {
    pub fn new(tile_size: f32) -> Self {
        Self {
            tile_size,
            tiles: HashMap::new(),
        }
    }

    pub fn world_to_grid(&self, world_pos: Vec2) -> (i32, i32) {
        let gx = (world_pos.x / self.tile_size).floor() as i32;
        let gy = (world_pos.y / self.tile_size).floor() as i32;
        (gx, gy)
    }

    pub fn grid_to_world(&self, gx: i32, gy: i32) -> Vec2 {
        Vec2::new(
            (gx as f32 + 0.5) * self.tile_size,
            (gy as f32 + 0.5) * self.tile_size,
        )
    }

    pub fn set_tile(&mut self, gx: i32, gy: i32, tile_type: TileType, solid: bool) {
        self.set_tile_ex(gx, gy, tile_type, solid, false);
    }

    pub fn set_tile_ex(
        &mut self,
        gx: i32,
        gy: i32,
        tile_type: TileType,
        solid: bool,
        one_way: bool,
    ) {
        if tile_type == TileType::Empty {
            self.tiles.remove(&(gx, gy));
        } else {
            self.tiles.insert(
                (gx, gy),
                TileData {
                    tile_type,
                    color: tile_type.default_color(),
                    solid,
                    one_way,
                },
            );
        }
    }

    pub fn remove_tile(&mut self, gx: i32, gy: i32) -> Option<TileData> {
        self.tiles.remove(&(gx, gy))
    }

    pub fn get_tile(&self, gx: i32, gy: i32) -> Option<&TileData> {
        self.tiles.get(&(gx, gy))
    }

    pub fn count(&self) -> usize {
        self.tiles.len()
    }

    pub fn clear(&mut self) {
        self.tiles.clear();
    }

    /// Çarpışma çözücü için katı ve çift yönlü engel olan karoların dünya konumlarını ve boyutlarını döndürür.
    pub fn solid_aabb_boxes(&self) -> Vec<(Vec2, Vec2)> {
        let sz = Vec2::splat(self.tile_size);
        self.tiles
            .iter()
            .filter(|(_, data)| data.solid && !data.one_way)
            .map(|(&(gx, gy), _)| (self.grid_to_world(gx, gy), sz))
            .collect()
    }

    /// Tek yönlü (jump-through) platform karolarının dünya konumlarını ve boyutlarını döndürür.
    pub fn one_way_aabb_boxes(&self) -> Vec<(Vec2, Vec2)> {
        let sz = Vec2::splat(self.tile_size);
        self.tiles
            .iter()
            .filter(|(_, data)| data.one_way)
            .map(|(&(gx, gy), _)| (self.grid_to_world(gx, gy), sz))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tilemap_coordinates_and_placement() {
        let mut map = Tilemap2DComponent::new(32.0);
        let world_pos = Vec2::new(48.0, 70.0);
        let (gx, gy) = map.world_to_grid(world_pos);
        assert_eq!((gx, gy), (1, 2));

        map.set_tile(gx, gy, TileType::Stone, true);
        assert_eq!(map.count(), 1);
        assert!(map.get_tile(gx, gy).unwrap().solid);

        let center = map.grid_to_world(gx, gy);
        assert_eq!(center, Vec2::new(48.0, 80.0));

        let solids = map.solid_aabb_boxes();
        assert_eq!(solids.len(), 1);
        assert_eq!(solids[0].0, center);

        map.remove_tile(gx, gy);
        assert_eq!(map.count(), 0);
    }
}
