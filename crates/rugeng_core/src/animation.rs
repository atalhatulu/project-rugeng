//! 2D Sprite Sheet & Kare Animasyon (Frame-by-Frame Animator) Sistemi
//!
//! Kare tabanlı 2D animasyon klipleri, zamanlayıcı, oynatma/duraklatma ve döngü yönetimi.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnimationClip {
    pub name: String,
    pub frames: Vec<String>, // Atlas bölge isimleri (örneğin ["ship_f1", "ship_f2"])
    pub fps: f32,
    pub looping: bool,
}

impl AnimationClip {
    pub fn new(name: impl Into<String>, frames: Vec<String>, fps: f32, looping: bool) -> Self {
        Self {
            name: name.into(),
            frames,
            fps: fps.max(1.0),
            looping,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Animation2DComponent {
    pub clips: HashMap<String, AnimationClip>,
    pub current_clip: String,
    pub current_frame_idx: usize,
    pub timer: f32,
    pub is_playing: bool,
}

impl Default for Animation2DComponent {
    fn default() -> Self {
        Self {
            clips: HashMap::new(),
            current_clip: String::new(),
            current_frame_idx: 0,
            timer: 0.0,
            is_playing: true,
        }
    }
}

impl Animation2DComponent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_clip(mut self, clip: AnimationClip) -> Self {
        if self.current_clip.is_empty() {
            self.current_clip = clip.name.clone();
        }
        self.clips.insert(clip.name.clone(), clip);
        self
    }

    pub fn add_clip(&mut self, clip: AnimationClip) {
        if self.current_clip.is_empty() {
            self.current_clip = clip.name.clone();
        }
        self.clips.insert(clip.name.clone(), clip);
    }

    pub fn play(&mut self, clip_name: &str) {
        if self.clips.contains_key(clip_name) {
            if self.current_clip != clip_name {
                self.current_clip = clip_name.to_string();
                self.current_frame_idx = 0;
                self.timer = 0.0;
            }
            self.is_playing = true;
        }
    }

    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    pub fn resume(&mut self) {
        self.is_playing = true;
    }

    pub fn current_texture_name(&self) -> Option<&str> {
        let clip = self.clips.get(&self.current_clip)?;
        clip.frames.get(self.current_frame_idx).map(|s| s.as_str())
    }

    /// Zamanı ilerletir; eğer kare değiştiyse `true` döndürür.
    pub fn update(&mut self, dt: f32) -> bool {
        if !self.is_playing {
            return false;
        }

        let Some(clip) = self.clips.get(&self.current_clip) else {
            return false;
        };

        if clip.frames.is_empty() {
            return false;
        }

        self.timer += dt;
        let frame_duration = 1.0 / clip.fps;

        if self.timer >= frame_duration {
            self.timer -= frame_duration;
            let next_idx = self.current_frame_idx + 1;
            if next_idx >= clip.frames.len() {
                if clip.looping {
                    self.current_frame_idx = 0;
                } else {
                    self.is_playing = false;
                    return false;
                }
            } else {
                self.current_frame_idx = next_idx;
            }
            return true;
        }

        false
    }
}

/// Durumlar arası geçiş kuralı
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StateTransition {
    pub target_state: String,
    pub condition_param: String,
}

/// Animasyon Durum Makinesi (Animation State Machine - ASM)
///
/// Mantıksal durumlar (Idle, Walk, Jump, Fall, Attack vb.) arasındaki geçişleri
/// parametrelere (bool, float) göre otomatik yönetir ve ilgili AnimationClip'i oynatır.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AnimationStateMachine {
    pub states: HashMap<String, AnimationClip>,
    pub current_state: String,
    pub bool_params: HashMap<String, bool>,
    pub float_params: HashMap<String, f32>,
    pub transitions: HashMap<String, Vec<StateTransition>>,
    pub animator: Animation2DComponent,
}

impl AnimationStateMachine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_state(&mut self, name: &str, clip: AnimationClip) {
        if self.current_state.is_empty() {
            self.current_state = name.to_string();
            self.animator.play(&clip.name);
        }
        self.animator.add_clip(clip.clone());
        self.states.insert(name.to_string(), clip);
    }

    pub fn add_transition(&mut self, from_state: &str, target_state: &str, condition_param: &str) {
        self.transitions
            .entry(from_state.to_string())
            .or_default()
            .push(StateTransition {
                target_state: target_state.to_string(),
                condition_param: condition_param.to_string(),
            });
    }

    pub fn set_bool(&mut self, name: &str, value: bool) {
        self.bool_params.insert(name.to_string(), value);
    }

    pub fn get_bool(&self, name: &str) -> bool {
        self.bool_params.get(name).copied().unwrap_or(false)
    }

    pub fn set_float(&mut self, name: &str, value: f32) {
        self.float_params.insert(name.to_string(), value);
    }

    pub fn get_float(&self, name: &str) -> f32 {
        self.float_params.get(name).copied().unwrap_or(0.0)
    }

    pub fn set_state(&mut self, new_state: &str) {
        if self.current_state != new_state {
            if let Some(clip) = self.states.get(new_state) {
                self.current_state = new_state.to_string();
                self.animator.play(&clip.name);
            }
        }
    }

    pub fn update(&mut self, dt: f32) -> bool {
        // Geçiş kontrolleri (Transitions)
        if let Some(trans_list) = self.transitions.get(&self.current_state).cloned() {
            for trans in trans_list {
                if let Some(&val) = self.bool_params.get(&trans.condition_param) {
                    if val {
                        self.set_state(&trans.target_state);
                        break;
                    }
                }
            }
        }

        self.animator.update(dt)
    }

    pub fn current_texture_name(&self) -> Option<&str> {
        self.animator.current_texture_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_frame_progression() {
        let clip = AnimationClip::new(
            "run",
            vec!["f1".to_string(), "f2".to_string(), "f3".to_string()],
            10.0, // Her kare 0.1s
            true,
        );

        let mut anim = Animation2DComponent::new().with_clip(clip);
        assert_eq!(anim.current_texture_name(), Some("f1"));

        // 0.05 saniye sonra hala f1
        let changed = anim.update(0.05);
        assert!(!changed);
        assert_eq!(anim.current_frame_idx, 0);

        // 0.06 saniye daha geçince (toplam 0.11s) f2'ye geçmeli
        let changed = anim.update(0.06);
        assert!(changed);
        assert_eq!(anim.current_frame_idx, 1);
        assert_eq!(anim.current_texture_name(), Some("f2"));

        // 0.1s daha geçince f3
        anim.update(0.1);
        assert_eq!(anim.current_frame_idx, 2);

        // 0.1s daha geçince döngü başa (f1) dönmeli
        anim.update(0.1);
        assert_eq!(anim.current_frame_idx, 0);
    }

    #[test]
    fn test_animation_state_machine_transitions() {
        let idle_clip = AnimationClip::new("idle_clip", vec!["idle_1".to_string()], 5.0, true);
        let walk_clip = AnimationClip::new(
            "walk_clip",
            vec!["w1".to_string(), "w2".to_string()],
            10.0,
            true,
        );
        let jump_clip = AnimationClip::new("jump_clip", vec!["j1".to_string()], 5.0, false);

        let mut asm = AnimationStateMachine::new();
        asm.add_state("Idle", idle_clip);
        asm.add_state("Walk", walk_clip);
        asm.add_state("Jump", jump_clip);

        asm.add_transition("Idle", "Walk", "is_moving");
        asm.add_transition("Walk", "Jump", "is_jumping");

        assert_eq!(asm.current_state, "Idle");
        assert_eq!(asm.current_texture_name(), Some("idle_1"));

        // is_moving = false -> Durum değişmemeli
        asm.update(0.1);
        assert_eq!(asm.current_state, "Idle");

        // is_moving = true -> Walk durumuna geçmeli
        asm.set_bool("is_moving", true);
        asm.update(0.01);
        assert_eq!(asm.current_state, "Walk");
        assert_eq!(asm.current_texture_name(), Some("w1"));

        // is_jumping = true -> Jump durumuna geçmeli
        asm.set_bool("is_jumping", true);
        asm.update(0.01);
        assert_eq!(asm.current_state, "Jump");
        assert_eq!(asm.current_texture_name(), Some("j1"));
    }
}
