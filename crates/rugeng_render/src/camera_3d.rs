//! 3D Işık ve Kamera Tanımları (Directional Light, Ambient Light, Perspective Camera).

use glam::{Mat4, Vec3};

/// Yönlü Işık Kaynağı (Güneş / Sonsuz Uzaklık)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectionalLight {
    /// Işığın geldiği yön vektörü (ışık kaynağına doğru normalize yön, örn: normalize(1.0, 2.0, 1.0))
    pub direction: Vec3,
    /// Işık rengi (RGB f32: 0.0 .. 1.0)
    pub color: [f32; 3],
    /// Parlaklık katsayısı
    pub intensity: f32,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vec3::new(0.5, 1.0, 0.7).normalize(),
            color: [1.0, 0.98, 0.9],
            intensity: 1.0,
        }
    }
}

impl DirectionalLight {
    pub fn new(direction: Vec3, color: [f32; 3], intensity: f32) -> Self {
        Self {
            direction: direction.normalize(),
            color,
            intensity,
        }
    }
}

/// Ortam Işığı (Ambient Light)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AmbientLight {
    pub color: [f32; 3],
    pub intensity: f32,
}

impl Default for AmbientLight {
    fn default() -> Self {
        Self {
            color: [0.15, 0.18, 0.22],
            intensity: 0.25,
        }
    }
}

impl AmbientLight {
    pub fn new(color: [f32; 3], intensity: f32) -> Self {
        Self { color, intensity }
    }
}

/// 3D Perspektif Kamera (Vulkan Sağ El Kuralı ve [0, 1] derinlik uyumlu)
#[derive(Clone, Copy, Debug)]
pub struct Camera3D {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y_radians: f32,
    pub aspect_ratio: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            eye: Vec3::new(0.0, 3.0, 6.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov_y_radians: 60.0f32.to_radians(),
            aspect_ratio: 16.0 / 9.0,
            z_near: 0.1,
            z_far: 1000.0,
        }
    }
}

impl Camera3D {
    pub fn new(
        eye: Vec3,
        target: Vec3,
        up: Vec3,
        fov_y_radians: f32,
        aspect_ratio: f32,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        Self {
            eye,
            target,
            up,
            fov_y_radians,
            aspect_ratio,
            z_near,
            z_far,
        }
    }

    /// Bakış (View) matrisini üretir (`Mat4::look_at_rh`).
    #[inline(always)]
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye, self.target, self.up)
    }

    /// Vulkan NDC standardına uygun ([0, 1] Z aralığı) perspektif projeksiyon matrisi.
    #[inline(always)]
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(
            self.fov_y_radians,
            self.aspect_ratio,
            self.z_near,
            self.z_far,
        )
    }

    /// View-Projection matrisi (`Proj * View`).
    #[inline(always)]
    pub fn view_projection(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_matrices() {
        let cam = Camera3D::default();
        let view = cam.view_matrix();
        let proj = cam.projection_matrix();
        let vp = cam.view_projection();

        assert_eq!(vp, proj * view);

        // Hedef nokta view uzayında (0, 0, -dist) olmalı
        let target_in_view = view.transform_point3(cam.target);
        assert!(target_in_view.z < 0.0);
    }
}
