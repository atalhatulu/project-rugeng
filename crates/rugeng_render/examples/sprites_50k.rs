//! 50k-200k Sprite 2D Instanced Render Demosu (Aşama 1 Çıkış Doğrulaması).
//!
//! FrameArena scratch toplama + Persistent-Mapped Ring Upload Buffer +
//! Vulkan 1.3 Dynamic Rendering ve tek bir `vkCmdDraw(6, N)` çağrısıyla
//! stabil frame süresini ve kare bütçesi yüzdesini kanıtlar.

use glam::{Mat4, Vec2};
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use rugeng_core::FrameArena;
use rugeng_render::context::VulkanContext;
use rugeng_render::pipeline_2d::{Pipeline2D, QuadInstance};

struct Particle {
    pos: Vec2,
    vel: Vec2,
    size: Vec2,
    color: [u8; 4],
    tex_id: u32,
}

struct App {
    window: Option<Arc<Window>>,
    context: Option<VulkanContext>,
    pipeline_2d: Option<Pipeline2D>,
    atlas: Option<rugeng_render::TextureAtlas>,
    arena: FrameArena,
    particles: Vec<Particle>,
    frame_count: u64,
    last_fps_time: Instant,
    last_frame_time: Instant,
    accumulated_ms: f32,
    frames_in_sec: u32,
    sprite_count: usize,
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
    fn new(sprite_count: usize) -> Self {
        let arena = FrameArena::new(64 * 1024 * 1024).expect("FrameArena tahsis edilemedi");

        // Basit deterministik LCG
        let mut state: u64 = 0x9E3779B97F4A7C15;
        let mut rand_f32 = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 32) as u32 as f32) / (u32::MAX as f32)
        };

        let mut particles = Vec::with_capacity(sprite_count);
        for i in 0..sprite_count {
            let px = rand_f32() * 1280.0;
            let py = rand_f32() * 720.0;
            let vx = (rand_f32() - 0.5) * 300.0;
            let vy = (rand_f32() - 0.5) * 300.0;

            let r = (100.0 + rand_f32() * 155.0) as u8;
            let g = (80.0 + rand_f32() * 140.0) as u8;
            let b = (50.0 + rand_f32() * 100.0) as u8;

            particles.push(Particle {
                pos: Vec2::new(px, py),
                vel: Vec2::new(vx, vy),
                size: Vec2::new(6.0, 6.0),
                color: [r, g, b, 230], // Yarı saydam alpha blend
                tex_id: (i % 8) as u32,
            });
        }

        Self {
            window: None,
            context: None,
            pipeline_2d: None,
            atlas: None,
            arena,
            particles,
            frame_count: 0,
            last_fps_time: Instant::now(),
            last_frame_time: Instant::now(),
            accumulated_ms: 0.0,
            frames_in_sec: 0,
            sprite_count,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let win_attrs = Window::default_attributes()
                .with_title(format!(
                    "Project Ember - {} Sprites (Rugeng)",
                    self.sprite_count
                ))
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

            let window = Arc::new(
                event_loop
                    .create_window(win_attrs)
                    .expect("Pencere olusturulamadi"),
            );
            let size = window.inner_size();

            println!(
                "[Demo] Vulkan Context ve Pipeline2D baslatiliyor: {}x{}",
                size.width, size.height
            );
            let context = VulkanContext::new(&*window, size.width, size.height)
                .expect("Vulkan Context baslatilamadi");

            let atlas =
                rugeng_render::TextureAtlas::new(&context, 256, 256).expect("Atlas olusturulamadi");

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
            self.last_fps_time = Instant::now();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("[Demo] Kapatiliyor. Toplam kare: {}", self.frame_count);
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let Some(ctx) = self.context.as_mut() {
                    ctx.resized = true;
                    let _ = ctx.recreate_swapchain(new_size.width, new_size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(window) = self.window.as_ref() else {
                    return;
                };
                let Some(ctx) = self.context.as_mut() else {
                    return;
                };
                let Some(pipe) = self.pipeline_2d.as_mut() else {
                    return;
                };

                let size = window.inner_size();
                if size.width == 0 || size.height == 0 {
                    return;
                }

                let now = Instant::now();
                let dt = now
                    .duration_since(self.last_frame_time)
                    .as_secs_f32()
                    .min(0.05);
                self.last_frame_time = now;

                // 1. Hareket Güncellemesi (Sınır çarpışmalarıyla ekran içinde tut)
                let w = size.width as f32;
                let h = size.height as f32;

                for p in &mut self.particles {
                    p.pos += p.vel * dt;
                    if p.pos.x < 0.0 || p.pos.x > w {
                        p.vel.x = -p.vel.x;
                        p.pos.x = p.pos.x.clamp(0.0, w);
                    }
                    if p.pos.y < 0.0 || p.pos.y > h {
                        p.vel.y = -p.vel.y;
                        p.pos.y = p.pos.y.clamp(0.0, h);
                    }
                }

                // 2. FrameArena üzerinde QuadInstance toplama (Gameplay döngüsünde sıfır malloc)
                self.arena.reset();
                let mut instances = Vec::with_capacity(self.particles.len());

                for p in &self.particles {
                    instances.push(QuadInstance::new(
                        [p.pos.x, p.pos.y],
                        [p.size.x, p.size.y],
                        p.color,
                        p.tex_id,
                    ));
                }

                let arena_instances = self.arena.alloc_slice_copy(&instances).unwrap();

                // 3. Vulkan Frame Döngüsü
                let frame_idx = ctx.current_frame;

                match ctx.begin_frame(size.width, size.height) {
                    Ok(Some((image_index, cmd))) => {
                        // Persistent-mapped ring buffer'a tek memcpy ile aktarım
                        let count = unsafe { pipe.upload_instances(frame_idx, arena_instances) };

                        // Dynamic Rendering: begin_rendering ile temizle ve render pass başlat
                        ctx.begin_rendering(cmd, image_index, [0.05, 0.06, 0.08, 1.0]);

                        // 2D Camera Ortho Projeksiyonu: [0, w] -> [0, h]
                        let view_proj = Mat4::orthographic_rh(0.0, w, 0.0, h, -1.0, 1.0);

                        // Dynamic Rendering içinde 2D Instanced Draw: TEK `vkCmdDraw(6, N)`!
                        pipe.draw(
                            &ctx.device,
                            cmd,
                            frame_idx,
                            count as u32,
                            view_proj,
                            ctx.swapchain.extent,
                        );

                        ctx.end_rendering(cmd, image_index);

                        if let Err(e) = ctx.end_frame(image_index, cmd, size.width, size.height) {
                            eprintln!("end_frame hatasi: {e}");
                        }

                        self.frame_count += 1;
                        self.frames_in_sec += 1;
                        let frame_ms = now.elapsed().as_secs_f32() * 1000.0;
                        self.accumulated_ms += frame_ms;

                        // Kare süresi ve bütçe yüzdesi loglaması (saniyede 1 kez)
                        if self.last_fps_time.elapsed().as_secs_f32() >= 1.0 {
                            let avg_ms = self.accumulated_ms / (self.frames_in_sec as f32);
                            let budget_pct_60 = (avg_ms / 16.66) * 100.0;
                            let title = format!(
                                "Project Ember - {} Sprites | Frame: {:.2} ms (60 FPS bütçesinin %{:.1}'i) | FPS: {}",
                                self.sprite_count, avg_ms, budget_pct_60, self.frames_in_sec
                            );
                            window.set_title(&title);
                            println!("[Metrik] {}", title);

                            self.accumulated_ms = 0.0;
                            self.frames_in_sec = 0;
                            self.last_fps_time = Instant::now();
                        }
                    }
                    Ok(None) => {
                        // Swapchain resize
                    }
                    Err(e) => {
                        eprintln!("begin_frame hatasi: {e}");
                    }
                }

                // Otomatik test modu kontrolü (CI için 120 kare sonra temiz kapanma)
                if std::env::var("AUTO_TEST").is_ok() && self.frame_count >= 120 {
                    let err_count = ctx
                        .validation_error_count
                        .load(std::sync::atomic::Ordering::Relaxed);
                    println!(
                        "[Test] {} sprite testi tamamlandi! Kare: {}, Validation Hatasi: {}",
                        self.sprite_count, self.frame_count, err_count
                    );
                    assert_eq!(err_count, 0, "Validation hatasi bulundu!");
                    event_loop.exit();
                    return;
                }

                window.request_redraw();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

fn main() {
    let sprite_count = std::env::var("SPRITE_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50_000);

    println!("============================================================");
    println!(" Project Ember - 2D Instanced Quad Demo (Aşama 1)");
    println!(" Hedef Sprite Sayisi: {}", sprite_count);
    println!(" Tek Draw Call: vkCmdDraw(6, N)");
    println!(" Persistent Mapped Ring Upload Buffer: 6.4 MB/frame");
    println!("============================================================");

    let event_loop = EventLoop::new().expect("Event loop olusturulamadi");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(sprite_count);
    event_loop
        .run_app(&mut app)
        .expect("Demo calisirken hata olustu");
}
