use glam::Mat4;
use rugeng_render::context::VulkanContext;
use rugeng_render::pipeline_2d::{Pipeline2D, QuadInstance};
use std::sync::Arc;
use std::time::Instant;
use winit::event_loop::EventLoop;
use winit::window::Window;

#[allow(deprecated)]
fn main() {
    let event_loop = EventLoop::new().expect("EventLoop baslatilamadi");
    let win_attrs = Window::default_attributes()
        .with_title("RugEng Render Benchmark (IMMEDIATE/MAILBOX)")
        .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
        .with_visible(false);

    let window = Arc::new(
        event_loop
            .create_window(win_attrs)
            .expect("Pencere acilamadi"),
    );
    let size = window.inner_size();

    std::env::set_var("RUGENG_PRESENT_MODE", "IMMEDIATE");
    let mut ctx =
        VulkanContext::new(&*window, size.width, size.height).expect("Vulkan baslatilamadi");

    // Dummy atlas
    let (atlas_view, atlas_sampler) = unsafe {
        let sampler_info = ash::vk::SamplerCreateInfo::default();
        let s = ctx.device.create_sampler(&sampler_info, None).unwrap();
        (ash::vk::ImageView::null(), s)
    };

    let mut pipe = Pipeline2D::new(
        &ctx.device,
        (*ctx.allocator).clone(),
        ctx.swapchain.format,
        2,
        atlas_view,
        atlas_sampler,
    )
    .expect("Pipeline2D baslatilamadi");

    let counts = [50_000, 100_000, 200_000, 500_000];

    println!("\n==========================================================================================");
    println!(" [RUGENG RENDER BENCHMARK] Present Mode: IMMEDIATE / MAILBOX | Cozunurluk: 1280x720");
    println!(" Kriter Hedefi: < 5.0 ms");
    println!("==========================================================================================");

    for &n in &counts {
        let mut instances = Vec::with_capacity(n);
        for i in 0..n {
            let fi = i as f32;
            instances.push(QuadInstance::new(
                [(fi * 13.0) % 1280.0, (fi * 7.0) % 720.0],
                [16.0, 16.0],
                [255, 128, 64, 255],
                0,
            ));
        }

        // 20 warmup karesi
        for _ in 0..20 {
            let frame_idx = ctx.current_frame;
            if let Ok(Some((img_idx, cmd))) = ctx.begin_frame(size.width, size.height) {
                let cnt = unsafe { pipe.upload_instances(frame_idx, &instances) };
                ctx.begin_rendering(cmd, img_idx, [0.1, 0.1, 0.1, 1.0]);
                let vp = Mat4::orthographic_rh(0.0, 1280.0, 0.0, 720.0, -1.0, 1.0);
                pipe.draw(
                    &ctx.device,
                    cmd,
                    frame_idx,
                    cnt as u32,
                    vp,
                    ctx.swapchain.extent,
                );
                ctx.end_rendering(cmd, img_idx);
                let _ = ctx.end_frame(img_idx, cmd, size.width, size.height);
            }
        }
        unsafe {
            let _ = ctx.device.device_wait_idle();
        }

        // 100 olcum karesi (CPU frame time + GPU present / total time ayrık)
        let mut cpu_times_ms = Vec::with_capacity(100);
        let mut total_frame_times_ms = Vec::with_capacity(100);

        for _ in 0..100 {
            let t_start = Instant::now();
            let frame_idx = ctx.current_frame;
            if let Ok(Some((img_idx, cmd))) = ctx.begin_frame(size.width, size.height) {
                let cnt = unsafe { pipe.upload_instances(frame_idx, &instances) };
                ctx.begin_rendering(cmd, img_idx, [0.1, 0.1, 0.1, 1.0]);
                let vp = Mat4::orthographic_rh(0.0, 1280.0, 0.0, 720.0, -1.0, 1.0);
                pipe.draw(
                    &ctx.device,
                    cmd,
                    frame_idx,
                    cnt as u32,
                    vp,
                    ctx.swapchain.extent,
                );
                ctx.end_rendering(cmd, img_idx);
                let cpu_ms = t_start.elapsed().as_secs_f64() * 1000.0;
                cpu_times_ms.push(cpu_ms);

                let _ = ctx.end_frame(img_idx, cmd, size.width, size.height);
                let total_ms = t_start.elapsed().as_secs_f64() * 1000.0;
                total_frame_times_ms.push(total_ms);
            }
        }
        unsafe {
            let _ = ctx.device.device_wait_idle();
        }

        cpu_times_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        total_frame_times_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let avg_cpu: f64 = cpu_times_ms.iter().sum::<f64>() / cpu_times_ms.len() as f64;
        let p99_cpu = cpu_times_ms[(cpu_times_ms.len() as f64 * 0.99) as usize];

        let avg_total: f64 =
            total_frame_times_ms.iter().sum::<f64>() / total_frame_times_ms.len() as f64;
        let p99_total = total_frame_times_ms[(total_frame_times_ms.len() as f64 * 0.99) as usize];

        let status = if avg_total < 5.0 {
            "SAGLANDI"
        } else {
            "SAGLANMADI (BASARISIZ)"
        };

        println!("Sprite: {:>6} | CPU Ort: {:>5.2}ms (p99: {:>5.2}ms) | Toplam Ort: {:>5.2}ms (p99: {:>5.2}ms) | Durum: {}",
            n, avg_cpu, p99_cpu, avg_total, p99_total, status);
    }
    println!("==========================================================================================\n");

    unsafe {
        let _ = ctx.device.device_wait_idle();
        pipe.destroy(&ctx.device);
    }
}
