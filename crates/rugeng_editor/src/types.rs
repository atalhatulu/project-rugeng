use glam::{Vec2, Vec3};
use rugeng_core::{
    Animation2DComponent, Children, Collider2D, Entity, Mesh3DComponent, NodeName, Parent,
    ParticleEmitterComponent, ScriptComponent, SpriteColor, SpriteTexture, Text2DComponent,
    TileData, TileType, Tilemap2DComponent, Transform2D, Transform3D, Velocity2D, ZIndex2D,
};

#[derive(Clone, Debug)]
pub struct EntitySnapshot {
    pub entity: Entity,
    pub name: Option<NodeName>,
    pub transform: Transform2D,
    pub color: Option<SpriteColor>,
    pub texture: Option<SpriteTexture>,
    pub collider: Option<Collider2D>,
    pub velocity: Option<Velocity2D>,
    pub script: Option<ScriptComponent>,
    pub text: Option<Text2DComponent>,
    pub animation: Option<Animation2DComponent>,
    pub tilemap: Option<Tilemap2DComponent>,
    pub transform_3d: Option<Transform3D>,
    pub mesh_3d: Option<Mesh3DComponent>,
    pub parent: Option<Parent>,
    pub children: Option<Children>,
    pub z_index: Option<ZIndex2D>,
    pub particle: Option<ParticleEmitterComponent>,
}

#[derive(Clone, Debug)]
pub enum UndoAction {
    CreateEntity(Entity),
    DeleteEntity(Box<EntitySnapshot>),
    MoveEntity {
        entity: Entity,
        old_transform: Transform2D,
        new_transform: Transform2D,
    },
    TileChange {
        entity: Entity,
        gx: i32,
        gy: i32,
        old_tile: Option<TileData>,
        new_tile: Option<TileData>,
    },
    /// Denetçi (Inspector) 2D Transform düğmesiyle yapılan tek tık değişiklik.
    Transform2DChanged {
        entity: Entity,
        old: Transform2D,
        new: Transform2D,
    },
    /// Denetçi (Inspector) 3D Transform düğmesiyle yapılan tek tık değişiklik.
    Transform3DChanged {
        entity: Entity,
        old: Transform3D,
        new: Transform3D,
    },
    /// Denetçi doku atama/kaldırma eylemi (None = bileşen yoktu/kaldırıldı).
    SpriteTextureChanged {
        entity: Entity,
        old: Option<SpriteTexture>,
        new: Option<SpriteTexture>,
    },
    SpriteColorChanged {
        entity: Entity,
        old: Option<SpriteColor>,
        new: Option<SpriteColor>,
    },
    ZIndexChanged {
        entity: Entity,
        old: Option<ZIndex2D>,
        new: Option<ZIndex2D>,
    },
    Collider2DChanged {
        entity: Entity,
        old: Option<Collider2D>,
        new: Option<Collider2D>,
    },
    ScriptChanged {
        entity: Entity,
        old: Option<ScriptComponent>,
        new: Option<ScriptComponent>,
    },
    /// F2 ile yeniden adlandırma (NodeName bileşeni).
    NodeNameChanged {
        entity: Entity,
        old: Option<String>,
        new: Option<String>,
    },
}

/// Satır içi metin düzenleme durumu (F2: düğüm yeniden adlandırma).
/// Yan etkisiz tutulur; birim testleriyle doğrulanır. `caret` KAREKTER
/// cinsindendir (byte değil) — Türkçe/çok baytlık metinlerde kaymaz.
#[derive(Clone, Debug, PartialEq)]
pub struct InlineTextEdit {
    pub entity: Entity,
    pub buffer: String,
    pub caret: usize,
}

impl InlineTextEdit {
    pub const MAX_LEN: usize = 64;

    /// Karakteri imleç konumuna ekler; kontrol karakterlerini ve taşmayı reddeder.
    pub fn insert(&mut self, ch: char) {
        if ch.is_control() || self.buffer.chars().count() >= Self::MAX_LEN {
            return;
        }
        let mut cs: Vec<char> = self.buffer.chars().collect();
        cs.insert(self.caret.min(cs.len()), ch);
        self.buffer = cs.into_iter().collect();
        self.caret += 1;
    }

    /// İmlecin solundaki karakteri siler.
    pub fn backspace(&mut self) -> bool {
        let mut cs: Vec<char> = self.buffer.chars().collect();
        self.caret = self.caret.min(cs.len());
        if self.caret == 0 {
            return false;
        }
        cs.remove(self.caret - 1);
        self.buffer = cs.into_iter().collect();
        self.caret -= 1;
        true
    }

    /// İmlecin sağındaki karakteri siler (Delete tuşu).
    pub fn delete_forward(&mut self) -> bool {
        let mut cs: Vec<char> = self.buffer.chars().collect();
        self.caret = self.caret.min(cs.len());
        if self.caret < cs.len() {
            cs.remove(self.caret);
            self.buffer = cs.into_iter().collect();
            true
        } else {
            false
        }
    }

    /// İmleci taşır; [0, len] aralığına kenetler.
    pub fn move_caret(&mut self, delta: i32) {
        let len = self.buffer.chars().count() as i32;
        self.caret = (self.caret as i32 + delta).clamp(0, len) as usize;
    }

    pub fn home(&mut self) {
        self.caret = 0;
    }

    pub fn end(&mut self) {
        self.caret = self.buffer.chars().count();
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct BoxSelection {
    pub start_world: Vec2,
    pub current_world: Vec2,
}

#[derive(Clone, Debug)]
pub struct ContextMenuItem {
    pub label: String,
    pub shortcut: String,
    pub action_id: String,
}

#[derive(Clone, Debug)]
pub struct ContextMenu {
    pub screen_pos: Vec2,
    pub target_entity: Option<Entity>,
    pub items: Vec<ContextMenuItem>,
}

pub struct LogMessage {
    pub level: &'static str,
    pub text: String,
}

pub struct EditorCamera {
    pub pos: Vec2,
    pub zoom: f32,
    pub is_panning: bool,
    pub last_mouse: Vec2,
    pub trauma: f32,
    pub shake_timer: f32,
    pub follow_target: Option<Entity>,
}

impl EditorCamera {
    pub fn new() -> Self {
        Self {
            pos: Vec2::ZERO,
            zoom: 0.826,
            is_panning: false,
            last_mouse: Vec2::ZERO,
            trauma: 0.0,
            shake_timer: 0.0,
            follow_target: None,
        }
    }

    pub fn add_trauma(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).clamp(0.0, 1.0);
    }

    pub fn update(&mut self, dt: f32, target_pos: Option<Vec2>) {
        if self.trauma > 0.0 {
            self.trauma = (self.trauma - dt * 1.5).max(0.0);
            self.shake_timer += dt * 30.0;
        }
        if let Some(tp) = target_pos {
            self.pos = self.pos.lerp(tp, (dt * 6.0).min(1.0));
        }
    }

    pub fn get_effective_pos(&self) -> Vec2 {
        if self.trauma > 0.0 {
            let shake = self.trauma * self.trauma;
            let offset_x = (self.shake_timer.sin() * 12.0) * shake;
            let offset_y = ((self.shake_timer * 1.3).cos() * 12.0) * shake;
            self.pos + Vec2::new(offset_x, offset_y)
        } else {
            self.pos
        }
    }
}

/// 3D Serbest Uçuş & Editör Kamerası
pub struct EditorCamera3D {
    pub eye: Vec3,
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fov_degrees: f32,
    pub move_speed: f32,
    pub is_orbiting: bool,
    pub is_panning: bool,
    pub is_flying: bool,
    pub is_turbo: bool,
    pub last_mouse: Vec2,
}

impl Default for EditorCamera3D {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorCamera3D {
    pub fn new() -> Self {
        Self {
            eye: Vec3::new(0.0, 4.0, 8.0),
            target: Vec3::ZERO,
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: -0.4,
            fov_degrees: 60.0,
            move_speed: 8.0,
            is_orbiting: false,
            is_panning: false,
            is_flying: false,
            is_turbo: false,
            last_mouse: Vec2::ZERO,
        }
    }

    pub fn forward(&self) -> Vec3 {
        let cos_pitch = self.pitch.cos();
        Vec3::new(
            cos_pitch * self.yaw.cos(),
            self.pitch.sin(),
            cos_pitch * self.yaw.sin(),
        )
        .normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward()).normalize()
    }

    pub fn to_render_camera(&self, aspect_ratio: f32) -> rugeng_render::Camera3D {
        let fwd = self.forward();
        rugeng_render::Camera3D::new(
            self.eye,
            self.eye + fwd,
            Vec3::Y,
            self.fov_degrees.to_radians(),
            aspect_ratio,
            0.1,
            1000.0,
        )
    }

    pub fn dolly(&mut self, amount: f32) {
        let fwd = self.forward();
        self.eye += fwd * amount;
    }

    pub fn effective_speed(&self) -> f32 {
        if self.is_turbo {
            self.move_speed * 2.5
        } else {
            self.move_speed
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum WorkspaceMode {
    TwoD,
    ThreeD,
    Script,
    Game,
    AssetStore,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum LeftTab {
    Scene,
    Import,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum BottomShelfTab {
    Output,
    Debugger,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum SceneTab {
    #[allow(dead_code)]
    Player,
    World,
    #[allow(dead_code)]
    Empty,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum OpenMenu {
    Scene,
    Edit,
    Add,
    Project,
    Debug,
    Help,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ViewportTool {
    Select,
    Move,
    Rotate,
    Scale,
    TileBrush,
    TileEraser,
}

pub const VIEWPORT_TOOLS: [(ViewportTool, &str, f32); 6] = [
    (ViewportTool::Select, "Sec [Q]", 60.0),
    (ViewportTool::Move, "Tasi [W]", 66.0),
    (ViewportTool::Rotate, "Dondur [E]", 80.0),
    (ViewportTool::Scale, "Boyut [R]", 74.0),
    (ViewportTool::TileBrush, "Firca", 50.0),
    (ViewportTool::TileEraser, "Silgi", 48.0),
];

pub const TILE_PALETTE_ITEMS: [(&str, TileType, f32); 5] = [
    ("Cimen", TileType::Grass, 42.0),
    ("Tas", TileType::Stone, 36.0),
    ("Su", TileType::Water, 34.0),
    ("Lav", TileType::Lava, 34.0),
    ("Metal", TileType::Metal, 42.0),
];

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum DragMode {
    None,
    Translate {
        start_entity_pos: Vec2,
        start_world_mouse: Vec2,
    },
    Scale {
        start_size: Vec2,
        start_world_mouse: Vec2,
    },
    Rotate {
        center: Vec2,
        start_rotation: f32,
        start_angle: f32,
    },
    Translate3D {
        axis: u8, // 0: X, 1: Y, 2: Z
        start_translation: Vec3,
        start_mouse: Vec2,
    },
    Rotate3D {
        axis: u8, // 0: X, 1: Y, 2: Z
        start_rotation: glam::Quat,
        start_mouse: Vec2,
    },
    Scale3D {
        axis: u8, // 0: X, 1: Y, 2: Z, 3: Uniform
        start_scale: Vec3,
        start_mouse: Vec2,
    },
}

#[derive(Clone, Debug)]
pub struct FileItem {
    pub name: String,
    pub is_dir: bool,
}

#[cfg(test)]
mod inline_text_edit_tests {
    use super::InlineTextEdit;
    use rugeng_core::Entity;

    fn edit(seed: &str) -> InlineTextEdit {
        InlineTextEdit {
            entity: Entity::new(1, 0),
            buffer: seed.to_string(),
            caret: seed.chars().count(),
        }
    }

    #[test]
    fn caret_inserts_in_middle_and_shifts_right() {
        let mut e = edit("abc");
        e.home();
        e.move_caret(1); // "a|bc"
        e.insert('ğ');
        assert_eq!(e.buffer, "ağbc");
        assert_eq!(e.caret, 2);
    }

    #[test]
    fn backspace_at_home_is_noop_delete_forward_eats_right() {
        let mut e = edit("abc");
        e.home();
        assert!(!e.backspace());
        assert_eq!(e.buffer, "abc");
        assert!(e.delete_forward());
        assert_eq!(e.buffer, "bc");
        e.end();
        assert!(!e.delete_forward());
    }

    #[test]
    fn caret_clamps_at_both_ends_on_multibyte_buffer() {
        let mut e = edit("şğı"); // 3 karakter, 6 bayt
        e.move_caret(-99);
        assert_eq!(e.caret, 0);
        e.move_caret(99);
        assert_eq!(e.caret, 3);
        e.move_caret(-1); // caret 2 → 'ğ' silinecek
        assert!(e.backspace());
        assert_eq!(e.buffer, "şı");
        assert_eq!(e.caret, 1);
    }

    #[test]
    fn delete_forward_on_multibyte_is_boundary_safe() {
        let mut e = edit("aşı");
        e.home();
        e.move_caret(1);
        assert!(e.delete_forward()); // 'ş' 2 bayt — byte dilimlemede patlardı
        assert_eq!(e.buffer, "aı");
    }

    #[test]
    fn insert_accepts_printable_unicode() {
        let mut e = edit("");
        for c in "Ğüplayer 1-#".chars() {
            e.insert(c);
        }
        assert_eq!(e.buffer, "Ğüplayer 1-#");
        assert_eq!(e.buffer.chars().count(), 12);
    }

    #[test]
    fn insert_rejects_control_chars() {
        let mut e = edit("ab");
        e.insert('\n');
        e.insert('\t');
        e.insert('\x1b');
        assert_eq!(e.buffer, "ab");
    }

    #[test]
    fn insert_enforces_max_len_in_chars_not_bytes() {
        let mut e = edit("");
        for _ in 0..(InlineTextEdit::MAX_LEN + 16) {
            e.insert('ğ'); // 2 byte'lık karakter: byte sayacı yanıltır
        }
        assert_eq!(e.buffer.chars().count(), InlineTextEdit::MAX_LEN);
    }

    #[test]
    fn backspace_pops_and_reports() {
        let mut e = edit("abc");
        assert!(e.backspace());
        assert_eq!(e.buffer, "ab");
        e.buffer.clear(); // stale caret 2 kalır — panik değil no-op olmalı
        assert!(!e.backspace());
        assert!(e.buffer.is_empty());
    }

    #[test]
    fn backspace_on_multibyte_char_is_char_boundary_safe() {
        let mut e = edit("şa");
        assert!(e.backspace());
        assert_eq!(e.buffer, "ş"); // byte dilimleme olsa paniklerdi
    }
}
