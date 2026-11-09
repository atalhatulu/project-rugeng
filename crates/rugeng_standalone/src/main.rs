//! Project Ember — Bağımsız Dağıtım Çalıştırıcısı (Standalone Runtime / Export Runner).
//!
//! Editör yükü olmaksızın, doğrudan `.scene` sahne dosyasını yükler,
//! Vulkan 1.3 tek draw call mimarisi, Doku Atlası, Betik Motoru ve Ses Sentezleyicisi ile
//! 240+ FPS bağımsız oyun deneyimi sunar.

use glam::{Mat4, Vec2};
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use rugeng_core::{
    ActionMap, Animation2DComponent, AudioEngine, Children, Collider2D, CommandBuffer, Entity,
    FrameArena, Parent, Registry, SceneData, ScriptComponent, SpriteColor, SpriteTexture,
    Text2DComponent, Tilemap2DComponent, Transform2D, Velocity2D,
};
use rugeng_render::context::VulkanContext;
use rugeng_render::font;
use rugeng_render::pipeline_2d::{Pipeline2D, QuadInstance};
use rugeng_render::texture::TextureAtlas;

#[derive(Clone, Copy, Debug)]
pub struct Projectile {
    pub is_player: bool,
    pub lifetime: f32,
}

struct App {
    window: Option<Arc<Window>>,
    context: Option<VulkanContext>,
    pipeline_2d: Option<Pipeline2D>,
    atlas: Option<TextureAtlas>,
    arena: FrameArena,
    registry: Registry,
    cmd_buffer: CommandBuffer,
    audio: AudioEngine,
    action_map: ActionMap,
    player_entity: Option<Entity>,
    last_frame_time: Instant,
    frame_time_ms: f32,
    fps: u32,
    fps_counter: u32,
    fps_timer: Instant,
    frame_count: u64,
    score: u32,
    scene_name: String,
    auto_exit_frames: Option<u64>,
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
    fn new(auto_exit_frames: Option<u64>, custom_scene_path: Option<String>) -> Self {
        let arena = FrameArena::new(64 * 1024 * 1024).expect("FrameArena tahsis edilemedi");
        let mut registry = Registry::new();
        registry.register_component::<Transform2D>();
        registry.register_component::<SpriteColor>();
        registry.register_component::<SpriteTexture>();
        registry.register_component::<Collider2D>();
        registry.register_component::<Velocity2D>();
        registry.register_component::<ScriptComponent>();
        registry.register_component::<Projectile>();
        registry.register_component::<Parent>();
        registry.register_component::<Children>();
        registry.register_component::<Text2DComponent>();
        registry.register_component::<Animation2DComponent>();
        registry.register_component::<Tilemap2DComponent>();

        let mut app = Self {
            window: None,
            context: None,
            pipeline_2d: None,
            atlas: None,
            arena,
            registry,
            cmd_buffer: CommandBuffer::new(),
            audio: AudioEngine::new(),
            action_map: ActionMap::new(),
            player_entity: None,
            last_frame_time: Instant::now(),
            frame_time_ms: 0.0,
            fps: 0,
            fps_counter: 0,
            fps_timer: Instant::now(),
            frame_count: 0,
            score: 0,
            scene_name: custom_scene_path
                .clone()
                .unwrap_or_else(|| "scenes/main.scene".to_string()),
            auto_exit_frames,
        };

        app.load_or_create_scene(custom_scene_path.as_deref());
        app
    }

    fn load_or_create_scene(&mut self, custom_path: Option<&str>) {
        let path = custom_path.unwrap_or("scenes/main.scene");

        // 1. Eğer .pak paketi ise PakReader ile sahneyi oku
        let loaded_scene = if path.ends_with(".pak") {
            if let Ok(pak) = rugeng_assets::PakReader::from_file(path) {
                if let Some(scene_bytes) = pak.get_file("scenes/main.scene").or_else(|| {
                    pak.list_files()
                        .iter()
                        .find(|f| f.ends_with(".scene"))
                        .and_then(|f| pak.get_file(f))
                }) {
                    println!(
                        "[Standalone] Pak arşivinden sahne bulundu ({} bayt).",
                        scene_bytes.len()
                    );
                    rugeng_assets::VersionedScene::from_bytes(scene_bytes)
                        .map(|v| v.to_core_scene())
                        .ok()
                } else {
                    None
                }
            } else {
                None
            }
        } else if let Ok(bytes) = std::fs::read(path) {
            rugeng_assets::VersionedScene::from_bytes(&bytes)
                .map(|v| v.to_core_scene())
                .ok()
                .or_else(|| SceneData::load_from_file(path).ok())
        } else {
            None
        };

        if let Some(scene) = loaded_scene {
            println!(
                "[Standalone] '{}' yuklendi ({} varlik).",
                path,
                scene.entities.len()
            );
            self.scene_name = scene.name;
            let mut id_map = std::collections::HashMap::new();

            for item in &scene.entities {
                let e = self.registry.create();
                id_map.insert(item.id, e);
                if self.player_entity.is_none() && item.texture.as_deref() == Some("player_ship") {
                    self.player_entity = Some(e);
                }
                self.registry.emplace(
                    e,
                    Transform2D {
                        pos: Vec2::new(item.pos[0], item.pos[1]),
                        size: Vec2::new(item.size[0], item.size[1]),
                        rotation: item.rotation,
                    },
                );
                self.registry.emplace(e, SpriteColor { color: item.color });
                if let Some(tex) = &item.texture {
                    self.registry
                        .emplace(e, SpriteTexture { name: tex.clone() });
                }
                if item.has_collider {
                    self.registry.emplace(
                        e,
                        Collider2D {
                            is_dynamic: item.is_dynamic,
                            is_sensor: item.is_sensor,
                        },
                    );
                }
                if let Some(v) = item.velocity {
                    self.registry.emplace(
                        e,
                        Velocity2D {
                            vel: Vec2::new(v[0], v[1]),
                        },
                    );
                }
                if let Some(scr) = &item.script {
                    let comp = match scr.as_str() {
                        "RotatorScript" => ScriptComponent::new_rotator(1.5),
                        "PatrolScript" => ScriptComponent::new_patrol(0, -250.0, 250.0, 90.0),
                        "PulseScript" => ScriptComponent::new_pulse(20.0, 40.0, 3.0),
                        _ => ScriptComponent::new_rotator(1.0),
                    };
                    self.registry.emplace(e, comp);
                }
                if let Some(txt) = &item.text {
                    self.registry.emplace(e, txt.clone());
                }
                if let Some(anim) = &item.animation {
                    self.registry.emplace(e, anim.clone());
                }
                if let Some(tm) = &item.tilemap {
                    self.registry.emplace(e, tm.clone());
                }
            }

            // Hiyerarşiyi (Parent / Children) bağla
            for item in &scene.entities {
                if let Some(pid) = item.parent_id {
                    if let (Some(&child_e), Some(&parent_e)) =
                        (id_map.get(&item.id), id_map.get(&pid))
                    {
                        self.registry.emplace(child_e, Parent { entity: parent_e });
                        if let Some(children) = self.registry.get_mut::<Children>(parent_e) {
                            children.add(child_e);
                        } else {
                            let mut ch = Children::new();
                            ch.add(child_e);
                            self.registry.emplace(parent_e, ch);
                        }
                    }
                }
            }
        } else {
            println!(
                "[Standalone] 'scenes/main.scene' bulunamadi, varsayilan demo sahnesi uretiliyor."
            );
            self.scene_name = "DefaultStandaloneScene".to_string();

            // Oyuncu Uzay Gemisi
            let p = self.registry.create();
            self.registry.emplace(
                p,
                Transform2D {
                    pos: Vec2::new(0.0, -180.0),
                    size: Vec2::new(48.0, 48.0),
                    rotation: 0.0,
                },
            );
            self.registry.emplace(
                p,
                SpriteColor {
                    color: [255, 255, 255, 255],
                },
            );
            self.registry.emplace(
                p,
                SpriteTexture {
                    name: "player_ship".to_string(),
                },
            );
            self.registry.emplace(
                p,
                Collider2D {
                    is_dynamic: true,
                    is_sensor: false,
                },
            );
            self.player_entity = Some(p);

            // Düşman Yaratıklar (Patrol Betikli)
            for i in 0..4 {
                let e = self.registry.create();
                let x = -200.0 + (i as f32) * 130.0;
                let y = 100.0 + ((i % 2) as f32) * 60.0;
                self.registry.emplace(
                    e,
                    Transform2D {
                        pos: Vec2::new(x, y),
                        size: Vec2::new(36.0, 36.0),
                        rotation: 0.0,
                    },
                );
                self.registry.emplace(
                    e,
                    SpriteColor {
                        color: [255, 255, 255, 255],
                    },
                );
                self.registry.emplace(
                    e,
                    SpriteTexture {
                        name: "enemy_bug".to_string(),
                    },
                );
                self.registry.emplace(
                    e,
                    Collider2D {
                        is_dynamic: true,
                        is_sensor: false,
                    },
                );
                self.registry
                    .emplace(e, ScriptComponent::new_patrol(0, x - 60.0, x + 60.0, 75.0));
            }

            // Dönen Enerji Kristalleri (Rotator & Pulse Betikli)
            let c1 = self.registry.create();
            self.registry.emplace(
                c1,
                Transform2D {
                    pos: Vec2::new(-160.0, 0.0),
                    size: Vec2::new(28.0, 28.0),
                    rotation: 0.0,
                },
            );
            self.registry.emplace(
                c1,
                SpriteColor {
                    color: [255, 255, 255, 255],
                },
            );
            self.registry.emplace(
                c1,
                SpriteTexture {
                    name: "energy_orb".to_string(),
                },
            );
            self.registry.emplace(c1, ScriptComponent::new_rotator(2.2));

            let c2 = self.registry.create();
            self.registry.emplace(
                c2,
                Transform2D {
                    pos: Vec2::new(160.0, 0.0),
                    size: Vec2::new(28.0, 28.0),
                    rotation: 0.0,
                },
            );
            self.registry.emplace(
                c2,
                SpriteColor {
                    color: [255, 255, 255, 255],
                },
            );
            self.registry.emplace(
                c2,
                SpriteTexture {
                    name: "heart_icon".to_string(),
                },
            );
            self.registry
                .emplace(c2, ScriptComponent::new_pulse(20.0, 36.0, 3.5));
        }
    }

    fn spawn_projectile(&mut self, pos: Vec2, vel: Vec2) {
        let b = self.registry.create();
        self.registry.emplace(
            b,
            Transform2D {
                pos,
                size: Vec2::new(10.0, 18.0),
                rotation: 0.0,
            },
        );
        self.registry.emplace(
            b,
            SpriteColor {
                color: [80, 230, 255, 255],
            },
        );
        self.registry.emplace(b, Velocity2D { vel });
        self.registry.emplace(
            b,
            Projectile {
                is_player: true,
                lifetime: 2.0,
            },
        );
        self.audio.play("sfx_laser");
    }

    fn update_game(&mut self, dt: f32) {
        // 1. Oyuncu Kontrolleri
        if let Some(pe) = self.player_entity {
            let mut move_dir = Vec2::ZERO;
            if self.action_map.is_action_pressed("move_left") {
                move_dir.x -= 1.0;
            }
            if self.action_map.is_action_pressed("move_right") {
                move_dir.x += 1.0;
            }
            if self.action_map.is_action_pressed("move_up") {
                move_dir.y += 1.0;
            }
            if self.action_map.is_action_pressed("move_down") {
                move_dir.y -= 1.0;
            }

            if move_dir != Vec2::ZERO {
                move_dir = move_dir.normalize();
                let speed = 320.0;
                if let Some(tr) = self.registry.get_mut::<Transform2D>(pe) {
                    tr.pos += move_dir * speed * dt;
                    tr.pos.x = tr.pos.x.clamp(-600.0, 600.0);
                    tr.pos.y = tr.pos.y.clamp(-320.0, 320.0);
                }
            }

            if self.action_map.is_action_just_pressed("action_fire") {
                if let Some(tr) = self.registry.get::<Transform2D>(pe) {
                    let fire_pos = tr.pos + Vec2::new(0.0, 24.0);
                    self.spawn_projectile(fire_pos, Vec2::new(0.0, 600.0));
                }
            }
        }

        // 2. Fizik ve Mermi Yaşam Döngüsü
        let pool_tr = unsafe { self.registry.storage_mut::<Transform2D>() };
        let pool_vel = unsafe { self.registry.storage_mut::<Velocity2D>() };
        let pool_proj = unsafe { self.registry.storage_mut::<Projectile>() };

        let (proj_entities, projs) = pool_proj.split_entities_and_components_mut();
        for (idx, &e) in proj_entities.iter().enumerate() {
            let p = &mut projs[idx];
            p.lifetime -= dt;
            if p.lifetime <= 0.0 {
                self.cmd_buffer.destroy(e);
            }
        }

        let (vel_entities, vels) = pool_vel.split_entities_and_components_mut();
        for (idx, &e) in vel_entities.iter().enumerate() {
            let vel = &mut vels[idx];
            if let Some(tr) = pool_tr.get_mut(e) {
                tr.pos += vel.vel * dt;
            }
        }

        // 3. Betik Yaşam Döngüsü Güncellemesi (Aşama 5)
        let pool_scripts = unsafe { self.registry.storage_mut::<ScriptComponent>() };
        let (scr_entities, scrs) = pool_scripts.split_entities_and_components_mut();
        for (idx, &e) in scr_entities.iter().enumerate() {
            let scr = &mut scrs[idx];
            if let Some(tr) = pool_tr.get_mut(e) {
                scr.update(dt, &mut tr.pos, &mut tr.size, &mut tr.rotation);
            }
        }

        // 4. Çarpışma Tespiti: Mermiler & Düşmanlar
        let mut hits = Vec::new();
        let proj_list: Vec<(Entity, Vec2)> = proj_entities
            .iter()
            .filter_map(|&pe| pool_tr.get(pe).map(|tr| (pe, tr.pos)))
            .collect();

        let pool_col = unsafe { self.registry.storage::<Collider2D>() };
        let enemy_list: Vec<(Entity, Vec2, Vec2)> = pool_col
            .entities()
            .iter()
            .filter(|&&ee| {
                Some(ee) != self.player_entity && pool_col.get(ee).is_some_and(|c| !c.is_sensor)
            })
            .filter_map(|&ee| pool_tr.get(ee).map(|tr| (ee, tr.pos, tr.size)))
            .collect();

        for (pe, p_pos) in proj_list {
            for (ee, e_pos, e_size) in &enemy_list {
                let dx = (p_pos.x - e_pos.x).abs();
                let dy = (p_pos.y - e_pos.y).abs();
                if dx < e_size.x * 0.5 && dy < e_size.y * 0.5 {
                    hits.push((pe, *ee));
                    break;
                }
            }
        }

        for (pe, ee) in hits {
            self.cmd_buffer.destroy(pe);
            self.cmd_buffer.destroy(ee);
            self.score += 100;
            self.audio.play("sfx_explosion");
        }

        self.cmd_buffer.apply(&mut self.registry);
        self.action_map.end_frame();
    }

    fn render(&mut self, screen_w: f32, screen_h: f32) {
        if self.context.is_none() || self.pipeline_2d.is_none() {
            return;
        }

        self.arena.reset();
        let mut instances = Vec::with_capacity(4096);

        let center = Vec2::new(screen_w * 0.5, screen_h * 0.5);
        let atlas_ref = self.atlas.as_ref();

        // 1. Arka Plan Derinlik Yıldızları / Izgara
        let bg_color = [18, 20, 26, 255];
        instances.push(QuadInstance::new(
            [0.0, 0.0],
            [screen_w, screen_h],
            bg_color,
            0,
        ));

        // 2. Sahne Varlıklarını Çiz (Tek Draw Call, Atlas ve Düz Renk Hibrit)
        self.registry.view1_ref::<Transform2D, _>(|e, tr| {
            let col = self
                .registry
                .get::<SpriteColor>(e)
                .map(|s| s.color)
                .unwrap_or([255, 255, 255, 255]);
            let sub_reg = self
                .registry
                .get::<SpriteTexture>(e)
                .and_then(|st| atlas_ref.and_then(|a| a.get_region(&st.name)));

            let draw_x = center.x + tr.pos.x - tr.size.x * 0.5;
            let draw_y = center.y - tr.pos.y - tr.size.y * 0.5;

            if let Some(sub) = sub_reg {
                instances.push(QuadInstance::with_uv_rotated(
                    [draw_x, draw_y],
                    [tr.size.x, tr.size.y],
                    [sub.uv_min_packed, sub.uv_size_packed],
                    col,
                    1,
                    tr.rotation,
                ));
            } else {
                instances.push(QuadInstance::new_rotated(
                    [draw_x, draw_y],
                    [tr.size.x, tr.size.y],
                    col,
                    0,
                    tr.rotation,
                ));
            }
        });

        // 3. Standalone Oyun İçi Teşhis ve Skor Arayüzü (In-Game HUD)
        let hud_bg = [24, 28, 36, 220];
        let text_gold = [255, 220, 80, 255];
        let text_cyan = [80, 230, 255, 255];
        let text_white = [240, 240, 240, 255];

        instances.push(QuadInstance::new([12.0, 12.0], [380.0, 84.0], hud_bg, 0));
        instances.push(QuadInstance::new(
            [12.0, 12.0],
            [380.0, 2.0],
            [71, 140, 191, 255],
            0,
        ));

        font::draw_text(
            &mut instances,
            "PROJECT EMBER - STANDALONE RUNTIME",
            20.0,
            18.0,
            1.4,
            text_gold,
        );
        font::draw_text(
            &mut instances,
            &format!(
                "FPS: {:<4} | Frame: {:.2} ms | Varlik: {}",
                self.fps,
                self.frame_time_ms,
                self.registry.len()
            ),
            20.0,
            38.0,
            1.3,
            text_cyan,
        );
        font::draw_text(
            &mut instances,
            &format!("SKOR: {:06} | Sahne: {}", self.score, self.scene_name),
            20.0,
            56.0,
            1.3,
            text_white,
        );
        font::draw_text(
            &mut instances,
            "Kontrol: WASD/Oklar (Hareket), Space (Ates), Esc (Cikis)",
            20.0,
            74.0,
            1.1,
            [160, 170, 185, 255],
        );

        // 4. Vulkan 1.3 Dynamic Rendering ile Çizim
        let ctx = self.context.as_mut().unwrap();
        let pipe = self.pipeline_2d.as_mut().unwrap();
        let frame_idx = ctx.current_frame;

        if let Ok(Some((image_index, cmd))) = ctx.begin_frame(screen_w as u32, screen_h as u32) {
            let count = unsafe { pipe.upload_instances(frame_idx, &instances) };
            ctx.begin_rendering(cmd, image_index, [0.06, 0.07, 0.09, 1.0]);

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
            let _ = ctx.end_frame(image_index, cmd, screen_w as u32, screen_h as u32);
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let win_attrs = Window::default_attributes()
                .with_title("Project Ember — Standalone Game Runtime (Rugeng Engine)")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

            let window = Arc::new(
                event_loop
                    .create_window(win_attrs)
                    .expect("Pencere olusturulamadi"),
            );
            let size = window.inner_size();

            println!(
                "[Standalone Runtime] Vulkan Context ve Pipeline2D baslatiliyor: {}x{}",
                size.width, size.height
            );
            let context = VulkanContext::new(&*window, size.width, size.height)
                .expect("Vulkan Context baslatilamadi");

            let atlas = TextureAtlas::new(&context, 512, 512).expect("TextureAtlas olusturulamadi");

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

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!(
                    "[Standalone] Kapatiliyor. Toplam kare: {}",
                    self.frame_count
                );
                std::process::exit(0);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key,
                        state,
                        ..
                    },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                if let PhysicalKey::Code(key) = physical_key {
                    if pressed && key == KeyCode::Escape {
                        println!("[Standalone] ESC ile cikis yapildi.");
                        std::process::exit(0);
                    }
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
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                let btn_code = match button {
                    MouseButton::Left => 0,
                    MouseButton::Right => 1,
                    MouseButton::Middle => 2,
                    _ => 255,
                };
                if btn_code != 255 {
                    self.action_map.update_mouse_button(btn_code, pressed);
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame_time).as_secs_f32().min(0.05);
                self.last_frame_time = now;
                self.frame_time_ms = dt * 1000.0;

                self.fps_counter += 1;
                if self.fps_timer.elapsed().as_secs_f32() >= 1.0 {
                    self.fps = self.fps_counter;
                    self.fps_counter = 0;
                    self.fps_timer = Instant::now();
                }

                self.update_game(dt);

                let sz = if let Some(w) = self.window.as_ref() {
                    let sz = w.inner_size();
                    w.request_redraw();
                    Some(sz)
                } else {
                    None
                };

                if let Some(sz) = sz {
                    self.render(sz.width as f32, sz.height as f32);
                }

                self.frame_count += 1;
                if let Some(target) = self.auto_exit_frames {
                    if self.frame_count >= target {
                        println!(
                            "[Standalone Auto Test] {} kare basariyla tamamlandi. Cikis kodu 0.",
                            target
                        );
                        std::process::exit(0);
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut auto_exit = None;
    let mut scene_path = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--test-frames" && i + 1 < args.len() {
            auto_exit = args[i + 1].parse::<u64>().ok();
            i += 1;
        } else if args[i] == "--scene" && i + 1 < args.len() {
            scene_path = Some(args[i + 1].clone());
            i += 1;
        }
        i += 1;
    }

    println!("============================================================");
    println!(" Project Ember — Standalone Game Runtime (Rugeng Engine)");
    println!(" Tek Draw Call, Doku Atlası, Betik ve Ses Sistemi Aktif");
    println!("============================================================");

    let event_loop = EventLoop::new().expect("EventLoop olusturulamadi");
    let mut app = App::new(auto_exit, scene_path);
    let _ = event_loop.run_app(&mut app);
}
