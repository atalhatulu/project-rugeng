//! 2D Parçacık Sistemi (Particle System & Emitter)
//!
//! GPU instancing uyumlu, bellek tahsisatsız (zero-alloc) parçacık havuzu.

use glam::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub size: f32,
    pub initial_size: f32,
    pub life: f32,
    pub max_life: f32,
    pub color_start: [u8; 4],
    pub color_end: [u8; 4],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticleEmitterComponent {
    /// Canlı parçacıklar çalışma zamanı durumudur; sahneye kaydedilmez.
    #[serde(skip, default)]
    pub particles: Vec<Particle>,
    pub max_particles: usize,
    pub emission_rate: f32, // Saniyede üretilen parçacık sayısı
    /// Yükleme anında sıfırdan başlar; kaydedilmez.
    #[serde(skip, default)]
    pub emission_timer: f32,
    pub lifetime: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub size_start: f32,
    pub size_end: f32,
    pub color_start: [u8; 4],
    pub color_end: [u8; 4],
    pub spread_angle: f32, // Radyan cinsinden yayılma açısı
    pub base_angle: f32,
    pub is_active: bool,
    pub is_burst: bool, // Tek seferlik patlama mı
}

impl ParticleEmitterComponent {
    pub fn new_explosion(max_particles: usize, origin: Vec2) -> Self {
        let mut emitter = Self {
            particles: Vec::with_capacity(max_particles),
            max_particles,
            emission_rate: 0.0,
            emission_timer: 0.0,
            lifetime: 0.6,
            speed_min: 60.0,
            speed_max: 180.0,
            size_start: 8.0,
            size_end: 1.0,
            color_start: [255, 200, 50, 255],    // Sarı-turuncu alev
            color_end: [228, 60, 40, 0],         // Kırmızı ve şeffaflaşan
            spread_angle: std::f32::consts::TAU, // 360 derece tam yayılma
            base_angle: 0.0,
            is_active: true,
            is_burst: true,
        };
        emitter.burst(max_particles, origin);
        emitter
    }

    pub fn new_trail() -> Self {
        Self {
            particles: Vec::with_capacity(64),
            max_particles: 64,
            emission_rate: 40.0,
            emission_timer: 0.0,
            lifetime: 0.4,
            speed_min: 20.0,
            speed_max: 50.0,
            size_start: 6.0,
            size_end: 2.0,
            color_start: [71, 140, 191, 255], // Godot mavisi itki izi
            color_end: [30, 60, 120, 0],
            spread_angle: 0.4,
            base_angle: std::f32::consts::PI, // Gemi arkası (-X yönü)
            is_active: true,
            is_burst: false,
        }
    }

    pub fn burst(&mut self, count: usize, origin: Vec2) {
        let n = count.min(self.max_particles - self.particles.len());
        for i in 0..n {
            let angle = self.base_angle + (i as f32 / n as f32) * self.spread_angle;
            let speed = self.speed_min
                + ((i * 17) % 100) as f32 / 100.0 * (self.speed_max - self.speed_min);
            let vel = Vec2::new(angle.cos() * speed, angle.sin() * speed);
            self.particles.push(Particle {
                pos: origin,
                vel,
                size: self.size_start,
                initial_size: self.size_start,
                life: self.lifetime,
                max_life: self.lifetime,
                color_start: self.color_start,
                color_end: self.color_end,
            });
        }
    }

    pub fn update(&mut self, dt: f32, origin: Vec2) {
        // Sürekli emisyon
        if self.is_active && !self.is_burst && self.emission_rate > 0.0 {
            self.emission_timer += dt;
            let interval = 1.0 / self.emission_rate;
            while self.emission_timer >= interval {
                self.emission_timer -= interval;
                if self.particles.len() < self.max_particles {
                    let angle = self.base_angle
                        + (self.particles.len() as f32 * 0.3).sin() * (self.spread_angle * 0.5);
                    let speed = self.speed_min + (self.speed_max - self.speed_min) * 0.5;
                    let vel = Vec2::new(angle.cos() * speed, angle.sin() * speed);
                    self.particles.push(Particle {
                        pos: origin,
                        vel,
                        size: self.size_start,
                        initial_size: self.size_start,
                        life: self.lifetime,
                        max_life: self.lifetime,
                        color_start: self.color_start,
                        color_end: self.color_end,
                    });
                }
            }
        }

        // Parçacık güncelleme ve ömür kontrolü
        for p in &mut self.particles {
            p.life -= dt;
            p.pos += p.vel * dt;
            let t = 1.0 - (p.life / p.max_life).clamp(0.0, 1.0);
            p.size = self.size_start + (self.size_end - self.size_start) * t;
        }
        self.particles.retain(|p| p.life > 0.0);
    }
}

/// Parçacık rengini yaşam süresine göre lineer interpole eder.
pub fn interpolate_particle_color(start: [u8; 4], end: [u8; 4], t: f32) -> [u8; 4] {
    let t_clamped = t.clamp(0.0, 1.0);
    [
        (start[0] as f32 + (end[0] as f32 - start[0] as f32) * t_clamped) as u8,
        (start[1] as f32 + (end[1] as f32 - start[1] as f32) * t_clamped) as u8,
        (start[2] as f32 + (end[2] as f32 - start[2] as f32) * t_clamped) as u8,
        (start[3] as f32 + (end[3] as f32 - start[3] as f32) * t_clamped) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_emitter_burst_and_update() {
        let mut emitter = ParticleEmitterComponent::new_explosion(32, Vec2::new(50.0, 50.0));
        assert_eq!(emitter.particles.len(), 32);

        // 0.3 saniye güncelle (lifetime 0.6)
        emitter.update(0.3, Vec2::new(50.0, 50.0));
        assert!(!emitter.particles.is_empty());
        assert!((emitter.particles[0].pos - Vec2::new(50.0, 50.0)).length() > 0.0);

        // 0.4 saniye daha güncelle (toplam 0.7 > 0.6 lifetime)
        emitter.update(0.4, Vec2::new(50.0, 50.0));
        assert!(emitter.particles.is_empty());
    }

    #[test]
    fn test_particle_color_interpolation() {
        let start = [255, 0, 0, 255];
        let end = [0, 0, 255, 0];
        let mid = interpolate_particle_color(start, end, 0.5);
        assert_eq!(mid[0], 127);
        assert_eq!(mid[1], 0);
        assert_eq!(mid[2], 127);
        assert_eq!(mid[3], 127);
    }
}
