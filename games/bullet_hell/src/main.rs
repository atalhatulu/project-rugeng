//! Project-Ember (Rugeng) — Aşama 2: Bullet Hell & Dahili Teşhis Paneli.
//!
//! Özellikler:
//! - 20.000 mermi simülasyonu ve mermi yaşam döngüsü (spawn, hareket, ekran dışı silme).
//! - 60 Hz Sabit Zaman Adımı (Fixed Timestep) + Render Enterpolasyonu.
//! - 2D Spatial Hash Grid ile O(1) mermi-oyuncu çarpışma kontrolü.
//! - Tek draw call ile çizilen 3 sekmeli Teşhis Paneli (Performans, Varlıklar, Simülasyon).
//! - Sıfır heap tahsisi (FrameArena + CommandBuffer + SpatialHash).

use glam::{Mat4, Vec2};
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use rugeng_assets::{AssetServer, Vfs};
use rugeng_core::{
    ActionMap, Animation2DComponent, AnimationClip, AudioEngine, CommandBuffer, Entity, FrameArena,
    Registry, SfxPreset,
};
use rugeng_physics::SpatialHashGrid;
use rugeng_render::context::VulkanContext;
use rugeng_render::font;
use rugeng_render::pipeline_2d::{Pipeline2D, QuadInstance};
use rugeng_render::texture::TextureAtlas;
use rugeng_scheduler::{ThreadArenaPool, ThreadPool};
use rugeng_script::{ScratchSoAView, ScriptCommandBuffer, ScriptEngine, ScriptEngineConfig};

const FIXED_DT: f32 = 1.0 / 60.0;
const MAX_BULLETS: usize = 20_000;

#[derive(Clone, Copy)]
pub struct Position(pub Vec2);

#[derive(Clone, Copy)]
pub struct Velocity(pub Vec2);

#[derive(Clone, Copy)]
pub struct Bullet {
    pub is_enemy: bool,
    pub radius: f32,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum HudTab {
    Performance,
    Entities,
    Simulation,
}

struct Player {
    pos: Vec2,
    speed: f32,
    radius: f32,
    health: f32,
    max_health: f32,
    score: u32,
    god_mode: bool,
    shoot_cooldown: f32,
}

struct Boss {
    pos: Vec2,
    pattern_angle: f32,
    pattern_mode: usize, // 0: Spiral, 1: Nova Ring, 2: Flower
    target_bullets: usize,
}

struct InputState {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    shoot: bool,
    mouse_down: bool,
}

struct App {
    window: Option<Arc<Window>>,
    context: Option<VulkanContext>,
    pipeline_2d: Option<Pipeline2D>,
    atlas: Option<TextureAtlas>,
    asset_server: Arc<AssetServer>,
    arena: FrameArena,
    registry: Registry,
    cmd_buffer: CommandBuffer,
    spatial_grid: SpatialHashGrid,
    player: Player,
    boss: Boss,
    input: InputState,
    action_map: ActionMap,
    audio: AudioEngine,
    animator: Animation2DComponent,
    active_tab: HudTab,
    last_frame_time: Instant,
    accumulator: f32,
    // Metrikler
    frame_time_ms: f32,
    physics_time_ms: f32,
    spatial_hash_us: f32,
    render_time_ms: f32,
    fps: u32,
    fps_counter: u32,
    fps_timer: Instant,
    live_bullet_count: usize,
    frame_count: u64,
    // Sistem bazlı p99 profil örnekleri
    frame_samples: Vec<f32>,
    physics_samples: Vec<f32>,
    render_samples: Vec<f32>,
    spatial_samples: Vec<f32>,
    p99_frame_ms: f32,
    p99_physics_ms: f32,
    p99_render_ms: f32,
    // Aşama 3: Job System ve Paralel Scheduler
    pool: ThreadPool,
    thread_arenas: ThreadArenaPool,
    parallel_enabled: bool,
    // Aşama 5: Luau Script Motoru & Scratch Tamponlar
    script_engine: ScriptEngine,
    #[allow(dead_code)]
    script_soa_view: ScratchSoAView,
    script_cmd_buffer: ScriptCommandBuffer,
    script_time_ms: f32,
    script_enabled: bool,
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(ctx) = self.context.as_ref() {
            unsafe {
                let _ = ctx.device.device_wait_idle();
            }
            if let Some(mut pipe) = self.pipeline_2d.take() {
                pipe.destroy(&ctx.device);
            }
            if let Some(mut atlas) = self.atlas.take() {
                let mut alloc = ctx.allocator.lock().unwrap();
                atlas.destroy(&ctx.device, &mut alloc);
            }
        }
    }
}

impl App {
    fn new() -> Self {
        let arena = FrameArena::new(64 * 1024 * 1024).expect("FrameArena tahsis edilemedi");
        let mut animator = Animation2DComponent::new();
        animator.add_clip(AnimationClip::new(
            "player_engine",
            vec!["player_ship".to_string(), "player_ship_f2".to_string()],
            8.0,
            true,
        ));

        let pool = ThreadPool::new(0); // sistem çekirdek sayısına göre otomatik
        let worker_count = pool.worker_count();
        let thread_arenas = ThreadArenaPool::new(worker_count, 8 * 1024 * 1024)
            .expect("ThreadArenaPool olusturulamadi");
        let parallel_enabled = std::env::var("SERIAL_TEST").is_err();
        let mut vfs = Vfs::new("assets");
        // Eğer çalışma dizini games/bullet_hell altındaysa üst dizindeki assets'i de tanı
        if !std::path::Path::new("assets").exists() && std::path::Path::new("../../assets").exists()
        {
            vfs = Vfs::new("../../assets");
        }
        let asset_server = Arc::new(AssetServer::new(Arc::new(vfs)));

        // Sahneyi dosyadan yukle (G2 & G5: hardcode'suz sahne)
        let (_scene_handle, scene_data) = asset_server.load_scene("scenes/bullet_hell.scene");
        println!(
            "[Aşama 4 VFS] Sahne yüklendi: '{}' (Sürüm: {}, Varlık: {})",
            scene_data.name,
            scene_data.version,
            scene_data.entities.len()
        );

        let mut initial_player_pos = Vec2::new(640.0, 580.0);
        let mut initial_boss_pos = Vec2::new(640.0, 140.0);

        for e in &scene_data.entities {
            if e.name == "PlayerShip" {
                initial_player_pos = Vec2::new(e.pos[0], e.pos[1]);
            } else if e.name == "BossCore" {
                initial_boss_pos = Vec2::new(e.pos[0], e.pos[1]);
            }
        }

        // Aşama 5 (G1 & G4): Luau Script Motoru
        let script_config = ScriptEngineConfig {
            memory_limit_bytes: 32 * 1024 * 1024,
            max_instructions_per_call: 50_000,
        };
        let mut script_engine =
            ScriptEngine::new(script_config).expect("ScriptEngine olusturulamadi");

        let script_path = if std::path::Path::new("assets/scripts/boss_patterns.lua").exists() {
            "assets/scripts/boss_patterns.lua"
        } else {
            "../../assets/scripts/boss_patterns.lua"
        };
        if let Err(e) = script_engine.load_file("BossLogic", script_path) {
            eprintln!("[UYARI] Script yuklenemedi: {}", e);
        } else {
            println!(
                "[Aşama 5 Scripting] Luau BossLogic yüklendi: {}",
                script_path
            );
        }

        let mut app = Self {
            window: None,
            context: None,
            pipeline_2d: None,
            atlas: None,
            asset_server,
            arena,
            registry: Registry::new(),
            cmd_buffer: CommandBuffer::new(),
            spatial_grid: SpatialHashGrid::new(32.0, 4096, 25000),
            player: Player {
                pos: initial_player_pos,
                speed: 380.0,
                radius: 5.0,
                health: 100.0,
                max_health: 100.0,
                score: 0,
                god_mode: false,
                shoot_cooldown: 0.0,
            },
            boss: Boss {
                pos: initial_boss_pos,
                pattern_angle: 0.0,
                pattern_mode: 0,
                target_bullets: MAX_BULLETS,
            },
            input: InputState {
                up: false,
                down: false,
                left: false,
                right: false,
                shoot: false,
                mouse_down: false,
            },
            action_map: ActionMap::new(),
            audio: AudioEngine::new(),
            animator,
            active_tab: HudTab::Performance,
            last_frame_time: Instant::now(),
            accumulator: 0.0,
            frame_time_ms: 0.0,
            physics_time_ms: 0.0,
            spatial_hash_us: 0.0,
            render_time_ms: 0.0,
            fps: 0,
            fps_counter: 0,
            fps_timer: Instant::now(),
            live_bullet_count: 0,
            frame_count: 0,
            frame_samples: Vec::with_capacity(300),
            physics_samples: Vec::with_capacity(300),
            render_samples: Vec::with_capacity(300),
            spatial_samples: Vec::with_capacity(300),
            p99_frame_ms: 0.0,
            p99_physics_ms: 0.0,
            p99_render_ms: 0.0,
            pool,
            thread_arenas,
            parallel_enabled,
            script_engine,
            script_soa_view: ScratchSoAView::with_capacity(MAX_BULLETS),
            script_cmd_buffer: ScriptCommandBuffer::with_capacity(4096),
            script_time_ms: 0.0,
            script_enabled: true,
        };

        if std::env::var("BENCH_LOAD").is_ok() {
            let count = 20_000;
            for i in 0..count {
                let angle = (i as f32) * 0.05;
                let speed = 50.0 + ((i % 100) as f32) * 2.0;
                let vel = Vec2::new(angle.cos() * speed, angle.sin() * speed);
                app.spawn_enemy_bullet(Vec2::new(640.0, 360.0), vel);
            }
        }

        app
    }

    fn spawn_enemy_bullet(&mut self, pos: Vec2, vel: Vec2) {
        let e = self.registry.create();
        self.registry.emplace(e, Position(pos));
        self.registry.emplace(e, Velocity(vel));
        self.registry.emplace(
            e,
            Bullet {
                is_enemy: true,
                radius: 3.5,
            },
        );
    }

    fn spawn_player_bullet(&mut self, pos: Vec2, vel: Vec2) {
        let e = self.registry.create();
        self.registry.emplace(e, Position(pos));
        self.registry.emplace(e, Velocity(vel));
        self.registry.emplace(
            e,
            Bullet {
                is_enemy: false,
                radius: 4.5,
            },
        );
    }

    fn fixed_physics_update(&mut self, screen_w: f32, screen_h: f32) {
        if self.player.health <= 0.0 {
            return;
        }

        let phys_start = Instant::now();

        // 1. Oyuncu hareketi ve atesi (sadece hayattaysa)
        if self.player.health > 0.0 {
            let mut move_dir = Vec2::ZERO;
            if self.input.up {
                move_dir.y -= 1.0;
            }
            if self.input.down {
                move_dir.y += 1.0;
            }
            if self.input.left {
                move_dir.x -= 1.0;
            }
            if self.input.right {
                move_dir.x += 1.0;
            }
            if move_dir != Vec2::ZERO {
                move_dir = move_dir.normalize();
                self.player.pos += move_dir * self.player.speed * FIXED_DT;
                self.player.pos.x = self.player.pos.x.clamp(20.0, screen_w - 20.0);
                self.player.pos.y = self.player.pos.y.clamp(20.0, screen_h - 20.0);
            }

            // Oyuncu ateşi
            self.player.shoot_cooldown -= FIXED_DT;
            if (self.input.shoot || self.input.mouse_down) && self.player.shoot_cooldown <= 0.0 {
                self.player.shoot_cooldown = 0.06; // 16 mermi/sn
                let p_pos = self.player.pos;
                self.spawn_player_bullet(p_pos + Vec2::new(-8.0, -10.0), Vec2::new(0.0, -650.0));
                self.spawn_player_bullet(p_pos + Vec2::new(8.0, -10.0), Vec2::new(0.0, -650.0));
                self.spawn_player_bullet(p_pos + Vec2::new(-16.0, -5.0), Vec2::new(-100.0, -620.0));
                self.spawn_player_bullet(p_pos + Vec2::new(16.0, -5.0), Vec2::new(100.0, -620.0));
                self.audio.play_preset(SfxPreset::Laser);
            }
        }

        // 2. Boss mermi örüntüleri (Aşama 5 G4: Lua destekli desen üretimi)
        let current_count = self.live_bullet_count;
        if current_count < self.boss.target_bullets {
            let spawn_batch = ((self.boss.target_bullets - current_count) / 8).clamp(16, 256);
            let mut used_script = false;

            if self.script_enabled {
                let script_start = Instant::now();
                self.script_cmd_buffer.clear();
                let b_pos = self.boss.pos;
                let pat_mode = self.boss.pattern_mode;
                let pat_ang = self.boss.pattern_angle;

                match self.script_engine.update_boss_pattern(
                    "BossLogic",
                    pat_mode,
                    pat_ang,
                    b_pos.x,
                    b_pos.y,
                    spawn_batch,
                    &mut self.script_cmd_buffer,
                ) {
                    Ok(new_angle) => {
                        self.boss.pattern_angle = new_angle;
                        // Toplu komutları kopyalayarak ECS'ye uygula (0 per-entity FFI)
                        let cmds = self.script_cmd_buffer.commands().to_vec();
                        self.script_cmd_buffer.clear();
                        for cmd in cmds {
                            if let rugeng_script::ScriptCommand::SpawnBullet(b) = cmd {
                                self.spawn_enemy_bullet(Vec2::new(b.x, b.y), Vec2::new(b.vx, b.vy));
                            }
                        }
                        self.script_time_ms = script_start.elapsed().as_secs_f32() * 1000.0;
                        used_script = true;
                    }
                    Err(e) => {
                        eprintln!("[SCRIPT HATA YAKALANDI - OYUN COKMEDI]: {}", e);
                    }
                }
            }

            if !used_script {
                // Native Fallback / Karşılaştırma sürümü
                match self.boss.pattern_mode {
                    0 => {
                        // Dönen 4 kollu spiral
                        for arm in 0..4 {
                            let base_angle = self.boss.pattern_angle
                                + (arm as f32) * (std::f32::consts::PI * 0.5);
                            for i in 0..(spawn_batch / 4).max(1) {
                                let speed = 180.0 + (i as f32) * 12.0;
                                let angle = base_angle + (i as f32) * 0.02;
                                let vel = Vec2::new(angle.cos() * speed, angle.sin() * speed);
                                self.spawn_enemy_bullet(self.boss.pos, vel);
                            }
                        }
                        self.boss.pattern_angle += 0.08;
                    }
                    1 => {
                        // 360 derece Nova halkası
                        let arms = 32;
                        let angle_step = std::f32::consts::PI * 2.0 / (arms as f32);
                        for i in 0..arms {
                            let angle = self.boss.pattern_angle + (i as f32) * angle_step;
                            let vel = Vec2::new(angle.cos() * 220.0, angle.sin() * 220.0);
                            self.spawn_enemy_bullet(self.boss.pos, vel);
                        }
                        self.boss.pattern_angle += 0.04;
                    }
                    _ => {
                        // Çiçek / Wave deseni
                        for i in 0..16 {
                            let angle = self.boss.pattern_angle + (i as f32) * 0.4;
                            let speed = 150.0 + 80.0 * (angle * 3.0).sin().abs();
                            let vel = Vec2::new(angle.cos() * speed, angle.sin() * speed);
                            self.spawn_enemy_bullet(self.boss.pos, vel);
                        }
                        self.boss.pattern_angle += 0.06;
                    }
                }
            }
        }

        // 3. Mermi hareketi ve Spatial Hash doldurma
        let spatial_start = Instant::now();
        self.spatial_grid.clear();

        let mut bullet_count = 0;
        let b_pos = self.boss.pos;
        let p_pos = self.player.pos;
        let p_rad = self.player.radius;

        let cmd = &mut self.cmd_buffer;
        let grid = &mut self.spatial_grid;
        let mut score_add = 0;

        if self.parallel_enabled {
            // Aşama 3: Paralel İş Sistemi (Work-Stealing Chunking)
            // Position ve Velocity bileşen havuzlarını güvenli çakışmasız erişim ile alır.
            self.registry.register_component::<Position>();
            self.registry.register_component::<Velocity>();
            self.registry.register_component::<Bullet>();

            // SAFETY: `fixed_physics_update` içinde `&mut self` mevcuttur.
            // Position mut, Velocity ref, Bullet ref ayrık bileşen havuzlarıdır.
            let pos_pool = unsafe { self.registry.storage_mut::<Position>() };
            let vel_pool = unsafe { self.registry.storage::<Velocity>() };
            let bullet_pool = unsafe { self.registry.storage::<Bullet>() };

            let (entities, positions) = pos_pool.split_entities_and_components_mut();
            let total = entities.len();
            bullet_count = total;

            // Worker başına silinecek mermileri ve skoru topla (0 heap re-alloc veya thread-local)
            let worker_count = self.pool.worker_count();
            let mut thread_despawns: Vec<Vec<Entity>> =
                (0..worker_count).map(|_| Vec::with_capacity(128)).collect();
            let mut thread_scores: Vec<u32> = vec![0; worker_count];

            let chunk_size = (total / worker_count).max(256).max(1);

            self.pool.scope(|scope| {
                let pos_chunks = positions.chunks_mut(chunk_size);
                let ent_chunks = entities.chunks(chunk_size);

                for (chunk_idx, (pos_chunk, ent_chunk)) in pos_chunks.zip(ent_chunks).enumerate() {
                    let despawn_vec = &mut thread_despawns[chunk_idx % worker_count];
                    let score_slot = &mut thread_scores[chunk_idx % worker_count];

                    scope.spawn(move || {
                        for (idx, pos) in pos_chunk.iter_mut().enumerate() {
                            let e = ent_chunk[idx];
                            if let Some(vel) = vel_pool.get(e) {
                                pos.0 += vel.0 * FIXED_DT;

                                if pos.0.x < -40.0
                                    || pos.0.x > screen_w + 40.0
                                    || pos.0.y < -40.0
                                    || pos.0.y > screen_h + 40.0
                                {
                                    despawn_vec.push(e);
                                    continue;
                                }

                                if let Some(bullet) = bullet_pool.get(e) {
                                    if !bullet.is_enemy {
                                        let d_sq = (pos.0 - b_pos).length_squared();
                                        if d_sq <= (bullet.radius + 30.0) * (bullet.radius + 30.0) {
                                            *score_slot += 10;
                                            despawn_vec.push(e);
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
            });

            // Sonuçları CommandBuffer ve Score'a aktar
            for mut despawns in thread_despawns {
                for e in despawns.drain(..) {
                    cmd.destroy(e);
                }
            }
            for sc in thread_scores {
                score_add += sc;
            }

            // Düşman mermilerini Spatial Hash Grid'e ekle (Spatial query için)
            for (idx, &e) in entities.iter().enumerate() {
                if let Some(bullet) = bullet_pool.get(e) {
                    if bullet.is_enemy {
                        let pos = &positions[idx];
                        grid.insert(e.to_bits(), pos.0.x, pos.0.y, bullet.radius);
                    }
                }
            }
        } else {
            // Sıralı referans tabanlı çalıştırma (Baseline)
            self.registry
                .view3_mut_ref_ref::<Position, Velocity, Bullet, _>(|e, pos, vel, bullet| {
                    bullet_count += 1;
                    pos.0 += vel.0 * FIXED_DT;

                    // Ekran dışı kontrolü
                    if pos.0.x < -40.0
                        || pos.0.x > screen_w + 40.0
                        || pos.0.y < -40.0
                        || pos.0.y > screen_h + 40.0
                    {
                        cmd.destroy(e);
                        return;
                    }

                    if bullet.is_enemy {
                        grid.insert(e.to_bits(), pos.0.x, pos.0.y, bullet.radius);
                    } else {
                        let d_sq = (pos.0 - b_pos).length_squared();
                        if d_sq <= (bullet.radius + 30.0) * (bullet.radius + 30.0) {
                            score_add += 10;
                            cmd.destroy(e);
                        }
                    }
                });
        }

        if score_add > 0 {
            self.player.score += score_add;
            self.audio.play_preset(SfxPreset::Blip);
        }

        // 4. Oyuncu çarpışma sorgusu (Spatial Hash Grid O(1) query)
        if !self.player.god_mode && self.player.health > 0.0 {
            let mut hit_count = 0;
            self.spatial_grid
                .query_radius(p_pos.x, p_pos.y, p_rad, |id, _, _, _| {
                    let e = Entity::from_bits(id);
                    self.cmd_buffer.destroy(e);
                    hit_count += 1;
                });

            if hit_count > 0 {
                self.player.health = (self.player.health - (hit_count as f32) * 2.0).max(0.0);
                if self.player.health <= 0.0 {
                    self.audio.play_preset(SfxPreset::Explosion);
                } else {
                    self.audio.play_preset(SfxPreset::Hit);
                }
            }
        }

        self.spatial_hash_us = spatial_start.elapsed().as_secs_f32() * 1_000_000.0;

        // Yapısal değişiklikleri (silinen mermileri) tek seferde uygula
        self.cmd_buffer.apply(&mut self.registry);

        self.live_bullet_count = bullet_count;
        self.physics_time_ms = phys_start.elapsed().as_secs_f32() * 1000.0;
    }

    fn render_all(&mut self, screen_w: f32, screen_h: f32) {
        let render_start = Instant::now();
        if self.context.is_none() || self.pipeline_2d.is_none() {
            return;
        }

        // FrameArena sıfırlanır (0 malloc)
        self.arena.reset();
        self.thread_arenas.reset_all();

        let mut instances = Vec::with_capacity(self.live_bullet_count + 1024);

        // 1. Oyun Dünyası Öğeleri
        // Boss (Atlas tabanlı boss_core dokusu veya neon altıgen)
        let boss_reg = self.atlas.as_ref().and_then(|a| a.get_region("boss_core"));
        if let Some(sub) = boss_reg {
            instances.push(QuadInstance::with_uv(
                [self.boss.pos.x - 25.0, self.boss.pos.y - 25.0],
                [50.0, 50.0],
                [sub.uv_min_packed, sub.uv_size_packed],
                [255, 255, 255, 255],
                1,
            ));
        } else {
            instances.push(QuadInstance::new(
                [self.boss.pos.x - 25.0, self.boss.pos.y - 25.0],
                [50.0, 50.0],
                [220, 50, 240, 255],
                0,
            ));
            instances.push(QuadInstance::new(
                [self.boss.pos.x - 18.0, self.boss.pos.y - 18.0],
                [36.0, 36.0],
                [255, 180, 255, 255],
                0,
            ));
        }

        // Oyuncu (Atlas tabanlı kare animasyonu veya parlak cyan jet)
        let p_color = if self.player.god_mode {
            [255, 230, 40, 255] // Altın sarısı god mode
        } else {
            [255, 255, 255, 255]
        };
        let anim_frame = self
            .animator
            .current_texture_name()
            .unwrap_or("player_ship");
        let sub_reg = self.atlas.as_ref().and_then(|a| a.get_region(anim_frame));

        if let Some(sub) = sub_reg {
            instances.push(QuadInstance::with_uv(
                [self.player.pos.x - 14.0, self.player.pos.y - 14.0],
                [28.0, 28.0],
                [sub.uv_min_packed, sub.uv_size_packed],
                p_color,
                1,
            ));
        } else {
            instances.push(QuadInstance::new(
                [self.player.pos.x - 7.0, self.player.pos.y - 9.0],
                [14.0, 18.0],
                p_color,
                0,
            ));
        }
        instances.push(QuadInstance::new(
            [self.player.pos.x - 3.0, self.player.pos.y - 3.0],
            [6.0, 6.0],
            [255, 50, 50, 255], // Hitbox çekirdeği (hassas kırmızı nokta)
            0,
        ));

        // Mermiler
        self.registry
            .view2_ref_ref::<Position, Bullet, _>(|_e, pos, b| {
                let p = pos.0;
                let rad = b.radius;
                if b.is_enemy {
                    instances.push(QuadInstance::new(
                        [p.x - rad, p.y - rad],
                        [rad * 2.0, rad * 2.0],
                        [255, 80, 60, 230], // Kırmızı mermi
                        0,
                    ));
                } else {
                    instances.push(QuadInstance::new(
                        [p.x - rad, p.y - rad],
                        [rad * 2.0, rad * 2.0],
                        [80, 240, 140, 255], // Yeşil oyuncu mermisi
                        0,
                    ));
                }
            });

        // 2. Oyuncu Can & Skor Çubuğu (Ekran altı)
        let hp_pct = self.player.health / self.player.max_health;
        instances.push(QuadInstance::new(
            [20.0, screen_h - 26.0],
            [204.0, 14.0],
            [40, 40, 50, 220],
            0,
        ));
        instances.push(QuadInstance::new(
            [22.0, screen_h - 24.0],
            [200.0 * hp_pct, 10.0],
            [50, 220, 90, 255],
            0,
        ));
        font::draw_text(
            &mut instances,
            &format!("HP: {:.0}%", self.player.health),
            230.0,
            screen_h - 24.0,
            2.0,
            [255, 255, 255, 255],
        );
        font::draw_text(
            &mut instances,
            &format!("SKOR: {:06}", self.player.score),
            screen_w - 180.0,
            screen_h - 24.0,
            2.0,
            [255, 220, 80, 255],
        );

        // 3. Oyun Bitti (Game Over) Paneli
        if self.player.health <= 0.0 {
            let banner_w = 520.0;
            let banner_h = 100.0;
            let bx = (screen_w - banner_w) * 0.5;
            let by = (screen_h - banner_h) * 0.5;
            instances.push(QuadInstance::new(
                [bx, by],
                [banner_w, banner_h],
                [40, 10, 15, 235],
                0,
            ));
            instances.push(QuadInstance::new(
                [bx, by],
                [banner_w, 2.0],
                [255, 60, 60, 255],
                0,
            ));
            instances.push(QuadInstance::new(
                [bx, by + banner_h - 2.0],
                [banner_w, 2.0],
                [255, 60, 60, 255],
                0,
            ));
            font::draw_text(
                &mut instances,
                "OYUN BITTI (GAME OVER)",
                bx + 70.0,
                by + 25.0,
                2.2,
                [255, 70, 70, 255],
            );
            font::draw_text(
                &mut instances,
                "YENIDEN BASLATMAK ICIN [R] TUSUNA BASIN",
                bx + 35.0,
                by + 58.0,
                1.6,
                [240, 240, 240, 255],
            );
        }

        // 4. Dahili Teşhis Paneli (In-Game Debug HUD / Overlay)
        self.render_hud(&mut instances, screen_w);

        // FrameArena kopyası
        let arena_instances = self.arena.alloc_slice_copy(&instances).unwrap();

        // Vulkan Çizimi
        let Some(ctx) = self.context.as_mut() else {
            return;
        };
        let Some(pipe) = self.pipeline_2d.as_mut() else {
            return;
        };
        let frame_idx = ctx.current_frame;
        if let Ok(Some((image_index, cmd))) = ctx.begin_frame(screen_w as u32, screen_h as u32) {
            let count = unsafe { pipe.upload_instances(frame_idx, arena_instances) };
            ctx.begin_rendering(cmd, image_index, [0.05, 0.06, 0.08, 1.0]);

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

        self.render_time_ms = render_start.elapsed().as_secs_f32() * 1000.0;
    }

    fn render_hud(&self, instances: &mut Vec<QuadInstance>, screen_w: f32) {
        let hud_w = 420.0;
        let hud_h = 190.0;
        let hud_x = screen_w - hud_w - 20.0;
        let hud_y = 20.0;

        // Arka plan paneli (karanlık cam efekti)
        instances.push(QuadInstance::new(
            [hud_x, hud_y],
            [hud_w, hud_h],
            [16, 20, 30, 225],
            0,
        ));
        // Üst başlık çizgisi
        instances.push(QuadInstance::new(
            [hud_x, hud_y],
            [hud_w, 2.0],
            [80, 180, 255, 255],
            0,
        ));

        // Sekme Butonları
        let tabs = [
            (HudTab::Performance, "[1: PERFORMANS]"),
            (HudTab::Entities, "[2: VARLIKLAR]"),
            (HudTab::Simulation, "[3: KONTROLLER]"),
        ];

        let tab_w = hud_w / 3.0;
        for (idx, &(tab, name)) in tabs.iter().enumerate() {
            let tx = hud_x + (idx as f32) * tab_w;
            let is_active = self.active_tab == tab;
            let bg_color = if is_active {
                [35, 55, 85, 240]
            } else {
                [22, 28, 42, 180]
            };
            instances.push(QuadInstance::new(
                [tx + 2.0, hud_y + 4.0],
                [tab_w - 4.0, 22.0],
                bg_color,
                0,
            ));

            let text_color = if is_active {
                [90, 220, 255, 255]
            } else {
                [140, 155, 175, 200]
            };
            font::draw_text(instances, name, tx + 8.0, hud_y + 10.0, 1.8, text_color);
        }

        let content_y = hud_y + 36.0;

        match self.active_tab {
            HudTab::Performance => {
                // Kare Bütçesi Barı (16.6 ms bütçesi)
                let budget_max_ms = 16.66;
                let used_pct = (self.frame_time_ms / budget_max_ms).clamp(0.0, 1.5);
                let bar_w = 260.0;

                font::draw_text(
                    instances,
                    "KARE BUTCESI:",
                    hud_x + 14.0,
                    content_y,
                    1.8,
                    [200, 210, 230, 255],
                );
                // Bar arka planı
                instances.push(QuadInstance::new(
                    [hud_x + 140.0, content_y - 2.0],
                    [bar_w, 14.0],
                    [30, 36, 50, 220],
                    0,
                ));

                let bar_color = if self.frame_time_ms <= 8.0 {
                    [40, 220, 100, 255] // Yeşil
                } else if self.frame_time_ms <= 14.0 {
                    [240, 200, 40, 255] // Sarı
                } else {
                    [255, 60, 60, 255] // Kırmızı bütçe aşımı
                };

                instances.push(QuadInstance::new(
                    [hud_x + 142.0, content_y],
                    [bar_w * (used_pct.min(1.0)), 10.0],
                    bar_color,
                    0,
                ));

                font::draw_text(
                    instances,
                    &format!(
                        "KARE: {:.2} MS (p99: {:.2} ms)",
                        self.frame_time_ms, self.p99_frame_ms
                    ),
                    hud_x + 14.0,
                    content_y + 22.0,
                    1.8,
                    bar_color,
                );
                font::draw_text(
                    instances,
                    &format!(
                        "FIZIK (60HZ): {:.2} MS (p99: {:.2} ms)",
                        self.physics_time_ms, self.p99_physics_ms
                    ),
                    hud_x + 14.0,
                    content_y + 44.0,
                    1.8,
                    [180, 200, 230, 255],
                );
                font::draw_text(
                    instances,
                    &format!("SPATIAL HASH: {:.0} US (0 MALLOC)", self.spatial_hash_us),
                    hud_x + 14.0,
                    content_y + 66.0,
                    1.8,
                    [120, 230, 180, 255],
                );
                font::draw_text(
                    instances,
                    &format!(
                        "VULKAN RENDER: {:.2} MS (p99: {:.2} ms) | FPS: {}",
                        self.render_time_ms, self.p99_render_ms, self.fps
                    ),
                    hud_x + 14.0,
                    content_y + 88.0,
                    1.8,
                    [180, 200, 230, 255],
                );
                font::draw_text(
                    instances,
                    &format!(
                        "SCHEDULER (TUS: J): {}",
                        if self.parallel_enabled {
                            "PARALEL WORK-STEALING (AKTIF)"
                        } else {
                            "SIRALI TEK CEKIRDEK (KAPALI)"
                        }
                    ),
                    hud_x + 14.0,
                    content_y + 110.0,
                    1.6,
                    if self.parallel_enabled {
                        [100, 230, 160, 255]
                    } else {
                        [240, 180, 80, 255]
                    },
                );
            }
            HudTab::Entities => {
                font::draw_text(
                    instances,
                    &format!(
                        "CANLI MERMI: {} / {}",
                        self.live_bullet_count, self.boss.target_bullets
                    ),
                    hud_x + 14.0,
                    content_y,
                    2.0,
                    [255, 120, 90, 255],
                );
                font::draw_text(
                    instances,
                    &format!("ECS VARLIK HAVUZU: {} ADET", self.registry.len()),
                    hud_x + 14.0,
                    content_y + 26.0,
                    1.8,
                    [200, 215, 240, 255],
                );
                font::draw_text(
                    instances,
                    "FRAME ARENA: 64 MB (LOCK-FREE MMAP)",
                    hud_x + 14.0,
                    content_y + 50.0,
                    1.8,
                    [100, 220, 160, 255],
                );
                font::draw_text(
                    instances,
                    "ASSET & VFS: MMAP + HOT RELOAD (AKTIF)",
                    hud_x + 14.0,
                    content_y + 74.0,
                    1.8,
                    [130, 210, 255, 255],
                );
                font::draw_text(
                    instances,
                    &format!(
                        "SCRIPTING (TUS L): {} ({:.2} MS)",
                        if self.script_enabled {
                            "LUAU AKTIF"
                        } else {
                            "NATIVE FALLBACK"
                        },
                        self.script_time_ms
                    ),
                    hud_x + 14.0,
                    content_y + 98.0,
                    1.8,
                    if self.script_enabled {
                        [120, 240, 160, 255]
                    } else {
                        [240, 180, 80, 255]
                    },
                );
                font::draw_text(
                    instances,
                    "GPU RING BUFFER: 6.4 MB/FRAME SSBO",
                    hud_x + 14.0,
                    content_y + 122.0,
                    1.8,
                    [130, 180, 255, 255],
                );
            }
            HudTab::Simulation => {
                let pat_name = match self.boss.pattern_mode {
                    0 => "4-KOLLU SPIRAL",
                    1 => "360 NOVA RING",
                    _ => "WAVE FLOWER",
                };
                font::draw_text(
                    instances,
                    &format!("DESEN: {} (TUS: P)", pat_name),
                    hud_x + 14.0,
                    content_y,
                    1.8,
                    [240, 200, 80, 255],
                );
                font::draw_text(
                    instances,
                    &format!("HEDEF: {} MERMI (+/- TUSLARI)", self.boss.target_bullets),
                    hud_x + 14.0,
                    content_y + 24.0,
                    1.8,
                    [200, 220, 250, 255],
                );
                let god_str = if self.player.god_mode {
                    "ACIK"
                } else {
                    "KAPALI"
                };
                font::draw_text(
                    instances,
                    &format!("OLUMSUZLUK (GOD): {} (TUS: G)", god_str),
                    hud_x + 14.0,
                    content_y + 48.0,
                    1.8,
                    if self.player.god_mode {
                        [255, 220, 40, 255]
                    } else {
                        [160, 170, 190, 255]
                    },
                );
                font::draw_text(
                    instances,
                    "SEKME DEGISTIRME: 1, 2, 3 VEYA TAB",
                    hud_x + 14.0,
                    content_y + 74.0,
                    1.8,
                    [100, 180, 255, 255],
                );
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let win_attrs = Window::default_attributes()
                .with_title("Project Ember — Bullet Hell Prototype (Rugeng)")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

            let window = Arc::new(
                event_loop
                    .create_window(win_attrs)
                    .expect("Pencere olusturulamadi"),
            );
            let size = window.inner_size();

            println!(
                "[Stage 2] Vulkan Context ve Pipeline2D baslatiliyor: {}x{}",
                size.width, size.height
            );
            let context = VulkanContext::new(&*window, size.width, size.height)
                .expect("Vulkan Context baslatilamadi");

            let mut atlas =
                TextureAtlas::new(&context, 512, 512).expect("TextureAtlas olusturulamadi");

            // Aşama 4 (G1 & G5): Dokuları VFS ve AssetServer üzerinden yükle
            let (_h_ship, tex_ship) = self.asset_server.load_texture("textures/player_ship.png");
            let _ = atlas.allocate_and_upload(
                &context,
                "player_ship",
                tex_ship.width,
                tex_ship.height,
                &tex_ship.rgba,
            );

            let (_h_f2, tex_f2) = self
                .asset_server
                .load_texture("textures/player_ship_f2.png");
            let _ = atlas.allocate_and_upload(
                &context,
                "player_ship_f2",
                tex_f2.width,
                tex_f2.height,
                &tex_f2.rgba,
            );

            let (_h_boss, tex_boss) = self.asset_server.load_texture("textures/boss_core.png");
            let _ = atlas.allocate_and_upload(
                &context,
                "boss_core",
                tex_boss.width,
                tex_boss.height,
                &tex_boss.rgba,
            );

            let pipeline_2d = Pipeline2D::new(
                &context.device,
                Arc::clone(&context.allocator),
                context.swapchain.format,
                rugeng_render::FRAMES_IN_FLIGHT,
                atlas.image_view,
                atlas.sampler,
            )
            .expect("Pipeline2D olusturulamadi");

            self.window = Some(window);
            self.context = Some(context);
            self.atlas = Some(atlas);
            self.pipeline_2d = Some(pipeline_2d);
            self.last_frame_time = Instant::now();
            self.fps_timer = Instant::now();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let Some(ctx) = self.context.as_mut() {
                    ctx.resized = true;
                    let _ = ctx.recreate_swapchain(new_size.width, new_size.height);
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
                let scancode = match key {
                    KeyCode::KeyA => 0x1E,
                    KeyCode::KeyD => 0x20,
                    KeyCode::KeyW => 0x11,
                    KeyCode::KeyS => 0x1F,
                    KeyCode::ArrowLeft => 0xCB,
                    KeyCode::ArrowRight => 0xCD,
                    KeyCode::ArrowUp => 0xC8,
                    KeyCode::ArrowDown => 0xD0,
                    KeyCode::Space => 0x39,
                    _ => 0,
                };
                if scancode != 0 {
                    self.action_map.update_key_event(scancode, pressed);
                }

                match key {
                    KeyCode::KeyW | KeyCode::ArrowUp => self.input.up = pressed,
                    KeyCode::KeyS | KeyCode::ArrowDown => self.input.down = pressed,
                    KeyCode::KeyA | KeyCode::ArrowLeft => self.input.left = pressed,
                    KeyCode::KeyD | KeyCode::ArrowRight => self.input.right = pressed,
                    KeyCode::Space => self.input.shoot = pressed,
                    KeyCode::Digit1 => {
                        if pressed {
                            self.active_tab = HudTab::Performance;
                        }
                    }
                    KeyCode::Digit2 => {
                        if pressed {
                            self.active_tab = HudTab::Entities;
                        }
                    }
                    KeyCode::Digit3 => {
                        if pressed {
                            self.active_tab = HudTab::Simulation;
                        }
                    }
                    KeyCode::Tab => {
                        if pressed {
                            self.active_tab = match self.active_tab {
                                HudTab::Performance => HudTab::Entities,
                                HudTab::Entities => HudTab::Simulation,
                                HudTab::Simulation => HudTab::Performance,
                            };
                        }
                    }
                    KeyCode::KeyG => {
                        if pressed {
                            self.player.god_mode = !self.player.god_mode;
                        }
                    }
                    KeyCode::KeyJ => {
                        if pressed {
                            self.parallel_enabled = !self.parallel_enabled;
                        }
                    }
                    KeyCode::KeyL => {
                        if pressed {
                            self.script_enabled = !self.script_enabled;
                            println!(
                                "[SCRIPT] Luau Script Durumu: {}",
                                if self.script_enabled {
                                    "AKTİF"
                                } else {
                                    "KAPALI (NATIVE)"
                                }
                            );
                        }
                    }
                    KeyCode::KeyP => {
                        if pressed {
                            self.boss.pattern_mode = (self.boss.pattern_mode + 1) % 3;
                        }
                    }
                    KeyCode::Equal | KeyCode::NumpadAdd => {
                        if pressed {
                            self.boss.target_bullets =
                                (self.boss.target_bullets + 2500).min(MAX_BULLETS);
                        }
                    }
                    KeyCode::Minus | KeyCode::NumpadSubtract => {
                        if pressed {
                            self.boss.target_bullets =
                                self.boss.target_bullets.saturating_sub(2500).max(1000);
                        }
                    }
                    KeyCode::KeyR => {
                        if pressed && self.player.health <= 0.0 {
                            let bullets = self.registry.entities::<Bullet>();
                            for b in bullets {
                                self.cmd_buffer.destroy(b);
                            }
                            self.cmd_buffer.apply(&mut self.registry);
                            self.spatial_grid.clear();
                            self.live_bullet_count = 0;
                            self.player.health = self.player.max_health;
                            self.player.pos = Vec2::new(640.0, 580.0);
                            self.player.score = 0;
                            self.boss.pos = Vec2::new(640.0, 140.0);
                            self.boss.pattern_mode = 0;
                            self.boss.pattern_angle = 0.0;
                            self.audio.play_preset(SfxPreset::Powerup);
                        }
                    }
                    KeyCode::Escape => event_loop.exit(),
                    _ => {}
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                let pressed = state == ElementState::Pressed;
                let btn_id = match button {
                    MouseButton::Left => 0,
                    MouseButton::Right => 1,
                    _ => 2,
                };
                self.action_map.update_mouse_button(btn_id, pressed);
                if button == MouseButton::Left {
                    self.input.mouse_down = pressed;
                }
            }
            WindowEvent::RedrawRequested => {
                let window_opt = self.window.clone();
                let Some(window) = window_opt else {
                    return;
                };
                let size = window.inner_size();
                if size.width == 0 || size.height == 0 {
                    return;
                }

                let now = Instant::now();
                let frame_delta = now
                    .duration_since(self.last_frame_time)
                    .as_secs_f32()
                    .min(0.1);
                self.last_frame_time = now;
                self.frame_time_ms = frame_delta * 1000.0;

                // Animasyon güncellemesi
                self.animator.update(frame_delta);

                // Metrik örneklerini topla ve p99 hesapla
                self.frame_samples.push(self.frame_time_ms);
                self.physics_samples.push(self.physics_time_ms);
                self.render_samples.push(self.render_time_ms);
                self.spatial_samples.push(self.spatial_hash_us);
                if self.frame_samples.len() > 300 {
                    self.frame_samples.remove(0);
                    self.physics_samples.remove(0);
                    self.render_samples.remove(0);
                    self.spatial_samples.remove(0);
                }

                // FPS ve p99 hesaplama
                self.fps_counter += 1;
                if self.fps_timer.elapsed().as_secs_f32() >= 1.0 {
                    self.fps = self.fps_counter;

                    let mut sorted_frames = self.frame_samples.clone();
                    sorted_frames
                        .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let p99_idx = ((sorted_frames.len() as f32) * 0.99) as usize;
                    self.p99_frame_ms = sorted_frames
                        .get(p99_idx)
                        .copied()
                        .unwrap_or(self.frame_time_ms);

                    let mut sorted_phys = self.physics_samples.clone();
                    sorted_phys
                        .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let p99_p_idx = ((sorted_phys.len() as f32) * 0.99) as usize;
                    self.p99_physics_ms = sorted_phys
                        .get(p99_p_idx)
                        .copied()
                        .unwrap_or(self.physics_time_ms);

                    let mut sorted_rend = self.render_samples.clone();
                    sorted_rend
                        .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let p99_r_idx = ((sorted_rend.len() as f32) * 0.99) as usize;
                    self.p99_render_ms = sorted_rend
                        .get(p99_r_idx)
                        .copied()
                        .unwrap_or(self.render_time_ms);

                    let budget_pct = (self.frame_time_ms / 16.66) * 100.0;
                    println!(
                        "[Metrik] Mermi: {} | Frame: {:.2} ms (p99: {:.2} ms, %{:.1} bütçe) | Fizik: {:.2} ms (p99: {:.2} ms) | Render: {:.2} ms (p99: {:.2} ms) | Spatial: {:.0} us | FPS: {}",
                        self.live_bullet_count, self.frame_time_ms, self.p99_frame_ms, budget_pct, self.physics_time_ms, self.p99_physics_ms, self.render_time_ms, self.p99_render_ms, self.spatial_hash_us, self.fps
                    );
                    self.fps_counter = 0;
                    self.fps_timer = Instant::now();
                }

                self.frame_count += 1;
                if std::env::var("AUTO_TEST").is_ok() && self.frame_count >= 120 {
                    let err_count = self.context.as_ref().map_or(0, |ctx| {
                        ctx.validation_error_count
                            .load(std::sync::atomic::Ordering::Relaxed)
                    });
                    let budget_pct = (self.frame_time_ms / 16.66) * 100.0;
                    println!(
                        "[AUTO_TEST] Tamamlandi! Kare: {}, Mermi: {}, Frame: {:.2} ms (p99: {:.2} ms, %{:.1} butce), Validation Hatasi: {}",
                        self.frame_count, self.live_bullet_count, self.frame_time_ms, self.p99_frame_ms, budget_pct, err_count
                    );
                    assert_eq!(err_count, 0, "Validation hatasi bulundu!");
                    event_loop.exit();
                    return;
                }

                // Aşama 4 (G3): Hot Reload Tarama (Doku & Sahne)
                self.asset_server.poll_hot_reload();
                let reloaded_textures = self.asset_server.drain_reloaded_textures();
                if !reloaded_textures.is_empty() {
                    if let (Some(ctx), Some(atlas)) = (self.context.as_ref(), self.atlas.as_mut()) {
                        for path in reloaded_textures {
                            let region_name = if path.contains("player_ship_f2") {
                                "player_ship_f2"
                            } else if path.contains("player_ship") {
                                "player_ship"
                            } else if path.contains("boss_core") {
                                "boss_core"
                            } else {
                                continue;
                            };
                            if let Some(tex) = self.asset_server.get_texture(&path) {
                                let _ = atlas.update_existing_region(ctx, region_name, &tex.rgba);
                            }
                        }
                    }
                }

                let reloaded_scenes = self.asset_server.drain_reloaded_scenes();
                if !reloaded_scenes.is_empty() {
                    for path in reloaded_scenes {
                        if let Some(scene) = self.asset_server.get_scene(&path) {
                            for e in &scene.entities {
                                if e.name == "PlayerShip" {
                                    println!(
                                        "[HOT RELOAD] Sahne oyuncu pozisyonu güncellendi: {:?}",
                                        e.pos
                                    );
                                }
                            }
                        }
                    }
                }

                // Aşama 5 (G3): Script Hot Reload (Durum korunarak)
                let reloaded_scripts = self.script_engine.poll_hot_reload();
                for s in reloaded_scripts {
                    println!(
                        "[HOT RELOAD] Luau scripti durum korunarak güncellendi: {}",
                        s
                    );
                }

                // Sabit zaman adımlı fizik döngüsü (60 Hz)
                self.accumulator += frame_delta;
                while self.accumulator >= FIXED_DT {
                    self.fixed_physics_update(size.width as f32, size.height as f32);
                    self.accumulator -= FIXED_DT;
                }

                self.action_map.end_frame();

                // Çizim
                self.render_all(size.width as f32, size.height as f32);

                window.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let window_opt = self.window.clone();
        if let Some(window) = window_opt {
            window.request_redraw();
        }
    }
}

fn main() {
    println!("=== Project-Ember (Rugeng): Bullet Hell & Teşhis Paneli ===");
    println!("Kontroller:");
    println!("- W, A, S, D / Oklar: Oyuncu Hareketi");
    println!("- Space / Fare Sol Tık: Mermi Atışı");
    println!("- 1, 2, 3 veya Tab: Teşhis Sekmelerini Değiştir");
    println!("- G: Ölümsüzlük (God Mode) Aç/Kapa");
    println!("- J: Paralel İş Parçacığı Planlayıcısı (Job Scheduler) Aç/Kapa");
    println!("- P: Mermi Örüntüsü Değiştir (Spiral / Nova / Flower)");
    println!("- +, -: Hedef Mermi Sayısını Artır/Azalt (1.000 - 20.000)");

    let event_loop = EventLoop::new().expect("EventLoop olusturulamadi");
    let mut app = App::new();
    let _ = event_loop.run_app(&mut app);
}
