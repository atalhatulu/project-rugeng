//! 3D Spatial Hash Grid (3D Geniş Faz Hızlandırıcısı).
//!
//! O(N^2) nesne-nesne testini O(N) seviyesine indirir.
//! Yarıçap duyarlı ekleme (radius / half_extents) ile hücre sınırındaki nesneleri
//! kapsayan tüm 3D hücrelere ekler ve sorgu anında tekilleştirir.

use glam::Vec3;

pub const DEFAULT_CELL_SIZE_3D: f32 = 4.0;
pub const DEFAULT_BUCKET_COUNT_3D: usize = 4096; // 2'nin kuvveti

#[derive(Clone, Copy, Debug)]
pub struct SpatialEntry3D {
    pub id: u32,
    pub center: Vec3,
    pub radius: f32,
    pub next: u32,
}

pub struct SpatialHashGrid3D {
    cell_size: f32,
    inv_cell_size: f32,
    bucket_mask: usize,
    heads: Vec<u32>,
    entries: Vec<SpatialEntry3D>,
}

impl SpatialHashGrid3D {
    pub fn new(cell_size: f32, bucket_count: usize, initial_capacity: usize) -> Self {
        assert!(
            bucket_count.is_power_of_two(),
            "bucket_count must be a power of two"
        );
        Self {
            cell_size: cell_size.max(0.1),
            inv_cell_size: 1.0 / cell_size.max(0.1),
            bucket_mask: bucket_count - 1,
            heads: vec![u32::MAX; bucket_count],
            entries: Vec::with_capacity(initial_capacity),
        }
    }

    #[inline]
    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }

    #[inline]
    fn hash_cell(&self, cx: i32, cy: i32, cz: i32) -> usize {
        // Asal katsayılı 3D hash
        let h = (cx as u32).wrapping_mul(73856093)
            ^ (cy as u32).wrapping_mul(19349663)
            ^ (cz as u32).wrapping_mul(83492791);
        (h as usize) & self.bucket_mask
    }

    #[inline]
    pub fn clear(&mut self) {
        self.heads.fill(u32::MAX);
        self.entries.clear();
    }

    /// Bir 3D nesneyi yarıçap duyarlı olarak kapsadığı tüm hücrelere ekler.
    pub fn insert(&mut self, id: u32, center: Vec3, radius: f32) {
        let r = radius.max(0.0);
        let min_cx = ((center.x - r) * self.inv_cell_size).floor() as i32;
        let max_cx = ((center.x + r) * self.inv_cell_size).floor() as i32;
        let min_cy = ((center.y - r) * self.inv_cell_size).floor() as i32;
        let max_cy = ((center.y + r) * self.inv_cell_size).floor() as i32;
        let min_cz = ((center.z - r) * self.inv_cell_size).floor() as i32;
        let max_cz = ((center.z + r) * self.inv_cell_size).floor() as i32;

        for cx in min_cx..=max_cx {
            for cy in min_cy..=max_cy {
                for cz in min_cz..=max_cz {
                    let bucket = self.hash_cell(cx, cy, cz);
                    let entry_idx = self.entries.len() as u32;
                    let next = self.heads[bucket];
                    self.entries.push(SpatialEntry3D {
                        id,
                        center,
                        radius: r,
                        next,
                    });
                    self.heads[bucket] = entry_idx;
                }
            }
        }
    }

    /// Verilen küresel hacimle kesişen aday nesne ID'lerini döner (Tekilleştirilmiş).
    pub fn query_sphere(&self, center: Vec3, radius: f32, out_ids: &mut Vec<u32>) {
        out_ids.clear();
        let r = radius.max(0.0);
        let min_cx = ((center.x - r) * self.inv_cell_size).floor() as i32;
        let max_cx = ((center.x + r) * self.inv_cell_size).floor() as i32;
        let min_cy = ((center.y - r) * self.inv_cell_size).floor() as i32;
        let max_cy = ((center.y + r) * self.inv_cell_size).floor() as i32;
        let min_cz = ((center.z - r) * self.inv_cell_size).floor() as i32;
        let max_cz = ((center.z + r) * self.inv_cell_size).floor() as i32;

        for cx in min_cx..=max_cx {
            for cy in min_cy..=max_cy {
                for cz in min_cz..=max_cz {
                    let bucket = self.hash_cell(cx, cy, cz);
                    let mut curr = self.heads[bucket];
                    while curr != u32::MAX {
                        let entry = &self.entries[curr as usize];
                        let diff = entry.center - center;
                        let max_dist = entry.radius + r;
                        if diff.length_squared() <= max_dist * max_dist
                            && !out_ids.contains(&entry.id)
                        {
                            out_ids.push(entry.id);
                        }
                        curr = entry.next;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_hash_3d_cell_boundary() {
        let mut grid = SpatialHashGrid3D::new(4.0, 256, 128);

        // Hücre sınırı x = 4.0 üzerinde duran bir küre (merkez: 3.9, yarıçap: 0.5 -> x aralığı [3.4, 4.4])
        // Bu küre hem cx=0 (0..4) hem de cx=1 (4..8) hücresine girmelidir.
        grid.insert(101, Vec3::new(3.9, 2.0, 2.0), 0.5);

        // cx=1 hücresinden sorgula (merkez: 4.2, yarıçap: 0.1)
        let mut results = Vec::new();
        grid.query_sphere(Vec3::new(4.2, 2.0, 2.0), 0.1, &mut results);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], 101);
    }
}
