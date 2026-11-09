//! Project Rugeng — Ortak ECS Bileşenleri (Core Components).
//!
//! Tüm alt sistemler (editor, standalone, bullet_hell, render, physics)
//! tarafından paylaşılan temel 2D bileşenler. Kod tekrarını önler ve
//! serileştirilebilir veri yapısı sunar.

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// 2D Pozisyon, Boyut ve Rotasyon bileşeni
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transform2D {
    pub pos: Vec2,
    pub size: Vec2,
    pub rotation: f32,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            pos: Vec2::ZERO,
            size: Vec2::splat(32.0),
            rotation: 0.0,
        }
    }
}

impl Transform2D {
    pub fn new(pos: Vec2, size: Vec2, rotation: f32) -> Self {
        Self {
            pos,
            size,
            rotation,
        }
    }
}

/// Düz renkli quad çizim bileşeni (RGBA8)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteColor {
    pub color: [u8; 4],
}

impl Default for SpriteColor {
    fn default() -> Self {
        Self {
            color: [255, 255, 255, 255],
        }
    }
}

impl SpriteColor {
    pub fn new(color: [u8; 4]) -> Self {
        Self { color }
    }
}

/// Doku atlasından adlandırılmış bölge referansı
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteTexture {
    pub name: String,
}

impl SpriteTexture {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

/// 2D Katman Derinliği / Sıralama Bileşeni (Z-Index)
///
/// 2D sahnede çizim önceliğini (küçük değerler arka planda, büyükler ön planda)
/// belirler. 3D çalışma alanında (2.5D derinlik önizlemesinde) ise Z ekseninde
/// fiziksel katman ayrımı oluşturur (ör. Z = z * 0.25).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct ZIndex2D {
    pub z: i32,
}

impl ZIndex2D {
    pub fn new(z: i32) -> Self {
        Self { z }
    }
}

/// 2D Kutu çarpışma bileşeni (AABB)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collider2D {
    pub is_dynamic: bool,
    pub is_sensor: bool,
}

impl Collider2D {
    pub fn solid_static() -> Self {
        Self {
            is_dynamic: false,
            is_sensor: false,
        }
    }

    pub fn solid_dynamic() -> Self {
        Self {
            is_dynamic: true,
            is_sensor: false,
        }
    }

    pub fn sensor() -> Self {
        Self {
            is_dynamic: false,
            is_sensor: true,
        }
    }
}

/// 2D Doğrusal hız bileşeni
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Velocity2D {
    pub vel: Vec2,
}

impl Default for Velocity2D {
    fn default() -> Self {
        Self { vel: Vec2::ZERO }
    }
}

impl Velocity2D {
    pub fn new(vel: Vec2) -> Self {
        Self { vel }
    }
}

/// Sahne içi 2D vektörel metin çizim bileşeni
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Text2DComponent {
    pub text: String,
    pub scale: f32,
    pub color: [u8; 4],
}

impl Default for Text2DComponent {
    fn default() -> Self {
        Self {
            text: String::new(),
            scale: 1.0,
            color: [255, 255, 255, 255],
        }
    }
}

impl Text2DComponent {
    pub fn new(text: impl Into<String>, scale: f32, color: [u8; 4]) -> Self {
        Self {
            text: text.into(),
            scale,
            color,
        }
    }
}

/// UI Buton Etkileşim Durumu
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonState {
    Normal,
    Hovered,
    Pressed,
    Disabled,
}

/// 2D Arayüz Buton Bileşeni (UI Button)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UIButton2D {
    pub label: String,
    pub state: ButtonState,
    pub is_clicked: bool,
    pub bg_normal: [u8; 4],
    pub bg_hover: [u8; 4],
    pub bg_pressed: [u8; 4],
    pub text_color: [u8; 4],
}

impl Default for UIButton2D {
    fn default() -> Self {
        Self {
            label: "Button".to_string(),
            state: ButtonState::Normal,
            is_clicked: false,
            bg_normal: [45, 55, 72, 255],
            bg_hover: [66, 153, 225, 255],
            bg_pressed: [49, 130, 206, 255],
            text_color: [255, 255, 255, 255],
        }
    }
}

impl UIButton2D {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..Default::default()
        }
    }

    /// Fare imlecinin buton üzerinde olup olmadığını ve tıklanmayı test eder
    pub fn update(
        &mut self,
        btn_pos: Vec2,
        btn_size: Vec2,
        mouse_pos: Vec2,
        mouse_down: bool,
        mouse_just_released: bool,
    ) {
        if self.state == ButtonState::Disabled {
            self.is_clicked = false;
            return;
        }

        let inside = mouse_pos.x >= btn_pos.x
            && mouse_pos.x <= btn_pos.x + btn_size.x
            && mouse_pos.y >= btn_pos.y
            && mouse_pos.y <= btn_pos.y + btn_size.y;

        if inside {
            if mouse_down {
                self.state = ButtonState::Pressed;
                self.is_clicked = false;
            } else if mouse_just_released && self.state == ButtonState::Pressed {
                self.state = ButtonState::Hovered;
                self.is_clicked = true;
            } else {
                self.state = ButtonState::Hovered;
                self.is_clicked = false;
            }
        } else {
            self.state = ButtonState::Normal;
            self.is_clicked = false;
        }
    }

    pub fn current_bg_color(&self) -> [u8; 4] {
        match self.state {
            ButtonState::Normal => self.bg_normal,
            ButtonState::Hovered => self.bg_hover,
            ButtonState::Pressed => self.bg_pressed,
            ButtonState::Disabled => [30, 30, 30, 180],
        }
    }
}

/// Basit UI Menü Bileşeni (Seçenekler listesi ve Gezinme)
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UIMenu2D {
    pub title: String,
    pub options: Vec<String>,
    pub selected_index: usize,
    pub is_active: bool,
}

impl UIMenu2D {
    pub fn new(title: impl Into<String>, options: Vec<String>) -> Self {
        Self {
            title: title.into(),
            options,
            selected_index: 0,
            is_active: true,
        }
    }

    pub fn next(&mut self) {
        if !self.options.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.options.len();
        }
    }

    pub fn prev(&mut self) {
        if !self.options.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.options.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn selected_option(&self) -> Option<&str> {
        self.options.get(self.selected_index).map(|s| s.as_str())
    }
}

/// 3D Mesh Seçim Bileşeni (Cube, Sphere, Plane, Custom OBJ)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mesh3DComponent {
    pub mesh_name: String,
    pub color: [f32; 4],
    pub roughness: f32,
    pub metallic: f32,
}

impl Default for Mesh3DComponent {
    fn default() -> Self {
        Self {
            mesh_name: "cube".to_string(),
            color: [0.8, 0.8, 0.8, 1.0],
            roughness: 0.5,
            metallic: 0.0,
        }
    }
}

impl Mesh3DComponent {
    pub fn new(mesh_name: impl Into<String>, color: [f32; 4]) -> Self {
        Self {
            mesh_name: mesh_name.into(),
            color,
            roughness: 0.5,
            metallic: 0.0,
        }
    }

    pub fn with_pbr(
        mesh_name: impl Into<String>,
        color: [f32; 4],
        roughness: f32,
        metallic: f32,
    ) -> Self {
        Self {
            mesh_name: mesh_name.into(),
            color,
            roughness: roughness.clamp(0.04, 1.0),
            metallic: metallic.clamp(0.0, 1.0),
        }
    }
}

/// Kullanıcının atadığı düğüm adı (F2 ile yeniden adlandırma).
/// Bileşen yoksa editör `Node2D_#i:g` üretik adını gösterir; sahne
/// dosyasındaki `name` alanı yüklemede bu bileşene dönüştürülür.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeName {
    pub name: String,
}

impl NodeName {
    #[inline]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

/// 3D Noktasal Işık Bileşeni (Point Light).
///
/// Renk (RGB), Işık Şiddeti (Intensity) ve Etki Yarıçapı (Radius / Range).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointLight {
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            radius: 10.0,
        }
    }
}

impl PointLight {
    pub fn new(color: [f32; 3], intensity: f32, radius: f32) -> Self {
        Self {
            color,
            intensity: intensity.max(0.0),
            radius: radius.max(0.01),
        }
    }
}
