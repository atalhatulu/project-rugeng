//! 2D Raycasting (Işın Kesişim ve Izgara Taraması) Sistemi
//!
//! AABB kutuları ve Tilemap2D ızgarasına karşı yüksek performanslı
//! analitik ışın kesişim ve DDA (Digital Differential Analyzer) tarama fonksiyonları.

use glam::{Vec2, Vec3};
use rugeng_core::tilemap::{TileData, Tilemap2DComponent};
use rugeng_core::{BoundingSphere, AABB3D};

/// 2 Boyutlu Işın (Ray)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray2D {
    pub origin: Vec2,
    pub dir: Vec2,
    pub max_dist: f32,
}

impl Ray2D {
    /// Yeni bir 2D ışın oluşturur. Yön vektörü otomatik olarak normalize edilir.
    pub fn new(origin: Vec2, dir: Vec2, max_dist: f32) -> Self {
        let len = dir.length();
        let dir = if len > 1e-6 { dir / len } else { Vec2::ZERO };
        Self {
            origin,
            dir,
            max_dist: max_dist.max(0.0),
        }
    }

    /// Işın üzerinde belirli bir mesafedeki konumu döndürür.
    #[inline]
    pub fn point_at(&self, dist: f32) -> Vec2 {
        self.origin + self.dir * dist
    }
}

/// 2 Boyutlu Işın Kesişim Sonucu (Ray Hit)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit2D {
    /// Kesişimin gerçekleştiği dünya koordinatı
    pub point: Vec2,
    /// Çarpan yüzeyin dışa bakan normal vektörü
    pub normal: Vec2,
    /// Işın başlangıcından kesişim noktasına olan mesafe
    pub dist: f32,
}

/// Bir 2D ışının verilen AABB kutusu ile kesişimini analitik slab (dilim) yöntemiyle test eder.
pub fn ray_intersect_aabb(ray: &Ray2D, center: Vec2, size: Vec2) -> Option<RayHit2D> {
    if ray.max_dist <= 0.0 || ray.dir == Vec2::ZERO {
        return None;
    }

    let half = size * 0.5;
    let min = center - half;
    let max = center + half;

    let mut t_near = f32::NEG_INFINITY;
    let mut t_far = f32::INFINITY;
    let mut hit_normal = Vec2::ZERO;

    // X Ekseni Dilim Testi
    if ray.dir.x.abs() < 1e-8 {
        if ray.origin.x < min.x || ray.origin.x > max.x {
            return None;
        }
    } else {
        let inv_d = 1.0 / ray.dir.x;
        let mut t1 = (min.x - ray.origin.x) * inv_d;
        let mut t2 = (max.x - ray.origin.x) * inv_d;
        let mut n1 = Vec2::new(-1.0, 0.0);

        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
            n1 = Vec2::new(1.0, 0.0);
        }

        if t1 > t_near {
            t_near = t1;
            hit_normal = n1;
        }
        if t2 < t_far {
            t_far = t2;
        }

        if t_near > t_far {
            return None;
        }
    }

    // Y Ekseni Dilim Testi
    if ray.dir.y.abs() < 1e-8 {
        if ray.origin.y < min.y || ray.origin.y > max.y {
            return None;
        }
    } else {
        let inv_d = 1.0 / ray.dir.y;
        let mut t1 = (min.y - ray.origin.y) * inv_d;
        let mut t2 = (max.y - ray.origin.y) * inv_d;
        let mut n1 = Vec2::new(0.0, -1.0);

        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
            n1 = Vec2::new(0.0, 1.0);
        }

        if t1 > t_near {
            t_near = t1;
            hit_normal = n1;
        }
        if t2 < t_far {
            t_far = t2;
        }

        if t_near > t_far {
            return None;
        }
    }

    if t_far <= 1e-6 {
        return None;
    }

    let (dist, point, normal) = if t_near < 0.0 {
        // Işın başlangıcı AABB'nin içinde
        (0.0, ray.origin, -ray.dir)
    } else {
        (t_near, ray.origin + ray.dir * t_near, hit_normal)
    };

    if dist > ray.max_dist {
        return None;
    }

    Some(RayHit2D {
        point,
        normal,
        dist,
    })
}

/// Bir 2D ışının Tilemap2D haritasındaki katı karolarla kesişimini DDA algoritmasıyla test eder.
pub fn raycast_grid(ray: &Ray2D, tilemap: &Tilemap2DComponent) -> Option<RayHit2D> {
    raycast_grid_custom(ray, tilemap, |tile| tile.solid)
}

/// Özel bir karo filtre fonksiyonuyla (predicate) Tilemap2D üzerinde ışın taraması yapar.
pub fn raycast_grid_custom<F>(
    ray: &Ray2D,
    tilemap: &Tilemap2DComponent,
    mut predicate: F,
) -> Option<RayHit2D>
where
    F: FnMut(&TileData) -> bool,
{
    if ray.max_dist <= 0.0 || ray.dir == Vec2::ZERO || tilemap.tile_size <= 0.0 {
        return None;
    }

    let ts = tilemap.tile_size;

    // 1. Işının başladığı ilk karoyu kontrol et
    let (mut current_gx, mut current_gy) = tilemap.world_to_grid(ray.origin);
    if let Some(tile) = tilemap.get_tile(current_gx, current_gy) {
        if predicate(tile) {
            let center = tilemap.grid_to_world(current_gx, current_gy);
            if let Some(hit) = ray_intersect_aabb(ray, center, Vec2::splat(ts)) {
                if hit.dist <= ray.max_dist {
                    return Some(hit);
                }
            }
        }
    }

    // 2. DDA (Amanatides-Woo) parametrelerini hazırla
    let dx = ray.dir.x;
    let dy = ray.dir.y;

    let step_x = if dx > 0.0 {
        1
    } else if dx < 0.0 {
        -1
    } else {
        0
    };
    let step_y = if dy > 0.0 {
        1
    } else if dy < 0.0 {
        -1
    } else {
        0
    };

    let mut t_max_x = if dx > 0.0 {
        let next_x = (current_gx + 1) as f32 * ts;
        (next_x - ray.origin.x) / dx
    } else if dx < 0.0 {
        let next_x = current_gx as f32 * ts;
        (next_x - ray.origin.x) / dx
    } else {
        f32::INFINITY
    };

    let mut t_max_y = if dy > 0.0 {
        let next_y = (current_gy + 1) as f32 * ts;
        (next_y - ray.origin.y) / dy
    } else if dy < 0.0 {
        let next_y = current_gy as f32 * ts;
        (next_y - ray.origin.y) / dy
    } else {
        f32::INFINITY
    };

    let t_delta_x = if dx.abs() > 1e-8 {
        ts / dx.abs()
    } else {
        f32::INFINITY
    };

    let t_delta_y = if dy.abs() > 1e-8 {
        ts / dy.abs()
    } else {
        f32::INFINITY
    };

    let max_steps = ((ray.max_dist / ts) * 2.5).ceil() as usize + 8;
    for _ in 0..max_steps {
        let traveled = if t_max_x < t_max_y {
            let t = t_max_x;
            current_gx += step_x;
            t_max_x += t_delta_x;
            t
        } else {
            let t = t_max_y;
            current_gy += step_y;
            t_max_y += t_delta_y;
            t
        };

        if traveled > ray.max_dist {
            break;
        }

        if let Some(tile) = tilemap.get_tile(current_gx, current_gy) {
            if predicate(tile) {
                let center = tilemap.grid_to_world(current_gx, current_gy);
                if let Some(hit) = ray_intersect_aabb(ray, center, Vec2::splat(ts)) {
                    if hit.dist <= ray.max_dist {
                        return Some(hit);
                    }
                }
            }
        }
    }

    None
}

/// 3 Boyutlu Işın (Ray3D)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray3D {
    pub origin: Vec3,
    pub dir: Vec3,
    pub max_dist: f32,
}

impl Ray3D {
    /// Yeni bir 3D ışın oluşturur. Yön vektörü otomatik olarak normalize edilir.
    pub fn new(origin: Vec3, dir: Vec3, max_dist: f32) -> Self {
        let len = dir.length();
        let dir = if len > 1e-6 { dir / len } else { Vec3::ZERO };
        Self {
            origin,
            dir,
            max_dist: max_dist.max(0.0),
        }
    }

    /// Işın üzerinde belirli bir mesafedeki konumu döndürür.
    #[inline]
    pub fn point_at(&self, dist: f32) -> Vec3 {
        self.origin + self.dir * dist
    }
}

/// 3 Boyutlu Işın Kesişim Sonucu (RayHit3D)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit3D {
    /// Kesişimin gerçekleştiği 3D dünya koordinatı
    pub point: Vec3,
    /// Çarpan yüzeyin dışa bakan normal vektörü
    pub normal: Vec3,
    /// Işın başlangıcından kesişim noktasına olan mesafe
    pub dist: f32,
}

/// 3D Işının verilen eksen hizalı sınırlayıcı kutu (AABB3D) ile kesişimini analitik slab yöntemiyle test eder.
pub fn ray_intersect_aabb_3d(ray: &Ray3D, aabb: &AABB3D) -> Option<RayHit3D> {
    if ray.max_dist <= 0.0 || ray.dir == Vec3::ZERO {
        return None;
    }

    let mut t_near = f32::NEG_INFINITY;
    let mut t_far = f32::INFINITY;
    let mut hit_normal = Vec3::ZERO;

    // X Ekseni Dilim Testi
    if ray.dir.x.abs() < 1e-8 {
        if ray.origin.x < aabb.min.x || ray.origin.x > aabb.max.x {
            return None;
        }
    } else {
        let inv_d = 1.0 / ray.dir.x;
        let mut t1 = (aabb.min.x - ray.origin.x) * inv_d;
        let mut t2 = (aabb.max.x - ray.origin.x) * inv_d;
        let mut n1 = Vec3::new(-1.0, 0.0, 0.0);

        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
            n1 = Vec3::new(1.0, 0.0, 0.0);
        }

        if t1 > t_near {
            t_near = t1;
            hit_normal = n1;
        }
        if t2 < t_far {
            t_far = t2;
        }

        if t_near > t_far {
            return None;
        }
    }

    // Y Ekseni Dilim Testi
    if ray.dir.y.abs() < 1e-8 {
        if ray.origin.y < aabb.min.y || ray.origin.y > aabb.max.y {
            return None;
        }
    } else {
        let inv_d = 1.0 / ray.dir.y;
        let mut t1 = (aabb.min.y - ray.origin.y) * inv_d;
        let mut t2 = (aabb.max.y - ray.origin.y) * inv_d;
        let mut n1 = Vec3::new(0.0, -1.0, 0.0);

        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
            n1 = Vec3::new(0.0, 1.0, 0.0);
        }

        if t1 > t_near {
            t_near = t1;
            hit_normal = n1;
        }
        if t2 < t_far {
            t_far = t2;
        }

        if t_near > t_far {
            return None;
        }
    }

    // Z Ekseni Dilim Testi
    if ray.dir.z.abs() < 1e-8 {
        if ray.origin.z < aabb.min.z || ray.origin.z > aabb.max.z {
            return None;
        }
    } else {
        let inv_d = 1.0 / ray.dir.z;
        let mut t1 = (aabb.min.z - ray.origin.z) * inv_d;
        let mut t2 = (aabb.max.z - ray.origin.z) * inv_d;
        let mut n1 = Vec3::new(0.0, 0.0, -1.0);

        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
            n1 = Vec3::new(0.0, 0.0, 1.0);
        }

        if t1 > t_near {
            t_near = t1;
            hit_normal = n1;
        }
        if t2 < t_far {
            t_far = t2;
        }

        if t_near > t_far {
            return None;
        }
    }

    if t_far <= 1e-6 {
        return None;
    }

    let (dist, point, normal) = if t_near < 0.0 {
        // Işın başlangıcı AABB'nin içinde
        (0.0, ray.origin, -ray.dir)
    } else {
        (t_near, ray.origin + ray.dir * t_near, hit_normal)
    };

    if dist > ray.max_dist {
        return None;
    }

    Some(RayHit3D {
        point,
        normal,
        dist,
    })
}

/// 3D Işının sınırlayıcı küre (BoundingSphere) ile kesişimini analitik kuadratik yöntemle test eder.
pub fn ray_intersect_sphere(ray: &Ray3D, sphere: &BoundingSphere) -> Option<RayHit3D> {
    if ray.max_dist <= 0.0 || ray.dir == Vec3::ZERO || sphere.radius <= 0.0 {
        return None;
    }

    let oc = ray.origin - sphere.center;
    let b = oc.dot(ray.dir);
    let c = oc.dot(oc) - sphere.radius * sphere.radius;

    // Işın kürenin dışından başlıyor ve küreden uzağa bakıyorsa kesişemez
    if c > 0.0 && b > 0.0 {
        return None;
    }

    let discriminant = b * b - c;
    if discriminant < 0.0 {
        return None;
    }

    let disc_sqrt = discriminant.sqrt();
    let mut t = -b - disc_sqrt;

    if t < 0.0 {
        // Işın başlangıcı kürenin içinde
        t = -b + disc_sqrt;
        if t < 0.0 || t > ray.max_dist {
            return None;
        }
        let point = ray.point_at(0.0);
        let normal = -ray.dir;
        return Some(RayHit3D {
            point,
            normal,
            dist: 0.0,
        });
    }

    if t > ray.max_dist {
        return None;
    }

    let point = ray.point_at(t);
    let normal = (point - sphere.center).normalize();

    Some(RayHit3D {
        point,
        normal,
        dist: t,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rugeng_core::tilemap::TileType;

    #[test]
    fn test_ray_aabb_horizontal_hit() {
        let ray = Ray2D::new(Vec2::new(0.0, 50.0), Vec2::new(1.0, 0.0), 100.0);
        let center = Vec2::new(50.0, 50.0);
        let size = Vec2::new(20.0, 20.0);

        let hit = ray_intersect_aabb(&ray, center, size).expect("Hit bekleniydi");
        assert_eq!(hit.point, Vec2::new(40.0, 50.0));
        assert_eq!(hit.normal, Vec2::new(-1.0, 0.0));
        assert!((hit.dist - 40.0).abs() < 1e-4);
    }

    #[test]
    fn test_ray_aabb_vertical_hit() {
        let ray = Ray2D::new(Vec2::new(50.0, 100.0), Vec2::new(0.0, -1.0), 100.0);
        let center = Vec2::new(50.0, 50.0);
        let size = Vec2::new(20.0, 20.0);

        let hit = ray_intersect_aabb(&ray, center, size).expect("Hit bekleniyordu");
        assert_eq!(hit.point, Vec2::new(50.0, 60.0));
        assert_eq!(hit.normal, Vec2::new(0.0, 1.0));
        assert!((hit.dist - 40.0).abs() < 1e-4);
    }

    #[test]
    fn test_ray_aabb_miss() {
        let ray = Ray2D::new(Vec2::new(0.0, 10.0), Vec2::new(1.0, 0.0), 100.0);
        let center = Vec2::new(50.0, 50.0);
        let size = Vec2::new(20.0, 20.0);

        assert!(ray_intersect_aabb(&ray, center, size).is_none());
    }

    #[test]
    fn test_ray_aabb_max_dist_exceeded() {
        let ray = Ray2D::new(Vec2::new(0.0, 50.0), Vec2::new(1.0, 0.0), 30.0);
        let center = Vec2::new(50.0, 50.0);
        let size = Vec2::new(20.0, 20.0);

        // Kutu 40.0 mesafede basliyor, max_dist 30.0 oldugu icin isin erisememeli
        assert!(ray_intersect_aabb(&ray, center, size).is_none());
    }

    #[test]
    fn test_raycast_grid_hits_first_solid_tile() {
        let mut map = Tilemap2DComponent::new(32.0);
        // (3, 0) konumuna kati tas karo ekle (X: 96..128, Y: 0..32)
        map.set_tile(3, 0, TileType::Stone, true);
        // (5, 0) konumuna da tas karo ekle (daha uzakta)
        map.set_tile(5, 0, TileType::Stone, true);

        let ray = Ray2D::new(Vec2::new(0.0, 16.0), Vec2::new(1.0, 0.0), 300.0);
        let hit = raycast_grid(&ray, &map).expect("Izgara kesişimi bekleniyordu");

        // Ilk katı karo 3 * 32 = 96.0 noktasında
        assert!((hit.point.x - 96.0).abs() < 1e-3);
        assert!((hit.dist - 96.0).abs() < 1e-3);
        assert_eq!(hit.normal, Vec2::new(-1.0, 0.0));
    }

    #[test]
    fn test_raycast_grid_ignores_empty_or_non_solid() {
        let mut map = Tilemap2DComponent::new(32.0);
        // Su karosu ekle (varsayilan olarak kati degil)
        map.set_tile(2, 0, TileType::Water, false);

        let ray = Ray2D::new(Vec2::new(0.0, 16.0), Vec2::new(1.0, 0.0), 200.0);
        assert!(raycast_grid(&ray, &map).is_none());
    }

    #[test]
    fn test_ray_on_boundary_pointing_outward_misses() {
        let center = Vec2::new(0.0, 0.0);
        let size = Vec2::new(20.0, 20.0); // [-10, 10] x [-10, 10]

        // Sağ sınırda (x=10.0), dışarıya doğru (+X) bakan ışın kutuyu kesmemeli
        let ray = Ray2D::new(Vec2::new(10.0, 0.0), Vec2::new(1.0, 0.0), 50.0);
        assert!(ray_intersect_aabb(&ray, center, size).is_none());

        // Sol sınırda (x=-10.0), dışarıya doğru (-X) bakan ışın kutuyu kesmemeli
        let ray_left = Ray2D::new(Vec2::new(-10.0, 0.0), Vec2::new(-1.0, 0.0), 50.0);
        assert!(ray_intersect_aabb(&ray_left, center, size).is_none());
    }

    #[test]
    fn test_raycast_grid_from_tile_boundary_pointing_away() {
        let mut map = Tilemap2DComponent::new(32.0);
        // (0, 0) katı taş karo (X: 0..32, Y: 0..32)
        map.set_tile(0, 0, TileType::Stone, true);
        // (-2, 0) katı taş karo (X: -64..-32, Y: 0..32)
        map.set_tile(-2, 0, TileType::Stone, true);

        // x=0.0 sınırında, sola doğru (katı karonun dışına) bakan ışın
        // arkasındaki (0,0) karosuna çarpmamalı, (-2, 0) karosuna çarpmalı
        let ray = Ray2D::new(Vec2::new(0.0, 16.0), Vec2::new(-1.0, 0.0), 200.0);
        let hit = raycast_grid(&ray, &map).expect("Ilerideki (-2, 0) karosuna carpilmali");
        assert!((hit.dist - 32.0).abs() < 1e-3);
        assert_eq!(hit.normal, Vec2::new(1.0, 0.0));
    }

    #[test]
    fn test_ray_3d_aabb_hit_and_miss() {
        let aabb = AABB3D::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));

        // +Z yönünden merkeze doğru vuruş
        let ray = Ray3D::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0), 10.0);
        let hit = ray_intersect_aabb_3d(&ray, &aabb).expect("AABB3D vurus bekleniyor");
        assert_eq!(hit.point, Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(hit.normal, Vec3::new(0.0, 0.0, 1.0));
        assert!((hit.dist - 4.0).abs() < 1e-4);

        // Iska
        let miss_ray = Ray3D::new(Vec3::new(5.0, 5.0, 5.0), Vec3::new(0.0, 0.0, -1.0), 10.0);
        assert!(ray_intersect_aabb_3d(&miss_ray, &aabb).is_none());

        // Max mesafe yetersiz
        let short_ray = Ray3D::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0), 2.0);
        assert!(ray_intersect_aabb_3d(&short_ray, &aabb).is_none());
    }

    #[test]
    fn test_ray_3d_sphere_hit_and_miss() {
        let sphere = BoundingSphere::new(Vec3::new(0.0, 2.0, 0.0), 1.5);

        // +Y yönünden tepeye vuruş
        let ray = Ray3D::new(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.0, -1.0, 0.0), 20.0);
        let hit = ray_intersect_sphere(&ray, &sphere).expect("Kure vurus bekleniyor");
        assert_eq!(hit.point, Vec3::new(0.0, 3.5, 0.0));
        assert_eq!(hit.normal, Vec3::new(0.0, 1.0, 0.0));
        assert!((hit.dist - 6.5).abs() < 1e-4);

        // Küreyi teğet geçen veya ıskalayan ışın
        let miss_ray = Ray3D::new(Vec3::new(3.0, 10.0, 0.0), Vec3::new(0.0, -1.0, 0.0), 20.0);
        assert!(ray_intersect_sphere(&miss_ray, &sphere).is_none());
    }
}
