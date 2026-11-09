//! 3D İskelet Animasyonu ve Kemik Hiyerarşisi Modülü.
//!
//! Bu modül, kemik hiyerarşisi (`Skeleton`, `Joint`), ters bağlanma matrisleri (inverse bind matrices),
//! animasyon kanalları (`AnimationChannel3D`, `AnimationClip3D`) ve ciltleme (skinning) matris paletini yönetir.

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

pub const MAX_JOINTS: usize = 32;

/// Animasyon hedef parametresi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimationProperty {
    Translation,
    Rotation,
    Scale,
}

/// Enterpolasyon türü
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Interpolation {
    #[default]
    Linear,
    Step,
}

/// Tek bir eklem/kemik (Joint)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
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

    /// Klip üzerindeki zamanı örnekler ve her eklem için yerel (translation, rotation, scale) dizisi döner.
    pub fn sample_pose(&self, time: f32, joint_count: usize) -> Vec<(Vec3, Quat, Vec3)> {
        let sample_time = if self.duration > 0.0 {
            time % self.duration
        } else {
            0.0
        };

        let mut pose = vec![(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE); joint_count];
        for channel in &self.channels {
            if channel.joint_index >= joint_count {
                continue;
            }
            match channel.property {
                AnimationProperty::Translation => {
                    if let Some(pos) = channel.sample_vec3(sample_time) {
                        pose[channel.joint_index].0 = pos;
                    }
                }
                AnimationProperty::Rotation => {
                    if let Some(rot) = channel.sample_quat(sample_time) {
                        pose[channel.joint_index].1 = rot;
                    }
                }
                AnimationProperty::Scale => {
                    if let Some(scale) = channel.sample_vec3(sample_time) {
                        pose[channel.joint_index].2 = scale;
                    }
                }
            }
        }
        pose
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
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Animation3DComponent {
    pub skeleton: Skeleton,
    pub clips: Vec<AnimationClip3D>,
    pub current_clip_index: usize,
    pub current_time: f32,
    pub playback_speed: f32,
    pub is_playing: bool,
    #[serde(skip, default = "default_palette")]
    pub current_palette: [Mat4; MAX_JOINTS],
}

fn default_palette() -> [Mat4; MAX_JOINTS] {
    [Mat4::IDENTITY; MAX_JOINTS]
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

/// 3D Standart Karakter Animasyon Durumları
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AnimationState3D {
    #[default]
    Idle,
    Run,
    Jump,
    Fall,
}

/// 3D Animasyon Durum Makinesi ve Harmanlama (Cross-fade Lerp Blending)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnimationStateMachine3D {
    pub current_state: AnimationState3D,
    pub previous_state: Option<AnimationState3D>,
    /// Harmanlama süresi (varsayılan: 0.15s)
    pub blend_duration: f32,
    /// Harmanlama zamanlayıcısı (0.0 .. blend_duration)
    pub blend_timer: f32,
    /// Durumların klip indeksleri haritası: [Idle, Run, Jump, Fall]
    pub clip_indices: [Option<usize>; 4],
    /// Mevcut durumun çalma süresi
    pub current_time: f32,
    /// Önceki durumun çalma süresi
    pub previous_time: f32,
}

impl Default for AnimationStateMachine3D {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationStateMachine3D {
    pub fn new() -> Self {
        Self {
            current_state: AnimationState3D::Idle,
            previous_state: None,
            blend_duration: 0.15,
            blend_timer: 0.0,
            clip_indices: [None; 4],
            current_time: 0.0,
            previous_time: 0.0,
        }
    }

    #[inline]
    fn state_to_idx(state: AnimationState3D) -> usize {
        match state {
            AnimationState3D::Idle => 0,
            AnimationState3D::Run => 1,
            AnimationState3D::Jump => 2,
            AnimationState3D::Fall => 3,
        }
    }

    /// Bir duruma klip indeksi atar.
    pub fn bind_clip(&mut self, state: AnimationState3D, clip_index: usize) {
        self.clip_indices[Self::state_to_idx(state)] = Some(clip_index);
    }

    /// Yeni bir duruma yumuşak geçiş başlatır.
    pub fn transition_to(&mut self, new_state: AnimationState3D) {
        if self.current_state == new_state {
            return;
        }
        self.previous_state = Some(self.current_state);
        self.previous_time = self.current_time;
        self.current_state = new_state;
        self.current_time = 0.0;
        self.blend_timer = 0.0;
    }

    /// Kontrolcü hızı (`velocity`) ve zemin durumuna (`is_on_ground`) göre durumu otomatik günceller.
    pub fn update_from_velocity(&mut self, velocity: Vec3, is_on_ground: bool) {
        let horiz_speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
        let target_state = if is_on_ground {
            if horiz_speed > 0.2 {
                AnimationState3D::Run
            } else {
                AnimationState3D::Idle
            }
        } else if velocity.y > 0.5 {
            AnimationState3D::Jump
        } else {
            AnimationState3D::Fall
        };

        self.transition_to(target_state);
    }

    /// Mevcut harmanlama ağırlığını (0.0 = tamamen önceki durum, 1.0 = tamamen yeni durum) döndürür.
    pub fn blend_weight(&self) -> f32 {
        if self.previous_state.is_none() || self.blend_duration <= 0.0 {
            1.0
        } else {
            (self.blend_timer / self.blend_duration).clamp(0.0, 1.0)
        }
    }

    /// Durum makinesini ilerletir ve iskeleti (cross-fade / slerp ile) günceller.
    pub fn update(&mut self, dt: f32, clips: &[AnimationClip3D], skeleton: &mut Skeleton) {
        self.current_time += dt;
        self.previous_time += dt;

        let weight = if self.previous_state.is_some() {
            self.blend_timer += dt;
            let w = (self.blend_timer / self.blend_duration).clamp(0.0, 1.0);
            if self.blend_timer >= self.blend_duration {
                self.previous_state = None;
            }
            w
        } else {
            1.0
        };

        let joint_count = skeleton.joints.len();
        let curr_clip_idx = self.clip_indices[Self::state_to_idx(self.current_state)];
        let curr_pose = curr_clip_idx
            .and_then(|idx| clips.get(idx))
            .map(|clip| clip.sample_pose(self.current_time, joint_count));

        if weight >= 1.0 || self.previous_state.is_none() {
            if let Some(pose) = curr_pose {
                for (i, &(pos, rot, scale)) in pose.iter().enumerate() {
                    skeleton.joints[i].local_translation = pos;
                    skeleton.joints[i].local_rotation = rot;
                    skeleton.joints[i].local_scale = scale;
                }
                skeleton.update_global_transforms();
            }
            return;
        }

        // Önceki durumun pozu ile mevcut durumun pozunu harmanla (Cross-fade Lerp / Slerp)
        let prev_clip_idx = self
            .previous_state
            .and_then(|st| self.clip_indices[Self::state_to_idx(st)]);
        let prev_pose = prev_clip_idx
            .and_then(|idx| clips.get(idx))
            .map(|clip| clip.sample_pose(self.previous_time, joint_count));

        match (prev_pose, curr_pose) {
            (Some(prev), Some(curr)) => {
                for i in 0..joint_count {
                    let (p0, r0, s0) = prev[i];
                    let (p1, r1, s1) = curr[i];
                    skeleton.joints[i].local_translation = p0.lerp(p1, weight);
                    skeleton.joints[i].local_rotation = r0.slerp(r1, weight);
                    skeleton.joints[i].local_scale = s0.lerp(s1, weight);
                }
                skeleton.update_global_transforms();
            }
            (None, Some(curr)) => {
                for (i, &(pos, rot, scale)) in curr.iter().enumerate() {
                    skeleton.joints[i].local_translation = pos;
                    skeleton.joints[i].local_rotation = rot;
                    skeleton.joints[i].local_scale = scale;
                }
                skeleton.update_global_transforms();
            }
            (Some(prev), None) => {
                for (i, &(pos, rot, scale)) in prev.iter().enumerate() {
                    skeleton.joints[i].local_translation = pos;
                    skeleton.joints[i].local_rotation = rot;
                    skeleton.joints[i].local_scale = scale;
                }
                skeleton.update_global_transforms();
            }
            (None, None) => {}
        }
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

    #[test]
    fn test_animation_state_machine_crossfade_weights() {
        let mut sm = AnimationStateMachine3D::new();
        sm.blend_duration = 0.2;
        assert_eq!(sm.current_state, AnimationState3D::Idle);
        assert_eq!(sm.blend_weight(), 1.0);

        // Run durumuna geçiş tetikle
        sm.transition_to(AnimationState3D::Run);
        assert_eq!(sm.current_state, AnimationState3D::Run);
        assert_eq!(sm.previous_state, Some(AnimationState3D::Idle));
        assert_eq!(sm.blend_weight(), 0.0);

        // 0.1 saniye ilerlet (yarı harmanlama)
        let root = Joint::new("Root", None, Mat4::IDENTITY);
        let mut skel = Skeleton::new(vec![root]);
        sm.update(0.1, &[], &mut skel);

        assert!((sm.blend_weight() - 0.5).abs() < 1e-4);
        assert_eq!(sm.previous_state, Some(AnimationState3D::Idle));

        // 0.1 saniye daha ilerlet (harmanlama tamamlanmalı)
        sm.update(0.1, &[], &mut skel);
        assert_eq!(sm.blend_weight(), 1.0);
        assert_eq!(sm.previous_state, None);
    }

    #[test]
    fn test_state_machine_transition_on_velocity() {
        let mut sm = AnimationStateMachine3D::new();
        assert_eq!(sm.current_state, AnimationState3D::Idle);

        // 1. Karakter zeminde ve hareket ediyor -> Run olmalı
        sm.update_from_velocity(Vec3::new(2.5, 0.0, 0.0), true);
        assert_eq!(sm.current_state, AnimationState3D::Run);

        // 2. Karakter durdu -> Idle olmalı
        sm.update_from_velocity(Vec3::ZERO, true);
        assert_eq!(sm.current_state, AnimationState3D::Idle);

        // 3. Karakter zıpladı (havada ve yukarı doğru hız) -> Jump olmalı
        sm.update_from_velocity(Vec3::new(1.0, 5.0, 0.0), false);
        assert_eq!(sm.current_state, AnimationState3D::Jump);

        // 4. Karakter düşüşe geçti (havada ve dikey hız negatif veya küçük) -> Fall olmalı
        sm.update_from_velocity(Vec3::new(1.0, -3.0, 0.0), false);
        assert_eq!(sm.current_state, AnimationState3D::Fall);
    }

    #[test]
    fn test_animation_3d_component_serde_roundtrip() {
        let root = Joint::new("Root", None, Mat4::IDENTITY);
        let hand = Joint::new("Hand", Some(0), Mat4::IDENTITY);
        let skel = Skeleton::new(vec![root, hand]);

        let channel = AnimationChannel3D {
            joint_index: 1,
            property: AnimationProperty::Translation,
            timestamps: vec![0.0, 1.0],
            vec3_values: vec![Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)],
            quat_values: vec![],
            interpolation: Interpolation::Linear,
        };
        let clip = AnimationClip3D::new("Walk", 1.0, vec![channel]);
        let comp = Animation3DComponent::new(skel, vec![clip]);

        let json = serde_json::to_string(&comp).expect("serialize failed");
        let deserialized: Animation3DComponent =
            serde_json::from_str(&json).expect("deserialize failed");

        assert_eq!(deserialized.skeleton.joints.len(), 2);
        assert_eq!(deserialized.skeleton.joints[0].name, "Root");
        assert_eq!(deserialized.skeleton.joints[1].name, "Hand");
        assert_eq!(deserialized.clips.len(), 1);
        assert_eq!(deserialized.clips[0].name, "Walk");
        assert_eq!(deserialized.current_palette[0], Mat4::IDENTITY);
    }

    #[test]
    fn test_skinning_matrix_palette_computation() {
        // Joint 0: Root at origin
        // Joint 1: Child of Root, translated by (0, 2, 0)
        let root = Joint::new("Root", None, Mat4::IDENTITY);
        let child_inverse_bind = Mat4::from_translation(Vec3::new(0.0, -2.0, 0.0));
        let child = Joint::new("Child", Some(0), child_inverse_bind);
        let skel = Skeleton::new(vec![root, child]);

        // Channel animating Child translation to (0, 5, 0)
        let channel = AnimationChannel3D {
            joint_index: 1,
            property: AnimationProperty::Translation,
            timestamps: vec![0.0, 1.0],
            vec3_values: vec![Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.0, 5.0, 0.0)],
            quat_values: vec![],
            interpolation: Interpolation::Linear,
        };
        let clip = AnimationClip3D::new("Lift", 2.0, vec![channel]);
        let mut comp = Animation3DComponent::new(skel, vec![clip]);

        // At t = 0, child pose translation is (0, 2, 0).
        // Palette = GlobalPose * InverseBind = Translate(0, 2, 0) * Translate(0, -2, 0) = Identity
        comp.update(0.0);
        let diff0 = comp.current_palette[1] - Mat4::IDENTITY;
        assert!(diff0.to_cols_array().iter().all(|&v| v.abs() < 1e-4));

        // Advance by 1.0 sec: Child translation is now (0, 5, 0) at timestamp 1.0.
        // Palette = Translate(0, 5, 0) * Translate(0, -2, 0) = Translate(0, 3, 0)
        comp.update(1.0);
        let expected = Mat4::from_translation(Vec3::new(0.0, 3.0, 0.0));
        let diff1 = comp.current_palette[1] - expected;
        assert!(diff1.to_cols_array().iter().all(|&v| v.abs() < 1e-4));
    }
}
