//! Rugeng Core (Ember Engine Çekirdeği)
//!
//! Lock-Free Frame Arena, Flat & Paged Sparse-Set ECS ve AccessSet Scheduler modeli.

pub mod animation;
pub mod animation_3d;
pub mod arena;
pub mod audio;
pub mod command_buffer;
pub mod components;
pub mod hierarchy;
pub mod input;
pub mod particle;
pub mod prefab;
pub mod registry;
pub mod scene;
pub mod script;
pub mod sparse_set;
pub mod tilemap;
pub mod transform_3d;
pub mod types;

pub use animation::{Animation2DComponent, AnimationClip, AnimationStateMachine, StateTransition};
pub use animation_3d::{
    Animation3DComponent, AnimationChannel3D, AnimationClip3D, AnimationProperty, AnimationState3D,
    AnimationStateMachine3D, Interpolation, Joint, Skeleton, MAX_JOINTS,
};
pub use arena::{FrameArena, ScopedScratch, DEFAULT_ARENA_CAPACITY};
pub use audio::{
    AudioBus, AudioEngine, AudioMixer, AudioRingBuffer, AudioStreamDriver, AudioVoice, SfxPreset,
    SoundClip,
};
pub use command_buffer::CommandBuffer;
pub use components::{
    ButtonState, Collider2D, Mesh3DComponent, NodeName, SpriteColor, SpriteTexture,
    Text2DComponent, Transform2D, UIButton2D, UIMenu2D, Velocity2D, ZIndex2D,
};
pub use hierarchy::{compute_world_transform, Children, Parent};
pub use input::{ActionInput, ActionMap};
pub use particle::{interpolate_particle_color, Particle, ParticleEmitterComponent};
pub use prefab::Prefab;
pub use registry::{AccessSet, AccessType, AnyStorage, Registry, StorageCell};
pub use scene::{SceneData, SceneEntity};
pub use script::{ScriptBehavior, ScriptComponent};
pub use sparse_set::{
    FlatSparseSet, FlatSparseTable, PagedSparseSet, PagedSparseTable, SparseSet, SparseTable,
    SPARSE_NULL_INDEX, SPARSE_PAGE_COUNT, SPARSE_PAGE_SIZE,
};
pub use tilemap::{TileData, TileType, Tilemap2DComponent};
pub use transform_3d::{compute_world_transform_3d, BoundingSphere, Frustum, Transform3D, AABB3D};
pub use types::Entity;

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn test_script_rotator_and_patrol() {
        let mut script = ScriptComponent::new_rotator(2.0);
        let mut pos = Vec2::ZERO;
        let mut size = Vec2::new(32.0, 32.0);
        let mut rot = 0.0;
        script.update(0.5, &mut pos, &mut size, &mut rot);
        assert!((rot - 1.0).abs() < 1e-4);

        let mut patrol = ScriptComponent::new_patrol(0, 0.0, 100.0, 50.0);
        patrol.update(1.0, &mut pos, &mut size, &mut rot);
        assert!((pos.x - 50.0).abs() < 1e-4);
        patrol.update(1.5, &mut pos, &mut size, &mut rot);
        assert!((pos.x - 100.0).abs() < 1e-4);
    }

    #[test]
    fn test_input_action_mapping() {
        let mut map = ActionMap::new();
        assert!(!map.is_action_pressed("action_fire"));
        map.update_key_event(0x39, true); // Space
        assert!(map.is_action_pressed("action_fire"));
        assert!(map.is_action_just_pressed("action_fire"));
        map.end_frame();
        assert!(map.is_action_pressed("action_fire"));
        assert!(!map.is_action_just_pressed("action_fire"));
        map.update_key_event(0x39, false);
        assert!(!map.is_action_pressed("action_fire"));
    }

    #[test]
    fn test_audio_synthesizer_wav_encoding() {
        let mut audio = AudioEngine::new();
        assert!(audio.clips.contains_key("sfx_laser"));
        assert!(audio.clips.contains_key("sfx_explosion"));
        let clip = &audio.clips["sfx_laser"];
        let wav = clip.to_wav_bytes();
        assert!(wav.starts_with(b"RIFF"));
        assert_eq!(&wav[8..12], b"WAVE");
        audio.play("sfx_laser");
        assert_eq!(audio.last_played_sound, "sfx_laser");
        assert_eq!(audio.active_voices, 1);
    }

    #[test]
    fn test_scene_serialization_roundtrip() {
        let mut scene = SceneData::new("MainLevel");
        let mut tm = Tilemap2DComponent::default();
        tm.set_tile(1, 2, TileType::Stone, true);

        scene.entities.push(SceneEntity {
            id: 1,
            name: "PlayerShip".to_string(),
            pos: [100.0, 200.0],
            size: [48.0, 48.0],
            rotation: 0.25,
            color: [255, 255, 255, 255],
            texture: Some("player_ship".to_string()),
            has_collider: true,
            is_dynamic: true,
            is_sensor: true,
            velocity: Some([10.0, -20.0]),
            script: Some("RotatorScript".to_string()),
            parent_id: Some(42),
            text: Some(Text2DComponent::new(
                "Hello Rugeng",
                1.5,
                [255, 255, 0, 255],
            )),
            animation: Some(Animation2DComponent::new()),
            tilemap: Some(tm),
            transform_3d: Some(Transform3D::from_xyz(1.0, 2.0, 3.0)),
            mesh_3d: Some(Mesh3DComponent::with_pbr(
                "cube",
                [1.0, 0.0, 0.0, 1.0],
                0.2,
                0.8,
            )),
            z_index: 3,
            particle: Some({
                let mut emitter = ParticleEmitterComponent::new_trail();
                // Runtime alanlar serileştirilmediğinden eşitlik iddiası için sıfırlanır.
                emitter.particles.clear();
                emitter.emission_timer = 0.0;
                emitter
            }),
        });

        let json = scene.to_json().expect("JSON serilestirme basarisiz");
        let decoded = SceneData::from_json(&json).expect("JSON okuma basarisiz");
        assert_eq!(scene, decoded);
    }

    #[test]
    fn test_particle_runtime_state_not_serialized() {
        let mut scene = SceneData::new("FxTest");
        // new_explosion canlı parçacıklarla dolu bir yayıcı üretir; bunlar diske yazılmamalı.
        let emitter = ParticleEmitterComponent::new_explosion(32, Vec2::ZERO);
        assert!(!emitter.particles.is_empty());

        scene.entities.push(SceneEntity {
            id: 1,
            name: "Fx".to_string(),
            pos: [0.0, 0.0],
            size: [8.0, 8.0],
            rotation: 0.0,
            color: [255, 255, 255, 255],
            texture: None,
            has_collider: false,
            is_dynamic: false,
            is_sensor: false,
            velocity: None,
            script: None,
            parent_id: None,
            text: None,
            animation: None,
            tilemap: None,
            transform_3d: None,
            mesh_3d: None,
            z_index: 0,
            particle: Some(emitter),
        });

        let json = scene.to_json().expect("JSON serilestirme basarisiz");
        let decoded = SceneData::from_json(&json).expect("JSON okuma basarisiz");
        let got = decoded.entities[0]
            .particle
            .as_ref()
            .expect("Yayici konfigürasyonu yuklenemedi");
        assert!(
            got.particles.is_empty(),
            "Canli parcacıklar serilestirilmis; runtime alani siziyor"
        );
        assert_eq!(got.max_particles, 32);
        assert_eq!(got.lifetime, 0.6);
        assert!(got.is_burst);
    }

    #[test]
    fn test_audio_bus_mixer_volumes() {
        let mut audio = AudioEngine::new();
        audio.set_bus_volume(AudioBus::Master, 0.5);
        audio.set_bus_volume(AudioBus::Sfx, 0.8);
        audio.set_bus_volume(AudioBus::Music, 0.6);

        assert!((audio.get_bus_volume(AudioBus::Master) - 0.5).abs() < 1e-4);
        assert!((audio.effective_volume(AudioBus::Sfx) - 0.4).abs() < 1e-4);
        assert!((audio.effective_volume(AudioBus::Music) - 0.3).abs() < 1e-4);
    }

    #[test]
    fn test_ui_button_and_menu_interaction() {
        let mut btn = UIButton2D::new("Başlat");
        assert_eq!(btn.state, ButtonState::Normal);
        assert!(!btn.is_clicked);

        let pos = Vec2::new(100.0, 100.0);
        let size = Vec2::new(80.0, 30.0);

        // Hover
        btn.update(pos, size, Vec2::new(110.0, 110.0), false, false);
        assert_eq!(btn.state, ButtonState::Hovered);
        assert!(!btn.is_clicked);

        // Pressed
        btn.update(pos, size, Vec2::new(110.0, 110.0), true, false);
        assert_eq!(btn.state, ButtonState::Pressed);

        // Released -> Clicked
        btn.update(pos, size, Vec2::new(110.0, 110.0), false, true);
        assert_eq!(btn.state, ButtonState::Hovered);
        assert!(btn.is_clicked);

        let mut menu = UIMenu2D::new(
            "Ana Menü",
            vec!["Oyna".into(), "Ayarlar".into(), "Çıkış".into()],
        );
        assert_eq!(menu.selected_option(), Some("Oyna"));
        menu.next();
        assert_eq!(menu.selected_option(), Some("Ayarlar"));
        menu.next();
        assert_eq!(menu.selected_option(), Some("Çıkış"));
        menu.next();
        assert_eq!(menu.selected_option(), Some("Oyna"));
        menu.prev();
        assert_eq!(menu.selected_option(), Some("Çıkış"));
    }
}

#[cfg(test)]
mod node_name_tests {
    use crate::{NodeName, SceneData, SceneEntity};

    #[test]
    fn node_name_serde_roundtrip() {
        let n = NodeName::new("Oyuncu Gemisi Şşğ");
        let json = serde_json::to_string(&n).unwrap();
        let back: NodeName = serde_json::from_str(&json).unwrap();
        assert_eq!(n, back);
        assert_eq!(back.name, "Oyuncu Gemisi Şşğ");
    }

    #[test]
    fn scene_entity_custom_name_survives_roundtrip() {
        let mut scene = SceneData::new("ad-test");
        scene.entities.push(SceneEntity {
            id: 7,
            name: "Düşman Kulesi".to_string(),
            pos: [1.0, 2.0],
            size: [3.0, 4.0],
            rotation: 0.0,
            color: [255, 255, 255, 255],
            texture: None,
            has_collider: false,
            is_dynamic: true,
            is_sensor: false,
            z_index: 0,
            velocity: None,
            script: None,
            parent_id: None,
            text: None,
            animation: None,
            tilemap: None,
            transform_3d: None,
            mesh_3d: None,
            particle: None,
        });
        let json = serde_json::to_string(&scene).unwrap();
        let back: SceneData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.entities[0].name, "Düşman Kulesi");
    }
}
