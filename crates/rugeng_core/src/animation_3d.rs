//! 3D İskelet Animasyonu ve Kemik Hiyerarşisi Modülü.
//!
//! Bu modül, kemik hiyerarşisi (`Skeleton`, `Joint`), ters bağlanma matrisleri (inverse bind matrices),
//! animasyon kanalları (`AnimationChannel3D`, `AnimationClip3D`) ve ciltleme (skinning) matris paletini yönetir.

use glam::{Mat4, Quat, Vec3};

pub const MAX_JOINTS: usize = 32;

/// Animasyon hedef parametresi
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationProperty {
    Translation,
    Rotation,
    Scale,
}

/// Enterpolasyon türü
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Interpolation {
    #[default]
    Linear,
    Step,
}

/// Tek bir eklem/kemik (Joint)
#[derive(Clone, Debug)]
pub struct Joint {
    pub name: String,
    pub parent_index: Option<usize>,
    pub local_translation: Vec3,
    pub local_rotation: Quat,
    pub local_scale: Vec3,
    pub inverse_bind_matrix: Mat4,
    pub global_transform: Mat4,
}

impl Default for Joint {
    fn default() -> Self {
        Self {
            name: "Joint".to_string(),
            parent_index: None,
            local_translation: Vec3::ZERO,
            local_rotation: Quat::IDENTITY,
            local_scale: Vec3::ONE,
            inverse_bind_matrix: Mat4::IDENTITY,
            global_transform: Mat4::IDENTITY,
        }
    }
}

impl Joint {
    pub fn new(
        name: impl Into<String>,
        parent_index: Option<usize>,
        inverse_bind_matrix: Mat4,
    ) -> Self {
        Self {
            name: name.into(),
            parent_index,
            local_translation: Vec3::ZERO,
            local_rotation: Quat::IDENTITY,
            local_scale: Vec3::ONE,
            inverse_bind_matrix,
            global_transform: Mat4::IDENTITY,
        }
    }

    /// Yerel dönüşüm matrisini (T * R * S) hesaplar
    #[inline]
    pub fn local_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            self.local_scale,
            self.local_rotation,
            self.local_translation,
        )
    }
}

/// İskelet yapısı
#[derive(Clone, Debug, Default)]
pub struct Skeleton {
    pub joints: Vec<Joint>,
}

impl Skeleton {
    pub fn new(joints: Vec<Joint>) -> Self {
        let mut skel = Self { joints };
        skel.update_global_transforms();
        skel
    }

    /// Ebeveyn-çocuk hiyerarşisine göre her eklemin dünya (global) matrisini günceller
    pub fn update_global_transforms(&mut self) {
        for i in 0..self.joints.len() {
            let parent_idx = self.joints[i].parent_index;
            let local = self.joints[i].local_matrix();
            self.joints[i].global_transform = match parent_idx {
                Some(p) if p < i => self.joints[p].global_transform * local,
                _ => local,
            };
        }
    }

    /// Shader veya CPU skinning için her eklemin Skinning Matrisini (GlobalTransform * InverseBindMatrix) hesaplar.
    pub fn compute_skinning_palette(&self) -> [Mat4; MAX_JOINTS] {
        let mut palette = [Mat4::IDENTITY; MAX_JOINTS];
        for (i, joint) in self.joints.iter().enumerate().take(MAX_JOINTS) {
            palette[i] = joint.global_transform * joint.inverse_bind_matrix;
        }
        palette
    }
}

/// Animasyon Anahtar Kareleri (Keyframe verisi)
#[derive(Clone, Debug)]
pub struct AnimationChannel3D {
    pub joint_index: usize,
    pub property: AnimationProperty,
    pub timestamps: Vec<f32>,
    /// Translation veya Scale için Vec3 dizisi
    pub vec3_values: Vec<Vec3>,
    /// Rotation için Quat dizisi
    pub quat_values: Vec<Quat>,
    pub interpolation: Interpolation,
}

impl AnimationChannel3D {
    pub fn sample_vec3(&self, time: f32) -> Option<Vec3> {
        if self.timestamps.is_empty() || self.vec3_values.is_empty() {
            return None;
        }
        if self.timestamps.len() == 1 || time <= self.timestamps[0] {
            return Some(self.vec3_values[0]);
        }
        if time >= *self.timestamps.last().unwrap() {
            return Some(*self.vec3_values.last().unwrap());
        }

        // İkili arama (Binary search) ile zaman aralığını bul
        let mut idx = 0;
        while idx + 1 < self.timestamps.len() && self.timestamps[idx + 1] < time {
            idx += 1;
        }

        let t0 = self.timestamps[idx];
        let t1 = self.timestamps[idx + 1];
        let v0 = self.vec3_values[idx];
        let v1 = self.vec3_values[idx + 1];

        match self.interpolation {
            Interpolation::Step => Some(v0),
            Interpolation::Linear => {
                let factor = if (t1 - t0).abs() > 1e-6 {
                    (time - t0) / (t1 - t0)
                } else {
                    0.0
                };
                Some(v0.lerp(v1, factor))
            }
        }
    }

    pub fn sample_quat(&self, time: f32) -> Option<Quat> {
        if self.timestamps.is_empty() || self.quat_values.is_empty() {
            return None;
        }
        if self.timestamps.len() == 1 || time <= self.timestamps[0] {
            return Some(self.quat_values[0]);
        }
        if time >= *self.timestamps.last().unwrap() {
            return Some(*self.quat_values.last().unwrap());
        }

        let mut idx = 0;
        while idx + 1 < self.timestamps.len() && self.timestamps[idx + 1] < time {
            idx += 1;
        }

        let t0 = self.timestamps[idx];
        let t1 = self.timestamps[idx + 1];
        let q0 = self.quat_values[idx];
        let q1 = self.quat_values[idx + 1];

        match self.interpolation {
            Interpolation::Step => Some(q0),
            Interpolation::Linear => {
                let factor = if (t1 - t0).abs() > 1e-6 {
                    (time - t0) / (t1 - t0)
                } else {
                    0.0
                };
                Some(q0.slerp(q1, factor))
            }
        }
    }
}

/// 3D Animasyon Klibi
#[derive(Clone, Debug, Default)]
pub struct AnimationClip3D {
    pub name: String,
    pub duration: f32,
    pub channels: Vec<AnimationChannel3D>,
}

impl AnimationClip3D {
    pub fn new(name: impl Into<String>, duration: f32, channels: Vec<AnimationChannel3D>) -> Self {
        Self {
            name: name.into(),
            duration,
            channels,
        }
    }

    /// Zamanı klip üzerinde örnekler ve iskelet eklemlerinin yerel koordinatlarını günceller
    pub fn sample(&self, time: f32, skeleton: &mut Skeleton) {
        let sample_time = if self.duration > 0.0 {
            time % self.duration
        } else {
            0.0
        };

        for channel in &self.channels {
            if channel.joint_index >= skeleton.joints.len() {
                continue;
            }
            let joint = &mut skeleton.joints[channel.joint_index];
            match channel.property {
                AnimationProperty::Translation => {
                    if let Some(pos) = channel.sample_vec3(sample_time) {
                        joint.local_translation = pos;
                    }
                }
                AnimationProperty::Rotation => {
                    if let Some(rot) = channel.sample_quat(sample_time) {
                        joint.local_rotation = rot;
                    }
                }
                AnimationProperty::Scale => {
                    if let Some(scale) = channel.sample_vec3(sample_time) {
                        joint.local_scale = scale;
                    }
                }
            }
        }

        skeleton.update_global_transforms();
    }
}

/// ECS İskelet Animasyon Bileşeni
#[derive(Clone, Debug, Default)]
pub struct Animation3DComponent {
    pub skeleton: Skeleton,
    pub clips: Vec<AnimationClip3D>,
    pub current_clip_index: usize,
    pub current_time: f32,
    pub playback_speed: f32,
    pub is_playing: bool,
    pub current_palette: [Mat4; MAX_JOINTS],
}

impl Animation3DComponent {
    pub fn new(skeleton: Skeleton, clips: Vec<AnimationClip3D>) -> Self {
        let palette = skeleton.compute_skinning_palette();
        Self {
            skeleton,
            clips,
            current_clip_index: 0,
            current_time: 0.0,
            playback_speed: 1.0,
            is_playing: true,
            current_palette: palette,
        }
    }

    pub fn update(&mut self, dt: f32) {
        if !self.is_playing || self.clips.is_empty() {
            return;
        }

        self.current_time += dt * self.playback_speed;
        let clip = &self.clips[self.current_clip_index];
        clip.sample(self.current_time, &mut self.skeleton);
        self.current_palette = self.skeleton.compute_skinning_palette();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skeleton_bone_hierarchy_world_transform() {
        // Ebeveyn kemik (Root): (0, 1, 0) öteleme
        let root = Joint {
            name: "Root".to_string(),
            parent_index: None,
            local_translation: Vec3::new(0.0, 1.0, 0.0),
            local_rotation: Quat::IDENTITY,
            local_scale: Vec3::ONE,
            inverse_bind_matrix: Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)),
            global_transform: Mat4::IDENTITY,
        };

        // Çocuk kemik: (0, 2, 0) yerel öteleme -> Dünya ötelemesi (0, 3, 0) olmalı
        let child = Joint {
            name: "Child".to_string(),
            parent_index: Some(0),
            local_translation: Vec3::new(0.0, 2.0, 0.0),
            local_rotation: Quat::IDENTITY,
            local_scale: Vec3::ONE,
            inverse_bind_matrix: Mat4::from_translation(Vec3::new(0.0, -3.0, 0.0)),
            global_transform: Mat4::IDENTITY,
        };

        let mut skeleton = Skeleton::new(vec![root, child]);
        skeleton.update_global_transforms();

        let root_pos = skeleton.joints[0]
            .global_transform
            .transform_point3(Vec3::ZERO);
        let child_pos = skeleton.joints[1]
            .global_transform
            .transform_point3(Vec3::ZERO);

        assert!((root_pos - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-5);
        assert!((child_pos - Vec3::new(0.0, 3.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn test_skinning_palette_matrix_correctness() {
        // Bind pozisyonunda global_transform * inverse_bind_matrix = IDENTITY olmalı
        let joint = Joint {
            name: "Bone".to_string(),
            parent_index: None,
            local_translation: Vec3::new(5.0, 10.0, -2.0),
            local_rotation: Quat::IDENTITY,
            local_scale: Vec3::ONE,
            inverse_bind_matrix: Mat4::from_translation(Vec3::new(-5.0, -10.0, 2.0)),
            global_transform: Mat4::IDENTITY,
        };

        let skeleton = Skeleton::new(vec![joint]);
        let palette = skeleton.compute_skinning_palette();

        // Palette[0] matrisinin kimlik (identity) olup olmadığını kontrol et
        let diff = palette[0] - Mat4::IDENTITY;
        assert!(diff.abs_diff_eq(Mat4::ZERO, 1e-4));
    }

    #[test]
    fn test_animation_keyframe_interpolation_linear() {
        let channel = AnimationChannel3D {
            joint_index: 0,
            property: AnimationProperty::Translation,
            timestamps: vec![0.0, 1.0, 2.0],
            vec3_values: vec![
                Vec3::ZERO,
                Vec3::new(10.0, 0.0, 0.0),
                Vec3::new(10.0, 20.0, 0.0),
            ],
            quat_values: Vec::new(),
            interpolation: Interpolation::Linear,
        };

        // 0.5 saniyede: (5.0, 0.0, 0.0) olmalı
        let sampled = channel.sample_vec3(0.5).unwrap();
        assert!((sampled - Vec3::new(5.0, 0.0, 0.0)).length() < 1e-5);

        // 1.5 saniyede: (10.0, 10.0, 0.0) olmalı
        let sampled_15 = channel.sample_vec3(1.5).unwrap();
        assert!((sampled_15 - Vec3::new(10.0, 10.0, 0.0)).length() < 1e-5);
    }
}
