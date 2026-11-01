//! 3D Transform, Rotasyon (Kuaterniyon), Ölçek ve Hiyerarşik Yayılım.

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// 3D Pozisyon, Rotasyon (Kuaterniyon) ve Ölçek bileşeni
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transform3D {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform3D {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform3D {
    #[inline(always)]
    pub fn new(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }

    #[inline(always)]
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    #[inline(always)]
    pub fn from_translation_rotation(translation: Vec3, rotation: Quat) -> Self {
        Self {
            translation,
            rotation,
            scale: Vec3::ONE,
        }
    }

    #[inline(always)]
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self::from_translation(Vec3::new(x, y, z))
    }

    /// Yerel model dönüşüm matrisini üretir (`Translation * Rotation * Scale`).
    #[inline(always)]
    pub fn local_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Euler açılarından (radyan cinsinden pitch, yaw, roll) kuaterniyon oluşturarak rotasyonu ayarlar.
    #[inline(always)]
    pub fn set_rotation_euler(&mut self, pitch: f32, yaw: f32, roll: f32) {
        self.rotation = Quat::from_euler(glam::EulerRot::YXZ, yaw, pitch, roll);
    }
}

/// Ebeveyn ve çocuk Transform3D verilerini hiyerarşik olarak birleştirerek dünya matrisini hesaplar.
///
/// `parent_world_matrix * local_child_matrix`
#[inline(always)]
pub fn compute_world_transform_3d(parent_world: Mat4, child_local: &Transform3D) -> Mat4 {
    parent_world * child_local.local_matrix()
}

/// 3D Eksen Hizalı Sınırlayıcı Kutu (Axis-Aligned Bounding Box)
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AABB3D {
    pub min: Vec3,
    pub max: Vec3,
}

impl Default for AABB3D {
    fn default() -> Self {
        Self {
            min: Vec3::splat(-0.5),
            max: Vec3::splat(0.5),
        }
    }
}

impl AABB3D {
    #[inline(always)]
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    #[inline(always)]
    pub fn from_center_half_extents(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    #[inline(always)]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    #[inline(always)]
    pub fn extents(&self) -> Vec3 {
        self.max - self.min
    }

    #[inline(always)]
    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    /// Matris dönüşümü uygulanmış yeni AABB'yi hesaplar.
    pub fn transform(&self, transform: Mat4) -> Self {
        let corners = [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ];

        let mut new_min = Vec3::splat(f32::INFINITY);
        let mut new_max = Vec3::splat(f32::NEG_INFINITY);

        for c in corners {
            let transformed = transform.transform_point3(c);
            new_min = new_min.min(transformed);
            new_max = new_max.max(transformed);
        }

        Self {
            min: new_min,
            max: new_max,
        }
    }
}

/// 3D Sınırlayıcı Küre (Bounding Sphere)
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Default for BoundingSphere {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            radius: 1.0,
        }
    }
}

impl BoundingSphere {
    #[inline(always)]
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    /// AABB3D'yi kapsayan küre üretir.
    pub fn from_aabb(aabb: &AABB3D) -> Self {
        let center = aabb.center();
        let radius = (aabb.max - center).length();
        Self { center, radius }
    }

    /// Matris ile dönüştürülmüş küre (maksimum eksen ölçeği ile yarıçap ölçeklenir).
    pub fn transform(&self, transform: Mat4) -> Self {
        let center = transform.transform_point3(self.center);
        let scale_x = transform.x_axis.truncate().length();
        let scale_y = transform.y_axis.truncate().length();
        let scale_z = transform.z_axis.truncate().length();
        let max_scale = scale_x.max(scale_y).max(scale_z);
        Self {
            center,
            radius: self.radius * max_scale,
        }
    }
}

/// Kamera Görüş Hacmi (Frustum Düzlemleri: sol, sağ, alt, üst, yakın, uzak)
#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    /// 6 düzlem denklemi: dot(normal, p) + d = 0 (normalize edilmiş nx, ny, nz, d)
    pub planes: [glam::Vec4; 6],
}

impl Frustum {
    /// View-Projection matrisinden Frustum düzlemlerini çıkarır (Gribb/Hartmann algoritması).
    pub fn from_view_projection(vp: Mat4) -> Self {
        let row0 = vp.row(0);
        let row1 = vp.row(1);
        let row2 = vp.row(2);
        let row3 = vp.row(3);

        let mut planes = [
            row3 + row0, // Sol
            row3 - row0, // Sağ
            row3 + row1, // Alt
            row3 - row1, // Üst
            row2,        // Yakın (Vulkan NDC: 0 <= z <= w)
            row3 - row2, // Uzak
        ];

        // Düzlem normallerini normalize et
        for p in &mut planes {
            let normal_len = Vec3::new(p.x, p.y, p.z).length();
            if normal_len > 1e-6 {
                *p /= normal_len;
            }
        }

        Self { planes }
    }

    /// BoundingSphere frustum içinde veya kesişiyor mu?
    #[inline(always)]
    pub fn intersects_sphere(&self, sphere: &BoundingSphere) -> bool {
        for p in &self.planes {
            let dist = p.x * sphere.center.x + p.y * sphere.center.y + p.z * sphere.center.z + p.w;
            if dist < -sphere.radius {
                return false;
            }
        }
        true
    }

    /// AABB3D frustum içinde veya kesişiyor mu?
    pub fn intersects_aabb(&self, aabb: &AABB3D) -> bool {
        for p in &self.planes {
            // p-vertex: düzlem normali yönündeki en uzak köşe
            let px = if p.x >= 0.0 { aabb.max.x } else { aabb.min.x };
            let py = if p.y >= 0.0 { aabb.max.y } else { aabb.min.y };
            let pz = if p.z >= 0.0 { aabb.max.z } else { aabb.min.z };

            let dist = p.x * px + p.y * py + p.z * pz + p.w;
            if dist < 0.0 {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_3d_local_matrix_and_hierarchy() {
        let parent = Transform3D::from_xyz(10.0, 5.0, -2.0);
        let child = Transform3D::from_xyz(0.0, 2.0, 0.0);

        let parent_mat = parent.local_matrix();
        let world_child_mat = compute_world_transform_3d(parent_mat, &child);

        let child_origin_transformed = world_child_mat.transform_point3(Vec3::ZERO);
        assert!((child_origin_transformed.x - 10.0).abs() < 1e-5);
        assert!((child_origin_transformed.y - 7.0).abs() < 1e-5);
        assert!((child_origin_transformed.z - (-2.0)).abs() < 1e-5);
    }

    #[test]
    fn test_frustum_sphere_and_aabb_culling() {
        // Vulkan perspektif projeksiyonu (z: 0.1 .. 100.0)
        let proj = Mat4::perspective_rh(60.0f32.to_radians(), 16.0 / 9.0, 0.1, 100.0);
        let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
        let vp = proj * view;

        let frustum = Frustum::from_view_projection(vp);

        // Orijindeki nesne frustum içinde olmalı
        let sphere_inside = BoundingSphere::new(Vec3::ZERO, 1.0);
        assert!(frustum.intersects_sphere(&sphere_inside));

        // Kameranın arkasındaki nesne elenmeli (z = 10.0, kamera z=5 ve bakış yönü -z)
        let sphere_behind = BoundingSphere::new(Vec3::new(0.0, 0.0, 15.0), 1.0);
        assert!(!frustum.intersects_sphere(&sphere_behind));

        // Çok uzaktaki nesne elenmeli (z = -200.0, far=100.0)
        let sphere_too_far = BoundingSphere::new(Vec3::new(0.0, 0.0, -200.0), 1.0);
        assert!(!frustum.intersects_sphere(&sphere_too_far));

        // AABB Testi
        let aabb_inside = AABB3D::new(Vec3::new(-1.0, -1.0, -2.0), Vec3::new(1.0, 1.0, 0.0));
        assert!(frustum.intersects_aabb(&aabb_inside));

        let aabb_outside = AABB3D::new(Vec3::new(500.0, 0.0, 0.0), Vec3::new(510.0, 1.0, 1.0));
        assert!(!frustum.intersects_aabb(&aabb_outside));

        // Kameranın arkasındaki AABB
        let aabb_behind = AABB3D::new(Vec3::new(-1.0, -1.0, 10.0), Vec3::new(1.0, 1.0, 12.0));
        assert!(!frustum.intersects_aabb(&aabb_behind));

        // Sol sınırın dışındaki AABB
        let aabb_left = AABB3D::new(Vec3::new(-100.0, -1.0, -5.0), Vec3::new(-80.0, 1.0, -4.0));
        assert!(!frustum.intersects_aabb(&aabb_left));
    }
}
