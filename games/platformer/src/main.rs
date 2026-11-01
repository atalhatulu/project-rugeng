//! Rugeng 2D — Mario Tarzı Platform Oyunu Prototipi (Platformer)
//!
//! Özellikler:
//! - Kinematik Oyuncu Kontrolcüsü: Yerçekimi, ivmelenme, hava sürtünmesi, değişken zıplama gücü.
//! - Tilemap Entegrasyonu: Zemin, taş engeller, platform blokları ve boşluklar.
//! - Çift Yönlü Çarpışma: `resolve_aabb_collision_with_velocity` ile zemin / tavan / duvar ayrıştırması.
//! - Zemin Kontrolü (`is_on_floor`): Yerle temas halinde zıplama yetkisi ve dikey hız sıfırlama.
//! - Dinamik Kamera Takibi: Yumuşak X ekseni takibi (lerp) ve dikey ölüm sınırı (Pit Death).
//! - Toplanabilir Yıldızlar/Paralar: Mantar/altın toplama mekaniği ve anlık skor hesabı.
//! - Can ve Oyun Bitişi / Yeniden Başlatma (`[R]`).

use glam::{Mat4, Vec2};
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use rugeng_core::{
    ActionInput, ActionMap, AnimationClip, AnimationStateMachine, AudioBus, AudioEngine,
    CommandBuffer, FrameArena, Registry, SfxPreset, TileType, Tilemap2DComponent, UIButton2D,
};
use rugeng_physics::{resolve_aabb_collision_with_velocity, resolve_one_way_platform};
use rugeng_render::context::VulkanContext;
use rugeng_render::font;
use rugeng_render::pipeline_2d::{Pipeline2D, QuadInstance};
use rugeng_render::texture::TextureAtlas;

const FIXED_DT: f32 = 1.0 / 60.0;
const GRAVITY: f32 = 980.0; // px/s^2 aşağı doğru çekim
const RUN_ACCEL: f32 = 1400.0; // px/s^2 koşu ivmesi
const MAX_RUN_SPEED: f32 = 280.0; // Maksimum yatay hız
const FRICTION: f32 = 1200.0; // Zemin sürtünmesi
const JUMP_VELOCITY: f32 = -480.0; // Zıplama anlık hızı (Yukarı negatif Y)
const PIT_DEATH_Y: f32 = 720.0; // Bu seviyenin altına düşen oyuncu ölür

#[derive(Clone, Copy, Debug)]
pub struct Transform2D {
    pub pos: Vec2,
    pub size: Vec2,
    pub rotation: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Velocity2D {
    pub vel: Vec2,
}

#[derive(Clone, Copy, Debug)]
pub struct Coin {
    pub value: u32,
    pub collected: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct EnemyGoomba {
    pub dir: f32,
    pub speed: f32,
    pub alive: bool,
}

struct PlayerState {
    pos: Vec2,
    vel: Vec2,
    size: Vec2,
    is_on_floor: bool,
    health: i32,
    score: u32,
    facing_right: bool,
    anim_time: f32,
    anim_sm: AnimationStateMachine,
}

struct Camera2D {
    pos: Vec2,
    viewport_size: Vec2,
}

impl Camera2D {
    pub fn update(&mut self, target_x: f32, dt: f32) {
        // Yumuşak yatay takip (lerp)
        let target_cam_x = target_x - self.viewport_size.x * 0.35;
        self.pos.x += (target_cam_x - self.pos.x) * (dt * 6.0).min(1.0);
        // Minimum sol sınır
        if self.pos.x < 0.0 {
            self.pos.x = 0.0;
        }
    }
}

struct InputState {
    left: bool,
    right: bool,
    jump: bool,
    jump_held: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GameScreen {
    Playing,
    PauseMenu,
    SettingsMenu,
    ControlsMenu,
}

struct App {
    window: Option<Arc<Window>>,
    context: Option<VulkanContext>,
    pipeline_2d: Option<Pipeline2D>,
    atlas: Option<TextureAtlas>,
    arena: FrameArena,
    registry: Registry,
    #[allow(dead_code)]
    cmd_buffer: CommandBuffer,
    tilemap: Tilemap2DComponent,
    player: PlayerState,
    camera: Camera2D,
    input: InputState,
    action_map: ActionMap,
    audio: AudioEngine,
    current_screen: GameScreen,
    current_level_idx: usize,
    mouse_pos: Vec2,
    mouse_down: bool,
    mouse_just_released: bool,
    btn_resume: UIButton2D,
    btn_save: UIButton2D,
    btn_load: UIButton2D,
    btn_settings: UIButton2D,
    btn_controls: UIButton2D,
    btn_restart: UIButton2D,
    status_notification: Option<(String, Instant)>,
    last_frame_time: Instant,
    accumulator: f32,
    game_over: bool,
    level_finished: bool,
    fps: u32,
    fps_counter: u32,
    fps_timer: Instant,
}

impl App {
    fn new() -> Self {
        let mut action_map = ActionMap::new();
        // Varsayılan platformer girdileri (Aşama 6 G1: Girdi Eşleme):
        // 0x1E = KeyA, 0xCB = ArrowLeft
        // 0x20 = KeyD, 0xCD = ArrowRight
        // 0x39 = Space, 0x11 = KeyW, 0xC8 = ArrowUp
        action_map.bind("move_left", ActionInput::Key(0x1E));
        action_map.bind("move_left", ActionInput::Key(0xCB));
        action_map.bind("move_right", ActionInput::Key(0x20));
        action_map.bind("move_right", ActionInput::Key(0xCD));
        action_map.bind("jump", ActionInput::Key(0x39));
        action_map.bind("jump", ActionInput::Key(0x11));
        action_map.bind("jump", ActionInput::Key(0xC8));

        // Animasyon Durum Makinesi Kurulumu (Aşama 6 G1: ASM)
        let mut anim_sm = AnimationStateMachine::new();
        anim_sm.add_state(
            "idle",
            AnimationClip::new("idle", vec!["player_idle".to_string()], 4.0, true),
        );
        anim_sm.add_state(
            "run",
            AnimationClip::new(
                "run",
                vec!["player_run_1".to_string(), "player_run_2".to_string()],
                10.0,
                true,
            ),
        );
        anim_sm.add_state(
            "jump",
            AnimationClip::new("jump", vec!["player_jump".to_string()], 1.0, false),
        );

        anim_sm.add_transition("idle", "run", "is_moving");
        anim_sm.add_transition("idle", "jump", "is_jumping");
        anim_sm.add_transition("run", "idle", "is_idle");
        anim_sm.add_transition("run", "jump", "is_jumping");
        anim_sm.add_transition("jump", "idle", "is_idle");
        anim_sm.add_transition("jump", "run", "is_moving");

        let mut app = Self {
            window: None,
            context: None,
            pipeline_2d: None,
            atlas: None,
            arena: FrameArena::new(8 * 1024 * 1024).expect("FrameArena tahsis edilemedi"),
            registry: Registry::new(),
            cmd_buffer: CommandBuffer::new(),
            tilemap: Tilemap2DComponent::new(32.0),
            player: PlayerState {
                pos: Vec2::new(80.0, 360.0),
                vel: Vec2::ZERO,
                size: Vec2::new(26.0, 32.0),
                is_on_floor: false,
                health: 3,
                score: 0,
                facing_right: true,
                anim_time: 0.0,
                anim_sm,
            },
            camera: Camera2D {
                pos: Vec2::ZERO,
                viewport_size: Vec2::new(1280.0, 720.0),
            },
            input: InputState {
                left: false,
                right: false,
                jump: false,
                jump_held: false,
            },
            action_map,
            audio: AudioEngine::new(),
            current_screen: GameScreen::Playing,
            current_level_idx: 1,
            mouse_pos: Vec2::ZERO,
            mouse_down: false,
            mouse_just_released: false,
            btn_resume: UIButton2D::new("Devam Et (P)"),
            btn_save: UIButton2D::new("Oyunu Kaydet (F5)"),
            btn_load: UIButton2D::new("Oyunu Yukle (F9)"),
            btn_settings: UIButton2D::new("Ses Ayarlari"),
            btn_controls: UIButton2D::new("Kontroller"),
            btn_restart: UIButton2D::new("Bastan Basla (R)"),
            status_notification: None,
            last_frame_time: Instant::now(),
            accumulator: 0.0,
            game_over: false,
            level_finished: false,
            fps: 60,
            fps_counter: 0,
            fps_timer: Instant::now(),
        };

        app.build_test_level();
        app
    }

    fn get_save_path() -> std::path::PathBuf {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let candidate = dir.join("saves").join("platformer_quicksave.json");
                if dir.join("saves").exists() || std::fs::create_dir_all(dir.join("saves")).is_ok()
                {
                    return candidate;
                }
            }
        }
        let _ = std::fs::create_dir_all("saves");
        std::path::PathBuf::from("saves/platformer_quicksave.json")
    }

    /// Oyunu JSON formatında kaydeder (Aşama 6 G1: Kayıt/Yükleme)
    pub fn save_game_state(&mut self) {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct SaveData {
            score: u32,
            health: i32,
            level_idx: usize,
            player_pos: [f32; 2],
            master_vol: f32,
            sfx_vol: f32,
            music_vol: f32,
        }

        let data = SaveData {
            score: self.player.score,
            health: self.player.health,
            level_idx: self.current_level_idx,
            player_pos: [self.player.pos.x, self.player.pos.y],
            master_vol: self.audio.master_volume,
            sfx_vol: self.audio.sfx_volume,
            music_vol: self.audio.music_volume,
        };

        if let Ok(json) = serde_json::to_string_pretty(&data) {
            let path = Self::get_save_path();
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::write(&path, json).is_ok() {
                self.status_notification = Some(("OYUN KAYDEDILDI!".to_string(), Instant::now()));
                self.audio.play_preset(SfxPreset::Powerup);
                return;
            }
        }
        self.status_notification = Some(("KAYIT HATASI!".to_string(), Instant::now()));
    }

    /// Kaydedilmiş oyunu yükler (Aşama 6 G1: Kayıt/Yükleme)
    pub fn load_game_state(&mut self) {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct SaveData {
            score: u32,
            health: i32,
            level_idx: usize,
            player_pos: [f32; 2],
            master_vol: f32,
            sfx_vol: f32,
            music_vol: f32,
        }

        let path = Self::get_save_path();
        let content = std::fs::read_to_string(&path)
            .or_else(|_| std::fs::read_to_string("saves/platformer_quicksave.json"));

        if let Ok(json) = content {
            if let Ok(data) = serde_json::from_str::<SaveData>(&json) {
                self.player.score = data.score;
                self.player.health = data.health;
                self.current_level_idx = data.level_idx;
                self.player.pos = Vec2::new(data.player_pos[0], data.player_pos[1]);
                self.player.vel = Vec2::ZERO;
                self.game_over = false;
                self.level_finished = false;
                self.audio.set_bus_volume(AudioBus::Master, data.master_vol);
                self.audio.set_bus_volume(AudioBus::Sfx, data.sfx_vol);
                self.audio.set_bus_volume(AudioBus::Music, data.music_vol);
                self.status_notification = Some(("OYUN YUKLENDI!".to_string(), Instant::now()));
                self.audio.play_preset(SfxPreset::Coin);
                return;
            }
        }
        self.status_notification = Some(("KAYIT BULUNAMADI!".to_string(), Instant::now()));
    }

    /// Klasik Mario 1-1 esintili bir seviye inşa eder
    fn build_test_level(&mut self) {
        self.tilemap.clear();
        self.player.pos = Vec2::new(80.0, 360.0);
        self.player.vel = Vec2::ZERO;
        self.player.is_on_floor = false;
        self.player.health = 3;
        self.player.score = 0;
        self.game_over = false;
        self.level_finished = false;
        self.camera.pos = Vec2::ZERO;

        // Varlıkları temizle
        let coins = self.registry.entities::<Coin>();
        for c in coins {
            self.registry.destroy(c);
        }
        let goombas = self.registry.entities::<EnemyGoomba>();
        for g in goombas {
            self.registry.destroy(g);
        }

        let ground_y = 14; // y = 14 (14 * 32 = 448 px)

        // 1. Zemin (Boşluklarla Mario tarzı çukurlar)
        for gx in 0..120 {
            // Çukurlar (Pits): gx = 25..28 ve gx = 65..68 arası zemin yok
            if (25..29).contains(&gx) || (65..69).contains(&gx) {
                continue;
            }
            // Üst katman Çimen, altı Taş
            self.tilemap.set_tile(gx, ground_y, TileType::Grass, true);
            self.tilemap
                .set_tile(gx, ground_y + 1, TileType::Stone, true);
            self.tilemap
                .set_tile(gx, ground_y + 2, TileType::Stone, true);
        }

        // 2. Platform Tuğlaları ve Tek Yönlü Atlama Köprüleri (One-Way Jump-Through)
        // İlk yükselti (Tuğla dizisi)
        for gx in 10..16 {
            self.tilemap
                .set_tile(gx, ground_y - 4, TileType::Metal, true);
        }
        // İkinci yükselti (Alttan zıplanıp üstüne çıkılabilen tek yönlü platformlar)
        for gx in 20..24 {
            self.tilemap
                .set_tile_ex(gx, ground_y - 3, TileType::Stone, true, true);
        }

        // Çukur üstü tek yönlü atlama köprüsü
        self.tilemap
            .set_tile_ex(26, ground_y - 3, TileType::Metal, true, true);
        self.tilemap
            .set_tile_ex(27, ground_y - 3, TileType::Metal, true, true);

        // Boru (Pipe) benzetimi (Taş sütunlar)
        self.tilemap
            .set_tile(35, ground_y - 1, TileType::Metal, true);
        self.tilemap
            .set_tile(35, ground_y - 2, TileType::Metal, true);
        self.tilemap
            .set_tile(36, ground_y - 1, TileType::Metal, true);
        self.tilemap
            .set_tile(36, ground_y - 2, TileType::Metal, true);

        // Merdiven piramidi (Merdiven adımları)
        for step in 0..5 {
            for y_offset in 0..=step {
                self.tilemap
                    .set_tile(45 + step, ground_y - y_offset, TileType::Stone, true);
            }
        }

        // Bölüm sonu bayrak direği kaidesi
        for y_offset in 1..8 {
            self.tilemap
                .set_tile(105, ground_y - y_offset, TileType::Metal, false);
        }
        // Bayrak tepesi
        self.tilemap
            .set_tile(105, ground_y - 8, TileType::Lava, false);

        // 3. Toplanabilir Altın Paralar (Coins)
        let coin_coords = [
            (11.0, 9.0),
            (12.0, 9.0),
            (13.0, 9.0),
            (14.0, 9.0),
            (21.0, 10.0),
            (22.0, 10.0),
            (26.5, 10.0),
            (35.5, 11.0),
            (47.0, 8.0),
            (48.0, 8.0),
            (80.0, 13.0),
            (82.0, 13.0),
            (84.0, 13.0),
        ];
        for (cx, cy) in coin_coords {
            let e = self.registry.create();
            self.registry.emplace(
                e,
                Transform2D {
                    pos: Vec2::new(cx * 32.0, cy * 32.0),
                    size: Vec2::new(18.0, 18.0),
                    rotation: 0.0,
                },
            );
            self.registry.emplace(
                e,
                Coin {
                    value: 100,
                    collected: false,
                },
            );
        }

        // 4. Düşmanlar (Goomba Benzeri Yürüyen Devriye)
        let enemy_spots = [(30.0, 13.0), (55.0, 13.0), (75.0, 13.0), (90.0, 13.0)];
        for (ex, ey) in enemy_spots {
            let e = self.registry.create();
            self.registry.emplace(
                e,
                Transform2D {
                    pos: Vec2::new(ex * 32.0, ey * 32.0),
                    size: Vec2::new(26.0, 26.0),
                    rotation: 0.0,
                },
            );
            self.registry.emplace(
                e,
                Velocity2D {
                    vel: Vec2::new(-60.0, 0.0),
                },
            );
            self.registry.emplace(
                e,
                EnemyGoomba {
                    dir: -1.0,
                    speed: 60.0,
                    alive: true,
                },
            );
        }
    }

    /// 60 Hz Sabit Fizik Güncellemesi (Kinematic Character Controller)
    fn fixed_update(&mut self) {
        if self.current_screen != GameScreen::Playing || self.game_over || self.level_finished {
            return;
        }

        // --- 1. Oyuncu Yatay İvmelenmesi ve Sürtünme ---
        if self.input.left {
            self.player.vel.x -= RUN_ACCEL * FIXED_DT;
            self.player.facing_right = false;
        } else if self.input.right {
            self.player.vel.x += RUN_ACCEL * FIXED_DT;
            self.player.facing_right = true;
        } else {
            // Zemin sürtünmesi ile yavaşlama
            let sign = self.player.vel.x.signum();
            let mag = self.player.vel.x.abs();
            let new_mag = (mag - FRICTION * FIXED_DT).max(0.0);
            self.player.vel.x = new_mag * sign;
        }
        self.player.vel.x = self.player.vel.x.clamp(-MAX_RUN_SPEED, MAX_RUN_SPEED);

        // --- 2. Dikey Yerçekimi ve Zıplama ---
        self.player.vel.y += GRAVITY * FIXED_DT;

        if self.input.jump {
            if self.player.is_on_floor {
                self.player.vel.y = JUMP_VELOCITY;
                self.player.is_on_floor = false;
                self.audio.play_preset(SfxPreset::Jump);
            }
            self.input.jump = false; // Tek atımlık tüketim
        }
        // Değişken zıplama yüksekliği (Tuş erken bırakılırsa zıplamayı kes)
        if !self.input.jump_held && self.player.vel.y < -150.0 {
            self.player.vel.y = -150.0;
        }

        // Konum Tahmini
        let mut new_pos = self.player.pos + self.player.vel * FIXED_DT;
        let mut was_on_floor = false;

        // --- 3. Tilemap Katı Blok Çarpışma Çözümü (Çift Yönlü Katı) ---
        let solid_boxes = self.tilemap.solid_aabb_boxes();
        for (tile_pos, tile_size) in solid_boxes {
            // Oyuncuya yakın olan karoları filtrele (AABB geniş mesafe elemesi)
            let diff = (tile_pos - new_pos).abs();
            if diff.x > 80.0 || diff.y > 80.0 {
                continue;
            }

            let mut static_pos = tile_pos;
            let mut player_vel = self.player.vel;
            let collided = resolve_aabb_collision_with_velocity(
                &mut new_pos,
                Some(&mut player_vel),
                self.player.size,
                true,
                &mut static_pos,
                None,
                tile_size,
                false,
            );

            if collided {
                // Eğer oyuncu bloğun üzerine basıyorsa (aşağı doğru itilme/yukarı normal)
                if tile_pos.y > new_pos.y && player_vel.y <= 0.01 {
                    was_on_floor = true;
                }
                self.player.vel = player_vel;
            }
        }

        // --- 3.1. Tek Yönlü Platform Çözümü (One-Way Jump-Through) ---
        let one_way_boxes = self.tilemap.one_way_aabb_boxes();
        for (plat_pos, plat_size) in one_way_boxes {
            let diff = (plat_pos - new_pos).abs();
            if diff.x > 80.0 || diff.y > 80.0 {
                continue;
            }

            let collided = resolve_one_way_platform(
                &mut new_pos,
                &mut self.player.vel,
                self.player.size,
                plat_pos,
                plat_size,
            );

            if collided {
                was_on_floor = true;
            }
        }

        self.player.pos = new_pos;
        self.player.is_on_floor = was_on_floor;

        // Karakter animasyon durum makinesi ve koşu sayacı (Aşama 6 G1: ASM)
        let is_moving = self.player.is_on_floor && self.player.vel.x.abs() > 10.0;
        let is_jumping = !self.player.is_on_floor;
        let is_idle = self.player.is_on_floor && self.player.vel.x.abs() <= 10.0;

        self.player.anim_sm.set_bool("is_moving", is_moving);
        self.player.anim_sm.set_bool("is_jumping", is_jumping);
        self.player.anim_sm.set_bool("is_idle", is_idle);
        self.player
            .anim_sm
            .set_float("speed", self.player.vel.x.abs());
        self.player.anim_sm.update(FIXED_DT);

        if is_moving {
            self.player.anim_time += FIXED_DT * (self.player.vel.x.abs() / 25.0);
        } else if self.player.is_on_floor {
            self.player.anim_time = 0.0;
        }

        // --- 4. Düşman Hareketi ve Oyuncu Etkileşimi ---
        let p_pos = self.player.pos;
        let p_size = self.player.size;
        let mut player_hit = false;
        let mut enemy_stomped_list = Vec::new();

        self.registry
            .view3_ref_ref_ref::<Transform2D, Velocity2D, EnemyGoomba, _>(|e, tr, _vel, goomba| {
                if !goomba.alive {
                    return;
                }
                let e_pos = tr.pos;
                let e_size = tr.size;

                // Çarpışma kontrolü
                let diff = (e_pos - p_pos).abs();
                let half_sum = (p_size + e_size) * 0.5;
                if diff.x < half_sum.x && diff.y < half_sum.y {
                    // Oyuncu düşmanın kafasına yukarıdan zıplayarak mı basıyor?
                    if p_pos.y < e_pos.y - 10.0 && self.player.vel.y > 0.0 {
                        enemy_stomped_list.push(e);
                    } else {
                        player_hit = true;
                    }
                }
            });

        // Ezilen düşmanları yok et ve oyuncuyu zıplat
        for e in enemy_stomped_list {
            self.registry.destroy(e);
            self.player.vel.y = JUMP_VELOCITY * 0.75; // Küçük sıçrama
            self.player.score += 200;
            self.audio.play_preset(SfxPreset::Stomp);
        }

        // Oyuncu hasar aldı mı?
        if player_hit {
            self.player.health -= 1;
            if self.player.health <= 0 {
                self.game_over = true;
                self.audio.play_preset(SfxPreset::Explosion);
            } else {
                // Geri tepme (Knockback)
                self.player.vel = Vec2::new(
                    if self.player.facing_right {
                        -220.0
                    } else {
                        220.0
                    },
                    -280.0,
                );
                self.audio.play_preset(SfxPreset::Hit);
            }
        }

        // --- 5. Düşman Devriye (Patrol) Güncellemesi ---
        let mut enemies_to_update = Vec::new();
        self.registry
            .view3_ref_ref_ref::<Transform2D, Velocity2D, EnemyGoomba, _>(|e, tr, vel, goomba| {
                if !goomba.alive {
                    return;
                }
                let mut n_pos = tr.pos + vel.vel * FIXED_DT;
                let mut dir = goomba.dir;

                // Duvarlara veya boşluğa çarpınca yön değiştir
                for (tile_pos, tile_size) in &self.tilemap.solid_aabb_boxes() {
                    let diff = (*tile_pos - n_pos).abs();
                    if diff.x > 60.0 || diff.y > 60.0 {
                        continue;
                    }
                    let mut s_pos = *tile_pos;
                    let mut v = vel.vel;
                    if resolve_aabb_collision_with_velocity(
                        &mut n_pos,
                        Some(&mut v),
                        tr.size,
                        true,
                        &mut s_pos,
                        None,
                        *tile_size,
                        false,
                    ) {
                        // Yatayda çarptıysa yön tersle
                        if v.x.abs() < 5.0 {
                            dir = -dir;
                        }
                    }
                }
                enemies_to_update.push((e, n_pos, Vec2::new(dir * goomba.speed, 0.0), dir));
            });

        for (e, n_pos, n_vel, dir) in enemies_to_update {
            if let Some(tr) = self.registry.get_mut::<Transform2D>(e) {
                tr.pos = n_pos;
            }
            if let Some(vel) = self.registry.get_mut::<Velocity2D>(e) {
                vel.vel = n_vel;
            }
            if let Some(g) = self.registry.get_mut::<EnemyGoomba>(e) {
                g.dir = dir;
            }
        }

        // --- 6. Altın Toplama ---
        let mut collected_coins = Vec::new();
        self.registry
            .view2_ref_ref::<Transform2D, Coin, _>(|e, tr, coin| {
                if coin.collected {
                    return;
                }
                let diff = (tr.pos - p_pos).abs();
                if diff.x < (p_size.x + tr.size.x) * 0.5 && diff.y < (p_size.y + tr.size.y) * 0.5 {
                    collected_coins.push((e, coin.value));
                }
            });
        for (e, val) in collected_coins {
            self.registry.destroy(e);
            self.player.score += val;
            self.audio.play_preset(SfxPreset::Coin);
        }

        // --- 7. Çukura Düşme ve Bölüm Sonu Kontrolü ---
        if self.player.pos.y > PIT_DEATH_Y {
            if !self.game_over {
                self.audio.play_preset(SfxPreset::Explosion);
            }
            self.player.health = 0;
            self.game_over = true;
        }

        // Bayrak direğine ulaşıldı mı? (x >= 105 * 32 = 3360 px)
        if self.player.pos.x >= 3340.0 && !self.level_finished {
            self.level_finished = true;
            self.player.score += 5000;
            self.audio.play_preset(SfxPreset::Powerup);
        }

        // --- 8. Kamera Takibi ---
        self.camera.update(self.player.pos.x, FIXED_DT);
    }

    /// Render Döngüsü
    fn render(&mut self) {
        let Some(win) = self.window.clone() else {
            return;
        };
        let sz = win.inner_size();
        let screen_w = sz.width as f32;
        let screen_h = sz.height as f32;
        self.camera.viewport_size = Vec2::new(screen_w, screen_h);

        self.arena.reset();
        let mut instances = Vec::with_capacity(2048);

        let cam_x = self.camera.pos.x;

        // 1. Gökyüzü Arka Planı (Açık Retro Mavi)
        instances.push(QuadInstance::new(
            [0.0, 0.0],
            [screen_w, screen_h],
            [107, 140, 255, 255],
            0,
        ));

        // 2. Tilemap Çizimi (Ekran sınırlarına göre kırpılmış)
        for (&(gx, gy), tile_data) in &self.tilemap.tiles {
            let world_p = self.tilemap.grid_to_world(gx, gy);
            let screen_px = world_p.x - cam_x - self.tilemap.tile_size * 0.5;
            let screen_py = world_p.y - self.tilemap.tile_size * 0.5;

            // Görünür alanda mı?
            if screen_px + self.tilemap.tile_size < 0.0 || screen_px > screen_w {
                continue;
            }

            instances.push(QuadInstance::new(
                [screen_px, screen_py],
                [self.tilemap.tile_size, self.tilemap.tile_size],
                tile_data.color,
                0,
            ));

            // Blok üstü hafif gölge/derinlik kenarlığı
            instances.push(QuadInstance::new(
                [screen_px, screen_py],
                [self.tilemap.tile_size, 2.0],
                [255, 255, 255, 80],
                0,
            ));
        }

        // 3. Toplanabilir Altınlar (Dönen Paralar)
        self.registry
            .view2_ref_ref::<Transform2D, Coin, _>(|_e, tr, _coin| {
                let screen_px = tr.pos.x - cam_x - tr.size.x * 0.5;
                let screen_py = tr.pos.y - tr.size.y * 0.5;
                if screen_px + tr.size.x >= 0.0 && screen_px <= screen_w {
                    instances.push(QuadInstance::new(
                        [screen_px, screen_py],
                        [tr.size.x, tr.size.y],
                        [255, 215, 0, 255], // Altın Sarısı
                        0,
                    ));
                    instances.push(QuadInstance::new(
                        [screen_px + 3.0, screen_py + 3.0],
                        [tr.size.x - 6.0, tr.size.y - 6.0],
                        [255, 245, 140, 255],
                        0,
                    ));
                }
            });

        // 4. Düşmanlar (Goomba - Kestane Kahvesi)
        self.registry
            .view2_ref_ref::<Transform2D, EnemyGoomba, _>(|_e, tr, _g| {
                let screen_px = tr.pos.x - cam_x - tr.size.x * 0.5;
                let screen_py = tr.pos.y - tr.size.y * 0.5;
                if screen_px + tr.size.x >= 0.0 && screen_px <= screen_w {
                    instances.push(QuadInstance::new(
                        [screen_px, screen_py],
                        [tr.size.x, tr.size.y],
                        [178, 68, 38, 255], // Goomba Kahvesi
                        0,
                    ));
                    // Gözler
                    instances.push(QuadInstance::new(
                        [screen_px + 4.0, screen_py + 6.0],
                        [4.0, 8.0],
                        [255, 255, 255, 255],
                        0,
                    ));
                    instances.push(QuadInstance::new(
                        [screen_px + 14.0, screen_py + 6.0],
                        [4.0, 8.0],
                        [255, 255, 255, 255],
                        0,
                    ));
                }
            });

        // 5. Oyuncu (Mario - Canlı Kırmızı & Mavi Tulum)
        let p_screen_x = self.player.pos.x - cam_x - self.player.size.x * 0.5;
        let mut p_screen_y = self.player.pos.y - self.player.size.y * 0.5;
        let mut p_w = self.player.size.x;
        let mut p_h = self.player.size.y;

        // Havada ve koşuda dinamik deformasyon (Squash & Stretch)
        if !self.player.is_on_floor {
            // Zıplarken boy uzaması (Stretch)
            p_w *= 0.88;
            p_h *= 1.12;
            p_screen_y -= (p_h - self.player.size.y) * 0.5;
        } else if self.player.vel.x.abs() > 10.0 {
            // Koşarken hafif adım salınımı (Bobbing)
            let bob = (self.player.anim_time * 1.5).sin() * 2.0;
            p_screen_y += bob;
        }

        // Üst gövde / Şapka (Kırmızı)
        instances.push(QuadInstance::new(
            [p_screen_x, p_screen_y],
            [p_w, p_h * 0.5],
            [235, 45, 35, 255],
            0,
        ));
        // Alt tulum (Mavi)
        instances.push(QuadInstance::new(
            [p_screen_x, p_screen_y + p_h * 0.5],
            [p_w, p_h * 0.5],
            [35, 75, 215, 255],
            0,
        ));
        // Koşarken adım atan ayak pikselleri
        if self.player.is_on_floor && self.player.vel.x.abs() > 10.0 {
            let step_offset = (self.player.anim_time * 2.0).sin() * 4.0;
            let foot_x = if self.player.facing_right {
                p_screen_x + p_w * 0.5 + step_offset
            } else {
                p_screen_x + step_offset
            };
            instances.push(QuadInstance::new(
                [
                    foot_x.clamp(p_screen_x - 4.0, p_screen_x + p_w),
                    p_screen_y + p_h - 2.0,
                ],
                [8.0, 4.0],
                [160, 40, 30, 255],
                0,
            ));
        }

        // Göz / Bakış yönü
        let eye_x = if self.player.facing_right {
            p_screen_x + p_w * 0.7
        } else {
            p_screen_x + 4.0
        };
        instances.push(QuadInstance::new(
            [eye_x, p_screen_y + 4.0],
            [4.0, 4.0],
            [255, 255, 255, 255],
            0,
        ));

        // 6. HUD Göstergeleri (Can, Skor, İlerleme)
        instances.push(QuadInstance::new(
            [20.0, 20.0],
            [360.0, 48.0],
            [20, 25, 35, 220],
            0,
        ));
        instances.push(QuadInstance::new(
            [20.0, 20.0],
            [360.0, 2.0],
            [255, 220, 40, 255],
            0,
        ));

        font::draw_text(
            &mut instances,
            &format!("SKOR: {:06}", self.player.score),
            32.0,
            26.0,
            1.8,
            [255, 230, 80, 255],
        );
        font::draw_text(
            &mut instances,
            &format!("CAN: {}/3  |  FPS: {}", self.player.health.max(0), self.fps),
            32.0,
            46.0,
            1.5,
            [230, 240, 255, 255],
        );

        let progress = ((self.player.pos.x / 3340.0) * 100.0).clamp(0.0, 100.0);
        font::draw_text(
            &mut instances,
            &format!("ILERLEME: %{:.0}", progress),
            230.0,
            46.0,
            1.5,
            [120, 230, 160, 255],
        );

        // 7. Oyun Bitti (Game Over) Ekranı
        if self.game_over {
            let bw = 540.0;
            let bh = 120.0;
            let bx = (screen_w - bw) * 0.5;
            let by = (screen_h - bh) * 0.5;
            instances.push(QuadInstance::new([bx, by], [bw, bh], [30, 15, 20, 240], 0));
            instances.push(QuadInstance::new(
                [bx, by],
                [bw, 2.0],
                [240, 60, 60, 255],
                0,
            ));
            instances.push(QuadInstance::new(
                [bx, by + bh - 2.0],
                [bw, 2.0],
                [240, 60, 60, 255],
                0,
            ));

            font::draw_text(
                &mut instances,
                "OYUN BITTI (GAME OVER)",
                bx + 80.0,
                by + 30.0,
                2.2,
                [255, 80, 80, 255],
            );
            font::draw_text(
                &mut instances,
                "TEKRAR DENEMEK ICIN [R] TUSUNA BASIN",
                bx + 60.0,
                by + 70.0,
                1.6,
                [230, 230, 230, 255],
            );
        }

        // 9. Durum Bildirim Çubuğu (Kayıt/Yükleme Bilgisi)
        if let Some((ref text, time)) = self.status_notification {
            if time.elapsed().as_secs_f32() < 3.0 {
                let n_w = 320.0;
                let n_h = 36.0;
                let n_x = (screen_w - n_w) * 0.5;
                let n_y = 24.0;
                instances.push(QuadInstance::new(
                    [n_x, n_y],
                    [n_w, n_h],
                    [20, 30, 45, 230],
                    0,
                ));
                instances.push(QuadInstance::new(
                    [n_x, n_y],
                    [n_w, 2.0],
                    [80, 200, 255, 255],
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    text,
                    n_x + 20.0,
                    n_y + 10.0,
                    1.6,
                    [100, 220, 255, 255],
                );
            }
        }

        // 10. Menüler (Duraklatma, Ses Ayarları, Kontrol Remap)
        match self.current_screen {
            GameScreen::Playing => {
                // Oyun akarken ekranın sağ üst köşesinde küçük menü ipucu
                font::draw_text(
                    &mut instances,
                    "[ESC / P]: MENÜ  |  [F5]: KAYDET  |  [F9]: YÜKLE",
                    screen_w - 480.0,
                    26.0,
                    1.4,
                    [200, 220, 240, 200],
                );
            }
            GameScreen::PauseMenu => {
                // Karartma katmanı
                instances.push(QuadInstance::new(
                    [0.0, 0.0],
                    [screen_w, screen_h],
                    [10, 15, 25, 210],
                    0,
                ));

                let mw = 420.0;
                let mh = 360.0;
                let mx = (screen_w - mw) * 0.5;
                let my = (screen_h - mh) * 0.5;

                instances.push(QuadInstance::new([mx, my], [mw, mh], [25, 30, 42, 250], 0));
                instances.push(QuadInstance::new(
                    [mx, my],
                    [mw, 3.0],
                    [99, 179, 237, 255],
                    0,
                ));

                font::draw_text(
                    &mut instances,
                    "OYUN DURAKLATILDI",
                    mx + 70.0,
                    my + 25.0,
                    2.2,
                    [255, 255, 255, 255],
                );

                // Butonlar
                let btn_w = 280.0;
                let btn_h = 36.0;
                let btn_x = mx + (mw - btn_w) * 0.5;

                let render_btn =
                    |btn: &mut UIButton2D, y: f32, instances: &mut Vec<QuadInstance>| {
                        btn.update(
                            Vec2::new(btn_x, y),
                            Vec2::new(btn_w, btn_h),
                            self.mouse_pos,
                            self.mouse_down,
                            self.mouse_just_released,
                        );
                        instances.push(QuadInstance::new(
                            [btn_x, y],
                            [btn_w, btn_h],
                            btn.current_bg_color(),
                            0,
                        ));
                        instances.push(QuadInstance::new(
                            [btn_x, y],
                            [btn_w, 1.0],
                            [255, 255, 255, 60],
                            0,
                        ));
                        font::draw_text(
                            instances,
                            &btn.label,
                            btn_x + 20.0,
                            y + 10.0,
                            1.6,
                            btn.text_color,
                        );
                    };

                render_btn(&mut self.btn_resume, my + 80.0, &mut instances);
                render_btn(&mut self.btn_save, my + 125.0, &mut instances);
                render_btn(&mut self.btn_load, my + 170.0, &mut instances);
                render_btn(&mut self.btn_settings, my + 215.0, &mut instances);
                render_btn(&mut self.btn_controls, my + 260.0, &mut instances);
                render_btn(&mut self.btn_restart, my + 305.0, &mut instances);

                // Buton tıklama eylemleri
                if self.btn_resume.is_clicked {
                    self.current_screen = GameScreen::Playing;
                    self.audio.play_preset(SfxPreset::Blip);
                } else if self.btn_save.is_clicked {
                    self.save_game_state();
                } else if self.btn_load.is_clicked {
                    self.load_game_state();
                } else if self.btn_settings.is_clicked {
                    self.current_screen = GameScreen::SettingsMenu;
                    self.audio.play_preset(SfxPreset::Blip);
                } else if self.btn_controls.is_clicked {
                    self.current_screen = GameScreen::ControlsMenu;
                    self.audio.play_preset(SfxPreset::Blip);
                } else if self.btn_restart.is_clicked {
                    self.build_test_level();
                    self.current_screen = GameScreen::Playing;
                    self.audio.play_preset(SfxPreset::Blip);
                }
            }
            GameScreen::SettingsMenu => {
                instances.push(QuadInstance::new(
                    [0.0, 0.0],
                    [screen_w, screen_h],
                    [10, 15, 25, 210],
                    0,
                ));

                let mw = 480.0;
                let mh = 320.0;
                let mx = (screen_w - mw) * 0.5;
                let my = (screen_h - mh) * 0.5;

                instances.push(QuadInstance::new([mx, my], [mw, mh], [25, 30, 42, 250], 0));
                instances.push(QuadInstance::new(
                    [mx, my],
                    [mw, 3.0],
                    [72, 187, 120, 255],
                    0,
                ));

                font::draw_text(
                    &mut instances,
                    "SES MIKSERI AYARLARI",
                    mx + 80.0,
                    my + 25.0,
                    2.0,
                    [255, 255, 255, 255],
                );

                // Master Vol
                let m_vol_pct = (self.audio.master_volume * 100.0) as u32;
                font::draw_text(
                    &mut instances,
                    &format!("ANA SES (MASTER): %{}", m_vol_pct),
                    mx + 40.0,
                    my + 80.0,
                    1.6,
                    [220, 230, 240, 255],
                );
                // Ses barı
                instances.push(QuadInstance::new(
                    [mx + 40.0, my + 105.0],
                    [300.0, 12.0],
                    [45, 55, 72, 255],
                    0,
                ));
                instances.push(QuadInstance::new(
                    [mx + 40.0, my + 105.0],
                    [300.0 * self.audio.master_volume, 12.0],
                    [66, 153, 225, 255],
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "[- / +]",
                    mx + 360.0,
                    my + 102.0,
                    1.5,
                    [255, 220, 40, 255],
                );

                // SFX Vol
                let s_vol_pct = (self.audio.sfx_volume * 100.0) as u32;
                font::draw_text(
                    &mut instances,
                    &format!("EFEKT SESI (SFX): %{}", s_vol_pct),
                    mx + 40.0,
                    my + 140.0,
                    1.6,
                    [220, 230, 240, 255],
                );
                instances.push(QuadInstance::new(
                    [mx + 40.0, my + 165.0],
                    [300.0, 12.0],
                    [45, 55, 72, 255],
                    0,
                ));
                instances.push(QuadInstance::new(
                    [mx + 40.0, my + 165.0],
                    [300.0 * self.audio.sfx_volume, 12.0],
                    [72, 187, 120, 255],
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "[1 / 2]",
                    mx + 360.0,
                    my + 162.0,
                    1.5,
                    [255, 220, 40, 255],
                );

                // Music Vol
                let mus_vol_pct = (self.audio.music_volume * 100.0) as u32;
                font::draw_text(
                    &mut instances,
                    &format!("MUZIK SESI: %{}", mus_vol_pct),
                    mx + 40.0,
                    my + 200.0,
                    1.6,
                    [220, 230, 240, 255],
                );
                instances.push(QuadInstance::new(
                    [mx + 40.0, my + 225.0],
                    [300.0, 12.0],
                    [45, 55, 72, 255],
                    0,
                ));
                instances.push(QuadInstance::new(
                    [mx + 40.0, my + 225.0],
                    [300.0 * self.audio.music_volume, 12.0],
                    [237, 137, 54, 255],
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "[3 / 4]",
                    mx + 360.0,
                    my + 222.0,
                    1.5,
                    [255, 220, 40, 255],
                );

                font::draw_text(
                    &mut instances,
                    "[ESC / B]: Geri Don",
                    mx + 160.0,
                    my + 270.0,
                    1.6,
                    [180, 190, 200, 255],
                );
            }
            GameScreen::ControlsMenu => {
                instances.push(QuadInstance::new(
                    [0.0, 0.0],
                    [screen_w, screen_h],
                    [10, 15, 25, 210],
                    0,
                ));

                let mw = 520.0;
                let mh = 340.0;
                let mx = (screen_w - mw) * 0.5;
                let my = (screen_h - mh) * 0.5;

                instances.push(QuadInstance::new([mx, my], [mw, mh], [25, 30, 42, 250], 0));
                instances.push(QuadInstance::new(
                    [mx, my],
                    [mw, 3.0],
                    [237, 100, 166, 255],
                    0,
                ));

                font::draw_text(
                    &mut instances,
                    "KONTROL EŞLEME (INPUT REMAP)",
                    mx + 60.0,
                    my + 25.0,
                    2.0,
                    [255, 255, 255, 255],
                );

                font::draw_text(
                    &mut instances,
                    "SOLA HAREKET :  [A] veya [SOL OK]",
                    mx + 40.0,
                    my + 80.0,
                    1.6,
                    [220, 230, 240, 255],
                );
                font::draw_text(
                    &mut instances,
                    "SAGA HAREKET :  [D] veya [SAG OK]",
                    mx + 40.0,
                    my + 115.0,
                    1.6,
                    [220, 230, 240, 255],
                );
                font::draw_text(
                    &mut instances,
                    "ZIPLAMA      :  [BOSLUK] veya [W] / [YUKARI OK]",
                    mx + 40.0,
                    my + 150.0,
                    1.6,
                    [220, 230, 240, 255],
                );
                font::draw_text(
                    &mut instances,
                    "DURAKLAT / MENU: [ESC] veya [P]",
                    mx + 40.0,
                    my + 185.0,
                    1.6,
                    [220, 230, 240, 255],
                );
                font::draw_text(
                    &mut instances,
                    "HIZLI KAYIT  :  [F5]",
                    mx + 40.0,
                    my + 220.0,
                    1.6,
                    [220, 230, 240, 255],
                );
                font::draw_text(
                    &mut instances,
                    "HIZLI YUKLE  :  [F9]",
                    mx + 40.0,
                    my + 255.0,
                    1.6,
                    [220, 230, 240, 255],
                );

                font::draw_text(
                    &mut instances,
                    "[ESC / B]: Geri Don",
                    mx + 180.0,
                    my + 295.0,
                    1.6,
                    [180, 190, 200, 255],
                );
            }
        }

        // Vulkan Render & Sunum
        let Some(ctx) = self.context.as_mut() else {
            return;
        };
        let Some(pipe) = self.pipeline_2d.as_mut() else {
            return;
        };

        let arena_instances = self.arena.alloc_slice_copy(&instances).unwrap();
        let frame_idx = ctx.current_frame;

        if let Ok(Some((image_index, cmd))) = ctx.begin_frame(screen_w as u32, screen_h as u32) {
            let count = unsafe { pipe.upload_instances(frame_idx, arena_instances) };
            ctx.begin_rendering(cmd, image_index, [0.42, 0.55, 1.0, 1.0]);

            let view_proj = Mat4::orthographic_rh(0.0, screen_w, 0.0, screen_h, -1.0, 1.0);

            pipe.draw(
                &ctx.device,
                cmd,
                frame_idx,
                count as u32,
                view_proj,
                ctx.swapchain.extent,
            );

            ctx.end_rendering(cmd, image_index);
            if let Err(e) = ctx.end_frame(image_index, cmd, screen_w as u32, screen_h as u32) {
                eprintln!("end_frame hatasi: {e}");
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let win_attrs = Window::default_attributes()
                .with_title("Project Rugeng — 2D Platformer Prototype (Mario Style)")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

            let window = Arc::new(
                event_loop
                    .create_window(win_attrs)
                    .expect("Pencere olusturulamadi"),
            );
            let size = window.inner_size();

            println!(
                "[Platformer] Vulkan Context ve Pipeline2D baslatiliyor: {}x{}",
                size.width, size.height
            );
            let context = VulkanContext::new(&*window, size.width, size.height)
                .expect("Vulkan Context baslatilamadi");
            let atlas = TextureAtlas::new(&context, 512, 512).expect("TextureAtlas baslatilamadi");
            let pipeline_2d = Pipeline2D::new(
                &context.device,
                Arc::clone(&context.allocator),
                context.swapchain.format,
                rugeng_render::FRAMES_IN_FLIGHT,
                atlas.image_view,
                atlas.sampler,
            )
            .expect("Pipeline2D baslatilamadi");

            self.window = Some(window);
            self.context = Some(context);
            self.atlas = Some(atlas);
            self.pipeline_2d = Some(pipeline_2d);
            self.last_frame_time = Instant::now();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                if let Some(ctx) = self.context.as_mut() {
                    ctx.resized = true;
                    let _ = ctx.recreate_swapchain(new_size.width, new_size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = Vec2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == winit::event::MouseButton::Left {
                    if state == ElementState::Pressed {
                        self.mouse_down = true;
                        self.mouse_just_released = false;
                    } else {
                        self.mouse_down = false;
                        self.mouse_just_released = true;
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                match key {
                    KeyCode::KeyA => {
                        self.action_map.update_key_event(0x1E, pressed);
                        self.input.left = self.action_map.is_action_pressed("move_left");
                    }
                    KeyCode::ArrowLeft => {
                        self.action_map.update_key_event(0xCB, pressed);
                        self.input.left = self.action_map.is_action_pressed("move_left");
                    }
                    KeyCode::KeyD => {
                        self.action_map.update_key_event(0x20, pressed);
                        self.input.right = self.action_map.is_action_pressed("move_right");
                    }
                    KeyCode::ArrowRight => {
                        self.action_map.update_key_event(0xCD, pressed);
                        self.input.right = self.action_map.is_action_pressed("move_right");
                    }
                    KeyCode::Space => {
                        self.action_map.update_key_event(0x39, pressed);
                        if pressed && !self.input.jump_held {
                            self.input.jump = true;
                        }
                        self.input.jump_held = self.action_map.is_action_pressed("jump");
                    }
                    KeyCode::KeyW => {
                        self.action_map.update_key_event(0x11, pressed);
                        if pressed && !self.input.jump_held {
                            self.input.jump = true;
                        }
                        self.input.jump_held = self.action_map.is_action_pressed("jump");
                    }
                    KeyCode::ArrowUp => {
                        self.action_map.update_key_event(0xC8, pressed);
                        if pressed && !self.input.jump_held {
                            self.input.jump = true;
                        }
                        self.input.jump_held = self.action_map.is_action_pressed("jump");
                    }
                    KeyCode::KeyP => {
                        if pressed {
                            self.current_screen = match self.current_screen {
                                GameScreen::Playing => GameScreen::PauseMenu,
                                _ => GameScreen::Playing,
                            };
                            self.audio.play_preset(SfxPreset::Blip);
                        }
                    }
                    KeyCode::KeyB => {
                        if pressed {
                            self.current_screen = match self.current_screen {
                                GameScreen::SettingsMenu | GameScreen::ControlsMenu => {
                                    GameScreen::PauseMenu
                                }
                                GameScreen::PauseMenu => GameScreen::Playing,
                                GameScreen::Playing => GameScreen::PauseMenu,
                            };
                            self.audio.play_preset(SfxPreset::Blip);
                        }
                    }
                    KeyCode::F5 => {
                        if pressed {
                            self.save_game_state();
                        }
                    }
                    KeyCode::F9 => {
                        if pressed {
                            self.load_game_state();
                        }
                    }
                    KeyCode::Minus | KeyCode::NumpadSubtract => {
                        if pressed {
                            let nv = (self.audio.master_volume - 0.1).max(0.0);
                            self.audio.set_bus_volume(AudioBus::Master, nv);
                            self.audio.play_preset(SfxPreset::Blip);
                        }
                    }
                    KeyCode::Equal | KeyCode::NumpadAdd => {
                        if pressed {
                            let nv = (self.audio.master_volume + 0.1).min(1.0);
                            self.audio.set_bus_volume(AudioBus::Master, nv);
                            self.audio.play_preset(SfxPreset::Blip);
                        }
                    }
                    KeyCode::Digit1 => {
                        if pressed && self.current_screen == GameScreen::SettingsMenu {
                            let nv = (self.audio.sfx_volume - 0.1).max(0.0);
                            self.audio.set_bus_volume(AudioBus::Sfx, nv);
                            self.audio.play_preset(SfxPreset::Blip);
                        }
                    }
                    KeyCode::Digit2 => {
                        if pressed && self.current_screen == GameScreen::SettingsMenu {
                            let nv = (self.audio.sfx_volume + 0.1).min(1.0);
                            self.audio.set_bus_volume(AudioBus::Sfx, nv);
                            self.audio.play_preset(SfxPreset::Blip);
                        }
                    }
                    KeyCode::Digit3 => {
                        if pressed && self.current_screen == GameScreen::SettingsMenu {
                            let nv = (self.audio.music_volume - 0.1).max(0.0);
                            self.audio.set_bus_volume(AudioBus::Music, nv);
                            self.audio.play_preset(SfxPreset::Blip);
                        }
                    }
                    KeyCode::Digit4 => {
                        if pressed && self.current_screen == GameScreen::SettingsMenu {
                            let nv = (self.audio.music_volume + 0.1).min(1.0);
                            self.audio.set_bus_volume(AudioBus::Music, nv);
                            self.audio.play_preset(SfxPreset::Blip);
                        }
                    }
                    KeyCode::KeyR => {
                        if pressed && (self.game_over || self.level_finished) {
                            self.build_test_level();
                        }
                    }
                    KeyCode::Escape if pressed => match self.current_screen {
                        GameScreen::Playing => {
                            self.current_screen = GameScreen::PauseMenu;
                            self.audio.play_preset(SfxPreset::Blip);
                        }
                        GameScreen::PauseMenu => {
                            self.current_screen = GameScreen::Playing;
                            self.audio.play_preset(SfxPreset::Blip);
                        }
                        GameScreen::SettingsMenu | GameScreen::ControlsMenu => {
                            self.current_screen = GameScreen::PauseMenu;
                            self.audio.play_preset(SfxPreset::Blip);
                        }
                    },
                    _ => {}
                }
            }

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let mut dt = (now - self.last_frame_time).as_secs_f32();
                self.last_frame_time = now;
                if dt > 0.1 {
                    dt = 0.1;
                }

                // 60 Hz sabit fizik adımları
                self.accumulator += dt;
                while self.accumulator >= FIXED_DT {
                    self.fixed_update();
                    self.accumulator -= FIXED_DT;
                }

                self.render();
                self.mouse_just_released = false;
                self.action_map.end_frame();

                // FPS Hesaplama
                self.fps_counter += 1;
                if self.fps_timer.elapsed().as_secs_f32() >= 1.0 {
                    self.fps = self.fps_counter;
                    self.fps_counter = 0;
                    self.fps_timer = Instant::now();
                }

                if let Some(win) = self.window.as_ref() {
                    win.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    println!("=== Project Rugeng: 2D Platformer (Mario Style) ===");
    println!(
        "Kontroller: [A]/[D] veya [Sol]/[Sag] = Kos, [Bosluk]/[W] = Zipla, [R] = Yeniden Baslat"
    );

    let event_loop = EventLoop::new().expect("Event loop olusturulamadi");
    let mut app = App::new();
    let _ = event_loop.run_app(&mut app);
}
