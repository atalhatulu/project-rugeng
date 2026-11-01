//! 2D Spatial Hash Grid (Parçacık ve Mermi Çarpışma Hızlandırıcısı).
//!
//! O(N^2) mermi-nesne testini O(N) karmaşıklığına indirir.
//! Bellek karesel döngüde yeniden tahsis edilmez; kapasiteyi koruyan bağlı liste (linked-list buckets)
//! mantığıyla sıfır malloc garantisi sağlar.

pub const DEFAULT_CELL_SIZE: f32 = 64.0;
pub const DEFAULT_BUCKET_COUNT: usize = 4096; // 2'nin kuvveti (hızlı bitwise AND)

#[derive(Clone, Copy, Debug)]
pub struct SpatialEntry {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub next: u32,
}

pub struct SpatialHashGrid {
    cell_size: f32,
    inv_cell_size: f32,
    bucket_mask: usize,
    heads: Vec<u32>,
    entries: Vec<SpatialEntry>,
    max_radius: f32,
}

impl SpatialHashGrid {
    pub fn new(cell_size: f32, bucket_count: usize, initial_capacity: usize) -> Self {
        assert!(
            bucket_count.is_power_of_two(),
            "bucket_count must be a power of two"
        );
        Self {
            cell_size,
            inv_cell_size: 1.0 / cell_size,
            bucket_mask: bucket_count - 1,
            heads: vec![u32::MAX; bucket_count],
            entries: Vec::with_capacity(initial_capacity),
            max_radius: 0.0,
        }
    }

    #[inline]
    fn hash_cell(&self, cx: i32, cy: i32) -> usize {
        // Gelişmiş asal katsayılı 2D hash
        let h = (cx as u32).wrapping_mul(73856093) ^ (cy as u32).wrapping_mul(19349663);
        (h as usize) & self.bucket_mask
    }

    /// Yeni kare başında grid'i sıfırlar (bellek kapasitesi serbest bırakılmaz, 0 malloc)
    #[inline]
    pub fn clear(&mut self) {
        self.heads.fill(u32::MAX);
        self.entries.clear();
        self.max_radius = 0.0;
    }

    /// Bir nesneyi veya mermiyi grid'e ekler
    #[inline]
    pub fn insert(&mut self, id: u32, x: f32, y: f32, radius: f32) {
        if radius > self.max_radius {
            self.max_radius = radius;
        }

        let cx = (x * self.inv_cell_size).floor() as i32;
        let cy = (y * self.inv_cell_size).floor() as i32;
        let bucket = self.hash_cell(cx, cy);

        let entry_idx = self.entries.len() as u32;
        let next = self.heads[bucket];
        self.heads[bucket] = entry_idx;

        self.entries.push(SpatialEntry {
            id,
            x,
            y,
            radius,
            next,
        });
    }

    #[inline]
    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }

    #[inline]
    pub fn max_radius(&self) -> f32 {
        self.max_radius
    }

    /// Belirtilen daire içindeki komşu nesneleri sorgular
    #[inline]
    pub fn query_radius<F>(&self, x: f32, y: f32, radius: f32, mut callback: F)
    where
        F: FnMut(u32, f32, f32, f32),
    {
        if self.entries.is_empty() {
            return;
        }

        let search_r = radius + self.max_radius;
        let min_cx = ((x - search_r) * self.inv_cell_size).floor() as i32;
        let max_cx = ((x + search_r) * self.inv_cell_size).floor() as i32;
        let min_cy = ((y - search_r) * self.inv_cell_size).floor() as i32;
        let max_cy = ((y + search_r) * self.inv_cell_size).floor() as i32;

        let mut visited_inline = [usize::MAX; 36];
        let mut visited_overflow: Vec<usize> = Vec::new();
        let mut visited_count = 0usize;

        for cy in min_cy..=max_cy {
            for cx in min_cx..=max_cx {
                let bucket = self.hash_cell(cx, cy);

                let already_visited = if visited_count <= 36 {
                    visited_inline[..visited_count].contains(&bucket)
                } else {
                    visited_inline.contains(&bucket) || visited_overflow.contains(&bucket)
                };

                if already_visited {
                    continue;
                }

                if visited_count < 36 {
                    visited_inline[visited_count] = bucket;
                } else {
                    visited_overflow.push(bucket);
                }
                visited_count += 1;

                let mut curr = self.heads[bucket];
                while curr != u32::MAX {
                    let entry = &self.entries[curr as usize];
                    let dx = entry.x - x;
                    let dy = entry.y - y;
                    let total_r = radius + entry.radius;
                    if dx * dx + dy * dy <= total_r * total_r {
                        callback(entry.id, entry.x, entry.y, entry.radius);
                    }
                    curr = entry.next;
                }
            }
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_hash_insertion_and_query() {
        let mut grid = SpatialHashGrid::new(64.0, 1024, 100);

        // Birbirine yakın ve uzak nesneler
        grid.insert(1, 10.0, 10.0, 5.0);
        grid.insert(2, 20.0, 20.0, 5.0);
        grid.insert(3, 500.0, 500.0, 5.0);

        let mut found = Vec::new();
        grid.query_radius(12.0, 12.0, 15.0, |id, _, _, _| {
            found.push(id);
        });

        assert!(found.contains(&1));
        assert!(found.contains(&2));
        assert!(!found.contains(&3));
    }

    #[test]
    fn test_spatial_hash_large_radius_and_deduplication() {
        // Küçük bucket sayısı (4 bucket) kullanarak hash çakışmasını zorla
        let mut grid = SpatialHashGrid::new(32.0, 4, 10);

        // Büyük yarıçaplı nesne (komşu hücrelere taşar)
        grid.insert(100, 10.0, 10.0, 50.0);

        let mut matches = Vec::new();
        // 55 piksel uzaktan sorgula (10.0 + 50.0 + 5.0 = 65 > 55, kesişmeli)
        grid.query_radius(60.0, 10.0, 5.0, |id, _, _, _| {
            matches.push(id);
        });

        // Büyük yarıçap nedeniyle bulunmalı ve bucket çakışmasına rağmen SADECE 1 KEZ dönmeli
        assert_eq!(matches, vec![100]);
    }
}
