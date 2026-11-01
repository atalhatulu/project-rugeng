//! Katı AABB Çarpışma Ayrıştırma ve Tepki Sistemi (Solid Penetration Solver)

use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionManifold {
    pub normal: Vec2,
    pub penetration: f32,
}

/// İki AABB kutusunun kesişim derinliğini ve ayrışma normalini (MTV) hesaplar.
pub fn test_aabb_overlap(
    pos_a: Vec2,
    size_a: Vec2,
    pos_b: Vec2,
    size_b: Vec2,
) -> Option<CollisionManifold> {
    let half_a = size_a * 0.5;
    let half_b = size_b * 0.5;
    let delta = pos_b - pos_a;

    let ox = (half_a.x + half_b.x) - delta.x.abs();
    let oy = (half_a.y + half_b.y) - delta.y.abs();

    if ox > 0.0 && oy > 0.0 {
        // En küçük örtüşme eksenini ayırma ekseni olarak seç (MTV)
        if ox < oy {
            let normal = if delta.x < 0.0 {
                Vec2::new(1.0, 0.0)
            } else {
                Vec2::new(-1.0, 0.0)
            };
            Some(CollisionManifold {
                normal,
                penetration: ox,
            })
        } else {
            let normal = if delta.y < 0.0 {
                Vec2::new(0.0, 1.0)
            } else {
                Vec2::new(0.0, -1.0)
            };
            Some(CollisionManifold {
                normal,
                penetration: oy,
            })
        }
    } else {
        None
    }
}

/// İki nesne arasındaki çakışmayı kütle durumlarına (dinamik/statik) göre pozisyonları iterek çözer.
pub fn resolve_aabb_collision(
    pos_a: &mut Vec2,
    size_a: Vec2,
    is_dynamic_a: bool,
    pos_b: &mut Vec2,
    size_b: Vec2,
    is_dynamic_b: bool,
) -> bool {
    // İkisi de statikse itme veya ayrıştırma yapılmaz
    if !is_dynamic_a && !is_dynamic_b {
        return false;
    }

    if let Some(manifold) = test_aabb_overlap(*pos_a, size_a, *pos_b, size_b) {
        if is_dynamic_a && is_dynamic_b {
            // İkisi de dinamikse yarı yarıya it
            let push = manifold.normal * (manifold.penetration * 0.5);
            *pos_a += push;
            *pos_b -= push;
        } else if is_dynamic_a && !is_dynamic_b {
            // Sadece A dinamikse A'yı tamamen dışarı it
            *pos_a += manifold.normal * manifold.penetration;
        } else if !is_dynamic_a && is_dynamic_b {
            // Sadece B dinamikse B'yi tamamen dışarı it
            *pos_b -= manifold.normal * manifold.penetration;
        }
        true
    } else {
        false
    }
}

/// Çarpışmayı ayrıştırır ve çakışma eksenindeki hız bileşenlerini sıfırlar.
#[allow(clippy::too_many_arguments)]
pub fn resolve_aabb_collision_with_velocity(
    pos_a: &mut Vec2,
    vel_a: Option<&mut Vec2>,
    size_a: Vec2,
    is_dynamic_a: bool,
    pos_b: &mut Vec2,
    vel_b: Option<&mut Vec2>,
    size_b: Vec2,
    is_dynamic_b: bool,
) -> bool {
    if !is_dynamic_a && !is_dynamic_b {
        return false;
    }

    if let Some(manifold) = test_aabb_overlap(*pos_a, size_a, *pos_b, size_b) {
        if is_dynamic_a && is_dynamic_b {
            let push = manifold.normal * (manifold.penetration * 0.5);
            *pos_a += push;
            *pos_b -= push;

            if let Some(va) = vel_a {
                let dot = va.dot(manifold.normal);
                if dot < 0.0 {
                    *va -= manifold.normal * dot;
                }
            }
            if let Some(vb) = vel_b {
                let dot = vb.dot(manifold.normal);
                if dot > 0.0 {
                    *vb -= manifold.normal * dot;
                }
            }
        } else if is_dynamic_a {
            *pos_a += manifold.normal * manifold.penetration;
            if let Some(va) = vel_a {
                let dot = va.dot(manifold.normal);
                if dot < 0.0 {
                    *va -= manifold.normal * dot;
                }
            }
        } else if is_dynamic_b {
            *pos_b -= manifold.normal * manifold.penetration;
            if let Some(vb) = vel_b {
                let dot = vb.dot(manifold.normal);
                if dot > 0.0 {
                    *vb -= manifold.normal * dot;
                }
            }
        }
        true
    } else {
        false
    }
}

/// Tek yönlü zemin platformu çarpışma çözücüsü (One-Way / Jump-Through Platform).
///
/// Karakter platformun altından yukarı doğru zıplarken veya içerisindeyken içinden geçer;
/// sadece aşağı doğru düşerken ve platformun üst yüzeyine bastığı anda (normal: -Y)
/// karakteri platformun üzerine oturtur ve dikey düşme hızını sıfırlar.
pub fn resolve_one_way_platform(
    player_pos: &mut Vec2,
    player_vel: &mut Vec2,
    player_size: Vec2,
    platform_pos: Vec2,
    platform_size: Vec2,
) -> bool {
    // Sadece aşağı doğru düşüşte devreye girer
    if player_vel.y <= 0.0 {
        return false;
    }

    let p_bottom = player_pos.y + player_size.y * 0.5;
    let plat_top = platform_pos.y - platform_size.y * 0.5;
    let plat_bottom = platform_pos.y + platform_size.y * 0.5;

    // Yatayda platform sınırları içerisinde mi?
    let p_left = player_pos.x - player_size.x * 0.5;
    let p_right = player_pos.x + player_size.x * 0.5;
    let plat_left = platform_pos.x - platform_size.x * 0.5;
    let plat_right = platform_pos.x + platform_size.x * 0.5;

    if p_right <= plat_left || p_left >= plat_right {
        return false;
    }

    // Karakterin alt tabanı, platformun üst yüzeyini bu karede geçmiş mi?
    // (Aşırı penetrasyonu veya alttan geçişi engellemek için tolerans penceresi)
    let margin = (player_vel.y * (1.0 / 60.0) + 8.0).max(12.0);
    if p_bottom >= plat_top && p_bottom <= plat_bottom + margin {
        // Platformun tam üzerine oturt
        player_pos.y = plat_top - player_size.y * 0.5;
        player_vel.y = 0.0;
        true
    } else {
        false
    }
}

/// 45 Derecelik Eğim / Rampa Türleri (2D Slope Types)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlopeType {
    /// Sağa doğru yükselen rampa (X arttıkça zemin yükselir / Y koordinatı azalır)
    UpRight,
    /// Sola doğru yükselen rampa (X azaldıkça zemin yükselir / sağa gidildikçe zemin alçalır)
    UpLeft,
    /// Sağa doğru alçalan tavan eğimi
    CeilingUpRight,
    /// Sola doğru alçalan tavan eğimi
    CeilingUpLeft,
}

impl SlopeType {
    /// Belirtilen dünya X koordinatında rampanın eğik yüzey Y değerini hesaplar.
    pub fn surface_y_at(&self, slope_pos: Vec2, slope_size: Vec2, world_x: f32) -> f32 {
        let half_w = slope_size.x * 0.5;
        let half_h = slope_size.y * 0.5;
        let x_min = slope_pos.x - half_w;
        let y_min = slope_pos.y - half_h;
        let y_max = slope_pos.y + half_h;

        let t = ((world_x - x_min) / slope_size.x).clamp(0.0, 1.0);
        match self {
            SlopeType::UpRight => y_max - t * slope_size.y,
            SlopeType::UpLeft => y_min + t * slope_size.y,
            SlopeType::CeilingUpRight => y_min + t * slope_size.y,
            SlopeType::CeilingUpLeft => y_max - t * slope_size.y,
        }
    }

    /// Rampa yüzeyinin dışa bakan birim normal vektörünü döndürür.
    pub fn normal(&self) -> Vec2 {
        match self {
            SlopeType::UpRight => Vec2::new(-1.0, -1.0).normalize(),
            SlopeType::UpLeft => Vec2::new(1.0, -1.0).normalize(),
            SlopeType::CeilingUpRight => Vec2::new(-1.0, 1.0).normalize(),
            SlopeType::CeilingUpLeft => Vec2::new(1.0, 1.0).normalize(),
        }
    }
}

/// Bir dinamik varlık (AABB) ile 45 derecelik rampa (Slope) arasındaki çarpışmayı çözer.
///
/// Varlık rampanın yatay sınırları içerisindeyse ve tabanı rampa yüzeyinin altına
/// girmişse (penetrasyon), varlık dikey olarak rampa yüzeyine oturtulur.
pub fn resolve_slope_collision(
    entity_pos: &mut Vec2,
    entity_size: Vec2,
    slope_pos: Vec2,
    slope_size: Vec2,
    slope_type: SlopeType,
) -> bool {
    resolve_slope_collision_with_velocity(
        entity_pos,
        None,
        entity_size,
        slope_pos,
        slope_size,
        slope_type,
    )
}

/// Rampa çarpışmasını çözer ve aşağı yönlü penetrasyon hızını sıfırlar.
pub fn resolve_slope_collision_with_velocity(
    entity_pos: &mut Vec2,
    entity_vel: Option<&mut Vec2>,
    entity_size: Vec2,
    slope_pos: Vec2,
    slope_size: Vec2,
    slope_type: SlopeType,
) -> bool {
    let half_e = entity_size * 0.5;
    let e_left = entity_pos.x - half_e.x;
    let e_right = entity_pos.x + half_e.x;
    let e_top = entity_pos.y - half_e.y;
    let e_bottom = entity_pos.y + half_e.y;

    let half_s = slope_size * 0.5;
    let s_left = slope_pos.x - half_s.x;
    let s_right = slope_pos.x + half_s.x;
    let s_top = slope_pos.y - half_s.y;
    let s_bottom = slope_pos.y + half_s.y;

    // Yatay veya dikey AABB kutusu ile hiç kesişim yoksa
    if e_right <= s_left || e_left >= s_right || e_bottom <= s_top || e_top >= s_bottom {
        return false;
    }

    // Varlık gövdesinin rampa üzerindeki temas X noktası
    let contact_x = entity_pos.x.clamp(s_left, s_right);
    let surface_y = slope_type.surface_y_at(slope_pos, slope_size, contact_x);

    match slope_type {
        SlopeType::UpRight => {
            let pen_surface = e_bottom - surface_y;
            let pen_wall = s_right - e_left;
            let pen_floor = s_bottom - e_top;

            if pen_surface <= 0.0 || e_top >= s_bottom {
                return false;
            }

            // Alttan yukarı zıplayıp tabana çarpma
            if e_bottom > s_bottom && pen_floor < pen_surface {
                entity_pos.y = s_bottom + half_e.y;
                if let Some(vel) = entity_vel {
                    if vel.y < 0.0 {
                        vel.y = 0.0;
                    }
                }
                return true;
            }

            // Sağdaki dikey duvara çarpma (arkadan/sağdan gelme)
            if entity_pos.x > s_right || pen_wall < pen_surface {
                entity_pos.x = s_right + half_e.x;
                if let Some(vel) = entity_vel {
                    if vel.x < 0.0 {
                        vel.x = 0.0;
                    }
                }
                return true;
            }

            // Eğimli yüzey (Hipotenüs) çözümü
            entity_pos.y -= pen_surface;
            if let Some(vel) = entity_vel {
                if vel.y > 0.0 {
                    vel.y = 0.0;
                }
            }
            true
        }
        SlopeType::UpLeft => {
            let pen_surface = e_bottom - surface_y;
            let pen_wall = e_right - s_left;
            let pen_floor = s_bottom - e_top;

            if pen_surface <= 0.0 || e_top >= s_bottom {
                return false;
            }

            // Alttan tabana çarpma
            if e_bottom > s_bottom && pen_floor < pen_surface {
                entity_pos.y = s_bottom + half_e.y;
                if let Some(vel) = entity_vel {
                    if vel.y < 0.0 {
                        vel.y = 0.0;
                    }
                }
                return true;
            }

            // Soldaki dikey duvara çarpma (soldan gelme)
            if entity_pos.x < s_left || pen_wall < pen_surface {
                entity_pos.x = s_left - half_e.x;
                if let Some(vel) = entity_vel {
                    if vel.x > 0.0 {
                        vel.x = 0.0;
                    }
                }
                return true;
            }

            // Eğimli yüzey çözümü
            entity_pos.y -= pen_surface;
            if let Some(vel) = entity_vel {
                if vel.y > 0.0 {
                    vel.y = 0.0;
                }
            }
            true
        }
        SlopeType::CeilingUpRight => {
            let pen_surface = surface_y - e_top;
            let pen_wall = e_right - s_left;
            let pen_roof = e_bottom - s_top;

            if pen_surface <= 0.0 || e_bottom <= s_top {
                return false;
            }

            if e_top < s_top && pen_roof < pen_surface {
                entity_pos.y = s_top - half_e.y;
                if let Some(vel) = entity_vel {
                    if vel.y > 0.0 {
                        vel.y = 0.0;
                    }
                }
                return true;
            }

            if entity_pos.x < s_left || pen_wall < pen_surface {
                entity_pos.x = s_left - half_e.x;
                if let Some(vel) = entity_vel {
                    if vel.x > 0.0 {
                        vel.x = 0.0;
                    }
                }
                return true;
            }

            entity_pos.y += pen_surface;
            if let Some(vel) = entity_vel {
                if vel.y < 0.0 {
                    vel.y = 0.0;
                }
            }
            true
        }
        SlopeType::CeilingUpLeft => {
            let pen_surface = surface_y - e_top;
            let pen_wall = s_right - e_left;
            let pen_roof = e_bottom - s_top;

            if pen_surface <= 0.0 || e_bottom <= s_top {
                return false;
            }

            if e_top < s_top && pen_roof < pen_surface {
                entity_pos.y = s_top - half_e.y;
                if let Some(vel) = entity_vel {
                    if vel.y > 0.0 {
                        vel.y = 0.0;
                    }
                }
                return true;
            }

            if entity_pos.x > s_right || pen_wall < pen_surface {
                entity_pos.x = s_right + half_e.x;
                if let Some(vel) = entity_vel {
                    if vel.x < 0.0 {
                        vel.x = 0.0;
                    }
                }
                return true;
            }

            entity_pos.y += pen_surface;
            if let Some(vel) = entity_vel {
                if vel.y < 0.0 {
                    vel.y = 0.0;
                }
            }
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_penetration_resolution() {
        let mut pos_a = Vec2::new(0.0, 0.0);
        let size_a = Vec2::new(32.0, 32.0);
        let mut pos_b = Vec2::new(30.0, 0.0); // 2 piksel X örtüşmesi
        let size_b = Vec2::new(32.0, 32.0);

        let resolved = resolve_aabb_collision(&mut pos_a, size_a, true, &mut pos_b, size_b, true);
        assert!(resolved);
        // İkisi de eşit olarak 1 piksel geriye itilmeli
        assert!((pos_a.x - (-1.0)).abs() < 1e-4);
        assert!((pos_b.x - 31.0).abs() < 1e-4);

        // Tekrar test edildiğinde artık çakışmamalı
        assert!(test_aabb_overlap(pos_a, size_a, pos_b, size_b).is_none());
    }

    #[test]
    fn test_static_static_collision_returns_false() {
        let mut pos_a = Vec2::new(0.0, 0.0);
        let size_a = Vec2::new(32.0, 32.0);
        let mut pos_b = Vec2::new(10.0, 0.0); // Örtüşüyor ama ikisi de statik
        let size_b = Vec2::new(32.0, 32.0);

        let resolved = resolve_aabb_collision(&mut pos_a, size_a, false, &mut pos_b, size_b, false);
        assert!(!resolved);
        // Pozisyonlar değişmemiş olmalı
        assert_eq!(pos_a, Vec2::new(0.0, 0.0));
        assert_eq!(pos_b, Vec2::new(10.0, 0.0));
    }

    #[test]
    fn test_aabb_velocity_resolution() {
        let mut pos_a = Vec2::new(0.0, 0.0);
        let mut vel_a = Vec2::new(100.0, 50.0);
        let size_a = Vec2::new(32.0, 32.0);

        let mut pos_b = Vec2::new(30.0, 0.0); // B sağda, statik duvar
        let size_b = Vec2::new(32.0, 32.0);

        let resolved = resolve_aabb_collision_with_velocity(
            &mut pos_a,
            Some(&mut vel_a),
            size_a,
            true,
            &mut pos_b,
            None,
            size_b,
            false,
        );
        assert!(resolved);
        // Normal (-1.0, 0.0) yönünde: A sola itildi
        assert!(pos_a.x < 0.0);
        // A'nın X hızı (duvara doğru) sıfırlanmalı, Y hızı korunmalı
        assert!((vel_a.x).abs() < 1e-4);
        assert!((vel_a.y - 50.0).abs() < 1e-4);
    }

    #[test]
    fn test_one_way_platform_upward_and_downward() {
        let plat_pos = Vec2::new(100.0, 200.0);
        let plat_size = Vec2::new(64.0, 16.0); // Üst kenar: 200 - 8 = 192.0
        let p_size = Vec2::new(20.0, 30.0); // Yarı boy: 15.0

        // 1. Yukarı zıplarken: İçinden serbestçe geçmeli
        let mut p_pos = Vec2::new(100.0, 205.0);
        let mut p_vel = Vec2::new(0.0, -300.0);
        let hit = resolve_one_way_platform(&mut p_pos, &mut p_vel, p_size, plat_pos, plat_size);
        assert!(!hit);
        assert_eq!(p_vel.y, -300.0);

        // 2. Aşağı doğru düşerken ve üst kenara bastığında: Platform üzerine oturmalı
        let mut p_pos_fall = Vec2::new(100.0, 185.0); // Tabanı: 185 + 15 = 200 (üst kenar 192'yi geçmiş)
        let mut p_vel_fall = Vec2::new(0.0, 200.0);
        let hit_fall = resolve_one_way_platform(
            &mut p_pos_fall,
            &mut p_vel_fall,
            p_size,
            plat_pos,
            plat_size,
        );
        assert!(hit_fall);
        // Yeni taban tam 192.0 olmalı, yani p_pos_fall.y = 192.0 - 15.0 = 177.0
        assert_eq!(p_pos_fall.y, 177.0);
        assert_eq!(p_vel_fall.y, 0.0);
    }

    #[test]
    fn test_slope_up_right_resolution_and_velocity() {
        let slope_pos = Vec2::new(100.0, 200.0);
        let slope_size = Vec2::new(32.0, 32.0); // X: [84, 116], Y: [184, 216]
        let p_size = Vec2::new(20.0, 20.0); // Yarı boy: 10.0

        // X = 100 noktasında: t = 0.5, yüzey Y = 216 - 16 = 200.0
        // Karakter tabanı 205.0'da (pos.y = 195.0), yani 5 piksel penetrasyon
        let mut p_pos = Vec2::new(100.0, 195.0);
        let mut p_vel = Vec2::new(60.0, 120.0);

        let resolved = resolve_slope_collision_with_velocity(
            &mut p_pos,
            Some(&mut p_vel),
            p_size,
            slope_pos,
            slope_size,
            SlopeType::UpRight,
        );

        assert!(resolved);
        // Taban tam 200.0'a oturmalı: p_pos.y = 200.0 - 10.0 = 190.0
        assert!((p_pos.y - 190.0).abs() < 1e-4);
        // Düşüş hızı sıfırlanmalı, yatay hız korunmalı
        assert_eq!(p_vel.y, 0.0);
        assert_eq!(p_vel.x, 60.0);
    }

    #[test]
    fn test_slope_up_left_resolution() {
        let slope_pos = Vec2::new(100.0, 200.0);
        let slope_size = Vec2::new(32.0, 32.0); // X: [84, 116], Y: [184, 216]
        let p_size = Vec2::new(20.0, 20.0);

        // X = 100 noktasında: t = 0.5, UpLeft yüzey Y = 184 + 16 = 200.0
        let mut p_pos = Vec2::new(100.0, 196.0); // Tabanı: 206.0
        let resolved =
            resolve_slope_collision(&mut p_pos, p_size, slope_pos, slope_size, SlopeType::UpLeft);

        assert!(resolved);
        assert!((p_pos.y - 190.0).abs() < 1e-4);
    }

    #[test]
    fn test_slope_no_collision_above_or_outside() {
        let slope_pos = Vec2::new(100.0, 200.0);
        let slope_size = Vec2::new(32.0, 32.0);
        let p_size = Vec2::new(20.0, 20.0);

        // Rampa yüzeyinin tamamen üstünde (X: 100, Y: 180 -> taban 190 < yüzey 200)
        let mut p_pos_above = Vec2::new(100.0, 180.0);
        assert!(!resolve_slope_collision(
            &mut p_pos_above,
            p_size,
            slope_pos,
            slope_size,
            SlopeType::UpRight
        ));
        assert_eq!(p_pos_above.y, 180.0);

        // Rampanın yatay sınırlarının tamamen dışında (X: 130)
        let mut p_pos_outside = Vec2::new(130.0, 195.0);
        assert!(!resolve_slope_collision(
            &mut p_pos_outside,
            p_size,
            slope_pos,
            slope_size,
            SlopeType::UpRight
        ));
    }

    #[test]
    fn test_slope_back_wall_collision_does_not_pop_up() {
        let slope_pos = Vec2::new(100.0, 200.0);
        let slope_size = Vec2::new(32.0, 32.0); // X: [84, 116], Y: [184, 216]
        let p_size = Vec2::new(16.0, 16.0); // half: 8.0

        // UpLeft rampasında dikey arka duvar X = 84'tür.
        // Karakter soldan gelip duvara çarptığında (örneğin X: 82.0, Y: 200.0, sağ kenar 90.0)
        // Karakter yukarı fırlatılmamalı, sola X = 84 - 8 = 76.0'a itilmeli ve vel.x sıfırlanmalı.
        let mut p_pos = Vec2::new(82.0, 200.0);
        let mut p_vel = Vec2::new(50.0, 0.0);
        let resolved = resolve_slope_collision_with_velocity(
            &mut p_pos,
            Some(&mut p_vel),
            p_size,
            slope_pos,
            slope_size,
            SlopeType::UpLeft,
        );
        assert!(resolved);
        assert!((p_pos.x - 76.0).abs() < 1e-4);
        assert_eq!(p_pos.y, 200.0); // Y koordinatı değişmemeli (yukarı zıplama yok)
        assert_eq!(p_vel.x, 0.0);

        // UpRight rampasında dikey arka duvar X = 116'dır.
        // Karakter sağdan gelip duvara çarptığında sağa X = 116 + 8 = 124.0'a itilmeli.
        let mut p_pos_right = Vec2::new(118.0, 200.0);
        let mut p_vel_right = Vec2::new(-50.0, 0.0);
        let resolved_right = resolve_slope_collision_with_velocity(
            &mut p_pos_right,
            Some(&mut p_vel_right),
            p_size,
            slope_pos,
            slope_size,
            SlopeType::UpRight,
        );
        assert!(resolved_right);
        assert!((p_pos_right.x - 124.0).abs() < 1e-4);
        assert_eq!(p_pos_right.y, 200.0);
        assert_eq!(p_vel_right.x, 0.0);
    }
}
