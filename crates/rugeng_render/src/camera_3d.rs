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

    /// Işık yönüne göre gölge haritası bakış matrisi (LookAt) üretir.
    /// `center`: Gölge hacminin odaklandığı dünya koordinatı (örn: sahne merkezi).
    /// `distance`: Işık kamerasının odak merkezine olan mesafesi.
    #[inline]
    pub fn light_view_matrix(&self, center: Vec3, distance: f32) -> Mat4 {
        let light_dir = if self.direction.length_squared() < 1e-6 {
            Vec3::new(0.5, 1.0, 0.7).normalize()
        } else {
            self.direction.normalize()
        };
        let eye = center + light_dir * distance;
        // Eğer ışık yönü tam Y eksenine paralel ise Up vektörünü Z olarak seç
        let up = if light_dir.dot(Vec3::Y).abs() > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        Mat4::look_at_rh(eye, center, up)
    }

    /// Yönlü ışık için ortografik projeksiyon matrisi üretir (Vulkan RH, Z in [0, 1]).
    /// `ortho_size`: Gölge kutusunun yarı genişliği/yüksekliği.
    /// `z_near` / `z_far`: Yakın ve uzak kırpma düzlemleri.
    #[inline]
    pub fn light_projection_matrix(&self, ortho_size: f32, z_near: f32, z_far: f32) -> Mat4 {
        let s = ortho_size.max(0.1);
        Mat4::orthographic_rh(-s, s, -s, s, z_near, z_far)
    }

    /// Işık uzayı View-Projection matrisini üretir (`Proj * View`).
    #[inline]
    pub fn light_space_matrix(
        &self,
        center: Vec3,
        distance: f32,
        ortho_size: f32,
        z_near: f32,
        z_far: f32,
    ) -> Mat4 {
        let view = self.light_view_matrix(center, distance);
        let proj = self.light_projection_matrix(ortho_size, z_near, z_far);
        proj * view
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

/// 3. Şahıs SpringArm / Kamera Çarpışma ve Sönümleme Kolu (SpringArm3D)
#[derive(Clone, Copy, Debug)]
pub struct SpringArm3D {
    /// Hedeflenen maksimum kol uzunluğu (mesafe)
    pub target_length: f32,
    /// Mevcut sönümlenmiş kol uzunluğu
    pub current_length: f32,
    /// Minimum izin verilen kol uzunluğu (karakter içine girmesini önler)
    pub min_length: f32,
    /// Sönümleme / yumuşatma hızı (lerp speed)
    pub smooth_speed: f32,
    /// Kamera küre yarıçapı (duvar penetrasyonunu önleme toleransı)
    pub probe_radius: f32,
}

impl Default for SpringArm3D {
    fn default() -> Self {
        Self {
            target_length: 5.0,
            current_length: 5.0,
            min_length: 0.5,
            smooth_speed: 15.0,
            probe_radius: 0.2,
        }
    }
}

impl SpringArm3D {
    pub fn new(target_length: f32, min_length: f32) -> Self {
        Self {
            target_length,
            current_length: target_length,
            min_length,
            smooth_speed: 15.0,
            probe_radius: 0.2,
        }
    }

    /// Hedef noktadan (`origin`), verilen bakış yönünün tersine doğru raycast / engel kontrolü yapar.
    /// Eğer engel bulunursa kol uzunluğunu kırpar ve kamera duvara gömülmez.
    pub fn update_with_hit(&mut self, dt: f32, hit_distance: Option<f32>) {
        let desired = match hit_distance {
            Some(dist) => (dist - self.probe_radius).clamp(self.min_length, self.target_length),
            None => self.target_length,
        };

        if dt <= 0.0 {
            self.current_length = desired;
            return;
        }

        // Eğer aniden duvara çarptıysak hemen yakınlaş (anlık geri çekilme),
        // Duvar bittiğinde ise yumuşakça (lerp) eski mesafeye açıl
        if desired < self.current_length {
            self.current_length = desired;
        } else {
            let t = (dt * self.smooth_speed).min(1.0);
            self.current_length = self.current_length + (desired - self.current_length) * t;
        }
    }
}

/// 3. Şahıs Serbest Yörünge Kamerası (OrbitCamera3D)
#[derive(Clone, Copy, Debug)]
pub struct OrbitCamera3D {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub spring_arm: SpringArm3D,
    pub sensitivity: f32,
    pub min_pitch: f32,
    pub max_pitch: f32,
}

impl Default for OrbitCamera3D {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: 0.35,
            spring_arm: SpringArm3D::default(),
            sensitivity: 0.003,
            min_pitch: -1.4, // ~ -80 derece
            max_pitch: 1.4,  // ~ +80 derece
        }
    }
}

impl OrbitCamera3D {
    pub fn new(target: Vec3, arm_length: f32) -> Self {
        Self {
            target,
            spring_arm: SpringArm3D::new(arm_length, 0.5),
            ..Default::default()
        }
    }

    /// Fare hareketiyle yaw ve pitch açılarını günceller.
    pub fn handle_mouse_delta(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * self.sensitivity;
        self.pitch = (self.pitch - dy * self.sensitivity).clamp(self.min_pitch, self.max_pitch);
    }

    /// Kameranın ileri bakış yön vektörünü hesaplar (target -> eye'ın baktığı yön).
    #[inline]
    pub fn forward(&self) -> Vec3 {
        let cos_pitch = self.pitch.cos();
        Vec3::new(
            cos_pitch * self.yaw.cos(),
            self.pitch.sin(),
            cos_pitch * self.yaw.sin(),
        )
        .normalize()
    }

    /// Kameranın sağ vektörü
    #[inline]
    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    /// Kameranın dünya konumunu hesaplar (target - forward * arm_length).
    #[inline]
    pub fn eye_position(&self) -> Vec3 {
        self.target - self.forward() * self.spring_arm.current_length
    }

    /// Render için `Camera3D` nesnesi üretir.
    pub fn to_camera_3d(&self, aspect_ratio: f32, fov_degrees: f32) -> Camera3D {
        Camera3D::new(
            self.eye_position(),
            self.target,
            Vec3::Y,
            fov_degrees.to_radians(),
            aspect_ratio,
            0.1,
            1000.0,
        )
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

    #[test]
    fn test_directional_light_shadow_matrices() {
        let sun = DirectionalLight::default();
        let center = Vec3::new(0.0, 1.0, 0.0);
        let dist = 20.0;
        let ortho = 15.0;
        let z_near = 0.5;
        let z_far = 50.0;

        let light_view = sun.light_view_matrix(center, dist);
        let light_proj = sun.light_projection_matrix(ortho, z_near, z_far);
        let light_space = sun.light_space_matrix(center, dist, ortho, z_near, z_far);

        assert_eq!(light_space, light_proj * light_view);

        // Merkez nokta view matrisinde tam z negatif ekseninde olmalı
        let c_in_light = light_view.transform_point3(center);
        assert!((c_in_light.x).abs() < 1e-4);
        assert!((c_in_light.y).abs() < 1e-4);
        assert!(c_in_light.z < 0.0);
    }

    #[test]
    fn test_spring_arm_camera_collision_clamping() {
        let mut arm = SpringArm3D::new(5.0, 0.5);
        assert_eq!(arm.current_length, 5.0);

        // 1. Engel yokken hedef mesafede kalmalı
        arm.update_with_hit(0.1, None);
        assert!((arm.current_length - 5.0).abs() < 1e-4);

        // 2. 2.0 metre mesafede bir duvar tespit edildiğinde probe_radius (0.2) çıkarılarak anında 1.8m'ye kenetlenmeli
        arm.update_with_hit(0.016, Some(2.0));
        assert!(
            (arm.current_length - 1.8).abs() < 1e-4,
            "Engel tespitinde yay kolu aninda kisalmali, len: {}",
            arm.current_length
        );

        // 3. Çok yakın bir duvarda min_length (0.5m) altına düşmemeli
        arm.update_with_hit(0.016, Some(0.3));
        assert!(
            (arm.current_length - 0.5).abs() < 1e-4,
            "Kol uzunlugu min_length (0.5) altina inmemeli"
        );

        // 4. Engel kalktığında eski 5.0m mesafesine yumuşakça (lerp) geri açılmalı
        arm.update_with_hit(0.1, None);
        assert!(
            arm.current_length > 0.5 && arm.current_length <= 5.0,
            "Engel kalkinca mesafe artmaya baslamali"
        );
    }
}
