//! 3D Kinematik Karakter Kontrolcüsü (CharacterController3D).
//!
//! Kapsül/AABB tabanlı kinematik karakter kontrolü:
//! - Yerçekimi ve dikey hız entegrasyonu
//! - Statik/kinematik dünya geometrisi (AABB3D) ile çarpışma tespiti ve MTV ayrıştırması
//! - Zemin tespiti (`is_on_ground`) ve zemin normali boyunca kayma
//! - İvmelenme, sürtünme ve zıplama mekaniği.

use crate::collision_3d::resolve_aabb3d_collision;
use glam::Vec3;
use rugeng_core::AABB3D;

/// 3D Kinematik Karakter Kontrolcüsü Konfigürasyonu
#[derive(Clone, Copy, Debug)]
pub struct CharacterController3DConfig {
    /// Karakterin AABB/kapsül yarı boyutları (half extents)
    pub half_extents: Vec3,
    /// Yerçekimi ivmesi (m/s^2, genellikle negatif Y, örn: -19.62)
    pub gravity: Vec3,
    /// Maksimum yatay hareket hızı (m/s)
    pub max_speed: f32,
    /// Yatay ivmelenme katsayısı (m/s^2)
    pub acceleration: f32,
    /// Zemin sürtünmesi / yavaşlama katsayısı
    pub friction: f32,
    /// Havadaki yönlendirme kontrol katsayısı (0.0 .. 1.0)
    pub air_control: f32,
    /// Zıplama dikey hızı (m/s)
    pub jump_speed: f32,
    /// Karakterin tırmanabileceği maksimum zemin eğim açısı kosinüsü (örn: 45 derece -> cos(45) ~ 0.707)
    pub min_ground_normal_y: f32,
    /// Zemin kontrolü için aşağı doğru tolerans mesafesi
    pub ground_snap_distance: f32,
}

impl Default for CharacterController3DConfig {
    fn default() -> Self {
        Self {
            half_extents: Vec3::new(0.4, 0.9, 0.4), // 0.8m genişlik, 1.8m boy, 0.8m derinlik
            gravity: Vec3::new(0.0, -19.62, 0.0),
            max_speed: 6.0,
            acceleration: 40.0,
            friction: 12.0,
            air_control: 0.3,
            jump_speed: 7.5,
            min_ground_normal_y: 0.707, // ~45 derece
            ground_snap_distance: 0.05,
        }
    }
}

/// 3D Kinematik Karakter Kontrolcüsü Durumu
#[derive(Clone, Debug)]
pub struct CharacterController3D {
    pub position: Vec3,
    pub velocity: Vec3,
    pub is_on_ground: bool,
    pub ground_normal: Vec3,
    pub config: CharacterController3DConfig,
}

impl CharacterController3D {
    pub fn new(position: Vec3, config: CharacterController3DConfig) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            is_on_ground: false,
            ground_normal: Vec3::Y,
            config,
        }
    }

    /// Karakterin mevcut sınırlayıcı kutusunu (AABB) üretir.
    #[inline]
    pub fn aabb(&self) -> AABB3D {
        AABB3D::from_center_half_extents(self.position, self.config.half_extents)
    }

    /// Zıplama komutu verir (Yalnızca zemindeyse uygulanır).
    pub fn jump(&mut self) -> bool {
        if self.is_on_ground {
            self.velocity.y = self.config.jump_speed;
            self.is_on_ground = false;
            true
        } else {
            false
        }
    }

    /// Bir simülasyon adımı (dt) ilerletir.
    /// `desired_move_dir`: Normalize edilmiş yatay hareket yönü (XZ düzleminde).
    /// `colliders`: Dünyadaki katı AABB3D engelleri.
    pub fn move_and_slide(&mut self, desired_move_dir: Vec3, dt: f32, colliders: &[AABB3D]) {
        if dt <= 0.0 {
            return;
        }

        // 1. İvmelenme ve sürtünme hesabı (Yatay hız: X ve Z)
        let accel_rate = if self.is_on_ground {
            self.config.acceleration
        } else {
            self.config.acceleration * self.config.air_control
        };

        let target_horiz_vel = desired_move_dir * self.config.max_speed;
        let mut horiz_vel = Vec3::new(self.velocity.x, 0.0, self.velocity.z);

        if desired_move_dir.length_squared() > 1e-4 {
            let diff = target_horiz_vel - horiz_vel;
            let max_change = accel_rate * dt;
            if diff.length() <= max_change {
                horiz_vel = target_horiz_vel;
            } else {
                horiz_vel += diff.normalize() * max_change;
            }
        } else if self.is_on_ground {
            // Zemin sürtünmesi ile durdurma
            let speed = horiz_vel.length();
            let drop = speed * self.config.friction * dt;
            let new_speed = (speed - drop).max(0.0);
            if speed > 1e-4 {
                horiz_vel *= new_speed / speed;
            } else {
                horiz_vel = Vec3::ZERO;
            }
        }

        self.velocity.x = horiz_vel.x;
        self.velocity.z = horiz_vel.z;

        // 2. Yerçekimi uygulama
        if !self.is_on_ground {
            self.velocity += self.config.gravity * dt;
            // Terminal hız sınırı
            if self.velocity.y < -40.0 {
                self.velocity.y = -40.0;
            }
        }

        // 3. Eksen ayrıştırmalı hareket ve çarpışma çözümü (Slide along normal)
        // Karakterin hareketini Y (dikey) ve XZ (yatay) olarak ayrı ayrı çözmek tünelleme ve basamak takılmalarını önler.
        self.is_on_ground = false;
        self.ground_normal = Vec3::Y;

        // Dikey hareket (Y)
        self.position.y += self.velocity.y * dt;
        let mut char_box = self.aabb();
        for obstacle in colliders {
            if let Some(manifold) = resolve_aabb3d_collision(&char_box, obstacle) {
                // manifold.normal A (char_box)'dan B (obstacle)'ye doğrudur.
                // A'yı ayırmak için A konumu -manifold.normal * penetration kadar ötelenmelidir.
                self.position -= manifold.normal * manifold.penetration;
                char_box = self.aabb();

                // Eğer manifold.normal.y < 0 ise engel karakterin altındadır (zemin).
                // Engelin karaktere uyguladığı zemin tepki normali -manifold.normal'dir.
                let obstacle_to_char_normal = -manifold.normal;
                if obstacle_to_char_normal.y > self.config.min_ground_normal_y {
                    self.is_on_ground = true;
                    self.ground_normal = obstacle_to_char_normal;
                    if self.velocity.y < 0.0 {
                        self.velocity.y = 0.0;
                    }
                } else if obstacle_to_char_normal.y < -0.1 {
                    // Tavana çarpma
                    if self.velocity.y > 0.0 {
                        self.velocity.y = 0.0;
                    }
                }
            }
        }

        // Yatay hareket (XZ)
        let horiz_step = Vec3::new(self.velocity.x * dt, 0.0, self.velocity.z * dt);
        self.position += horiz_step;
        char_box = self.aabb();

        for obstacle in colliders {
            if let Some(manifold) = resolve_aabb3d_collision(&char_box, obstacle) {
                self.position -= manifold.normal * manifold.penetration;
                char_box = self.aabb();

                let obstacle_to_char_normal = -manifold.normal;
                // Zemin kontrolü (Eğimli yüzeyler)
                if obstacle_to_char_normal.y > self.config.min_ground_normal_y {
                    self.is_on_ground = true;
                    self.ground_normal = obstacle_to_char_normal;
                } else {
                    // Duvara çarpışma: Hızı duvar normali boyunca sönümle / kaydır (Slide along wall)
                    let n = if obstacle_to_char_normal.length_squared() > 1e-4 {
                        obstacle_to_char_normal.normalize()
                    } else {
                        Vec3::ZERO
                    };
                    let v_dot_n = self.velocity.dot(n);
                    if v_dot_n < 0.0 {
                        self.velocity -= n * v_dot_n;
                    }
                }
            }
        }

        // 4. Küçük basamak / yerçekimi zemin yapışması (Ground Snap)
        if !self.is_on_ground && self.velocity.y <= 0.0 {
            let snap_test_box = AABB3D::from_center_half_extents(
                self.position - Vec3::new(0.0, self.config.ground_snap_distance, 0.0),
                self.config.half_extents,
            );
            for obstacle in colliders {
                if let Some(manifold) = resolve_aabb3d_collision(&snap_test_box, obstacle) {
                    let obstacle_to_char_normal = -manifold.normal;
                    if obstacle_to_char_normal.y > self.config.min_ground_normal_y {
                        self.position.y = obstacle.max.y + self.config.half_extents.y;
                        self.is_on_ground = true;
                        self.ground_normal = obstacle_to_char_normal;
                        self.velocity.y = 0.0;
                        break;
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
    fn test_character_controller_gravity_and_ground_clamp() {
        let config = CharacterController3DConfig::default();
        let mut controller = CharacterController3D::new(Vec3::new(0.0, 5.0, 0.0), config);

        // Zemin kutusu: Y = [-1.0, 0.0], X = [-10, 10], Z = [-10, 10]
        let ground = AABB3D::new(Vec3::new(-10.0, -1.0, -10.0), Vec3::new(10.0, 0.0, 10.0));
        let colliders = [ground];

        // Başlangıçta havada olmalı
        assert!(!controller.is_on_ground);

        // Birkaç kare simülasyon adımı ilerlet
        for _ in 0..60 {
            controller.move_and_slide(Vec3::ZERO, 1.0 / 60.0, &colliders);
        }

        // Karakter zemine inmiş olmalı.
        // ground.max.y = 0.0, half_extents.y = 0.9 -> Karakterin merkezi Y = 0.9 olmalı
        assert!(controller.is_on_ground, "Karakter zemine inmeli");
        assert!(
            (controller.position.y - 0.9).abs() < 0.05,
            "Karakter zemin seviyesine kenetlenmeli, pos.y: {}",
            controller.position.y
        );
        assert_eq!(controller.velocity.y, 0.0, "Dikey hiz sifirlanmali");

        // Ziplama testi
        let jumped = controller.jump();
        assert!(jumped, "Zemindeyken ziplayabilmeli");
        assert!(!controller.is_on_ground, "Zipladiktan sonra havalanmali");
        assert!(
            controller.velocity.y > 0.0,
            "Ziplama dikey hizi pozitif olmali"
        );
    }

    #[test]
    fn test_character_controller_slope_sliding() {
        let config = CharacterController3DConfig::default();
        let mut controller = CharacterController3D::new(Vec3::new(0.0, 0.9, 0.0), config);

        // Zemin kutusu
        let ground = AABB3D::new(Vec3::new(-10.0, -1.0, -10.0), Vec3::new(10.0, 0.0, 10.0));
        // Engel kutusu (Duvar): X = [2.0, 4.0], Y = [0.0, 3.0], Z = [-10.0, 10.0]
        let wall = AABB3D::new(Vec3::new(2.0, 0.0, -10.0), Vec3::new(4.0, 3.0, 10.0));
        let colliders = [ground, wall];

        // Karakteri duvara doğru (+X ve +Z çapraz) koştur
        let move_dir = Vec3::new(1.0, 0.0, 1.0).normalize();
        for _ in 0..30 {
            controller.move_and_slide(move_dir, 1.0 / 60.0, &colliders);
        }

        // Karakter duvarın içine girmemeli (X sınırına takılmalı ama Z yönünde kaymaya devam edebilmeli)
        // Duvar min.x = 2.0, half_extents.x = 0.4 -> Karakterin max x konumu ~ 1.6 olmalı
        assert!(
            controller.position.x <= 1.65,
            "Karakter duvari gecememeli, x: {}",
            controller.position.x
        );
        assert!(
            controller.position.z > 0.5,
            "Karakter duvara carptiginda Z ekseninde kayabilmeli (slide), z: {}",
            controller.position.z
        );
    }

    #[test]
    fn test_character_controller_ceiling_hit_cancels_upward_velocity() {
        let config = CharacterController3DConfig::default();
        // Karakter tavanın hemen altında başlasın: Tavan min.y = 2.5, half_extents.y = 0.9.
        // Başlangıç y = 1.55 ise, char.max.y = 2.45.
        // dt = 1/60s'de vel.y = 10.0 ile +0.166m yükselir -> char.max.y = 2.61 > 2.5 (tavanla çarpışır).
        let mut controller = CharacterController3D::new(Vec3::new(0.0, 1.55, 0.0), config);
        let ceiling = AABB3D::new(Vec3::new(-5.0, 2.5, -5.0), Vec3::new(5.0, 3.5, 5.0));
        let colliders = [ceiling];

        controller.velocity.y = 10.0; // Yukarı doğru fırlat
        controller.move_and_slide(Vec3::ZERO, 1.0 / 60.0, &colliders);

        // Tavan min.y = 2.5, half_extents.y = 0.9 -> Karakter max y konumu 1.6 olmalı
        assert!(
            controller.position.y <= 1.601,
            "Karakter tavani asmamali, pos.y: {}",
            controller.position.y
        );
        assert_eq!(
            controller.velocity.y, 0.0,
            "Tavana carptiginda yukari dogru dikey hiz sifirlanmali"
        );
    }

    #[test]
    fn test_character_controller_ground_snapping_on_step_down() {
        let config = CharacterController3DConfig {
            ground_snap_distance: 0.1,
            ..Default::default()
        };
        // Karakter zeminden sadece 0.04m yukarıda havada
        let mut controller = CharacterController3D::new(Vec3::new(0.0, 0.94, 0.0), config);
        let ground = AABB3D::new(Vec3::new(-5.0, -1.0, -5.0), Vec3::new(5.0, 0.0, 5.0));
        let colliders = [ground];

        controller.velocity.y = 0.0;
        controller.move_and_slide(Vec3::ZERO, 1.0 / 60.0, &colliders);

        assert!(
            controller.is_on_ground,
            "Snap toleransi dahilindeki zemin yakalanmali ve zeminde olmali"
        );
        assert!(
            (controller.position.y - 0.9).abs() < 1e-3,
            "Zemin seviyesine kenetlenmeli (snap), pos.y: {}",
            controller.position.y
        );
    }
}
