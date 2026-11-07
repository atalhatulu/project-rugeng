//! Project Ember — Aşama 7: 3D Temeli ve Etkileşimli Demo Sahnesi (`games/demo_3d`).
//!
//! Özellikler:
//! - Vulkan 1.3 Dynamic Rendering ve D32_SFLOAT derinlik tamponu (`DepthBuffer`).
//! - 3D Blinn-Phong Işıklandırma (Güneş Yönlü Işık, Ortam Işığı, Speküler Parıltı).
//! - Prosedürel Geometriler: Dönen renkli küpler, küreler ve geniş zemin ızgarası (Grid Plane).
//! - 3D Sahne Grafı & Hiyerarşi: Ebeveyn etrafında yörüngede dönen uydu küpler (`compute_world_transform_3d`).
//! - Bounding Sphere ve AABB3D Frustum Culling testi.
//! - Serbest Uçuş Kamerası (Free-fly Camera: WASD + Space/Shift + Yön Okları).
//! - Katmanlı Çizim (Layered Rendering): 3D Sahne Çizimi -> 2D Quad Pipeline ile 2D UI/HUD Çizimi.
//! - Otomatik test çıkışı (`--auto-exit-frames N`).

use glam::{Mat4, Quat, Vec3};
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use rugeng_core::{compute_world_transform_3d, BoundingSphere, Frustum, Transform3D, AABB3D};
use rugeng_render::camera_3d::{AmbientLight, Camera3D, DirectionalLight};
use rugeng_render::context::VulkanContext;
use rugeng_render::depth::DepthBuffer;
use rugeng_render::font;
use rugeng_render::gpu_mesh::GpuMesh3D;
use rugeng_render::mesh::Mesh3D;
use rugeng_render::pipeline_2d::{Pipeline2D, QuadInstance};
use rugeng_render::pipeline_3d::Pipeline3D;
use rugeng_render::texture::TextureAtlas;

struct DemoObject {
    transform: Transform3D,
    rotation_speed: Vec3,
    mesh_idx: usize, // 0: cube, 1: sphere, 2: plane
    #[allow(dead_code)]
    aabb: AABB3D,
    sphere: BoundingSphere,
    is_child: bool,
}

struct App {
    window: Option<Arc<Window>>,
    context: Option<VulkanContext>,
    depth_buffer: Option<DepthBuffer>,
    shadow_map: Option<rugeng_render::DirectionalShadowMap>,
    shadow_pipeline: Option<rugeng_render::ShadowPipeline3D>,
    shadow_sampler: Option<ash::vk::Sampler>,
    shadow_descriptor_set: Option<ash::vk::DescriptorSet>,
    pipeline_3d: Option<Pipeline3D>,
    pipeline_2d: Option<Pipeline2D>,
    atlas: Option<TextureAtlas>,

    gpu_cube: Option<GpuMesh3D>,
    gpu_sphere: Option<GpuMesh3D>,
    gpu_plane: Option<GpuMesh3D>,
    gpu_gltf: Option<GpuMesh3D>,

    objects: Vec<DemoObject>,
    camera: Camera3D,
    camera_yaw: f32,
    camera_pitch: f32,

    sun: DirectionalLight,
    ambient: AmbientLight,

    last_frame_time: Instant,
    frame_count: u64,
    fps: u32,
    fps_counter: u32,
    fps_timer: Instant,
    auto_exit_frames: Option<u64>,

    // Kamera hareket girdileri
    move_forward: bool,
    move_backward: bool,
    move_left: bool,
    move_right: bool,
    move_up: bool,
    move_down: bool,
    turn_left: bool,
    turn_right: bool,
    turn_up: bool,
    turn_down: bool,

    culled_count: usize,
    rendered_count: usize,
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(ctx) = self.context.as_ref() {
            unsafe {
                let _ = ctx.device.device_wait_idle();
            }
            let mut alloc = ctx.allocator.lock().unwrap();
            if let Some(mut m) = self.gpu_cube.take() {
                m.destroy(&ctx.device, &mut alloc);
            }
            if let Some(mut m) = self.gpu_sphere.take() {
                m.destroy(&ctx.device, &mut alloc);
            }
            if let Some(mut m) = self.gpu_plane.take() {
                m.destroy(&ctx.device, &mut alloc);
            }
            if let Some(mut m) = self.gpu_gltf.take() {
                m.destroy(&ctx.device, &mut alloc);
            }
            if let Some(mut db) = self.depth_buffer.take() {
                db.destroy(&ctx.device, &mut alloc);
            }
            if let Some(mut sm) = self.shadow_map.take() {
                sm.destroy(&ctx.device, &mut alloc);
            }
            if let Some(mut atlas) = self.atlas.take() {
                atlas.destroy(&ctx.device, &mut alloc);
            }
            drop(alloc);

            if let Some(sampler) = self.shadow_sampler.take() {
                unsafe {
                    ctx.device.destroy_sampler(sampler, None);
                }
            }
            if let Some(mut sp) = self.shadow_pipeline.take() {
                sp.destroy(&ctx.device);
            }
            if let Some(mut p3d) = self.pipeline_3d.take() {
                p3d.destroy(&ctx.device);
            }
            if let Some(mut p2d) = self.pipeline_2d.take() {
                p2d.destroy(&ctx.device);
            }
        }
    }
}

impl App {
    fn new(auto_exit_frames: Option<u64>) -> Self {
        let mut objects = Vec::new();

        // 1. Zemin Izgarası
        objects.push(DemoObject {
            transform: Transform3D::from_xyz(0.0, -1.0, 0.0),
            rotation_speed: Vec3::ZERO,
            mesh_idx: 2, // plane
            aabb: AABB3D::from_center_half_extents(Vec3::ZERO, Vec3::new(20.0, 0.1, 20.0)),
            sphere: BoundingSphere::new(Vec3::ZERO, 28.0),
            is_child: false,
        });

        // 2. Merkez Dönen Küp
        objects.push(DemoObject {
            transform: Transform3D::new(Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY, Vec3::splat(1.5)),
            rotation_speed: Vec3::new(0.5, 1.0, 0.2),
            mesh_idx: 0, // cube
            aabb: AABB3D::from_center_half_extents(Vec3::ZERO, Vec3::splat(0.75)),
            sphere: BoundingSphere::new(Vec3::ZERO, 1.3),
            is_child: false,
        });

        // 3. Ebeveyn etrafında yörünge dönen uydu küp (Hiyerarşi)
        objects.push(DemoObject {
            transform: Transform3D::new(Vec3::new(3.0, 0.0, 0.0), Quat::IDENTITY, Vec3::splat(0.6)),
            rotation_speed: Vec3::new(1.5, 2.0, 0.0),
            mesh_idx: 0, // cube
            aabb: AABB3D::from_center_half_extents(Vec3::ZERO, Vec3::splat(0.3)),
            sphere: BoundingSphere::new(Vec3::ZERO, 0.6),
            is_child: true,
        });

        // 4. Farklı konumlarda küreler
        let sphere_positions = [
            Vec3::new(-4.0, 1.0, -3.0),
            Vec3::new(4.0, 1.0, -4.0),
            Vec3::new(-3.0, 1.5, 4.0),
            Vec3::new(5.0, 1.2, 3.0),
            Vec3::new(0.0, 3.5, -2.0),
        ];

        for pos in sphere_positions {
            objects.push(DemoObject {
                transform: Transform3D::from_translation(pos),
                rotation_speed: Vec3::new(0.0, 0.8, 0.0),
                mesh_idx: 1, // sphere
                aabb: AABB3D::from_center_half_extents(Vec3::ZERO, Vec3::splat(1.0)),
                sphere: BoundingSphere::new(Vec3::ZERO, 1.0),
                is_child: false,
            });
        }

        // 5. glTF 2.0 PBR Modeli (Merkez sağda)
        objects.push(DemoObject {
            transform: Transform3D::new(Vec3::new(0.0, 2.5, 0.0), Quat::IDENTITY, Vec3::splat(1.8)),
            rotation_speed: Vec3::new(0.0, 1.5, 0.5),
            mesh_idx: 3, // gltf
            aabb: AABB3D::from_center_half_extents(Vec3::ZERO, Vec3::splat(1.0)),
            sphere: BoundingSphere::new(Vec3::ZERO, 1.2),
            is_child: false,
        });

        Self {
            window: None,
            context: None,
            depth_buffer: None,
            shadow_map: None,
            shadow_pipeline: None,
            shadow_sampler: None,
            shadow_descriptor_set: None,
            pipeline_3d: None,
            pipeline_2d: None,
            atlas: None,
            gpu_cube: None,
            gpu_sphere: None,
            gpu_plane: None,
            gpu_gltf: None,
            objects,
            camera: Camera3D::new(
                Vec3::new(0.0, 3.5, 9.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::Y,
                60.0f32.to_radians(),
                16.0 / 9.0,
                0.1,
                1000.0,
            ),
            camera_yaw: -std::f32::consts::FRAC_PI_2,
            camera_pitch: -0.2,
            sun: DirectionalLight::new(Vec3::new(0.8, 1.2, 0.6), [1.0, 0.98, 0.9], 1.2),
            ambient: AmbientLight::new([0.2, 0.22, 0.28], 0.35),
            last_frame_time: Instant::now(),
            frame_count: 0,
            fps: 0,
            fps_counter: 0,
            fps_timer: Instant::now(),
            auto_exit_frames,
            move_forward: false,
            move_backward: false,
            move_left: false,
            move_right: false,
            move_up: false,
            move_down: false,
            turn_left: false,
            turn_right: false,
            turn_up: false,
            turn_down: false,
            culled_count: 0,
            rendered_count: 0,
        }
    }

    fn update(&mut self, dt: f32) {
        // 1. Kamera Rotasyon ve Hareket Güncellemesi
        let rot_speed = 1.8 * dt;
        if self.turn_left {
            self.camera_yaw -= rot_speed;
        }
        if self.turn_right {
            self.camera_yaw += rot_speed;
        }
        if self.turn_up {
            self.camera_pitch = (self.camera_pitch + rot_speed).clamp(-1.4, 1.4);
        }
        if self.turn_down {
            self.camera_pitch = (self.camera_pitch - rot_speed).clamp(-1.4, 1.4);
        }

        let forward = Vec3::new(
            self.camera_yaw.cos() * self.camera_pitch.cos(),
            self.camera_pitch.sin(),
            self.camera_yaw.sin() * self.camera_pitch.cos(),
        )
        .normalize();

        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward).normalize();

        let move_speed = 8.0 * dt;
        if self.move_forward {
            self.camera.eye += forward * move_speed;
        }
        if self.move_backward {
            self.camera.eye -= forward * move_speed;
        }
        if self.move_left {
            self.camera.eye -= right * move_speed;
        }
        if self.move_right {
            self.camera.eye += right * move_speed;
        }
        if self.move_up {
            self.camera.eye += Vec3::Y * move_speed;
        }
        if self.move_down {
            self.camera.eye -= Vec3::Y * move_speed;
        }

        self.camera.target = self.camera.eye + forward;
        self.camera.up = up;

        // 2. Nesnelerin Rotasyonu
        for obj in &mut self.objects {
            if obj.rotation_speed.length_squared() > 1e-6 {
                let delta_rot = Quat::from_euler(
                    glam::EulerRot::XYZ,
                    obj.rotation_speed.x * dt,
                    obj.rotation_speed.y * dt,
                    obj.rotation_speed.z * dt,
                );
                obj.transform.rotation = (obj.transform.rotation * delta_rot).normalize();
            }
        }
    }

    fn render(&mut self) {
        let (screen_w, screen_h) = if let Some(w) = self.window.as_ref() {
            let size = w.inner_size();
            (size.width.max(1), size.height.max(1))
        } else {
            (1280, 720)
        };

        self.camera.aspect_ratio = screen_w as f32 / screen_h as f32;

        let ctx = match self.context.as_mut() {
            Some(c) => c,
            None => return,
        };

        // Pencere resize kontrolü: Depth buffer yeniden oluşturma
        if ctx.resized
            || self
                .depth_buffer
                .as_ref()
                .is_none_or(|db| db.width != screen_w || db.height != screen_h)
        {
            if let Some(mut old_db) = self.depth_buffer.take() {
                let mut alloc = ctx.allocator.lock().unwrap();
                old_db.destroy(&ctx.device, &mut alloc);
            }
            let new_db =
                DepthBuffer::new(&ctx.device, Arc::clone(&ctx.allocator), screen_w, screen_h)
                    .expect("DepthBuffer olusturulamadi");
            self.depth_buffer = Some(new_db);
        }

        let depth_view = self.depth_buffer.as_ref().unwrap().view;
        let frame_idx = ctx.current_frame;

        let frame_token = match ctx.begin_frame(screen_w, screen_h) {
            Ok(Some(token)) => token,
            Ok(None) => return, // Yeniden boyutlandırıldı
            Err(e) => {
                eprintln!("[3D Demo] begin_frame hatasi: {e}");
                return;
            }
        };

        let (image_index, cmd) = frame_token;

        // Işık uzayı matrisi (Yönlü ışık gölge kamerası)
        let light_space = self
            .sun
            .light_space_matrix(glam::Vec3::ZERO, 15.0, 12.0, 0.1, 40.0);

        let parent_world = self.objects[1].transform.local_matrix();

        // =========================================================================
        // 1. GÖLGE DERİNLİK GEÇİŞİ (SHADOW PASS)
        // =========================================================================
        if let (Some(shadow_map), Some(shadow_pipe)) =
            (self.shadow_map.as_ref(), self.shadow_pipeline.as_ref())
        {
            // Gölge haritasını derinlik eki moduna al
            shadow_map.transition_to_attachment(&ctx.device, cmd);

            let res = shadow_map.resolution;
            let depth_clear = ash::vk::ClearValue {
                depth_stencil: ash::vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            };

            let shadow_attachment = ash::vk::RenderingAttachmentInfo::default()
                .image_view(shadow_map.view)
                .image_layout(ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .load_op(ash::vk::AttachmentLoadOp::CLEAR)
                .store_op(ash::vk::AttachmentStoreOp::STORE)
                .clear_value(depth_clear);

            let shadow_rendering_info = ash::vk::RenderingInfo::default()
                .render_area(ash::vk::Rect2D {
                    offset: ash::vk::Offset2D { x: 0, y: 0 },
                    extent: ash::vk::Extent2D {
                        width: res,
                        height: res,
                    },
                })
                .layer_count(1)
                .depth_attachment(&shadow_attachment);

            unsafe {
                ctx.device.cmd_begin_rendering(cmd, &shadow_rendering_info);

                ctx.device.cmd_bind_pipeline(
                    cmd,
                    ash::vk::PipelineBindPoint::GRAPHICS,
                    shadow_pipe.pipeline,
                );

                let viewport = ash::vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: res as f32,
                    height: res as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                };
                ctx.device.cmd_set_viewport(cmd, 0, &[viewport]);

                let scissor = ash::vk::Rect2D {
                    offset: ash::vk::Offset2D { x: 0, y: 0 },
                    extent: ash::vk::Extent2D {
                        width: res,
                        height: res,
                    },
                };
                ctx.device.cmd_set_scissor(cmd, 0, &[scissor]);

                // Gölge veren tüm nesneleri çiz (Zemin hariç: mesh_idx != 2)
                for obj in &self.objects {
                    if obj.mesh_idx == 2 {
                        continue;
                    }
                    let world_mat = if obj.is_child {
                        compute_world_transform_3d(parent_world, &obj.transform)
                    } else {
                        obj.transform.local_matrix()
                    };

                    let light_space_mvp = light_space * world_mat;
                    let pc = rugeng_render::ShadowPushConstants {
                        light_space_mvp: light_space_mvp.to_cols_array(),
                    };
                    ctx.device.cmd_push_constants(
                        cmd,
                        shadow_pipe.layout,
                        ash::vk::ShaderStageFlags::VERTEX,
                        0,
                        bytemuck::bytes_of(&pc),
                    );

                    let gpu_mesh = match obj.mesh_idx {
                        0 => self.gpu_cube.as_ref().unwrap(),
                        1 => self.gpu_sphere.as_ref().unwrap(),
                        _ => self
                            .gpu_gltf
                            .as_ref()
                            .unwrap_or_else(|| self.gpu_cube.as_ref().unwrap()),
                    };

                    gpu_mesh.draw(&ctx.device, cmd);
                }

                ctx.device.cmd_end_rendering(cmd);
            }

            // Gölge haritasını gölgelendirici okuma moduna geçir
            shadow_map.transition_to_shader_read(&ctx.device, cmd);
        }

        // =========================================================================
        // 2. ANA SAHNE RENDER GEÇİŞİ (MAIN PASS)
        // =========================================================================
        ctx.begin_rendering_with_depth(cmd, image_index, [0.08, 0.10, 0.15, 1.0], Some(depth_view));

        // 3D Çizim Adımı (Frustum Culling ile)
        let vp = self.camera.view_projection();
        let frustum = Frustum::from_view_projection(vp);

        let p3d = self.pipeline_3d.as_ref().unwrap();
        let extent = ctx.swapchain.extent;

        let mut rendered = 0;
        let mut culled = 0;

        for i in 0..self.objects.len() {
            let obj = &self.objects[i];

            let world_mat = if obj.is_child {
                compute_world_transform_3d(parent_world, &obj.transform)
            } else {
                obj.transform.local_matrix()
            };

            // Frustum Culling
            let world_sphere = obj.sphere.transform(world_mat);
            if !frustum.intersects_sphere(&world_sphere) {
                culled += 1;
                continue;
            }

            rendered += 1;
            let gpu_mesh = match obj.mesh_idx {
                0 => self.gpu_cube.as_ref().unwrap(),
                1 => self.gpu_sphere.as_ref().unwrap(),
                2 => self.gpu_plane.as_ref().unwrap(),
                _ => self
                    .gpu_gltf
                    .as_ref()
                    .unwrap_or_else(|| self.gpu_cube.as_ref().unwrap()),
            };

            let roughness = match obj.mesh_idx {
                2 => 0.8,
                1 => 0.2,
                _ => 0.4,
            };
            let metallic = match obj.mesh_idx {
                2 => 0.05,
                1 => 0.85,
                _ => 0.2,
            };

            p3d.draw_mesh_pbr_viewport_ext(
                &ctx.device,
                cmd,
                &self.camera,
                gpu_mesh,
                world_mat,
                &self.sun,
                &self.ambient,
                roughness,
                metallic,
                false,
                false,
                self.shadow_descriptor_set,
                0.0,
                0.0,
                extent.width as f32,
                extent.height as f32,
            );
        }

        self.rendered_count = rendered;
        self.culled_count = culled;

        // 4. 2D HUD / UI Çizim Adımı (Aynı karede 3D sahne üzerine katman)
        let mut instances = Vec::with_capacity(256);

        // Üst Bilgi Çubuğu (Yarı saydam arkaplan paneli)
        instances.push(QuadInstance::new(
            [15.0, 15.0],
            [460.0, 140.0],
            [18, 22, 30, 210],
            0,
        ));

        // Metin Yazıları (Fontdue / Bitmap Text)
        font::draw_text(
            &mut instances,
            "PROJECT EMBER - 3D FOUNDATION DEMO",
            25.0,
            24.0,
            1.25,
            [255, 215, 0, 255],
        );

        let stats_text = format!(
            "FPS: {} | Rendered: {} | Culled: {}",
            self.fps, self.rendered_count, self.culled_count
        );
        font::draw_text(
            &mut instances,
            &stats_text,
            25.0,
            52.0,
            1.1,
            [230, 240, 255, 255],
        );

        let cam_text = format!(
            "Cam: ({:.1}, {:.1}, {:.1}) | D32_SFLOAT Depth Test: ON",
            self.camera.eye.x, self.camera.eye.y, self.camera.eye.z
        );
        font::draw_text(
            &mut instances,
            &cam_text,
            25.0,
            78.0,
            1.0,
            [150, 210, 255, 255],
        );

        font::draw_text(
            &mut instances,
            "Kontrol: WASD (Ucus) | Space/Shift (Yukari/Asagi) | Oklar (Bakis)",
            25.0,
            104.0,
            0.95,
            [180, 190, 205, 255],
        );

        font::draw_text(
            &mut instances,
            "Stage 11: Directional Shadow Map (2048x2048 D32_SFLOAT) + PCF 3x3: ON",
            25.0,
            126.0,
            0.9,
            [120, 230, 160, 255],
        );

        // 2D Pipeline Çizimi
        let p2d = self.pipeline_2d.as_mut().unwrap();
        let quad_count = unsafe { p2d.upload_instances(frame_idx, &instances) };
        let view_proj_2d =
            Mat4::orthographic_rh(0.0, screen_w as f32, 0.0, screen_h as f32, -1.0, 1.0);

        p2d.draw(
            &ctx.device,
            cmd,
            frame_idx,
            quad_count as u32,
            view_proj_2d,
            extent,
        );

        ctx.end_rendering(cmd, image_index);
        let _ = ctx.end_frame(image_index, cmd, screen_w, screen_h);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let win_attrs = Window::default_attributes()
                .with_title("Project Ember — 3D Foundation Demo (Rugeng Engine Stage 7)")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

            let window = Arc::new(
                event_loop
                    .create_window(win_attrs)
                    .expect("Pencere olusturulamadi"),
            );
            let size = window.inner_size();

            println!(
                "[3D Demo] Vulkan Context, DepthBuffer ve 3D/2D Pipeline baslatiliyor: {}x{}",
                size.width, size.height
            );

            let context = VulkanContext::new(&*window, size.width, size.height)
                .expect("Vulkan Context baslatilamadi");

            let depth_buffer = DepthBuffer::new(
                &context.device,
                Arc::clone(&context.allocator),
                size.width,
                size.height,
            )
            .expect("DepthBuffer baslatilamadi");

            let atlas = TextureAtlas::new(&context, 512, 512).expect("TextureAtlas baslatilamadi");

            // Gölge Haritası ve Gölge Pipeline'ı (2048x2048 D32_SFLOAT)
            let shadow_map = rugeng_render::DirectionalShadowMap::new(
                &context.device,
                Arc::clone(&context.allocator),
                rugeng_render::DirectionalShadowMap::DEFAULT_RESOLUTION,
            )
            .expect("DirectionalShadowMap baslatilamadi");

            let shadow_pipeline =
                rugeng_render::ShadowPipeline3D::new(&context.device, shadow_map.format)
                    .expect("ShadowPipeline3D baslatilamadi");

            let shadow_sampler = rugeng_render::create_shadow_sampler(&context.device)
                .expect("Shadow Sampler baslatilamadi");

            let pipeline_3d = Pipeline3D::new(
                &context.device,
                context.swapchain.format,
                depth_buffer.format,
                atlas.image_view,
                atlas.sampler,
            )
            .expect("Pipeline3D baslatilamadi");

            // Shadow Map Descriptor Set (Binding 0: Albedo, Binding 1: MR, Binding 2: ShadowMap)
            let shadow_descriptor_set = {
                let alloc_info = ash::vk::DescriptorSetAllocateInfo::default()
                    .descriptor_pool(pipeline_3d.descriptor_pool)
                    .set_layouts(std::slice::from_ref(&pipeline_3d.descriptor_set_layout));

                let d_set = unsafe {
                    context
                        .device
                        .allocate_descriptor_sets(&alloc_info)
                        .expect("Shadow Descriptor Set allocate edilemedi")[0]
                };

                let img_info_albedo = ash::vk::DescriptorImageInfo::default()
                    .image_layout(ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(atlas.image_view)
                    .sampler(atlas.sampler);

                let img_info_shadow = ash::vk::DescriptorImageInfo::default()
                    .image_layout(ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(shadow_map.view)
                    .sampler(shadow_sampler);

                let writes = [
                    ash::vk::WriteDescriptorSet::default()
                        .dst_set(d_set)
                        .dst_binding(0)
                        .descriptor_type(ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(std::slice::from_ref(&img_info_albedo)),
                    ash::vk::WriteDescriptorSet::default()
                        .dst_set(d_set)
                        .dst_binding(1)
                        .descriptor_type(ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(std::slice::from_ref(&img_info_albedo)),
                    ash::vk::WriteDescriptorSet::default()
                        .dst_set(d_set)
                        .dst_binding(2)
                        .descriptor_type(ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(std::slice::from_ref(&img_info_shadow)),
                ];

                unsafe {
                    context.device.update_descriptor_sets(&writes, &[]);
                }
                d_set
            };

            let pipeline_2d = Pipeline2D::new_with_depth(
                &context.device,
                Arc::clone(&context.allocator),
                context.swapchain.format,
                Some(depth_buffer.format),
                rugeng_render::FRAMES_IN_FLIGHT,
                atlas.image_view,
                atlas.sampler,
            )
            .expect("Pipeline2D baslatilamadi");

            // Prosedürel Mesh'lerin GPU'ya Yüklenmesi
            let cube_mesh = Mesh3D::create_cube(1.0, [0.9, 0.4, 0.2, 1.0]);
            let sphere_mesh = Mesh3D::create_sphere(1.0, 24, 16, [0.2, 0.6, 0.9, 1.0]);
            let plane_mesh = Mesh3D::create_plane(40.0, 40.0, [0.35, 0.38, 0.42, 1.0]);

            let gpu_cube =
                GpuMesh3D::from_mesh(&context.device, Arc::clone(&context.allocator), &cube_mesh)
                    .expect("Cube GPU mesh yuklenemedi");

            let gpu_sphere = GpuMesh3D::from_mesh(
                &context.device,
                Arc::clone(&context.allocator),
                &sphere_mesh,
            )
            .expect("Sphere GPU mesh yuklenemedi");

            let gpu_plane =
                GpuMesh3D::from_mesh(&context.device, Arc::clone(&context.allocator), &plane_mesh)
                    .expect("Plane GPU mesh yuklenemedi");

            let gpu_gltf = {
                let sample_path = "assets/models/sample.glb";
                if let Ok(model) = rugeng_assets::GltfModel::from_file(sample_path) {
                    if let Some(sub) = model.submeshes.first() {
                        let vertices: Vec<rugeng_render::mesh::Vertex3D> = sub
                            .vertices
                            .iter()
                            .map(|v| {
                                rugeng_render::mesh::Vertex3D::new(
                                    v.position, v.normal, v.uv, v.color,
                                )
                            })
                            .collect();
                        let mesh = rugeng_render::mesh::Mesh3D::new(vertices, sub.indices.clone());
                        GpuMesh3D::from_mesh(&context.device, Arc::clone(&context.allocator), &mesh)
                            .ok()
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            self.window = Some(window);
            self.context = Some(context);
            self.depth_buffer = Some(depth_buffer);
            self.shadow_map = Some(shadow_map);
            self.shadow_pipeline = Some(shadow_pipeline);
            self.shadow_sampler = Some(shadow_sampler);
            self.shadow_descriptor_set = Some(shadow_descriptor_set);
            self.pipeline_3d = Some(pipeline_3d);
            self.pipeline_2d = Some(pipeline_2d);
            self.atlas = Some(atlas);
            self.gpu_cube = Some(gpu_cube);
            self.gpu_sphere = Some(gpu_sphere);
            self.gpu_plane = Some(gpu_plane);
            self.gpu_gltf = gpu_gltf;
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(ctx) = self.context.as_mut() {
                    ctx.resized = true;
                    let _ = ctx.recreate_swapchain(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                match code {
                    KeyCode::Escape => event_loop.exit(),
                    KeyCode::KeyW => self.move_forward = pressed,
                    KeyCode::KeyS => self.move_backward = pressed,
                    KeyCode::KeyA => self.move_left = pressed,
                    KeyCode::KeyD => self.move_right = pressed,
                    KeyCode::Space => self.move_up = pressed,
                    KeyCode::ShiftLeft | KeyCode::ShiftRight => self.move_down = pressed,
                    KeyCode::ArrowLeft => self.turn_left = pressed,
                    KeyCode::ArrowRight => self.turn_right = pressed,
                    KeyCode::ArrowUp => self.turn_up = pressed,
                    KeyCode::ArrowDown => self.turn_down = pressed,
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame_time).as_secs_f32().min(0.1);
                self.last_frame_time = now;

                self.fps_counter += 1;
                if self.fps_timer.elapsed().as_secs_f32() >= 1.0 {
                    self.fps = self.fps_counter;
                    self.fps_counter = 0;
                    self.fps_timer = Instant::now();
                }

                self.update(dt);
                self.render();

                self.frame_count += 1;
                if let Some(max_frames) = self.auto_exit_frames {
                    if self.frame_count >= max_frames {
                        println!(
                            "[3D Demo] Otomatik cikis kosulu saglandi ({} kare tamamlandi).",
                            max_frames
                        );
                        event_loop.exit();
                        return;
                    }
                }

                if let Some(w) = self.window.as_ref() {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(w) = self.window.as_ref() {
            w.request_redraw();
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut auto_exit = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--auto-exit-frames" && i + 1 < args.len() {
            auto_exit = args[i + 1].parse::<u64>().ok();
            i += 1;
        }
        i += 1;
    }

    println!("=======================================================");
    println!(" Project Ember (Rugeng) — Asama 7: 3D Temeli Demo");
    println!(" Vulkan 1.3 Dynamic Rendering, D32_SFLOAT, Blinn-Phong");
    println!("=======================================================");

    let event_loop = EventLoop::new().expect("EventLoop olusturulamadi");
    let mut app = App::new(auto_exit);
    let _ = event_loop.run_app(&mut app);
}
