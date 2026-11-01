//! Vulkan 1.3 Clear Screen Örneği (Dynamic Rendering + Synchronization2).
//! Pencere açar, swapchain resize olaylarını yönetir ve ekranı Ember temalı renkle boyar.

use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use rugeng_render::context::VulkanContext;

struct App {
    window: Option<Arc<Window>>,
    context: Option<VulkanContext>,
    clear_color: [f32; 4],
    frame_count: u64,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            context: None,
            // Ember tarzı sıcak koyu kömür rengi
            clear_color: [0.08, 0.09, 0.11, 1.0],
            frame_count: 0,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let win_attrs = Window::default_attributes()
                .with_title("Project Ember - Vulkan 1.3 Clear Screen (Rugeng)")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

            let window = Arc::new(
                event_loop
                    .create_window(win_attrs)
                    .expect("Pencere olusturulamadi"),
            );
            let size = window.inner_size();

            println!(
                "Vulkan 1.3 Context baslatiliyor: {}x{}",
                size.width, size.height
            );
            let context = VulkanContext::new(&*window, size.width, size.height)
                .expect("Vulkan Context baslatilamadi");

            self.window = Some(window);
            self.context = Some(context);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!(
                    "Kapatma istegi alindi, toplam cizilen kare: {}",
                    self.frame_count
                );
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

                let size = window.inner_size();
                if size.width == 0 || size.height == 0 {
                    return;
                }

                // Dinamik renk animasyonu (sinüs dalgasıyla hafif nabız atışı)
                let time = (self.frame_count as f32) * 0.02;
                let r = 0.08 + 0.03 * time.sin().abs();
                let g = 0.09 + 0.02 * (time * 1.2).sin().abs();
                let b = 0.12 + 0.04 * (time * 0.8).cos().abs();
                self.clear_color = [r, g, b, 1.0];

                // Otomatik test modu (Resize / Minimize / Restore döngüsü)
                if std::env::var("AUTO_TEST").is_ok() {
                    match self.frame_count {
                        30 => {
                            println!("[Test Adimi 1/3] Otomatik Resize tetikleniyor: 800x600");
                            ctx.recreate_swapchain(800, 600).expect("Resize basarisiz");
                        }
                        60 => {
                            println!("[Test Adimi 2/3] Otomatik Minimize (0x0) simule ediliyor");
                            ctx.recreate_swapchain(0, 0).expect("Minimize basarisiz");
                        }
                        90 => {
                            println!("[Test Adimi 3/3] Otomatik Restore tetikleniyor: 1024x768");
                            ctx.recreate_swapchain(1024, 768)
                                .expect("Restore basarisiz");
                        }
                        120 => {
                            let err_count = ctx
                                .validation_error_count
                                .load(std::sync::atomic::Ordering::Relaxed);
                            println!("[Test] Dogrulama tamamlandi! Cizilen kare: {}, Validation Hatasi: {}", self.frame_count, err_count);
                            assert_eq!(err_count, 0, "Validation layer hatasi tespit edildi!");
                            event_loop.exit();
                            return;
                        }
                        _ => (),
                    }
                }

                match ctx.begin_frame(size.width, size.height) {
                    Ok(Some((image_index, cmd))) => {
                        ctx.draw_clear(cmd, image_index, self.clear_color);
                        if let Err(e) = ctx.end_frame(image_index, cmd, size.width, size.height) {
                            eprintln!("end_frame hatasi: {e}");
                        }
                        self.frame_count += 1;
                    }
                    Ok(None) => {
                        // Swapchain yeniden boyutlandırıldı veya atlandı
                    }
                    Err(e) => {
                        eprintln!("begin_frame hatasi: {e}");
                    }
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
    let event_loop = EventLoop::new().expect("Event loop olusturulamadi");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();
    println!("Uygulama baslatiliyor...");
    event_loop
        .run_app(&mut app)
        .expect("Uygulama calisirken hata olustu");
}
