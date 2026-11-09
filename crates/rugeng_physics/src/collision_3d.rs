//! 3D Çarpışma Geometrileri ve Çözücüleri (AABB, OBB, Kapsül, Küre).
//!
//! Minimum Translation Vector (MTV) tabanlı deterministik ve SIMD dostu çözücüler.

use glam::{Mat3, Quat, Vec3};
use rugeng_core::{BoundingSphere, AABB3D};

/// 3D Yönlendirilmiş Sınırlayıcı Kutu (Oriented Bounding Box)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OBB3D {
    pub center: Vec3,
    pub half_extents: Vec3,
    pub axes: [Vec3; 3], // Yerel X, Y, Z birim eksenleri
}

impl OBB3D {
    pub fn new(center: Vec3, half_extents: Vec3, rotation: Quat) -> Self {
        let rot_mat = Mat3::from_quat(rotation);
        Self {
            center,
            half_extents,
            axes: [
                rot_mat.x_axis.normalize(),
                rot_mat.y_axis.normalize(),
                rot_mat.z_axis.normalize(),
            ],
        }
    }

    pub fn from_aabb(aabb: &AABB3D) -> Self {
        Self {
            center: aabb.center(),
            half_extents: aabb.half_extents(),
            axes: [Vec3::X, Vec3::Y, Vec3::Z],
        }
    }
}

/// 3D Kapsül Collider (Capsule)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capsule3D {
    pub start: Vec3,
    pub end: Vec3,
    pub radius: f32,
}

impl Capsule3D {
    pub fn new(start: Vec3, end: Vec3, radius: f32) -> Self {
        Self {
            start,
            end,
            radius: radius.max(0.0),
        }
    }

    pub fn from_center_height(center: Vec3, height: f32, radius: f32) -> Self {
        let half_h = (height * 0.5 - radius).max(0.0);
        Self {
            start: center - Vec3::new(0.0, half_h, 0.0),
            end: center + Vec3::new(0.0, half_h, 0.0),
            radius: radius.max(0.0),
        }
    }
}

/// 3D Çarpışma Temas Bilgisi (Collision Manifold)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionManifold3D {
    /// A nesnesinden B nesnesine doğru yönelen temas normali
    pub normal: Vec3,
    /// Penetrasyon / İç içe geçme derinliği (pozitif)
    pub penetration: f32,
    /// Çarpışmanın gerçekleştiği yaklaşık temas noktası
    pub contact_point: Vec3,
}

/// İki AABB3D kutusu arasındaki penetrasyonu test eder ve Minimum Translation Vector (MTV) üretir.
/// Geri dönen normal A'dan B'ye doğrudur. A'yı B'den ayırmak için A'nın konumu `-normal * penetration` kadar ötelenmelidir.
pub fn resolve_aabb3d_collision(a: &AABB3D, b: &AABB3D) -> Option<CollisionManifold3D> {
    let overlap_x = (a.max.x.min(b.max.x) - a.min.x.max(b.min.x)).max(0.0);
    let overlap_y = (a.max.y.min(b.max.y) - a.min.y.max(b.min.y)).max(0.0);
    let overlap_z = (a.max.z.min(b.max.z) - a.min.z.max(b.min.z)).max(0.0);

    // Tüm eksenlerde pozitif overlap olmalı
    if overlap_x <= 0.0 || overlap_y <= 0.0 || overlap_z <= 0.0 {
        return None;
    }

    let a_center = a.center();
    let b_center = b.center();
    let dir = b_center - a_center;

    // Minimum penetrasyon eksenini bul (MTV - Minimum Translation Vector)
    if overlap_x <= overlap_y && overlap_x <= overlap_z {
        let sign = if dir.x >= 0.0 { 1.0 } else { -1.0 };
        let normal = Vec3::new(sign, 0.0, 0.0);
        let contact_x = if sign > 0.0 { a.max.x } else { a.min.x };
        let contact_point = Vec3::new(
            contact_x,
            (a_center.y + b_center.y) * 0.5,
            (a_center.z + b_center.z) * 0.5,
        );
        Some(CollisionManifold3D {
            normal,
            penetration: overlap_x,
            contact_point,
        })
    } else if overlap_y <= overlap_x && overlap_y <= overlap_z {
        let sign = if dir.y >= 0.0 { 1.0 } else { -1.0 };
        let normal = Vec3::new(0.0, sign, 0.0);
        let contact_y = if sign > 0.0 { a.max.y } else { a.min.y };
        let contact_point = Vec3::new(
            (a_center.x + b_center.x) * 0.5,
            contact_y,
            (a_center.z + b_center.z) * 0.5,
        );
        Some(CollisionManifold3D {
            normal,
            penetration: overlap_y,
            contact_point,
        })
    } else {
        let sign = if dir.z >= 0.0 { 1.0 } else { -1.0 };
        let normal = Vec3::new(0.0, 0.0, sign);
        let contact_z = if sign > 0.0 { a.max.z } else { a.min.z };
        let contact_point = Vec3::new(
            (a_center.x + b_center.x) * 0.5,
            (a_center.y + b_center.y) * 0.5,
            contact_z,
        );
        Some(CollisionManifold3D {
            normal,
            penetration: overlap_z,
            contact_point,
        })
    }
}

/// İki 3D küre arasındaki çarpışmayı çözer.
/// Normal A'dan B'ye doğrudur.
pub fn resolve_sphere3d_collision(
    a: &BoundingSphere,
    b: &BoundingSphere,
) -> Option<CollisionManifold3D> {
    let diff = b.center - a.center;
    let dist_sq = diff.length_squared();
    let total_radius = a.radius + b.radius;

    if dist_sq >= total_radius * total_radius {
        return None;
    }

    let dist = dist_sq.sqrt();
    let (normal, penetration) = if dist > 1e-6 {
        (diff / dist, total_radius - dist)
    } else {
        (Vec3::Y, total_radius)
    };

    let contact_point = a.center + normal * (a.radius - penetration * 0.5);

    Some(CollisionManifold3D {
        normal,
        penetration,
        contact_point,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_aabb3d_penetration_minimum_translation() {
        // A kutusu: merkez (0, 0, 0), boyut 2x2x2 -> min [-1, -1, -1], max [1, 1, 1]
        let a = AABB3D::from_center_half_extents(Vec3::ZERO, Vec3::splat(1.0));

        // B kutusu: merkez (1.8, 0, 0), boyut 2x2x2 -> min [0.8, -1, -1], max [2.8, 1, 1]
        // X overlap: 1.0 - 0.8 = 0.2
        // Y overlap: 2.0
        // Z overlap: 2.0
        // Minimum translation vector X ekseninde 0.2 olmalı
        let b = AABB3D::from_center_half_extents(Vec3::new(1.8, 0.0, 0.0), Vec3::splat(1.0));

        let manifold = resolve_aabb3d_collision(&a, &b).expect("Carpisma olmali");
        assert!((manifold.penetration - 0.2).abs() < 1e-5);
        assert_eq!(manifold.normal, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_sphere_sphere_3d_response() {
        let a = BoundingSphere::new(Vec3::ZERO, 1.0);
        let b = BoundingSphere::new(Vec3::new(0.0, 1.5, 0.0), 1.0);

        let manifold = resolve_sphere3d_collision(&a, &b).expect("Kureler carpismali");
        assert!((manifold.penetration - 0.5).abs() < 1e-5);
        assert!((manifold.normal.y - 1.0).abs() < 1e-5);
        assert!((manifold.contact_point.y - 0.75).abs() < 1e-5);
    }
}
