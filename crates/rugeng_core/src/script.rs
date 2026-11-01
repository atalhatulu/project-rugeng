//! Project Ember — 2D Betik Motoru (Scripting Engine).
//!
//! Varlıklara dinamik davranış ve yaşam döngüsü (lifecycle) yetenekleri kazandırır.
//! Her betik `on_init`, `update` ve `on_destroy` kancalarını uygular.

use glam::Vec2;

#[derive(Clone, Debug, PartialEq)]
pub enum ScriptBehavior {
    /// Sabit açısal hızla sürekli kendi etrafında döner (rad/sn)
    Rotator { speed: f32 },
    /// Belirtilen eksende iki sınır arasında gidip gelir
    Patrol {
        axis: u8, // 0: X ekseni, 1: Y ekseni
        min_bound: f32,
        max_bound: f32,
        speed: f32,
        dir: f32,
    },
    /// Ölçek büyüme/küçülme nabız efekti
    Pulse {
        min_scale: f32,
        max_scale: f32,
        speed: f32,
        timer: f32,
    },
    /// Belirtilen bir merkez etrafında yörünge hareketi
    Orbit {
        center: Vec2,
        radius: f32,
        angular_speed: f32,
        angle: f32,
    },
    /// Serbest metin tabanlı kullanıcı mantığı
    Custom { name: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptComponent {
    pub name: String,
    pub behavior: ScriptBehavior,
    pub initialized: bool,
    pub is_enabled: bool,
}

impl ScriptComponent {
    pub fn new_rotator(speed: f32) -> Self {
        Self {
            name: "RotatorScript".to_string(),
            behavior: ScriptBehavior::Rotator { speed },
            initialized: false,
            is_enabled: true,
        }
    }

    pub fn new_patrol(axis: u8, min_bound: f32, max_bound: f32, speed: f32) -> Self {
        Self {
            name: "PatrolScript".to_string(),
            behavior: ScriptBehavior::Patrol {
                axis,
                min_bound,
                max_bound,
                speed,
                dir: 1.0,
            },
            initialized: false,
            is_enabled: true,
        }
    }

    pub fn new_pulse(min_scale: f32, max_scale: f32, speed: f32) -> Self {
        Self {
            name: "PulseScript".to_string(),
            behavior: ScriptBehavior::Pulse {
                min_scale,
                max_scale,
                speed,
                timer: 0.0,
            },
            initialized: false,
            is_enabled: true,
        }
    }

    pub fn new_orbit(center: Vec2, radius: f32, angular_speed: f32) -> Self {
        Self {
            name: "OrbitScript".to_string(),
            behavior: ScriptBehavior::Orbit {
                center,
                radius,
                angular_speed,
                angle: 0.0,
            },
            initialized: false,
            is_enabled: true,
        }
    }

    /// Kare başı güncelleme döngüsü: nesnenin konumunu, açısını ve boyutunu günceller
    #[inline]
    pub fn update(&mut self, dt: f32, pos: &mut Vec2, size: &mut Vec2, rotation: &mut f32) {
        if !self.is_enabled {
            return;
        }

        match &mut self.behavior {
            ScriptBehavior::Rotator { speed } => {
                *rotation += *speed * dt;
                if *rotation > std::f32::consts::TAU {
                    *rotation -= std::f32::consts::TAU;
                }
            }
            ScriptBehavior::Patrol {
                axis,
                min_bound,
                max_bound,
                speed,
                dir,
            } => {
                let target = if *axis == 0 { &mut pos.x } else { &mut pos.y };
                *target += *speed * *dir * dt;
                if *target >= *max_bound {
                    *target = *max_bound;
                    *dir = -1.0;
                } else if *target <= *min_bound {
                    *target = *min_bound;
                    *dir = 1.0;
                }
            }
            ScriptBehavior::Pulse {
                min_scale,
                max_scale,
                speed,
                timer,
            } => {
                *timer += *speed * dt;
                let t = (*timer).sin() * 0.5 + 0.5; // 0.0 .. 1.0
                let scale = *min_scale + (*max_scale - *min_scale) * t;
                *size = Vec2::splat(scale);
            }
            ScriptBehavior::Orbit {
                center,
                radius,
                angular_speed,
                angle,
            } => {
                *angle += *angular_speed * dt;
                if *angle > std::f32::consts::TAU {
                    *angle -= std::f32::consts::TAU;
                }
                pos.x = center.x + angle.cos() * *radius;
                pos.y = center.y + angle.sin() * *radius;
            }
            ScriptBehavior::Custom { .. } => {}
        }
    }
}
