//! Project Ember — Godot 4 Tasarımlı Motor Çalışma Alanı (Rugeng Editor / Workbench).
//!
//! Kullanıcının ilettiği Godot 4 arayüzü referans alınarak tasarlanmıştır:
//! - Üst Menü Çubuğu (Scene, Project, Debug, Editor, Help, 2D/3D/Script/Game Çalışma Alanı ve Oynatma Butonları)
//! - Sol Sütun (Üst: Sahne Ağacı [Scene/Import], Alt: Dosya Sistemi [FileSystem/History])
//! - Orta Alan (Sahne Sekmeleri: player/world, Alt Araç Çubuğu, Cetvelli Sonsuz 2D Tuval, Zoom Rozeti)
//! - Alt Panel (Sadece Tuvalin Altında: Çıktı/Konsol [Output/Debugger/Audio/Animation], Hata/Uyarı Sayaçları)
//! - Sağ Sütun (Tam Boy Denetçi [Inspector/Signals], Arama Çubuğu, Modüler Bileşen Kartları)
//! - Vulkan 1.3 Dynamic Rendering ile tek draw call çizim.

use glam::Vec2;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use rugeng_core::{
    ActionMap, Animation2DComponent, AudioEngine, Children, Collider2D, CommandBuffer, Entity,
    FrameArena, Mesh3DComponent, Parent, ParticleEmitterComponent, Registry, ScriptComponent,
    SpriteColor, SpriteTexture, Text2DComponent, TileType, Tilemap2DComponent, Transform2D,
    Transform3D, Velocity2D, ZIndex2D,
};
use rugeng_render::camera_3d::{AmbientLight, DirectionalLight};
use rugeng_render::context::VulkanContext;
use rugeng_render::depth::{DepthBuffer, DEPTH_FORMAT};
use rugeng_render::font::TrueTypeFont;
use rugeng_render::gpu_mesh::GpuMesh3D;
use rugeng_render::mesh::Mesh3D;
use rugeng_render::pipeline_2d::Pipeline2D;
use rugeng_render::pipeline_3d::Pipeline3D;
use rugeng_render::texture::TextureAtlas;

mod input;
mod scene_ops;
mod simulation;
mod theme;
mod types;
mod ui;

use theme::*;
use types::*;
use ui::DockLayout;
pub(crate) struct EditorState {
    pub(crate) theme: EditorTheme,
    pub(crate) selected_entity: Option<Entity>,
    pub(crate) logs: Vec<LogMessage>,
    pub(crate) camera: EditorCamera,
    pub(crate) mouse_pos: Vec2,
    pub(crate) is_playing: bool,
    pub(crate) workspace: WorkspaceMode,
    pub(crate) left_tab: LeftTab,
    pub(crate) bottom_shelf_tab: BottomShelfTab,
    pub(crate) scene_tab: SceneTab,
    pub(crate) active_tool: ViewportTool,
    pub(crate) drag_mode: DragMode,
    pub(crate) snap_to_grid: bool,
    pub(crate) current_path: PathBuf,
    pub(crate) files_in_dir: Vec<FileItem>,
    pub(crate) selected_file: String,
    pub(crate) scene_snapshot: Vec<(Entity, Transform2D)>,
    pub(crate) scene_snapshot_3d: Vec<(Entity, Transform3D)>,
    pub(crate) current_scene_path: String,
    pub(crate) is_scene_dirty: bool,
    pub(crate) scene_counter: u32,
    pub(crate) selected_entities: Vec<Entity>,
    pub(crate) box_selection: Option<BoxSelection>,
    pub(crate) undo_stack: Vec<UndoAction>,
    pub(crate) redo_stack: Vec<UndoAction>,
    /// F2 satır içi yeniden adlandırma durumu (aktifse düzenleme modu).
    pub(crate) renaming: Option<InlineTextEdit>,
    pub(crate) show_add_component_popup: bool,
    pub(crate) active_menu: Option<OpenMenu>,
    pub(crate) active_tile_type: TileType,
    pub(crate) is_mouse_down: bool,
    pub(crate) drag_start_transform: Option<Transform2D>,
    pub(crate) camera_3d: EditorCamera3D,
    pub(crate) context_menu: Option<ContextMenu>,
    pub(crate) right_click_start: Option<Vec2>,
    pub(crate) right_click_down: bool,
    pub(crate) is_ctrl_pressed: bool,
    pub(crate) dock_left_w: f32,
    pub(crate) dock_right_w: f32,
    pub(crate) dock_bottom_h: f32,
}

impl EditorState {
    fn refresh_files(&mut self) {
        self.files_in_dir.clear();
        if let Ok(entries) = std::fs::read_dir(&self.current_path) {
            let mut list = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || name == "target" {
                    continue;
                }
                let is_dir = entry.file_type().is_ok_and(|ft| ft.is_dir());
                list.push(FileItem { name, is_dir });
            }
            list.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
            self.files_in_dir = list;
        }
    }
}

pub(crate) struct App {
    /// Manuel çift-tıklama algılaması (bu winit derlemesinde MouseEvent kanalı yok):
    /// son sol-tıklamanın anı ve konumu; 350 ms + 6 px penceresi içindeyse çift tık.
    pub(crate) last_click_at: Option<std::time::Instant>,
    pub(crate) last_click_pos: Vec2,
    pub(crate) pipeline_2d: Option<Pipeline2D>,
    pub(crate) pipeline_3d: Option<Pipeline3D>,
    pub(crate) depth_buffer: Option<DepthBuffer>,
    pub(crate) gpu_cube: Option<GpuMesh3D>,
    pub(crate) gpu_sphere: Option<GpuMesh3D>,
    pub(crate) gpu_plane: Option<GpuMesh3D>,
    pub(crate) gpu_cylinder: Option<GpuMesh3D>,
    pub(crate) gpu_cone: Option<GpuMesh3D>,
    pub(crate) gpu_pyramid: Option<GpuMesh3D>,
    pub(crate) gpu_capsule: Option<GpuMesh3D>,
    pub(crate) gpu_sprite_3d: Option<GpuMesh3D>,
    pub(crate) gpu_grid: Option<GpuMesh3D>,
    pub(crate) gpu_gltf_sample: Option<GpuMesh3D>,
    pub(crate) sun: DirectionalLight,
    pub(crate) ambient: AmbientLight,
    pub(crate) context: Option<VulkanContext>,
    pub(crate) atlas: Option<TextureAtlas>,
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) arena: FrameArena,
    pub(crate) registry: Registry,
    pub(crate) cmd_buffer: CommandBuffer,
    pub(crate) editor: EditorState,
    pub(crate) audio: AudioEngine,
    pub(crate) action_map: ActionMap,
    pub(crate) last_frame_time: Instant,
    pub(crate) frame_time_ms: f32,
    pub(crate) fps: u32,
    pub(crate) fps_counter: u32,
    pub(crate) fps_timer: Instant,
    pub(crate) frame_count: u64,
    pub(crate) ttf_font: Option<TrueTypeFont>,
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
            if let Some(mut m) = self.gpu_cylinder.take() {
                m.destroy(&ctx.device, &mut alloc);
            }
            if let Some(mut m) = self.gpu_cone.take() {
                m.destroy(&ctx.device, &mut alloc);
            }
            if let Some(mut m) = self.gpu_pyramid.take() {
                m.destroy(&ctx.device, &mut alloc);
            }
            if let Some(mut m) = self.gpu_capsule.take() {
                m.destroy(&ctx.device, &mut alloc);
            }
            if let Some(mut m) = self.gpu_sprite_3d.take() {
                m.destroy(&ctx.device, &mut alloc);
            }
            if let Some(mut m) = self.gpu_grid.take() {
                m.destroy(&ctx.device, &mut alloc);
            }
            if let Some(mut m) = self.gpu_gltf_sample.take() {
                m.destroy(&ctx.device, &mut alloc);
            }
            if let Some(mut db) = self.depth_buffer.take() {
                db.destroy(&ctx.device, &mut alloc);
            }
            if let Some(mut atlas) = self.atlas.take() {
                atlas.destroy(&ctx.device, &mut alloc);
            }
            drop(alloc);

            if let Some(mut pipe) = self.pipeline_2d.take() {
                pipe.destroy(&ctx.device);
            }
            if let Some(mut pipe) = self.pipeline_3d.take() {
                pipe.destroy(&ctx.device);
            }
        }
    }
}

impl App {
    fn new() -> Self {
        let arena = FrameArena::new(64 * 1024 * 1024).expect("FrameArena tahsis edilemedi");
        let mut registry = Registry::new();
        registry.register_component::<Transform2D>();
        registry.register_component::<SpriteColor>();
        registry.register_component::<SpriteTexture>();
        registry.register_component::<Collider2D>();
        registry.register_component::<Velocity2D>();
        registry.register_component::<ScriptComponent>();
        registry.register_component::<Text2DComponent>();
        registry.register_component::<Parent>();
        registry.register_component::<Children>();
        registry.register_component::<ParticleEmitterComponent>();
        registry.register_component::<Tilemap2DComponent>();
        registry.register_component::<Animation2DComponent>();
        registry.register_component::<Transform3D>();
        registry.register_component::<Mesh3DComponent>();
        registry.register_component::<ZIndex2D>();

        let mut editor = EditorState {
            theme: EditorTheme::dark(),
            selected_entity: None,
            logs: Vec::new(),
            camera: EditorCamera::new(),
            camera_3d: EditorCamera3D::new(),
            mouse_pos: Vec2::ZERO,
            is_playing: false,
            workspace: WorkspaceMode::TwoD,
            left_tab: LeftTab::Scene,
            bottom_shelf_tab: BottomShelfTab::Output,
            scene_tab: SceneTab::World,
            active_tool: ViewportTool::Select,
            drag_mode: DragMode::None,
            snap_to_grid: false,
            current_path: PathBuf::from("."),
            files_in_dir: Vec::new(),
            selected_file: "fish_eye.gdshader".to_string(),
            scene_snapshot: Vec::new(),
            scene_snapshot_3d: Vec::new(),
            current_scene_path: "scenes/main.scene".to_string(),
            is_scene_dirty: false,
            scene_counter: 1,
            selected_entities: Vec::new(),
            box_selection: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            renaming: None,
            show_add_component_popup: false,
            active_menu: None,
            active_tile_type: TileType::Grass,
            is_mouse_down: false,
            drag_start_transform: None,
            context_menu: None,
            right_click_start: None,
            right_click_down: false,
            is_ctrl_pressed: false,
            dock_left_w: DockLayout::DEFAULT_LEFT_W,
            dock_right_w: DockLayout::DEFAULT_RIGHT_W,
            dock_bottom_h: DockLayout::DEFAULT_BOTTOM_H,
        };

        editor.refresh_files();

        editor.logs.push(LogMessage {
            level: "INFO",
            text: "Godot Engine v4.7.2.stable.arch_linux (c) 2007-present Juan Linietsky, Ariel Manzur & Godot Contributors.".to_string(),
        });
        editor.logs.push(LogMessage {
            level: "WARN",
            text: "--- Failed to start Debug adapter server on port 6006: Already in use ---"
                .to_string(),
        });
        editor.logs.push(LogMessage {
            level: "WARN",
            text: "--- Failed to start GDScript language server on port 6005: Already in use ---"
                .to_string(),
        });
        editor.logs.push(LogMessage {
            level: "INFO",
            text: "Project Ember (Rugeng Engine) Vulkan 1.3 Calisma Alani hazir.".to_string(),
        });

        // Godot sahnesi için örnek varlıklar
        let e1 = registry.create();
        registry.emplace(
            e1,
            Transform2D {
                pos: Vec2::new(0.0, 0.0),
                size: Vec2::new(48.0, 48.0),
                rotation: 0.0,
            },
        );
        registry.emplace(
            e1,
            SpriteColor {
                color: [255, 255, 255, 255],
            },
        ); // Dokulu Gemi (player)
        registry.emplace(
            e1,
            SpriteTexture {
                name: "player_ship".to_string(),
            },
        );
        registry.emplace(e1, ScriptComponent::new_rotator(1.5));

        let e2 = registry.create();
        registry.emplace(
            e2,
            Transform2D {
                pos: Vec2::new(160.0, -60.0),
                size: Vec2::new(40.0, 40.0),
                rotation: 0.0,
            },
        );
        registry.emplace(
            e2,
            SpriteColor {
                color: [255, 255, 255, 255],
            },
        ); // Dokulu Yaratık (world)
        registry.emplace(
            e2,
            SpriteTexture {
                name: "enemy_bug".to_string(),
            },
        );
        registry.emplace(
            e2,
            Velocity2D {
                vel: Vec2::new(-35.0, 50.0),
            },
        );
        registry.emplace(e2, ScriptComponent::new_patrol(0, 80.0, 240.0, 75.0));

        let e3 = registry.create();
        registry.emplace(
            e3,
            Transform2D {
                pos: Vec2::new(-140.0, 80.0),
                size: Vec2::new(32.0, 32.0),
                rotation: 0.0,
            },
        );
        registry.emplace(
            e3,
            SpriteColor {
                color: [255, 255, 255, 255],
            },
        ); // Dokulu Can Kalbi (UI)
        registry.emplace(
            e3,
            SpriteTexture {
                name: "heart_icon".to_string(),
            },
        );
        registry.emplace(
            e3,
            Collider2D {
                is_dynamic: false,
                is_sensor: false,
            },
        );
        registry.emplace(
            e3,
            Velocity2D {
                vel: Vec2::new(25.0, -40.0),
            },
        );
        registry.emplace(e3, ScriptComponent::new_pulse(20.0, 44.0, 3.0));

        // 3D Örnek Varlıklar (Viewport 3D Modu İçin)
        let e_cube = registry.create();
        registry.emplace(e_cube, Transform3D::from_xyz(0.0, 1.0, 0.0));
        registry.emplace(
            e_cube,
            Mesh3DComponent::with_pbr("cube", [0.9, 0.3, 0.3, 1.0], 0.3, 0.2),
        );

        let e_sphere = registry.create();
        registry.emplace(e_sphere, Transform3D::from_xyz(2.5, 1.0, -1.0));
        registry.emplace(
            e_sphere,
            Mesh3DComponent::with_pbr("sphere", [0.3, 0.6, 0.9, 1.0], 0.1, 0.8),
        );

        Self {
            window: None,
            context: None,
            last_click_at: None,
            last_click_pos: Vec2::ZERO,
            pipeline_2d: None,
            pipeline_3d: None,
            depth_buffer: None,
            gpu_cube: None,
            gpu_sphere: None,
            gpu_plane: None,
            gpu_cylinder: None,
            gpu_cone: None,
            gpu_pyramid: None,
            gpu_capsule: None,
            gpu_sprite_3d: None,
            gpu_grid: None,
            gpu_gltf_sample: None,
            sun: DirectionalLight::default(),
            ambient: AmbientLight::default(),
            atlas: None,
            arena,
            registry,
            cmd_buffer: CommandBuffer::new(),
            editor,
            audio: AudioEngine::new(),
            action_map: {
                let mut map = ActionMap::new();
                map.bind("fly_up", rugeng_core::ActionInput::Key(0x12)); // E
                map.bind("fly_down", rugeng_core::ActionInput::Key(0x10)); // Q
                map
            },
            last_frame_time: Instant::now(),
            frame_time_ms: 0.0,
            fps: 0,
            fps_counter: 0,
            fps_timer: Instant::now(),
            frame_count: 0,
            ttf_font: {
                // Öncelik projenin assets/fonts dizini, ardından sistem fontları
                let paths = [
                    "assets/fonts/Roboto-Regular.ttf",
                    "../../assets/fonts/Roboto-Regular.ttf",
                    "/usr/share/fonts/TTF/DejaVuSans.ttf",
                ];
                paths.iter().find_map(|p| TrueTypeFont::from_file(p).ok())
            },
        }
    }
}

impl App {
    /// Basılı sol tuş olayında çağrılır: aynı yere 350 ms içinde gelen ikinci tık.
    /// Bayrağı her zaman günceller; true dönerse pencere sıfırlanır (üçüncü tık tek sayılır).
    pub(crate) fn is_double_click(&mut self) -> bool {
        let pos = self.editor.mouse_pos;
        let dc = self
            .last_click_at
            .map(|t| t.elapsed().as_millis() < 350 && pos.distance(self.last_click_pos) < 6.0)
            .unwrap_or(false);
        self.last_click_at = Some(std::time::Instant::now());
        self.last_click_pos = pos;
        if dc {
            self.last_click_at = None;
        }
        dc
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let win_attrs = Window::default_attributes()
                .with_title("cod-mw4 - Godot Engine (Project Ember Rugeng Workbench)")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

            let window = Arc::new(
                event_loop
                    .create_window(win_attrs)
                    .expect("Pencere olusturulamadi"),
            );
            let size = window.inner_size();

            println!(
                "[Workbench] Vulkan Context, Pipeline2D ve Pipeline3D baslatiliyor: {}x{}",
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
            .expect("DepthBuffer olusturulamadi");

            let atlas = TextureAtlas::new(&context, 512, 512).expect("TextureAtlas olusturulamadi");

            let pipeline_2d = Pipeline2D::new_with_depth(
                &context.device,
                Arc::clone(&context.allocator),
                context.swapchain.format,
                Some(DEPTH_FORMAT),
                rugeng_render::FRAMES_IN_FLIGHT,
                atlas.image_view,
                atlas.sampler,
            )
            .expect("Pipeline2D olusturulamadi");

            let pipeline_3d = Pipeline3D::new(
                &context.device,
                context.swapchain.format,
                DEPTH_FORMAT,
                atlas.image_view,
                atlas.sampler,
            )
            .expect("Pipeline3D olusturulamadi");

            let cube_mesh = Mesh3D::create_cube(1.0, [1.0, 1.0, 1.0, 1.0]);
            let sphere_mesh = Mesh3D::create_sphere(0.6, 24, 24, [1.0, 1.0, 1.0, 1.0]);
            let plane_mesh = Mesh3D::create_plane(10.0, 10.0, [0.6, 0.6, 0.6, 1.0]);
            let cylinder_mesh = Mesh3D::create_cylinder(0.5, 1.5, 24, [1.0, 1.0, 1.0, 1.0]);
            let cone_mesh = Mesh3D::create_cone(0.6, 1.5, 24, [1.0, 1.0, 1.0, 1.0]);
            let pyramid_mesh = Mesh3D::create_pyramid(1.2, 1.5, [1.0, 1.0, 1.0, 1.0]);
            let capsule_mesh = Mesh3D::create_capsule(0.4, 1.0, 16, 8, [1.0, 1.0, 1.0, 1.0]);
            let sprite_3d_mesh = Mesh3D::create_sprite_3d(1.5, 1.5, [1.0, 1.0, 1.0, 1.0]);
            let grid_mesh = Mesh3D::create_grid_and_axes(30.0, 1.0);

            let gpu_cube =
                GpuMesh3D::from_mesh(&context.device, Arc::clone(&context.allocator), &cube_mesh)
                    .ok();
            let gpu_sphere = GpuMesh3D::from_mesh(
                &context.device,
                Arc::clone(&context.allocator),
                &sphere_mesh,
            )
            .ok();
            let gpu_plane =
                GpuMesh3D::from_mesh(&context.device, Arc::clone(&context.allocator), &plane_mesh)
                    .ok();
            let gpu_cylinder = GpuMesh3D::from_mesh(
                &context.device,
                Arc::clone(&context.allocator),
                &cylinder_mesh,
            )
            .ok();
            let gpu_cone =
                GpuMesh3D::from_mesh(&context.device, Arc::clone(&context.allocator), &cone_mesh)
                    .ok();
            let gpu_pyramid = GpuMesh3D::from_mesh(
                &context.device,
                Arc::clone(&context.allocator),
                &pyramid_mesh,
            )
            .ok();
            let gpu_capsule = GpuMesh3D::from_mesh(
                &context.device,
                Arc::clone(&context.allocator),
                &capsule_mesh,
            )
            .ok();
            let gpu_sprite_3d = GpuMesh3D::from_mesh(
                &context.device,
                Arc::clone(&context.allocator),
                &sprite_3d_mesh,
            )
            .ok();
            let gpu_grid =
                GpuMesh3D::from_mesh(&context.device, Arc::clone(&context.allocator), &grid_mesh)
                    .ok();

            let gpu_gltf_sample = {
                let sample_path = "assets/models/sample.glb";
                if let Ok(gltf_model) = rugeng_assets::GltfModel::from_file(sample_path) {
                    if let Some(sub) = gltf_model.submeshes.first() {
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
            self.atlas = Some(atlas);
            self.pipeline_2d = Some(pipeline_2d);
            self.pipeline_3d = Some(pipeline_3d);
            self.gpu_cube = gpu_cube;
            self.gpu_sphere = gpu_sphere;
            self.gpu_plane = gpu_plane;
            self.gpu_cylinder = gpu_cylinder;
            self.gpu_cone = gpu_cone;
            self.gpu_pyramid = gpu_pyramid;
            self.gpu_capsule = gpu_capsule;
            self.gpu_sprite_3d = gpu_sprite_3d;
            self.gpu_grid = gpu_grid;
            self.gpu_gltf_sample = gpu_gltf_sample;
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
                    let mut alloc = ctx.allocator.lock().unwrap();
                    if let Some(mut db) = self.depth_buffer.take() {
                        db.destroy(&ctx.device, &mut alloc);
                    }
                    drop(alloc);
                    if new_size.width > 0 && new_size.height > 0 {
                        self.depth_buffer = DepthBuffer::new(
                            &ctx.device,
                            Arc::clone(&ctx.allocator),
                            new_size.width,
                            new_size.height,
                        )
                        .ok();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let m = Vec2::new(position.x as f32, position.y as f32);
                if self.editor.workspace == WorkspaceMode::ThreeD
                    && (self.editor.camera_3d.is_orbiting || self.editor.camera_3d.is_flying)
                {
                    let delta = m - self.editor.camera_3d.last_mouse;
                    self.editor.camera_3d.yaw += delta.x * 0.003;
                    self.editor.camera_3d.pitch =
                        (self.editor.camera_3d.pitch - delta.y * 0.003).clamp(-1.55, 1.55);
                } else if self.editor.workspace == WorkspaceMode::ThreeD
                    && self.editor.camera_3d.is_panning
                {
                    let delta = m - self.editor.camera_3d.last_mouse;
                    let right = self.editor.camera_3d.right();
                    let up = self.editor.camera_3d.up();
                    self.editor.camera_3d.eye -= (right * delta.x - up * delta.y) * 0.015;
                } else if self.editor.camera.is_panning {
                    let delta = m - self.editor.camera.last_mouse;
                    self.editor.camera.pos -= delta / self.editor.camera.zoom;
                } else {
                    if let Some(win) = self.window.clone() {
                        let sz = win.inner_size();
                        let screen_w = sz.width as f32;
                        let screen_h = sz.height as f32;
                        let left_w = self.editor.dock_left_w;
                        let right_w = self.editor.dock_right_w;
                        let bottom_h = self.editor.dock_bottom_h;
                        let menu_h = 30.0;
                        let scene_tab_h = 26.0;
                        let vp_tool_h = 24.0;
                        let vp_canvas_y = menu_h + scene_tab_h + vp_tool_h;
                        let vp_x = left_w;
                        let vp_w = (screen_w - left_w - right_w).max(100.0);
                        let vp_canvas_h = (screen_h - bottom_h - vp_canvas_y).max(100.0);
                        let vp_center =
                            Vec2::new(left_w + vp_w * 0.5, vp_canvas_y + vp_canvas_h * 0.5);
                        let current_world_m =
                            (m - vp_center) / self.editor.camera.zoom + self.editor.camera.pos;

                        // Kutu Seçimi (Box Selection) Güncellemesi
                        if let Some(box_sel) = self.editor.box_selection.as_mut() {
                            box_sel.current_world = current_world_m;
                        }

                        // Sürekli Karo Boyama / Silme (Tilemap Drag Paint)
                        if self.editor.is_mouse_down
                            && (self.editor.active_tool == ViewportTool::TileBrush
                                || self.editor.active_tool == ViewportTool::TileEraser)
                            && m.x >= vp_x
                            && m.x <= vp_x + vp_w
                            && m.y >= vp_canvas_y
                            && m.y <= vp_canvas_y + vp_canvas_h
                        {
                            self.apply_tile_tool(current_world_m);
                        }

                        if self.editor.drag_mode != DragMode::None {
                            if let Some(e) = self.editor.selected_entity {
                                match self.editor.drag_mode {
                                    DragMode::Translate {
                                        start_entity_pos,
                                        start_world_mouse,
                                    } => {
                                        let delta = current_world_m - start_world_mouse;
                                        let mut actual_delta = delta;
                                        if let Some(t) = self.registry.get_mut::<Transform2D>(e) {
                                            let mut target_pos = start_entity_pos + delta;
                                            if self.editor.snap_to_grid {
                                                let snap = 16.0;
                                                target_pos.x = (target_pos.x / snap).round() * snap;
                                                target_pos.y = (target_pos.y / snap).round() * snap;
                                            }
                                            actual_delta = target_pos - t.pos;
                                            t.pos = target_pos;
                                        }

                                        // Ebeveyn taşındığında çocukları da senkronize taşı (Hiyerarşi)
                                        if let Some(children) =
                                            self.registry.get::<Children>(e).cloned()
                                        {
                                            for &child_e in &children.list {
                                                if let Some(c_tr) =
                                                    self.registry.get_mut::<Transform2D>(child_e)
                                                {
                                                    c_tr.pos += actual_delta;
                                                }
                                            }
                                        }
                                    }
                                    DragMode::Scale {
                                        start_size,
                                        start_world_mouse,
                                    } => {
                                        let delta = (current_world_m - start_world_mouse) * 2.0;
                                        if let Some(t) = self.registry.get_mut::<Transform2D>(e) {
                                            let mut target_sz =
                                                (start_size + delta).max(Vec2::splat(8.0));
                                            if self.editor.snap_to_grid {
                                                let snap = 8.0;
                                                target_sz.x = (target_sz.x / snap).round() * snap;
                                                target_sz.y = (target_sz.y / snap).round() * snap;
                                            }
                                            t.size = target_sz;
                                        }
                                    }
                                    DragMode::Rotate {
                                        center,
                                        start_rotation,
                                        start_angle,
                                    } => {
                                        let screen_pos = vp_center
                                            + (center - self.editor.camera.pos)
                                                * self.editor.camera.zoom;
                                        let current_angle =
                                            (m.y - screen_pos.y).atan2(m.x - screen_pos.x);
                                        let angle_diff = current_angle - start_angle;
                                        if let Some(t) = self.registry.get_mut::<Transform2D>(e) {
                                            t.rotation = start_rotation + angle_diff;
                                        }
                                    }
                                    DragMode::Translate3D {
                                        axis,
                                        start_translation,
                                        start_mouse,
                                    } => {
                                        let aspect = vp_w / vp_canvas_h.max(1.0);
                                        let render_cam =
                                            self.editor.camera_3d.to_render_camera(aspect);
                                        let vp_mat = render_cam.view_projection();
                                        let project_dir = |dir: glam::Vec3| -> Vec2 {
                                            let sc0 = vp_mat
                                                * glam::Vec4::new(
                                                    start_translation.x,
                                                    start_translation.y,
                                                    start_translation.z,
                                                    1.0,
                                                );
                                            let p1 = start_translation + dir;
                                            let sc1 =
                                                vp_mat * glam::Vec4::new(p1.x, p1.y, p1.z, 1.0);
                                            if sc0.w > 0.01 && sc1.w > 0.01 {
                                                let s0 = Vec2::new(
                                                    (sc0.x / sc0.w * 0.5 + 0.5) * vp_w,
                                                    (1.0 - (sc0.y / sc0.w * 0.5 + 0.5))
                                                        * vp_canvas_h,
                                                );
                                                let s1 = Vec2::new(
                                                    (sc1.x / sc1.w * 0.5 + 0.5) * vp_w,
                                                    (1.0 - (sc1.y / sc1.w * 0.5 + 0.5))
                                                        * vp_canvas_h,
                                                );
                                                (s1 - s0).normalize_or_zero()
                                            } else {
                                                Vec2::X
                                            }
                                        };

                                        let mouse_delta = m - start_mouse;
                                        if let Some(t3) = self.registry.get_mut::<Transform3D>(e) {
                                            match axis {
                                                0 => {
                                                    let s_dir = project_dir(glam::Vec3::X);
                                                    let amount = mouse_delta.dot(s_dir) * 0.03;
                                                    t3.translation =
                                                        start_translation + glam::Vec3::X * amount;
                                                }
                                                1 => {
                                                    let s_dir = project_dir(glam::Vec3::Y);
                                                    let amount = mouse_delta.dot(s_dir) * 0.03;
                                                    t3.translation =
                                                        start_translation + glam::Vec3::Y * amount;
                                                }
                                                2 => {
                                                    let s_dir = project_dir(glam::Vec3::Z);
                                                    let amount = mouse_delta.dot(s_dir) * 0.03;
                                                    t3.translation =
                                                        start_translation + glam::Vec3::Z * amount;
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    DragMode::Rotate3D {
                                        axis,
                                        start_rotation,
                                        start_mouse,
                                    } => {
                                        let mouse_delta = m - start_mouse;
                                        let angle_delta = (mouse_delta.x - mouse_delta.y) * 0.02;
                                        if let Some(t3) = self.registry.get_mut::<Transform3D>(e) {
                                            let rot_axis = match axis {
                                                0 => glam::Vec3::X,
                                                1 => glam::Vec3::Y,
                                                _ => glam::Vec3::Z,
                                            };
                                            let delta_quat =
                                                glam::Quat::from_axis_angle(rot_axis, angle_delta);
                                            t3.rotation = delta_quat * start_rotation;
                                        }
                                    }
                                    DragMode::Scale3D {
                                        axis,
                                        start_scale,
                                        start_mouse,
                                    } => {
                                        let mouse_delta = m - start_mouse;
                                        let factor = 1.0 + (mouse_delta.x - mouse_delta.y) * 0.015;
                                        let safe_factor = factor.max(0.1);
                                        if let Some(t3) = self.registry.get_mut::<Transform3D>(e) {
                                            match axis {
                                                0 => {
                                                    t3.scale = glam::Vec3::new(
                                                        start_scale.x * safe_factor,
                                                        start_scale.y,
                                                        start_scale.z,
                                                    )
                                                }
                                                1 => {
                                                    t3.scale = glam::Vec3::new(
                                                        start_scale.x,
                                                        start_scale.y * safe_factor,
                                                        start_scale.z,
                                                    )
                                                }
                                                2 => {
                                                    t3.scale = glam::Vec3::new(
                                                        start_scale.x,
                                                        start_scale.y,
                                                        start_scale.z * safe_factor,
                                                    )
                                                }
                                                _ => t3.scale = start_scale * safe_factor,
                                            }
                                        }
                                    }
                                    DragMode::None => {}
                                    DragMode::SplitterLeft { .. }
                                    | DragMode::SplitterRight { .. }
                                    | DragMode::SplitterBottom { .. } => {}
                                }
                            }
                        }

                        // Panel Ayırıcı (Splitter) Sürükleme Mantığı (Varlık seçimi gerektirmez)
                        match self.editor.drag_mode {
                            DragMode::SplitterLeft {
                                start_left_w,
                                start_mouse_x,
                            } => {
                                let delta_x = m.x - start_mouse_x;
                                let new_w = (start_left_w + delta_x)
                                    .clamp(160.0, (screen_w * 0.45).max(160.0));
                                self.editor.dock_left_w = new_w;
                            }
                            DragMode::SplitterRight {
                                start_right_w,
                                start_mouse_x,
                            } => {
                                let delta_x = start_mouse_x - m.x; // Sola sürüklemek sağ paneli genişletir
                                let new_w = (start_right_w + delta_x)
                                    .clamp(180.0, (screen_w * 0.45).max(180.0));
                                self.editor.dock_right_w = new_w;
                            }
                            DragMode::SplitterBottom {
                                start_bottom_h,
                                start_mouse_y,
                            } => {
                                let delta_y = start_mouse_y - m.y; // Yukarı sürüklemek alt paneli genişletir
                                let new_h = (start_bottom_h + delta_y)
                                    .clamp(80.0, (screen_h * 0.5).max(80.0));
                                self.editor.dock_bottom_h = new_h;
                            }
                            _ => {}
                        }
                    }
                }
                self.editor.camera.last_mouse = m;
                self.editor.camera_3d.last_mouse = m;
                self.editor.mouse_pos = m;
            }
            WindowEvent::MouseInput { button, state, .. } => {
                let pressed = state == ElementState::Pressed;
                if button == MouseButton::Right {
                    if pressed {
                        self.editor.right_click_down = true;
                        self.editor.right_click_start = Some(self.editor.mouse_pos);
                        if self.editor.workspace == WorkspaceMode::ThreeD {
                            self.editor.camera_3d.is_flying = true;
                            self.editor.camera_3d.is_orbiting = true;
                        } else {
                            self.editor.camera.is_panning = true;
                        }
                    } else {
                        // Sağ tık bırakıldı
                        let was_down = self.editor.right_click_down;
                        self.editor.right_click_down = false;
                        if self.editor.workspace == WorkspaceMode::ThreeD {
                            self.editor.camera_3d.is_flying = false;
                            self.editor.camera_3d.is_orbiting = false;
                        } else {
                            self.editor.camera.is_panning = false;
                        }

                        let click_start = self.editor.right_click_start.take();
                        if was_down {
                            if let Some(start_pos) = click_start {
                                let delta = (self.editor.mouse_pos - start_pos).length();
                                // Eğer fare neredeyse hiç hareket etmediyse (< 6.0 px), bu bir sağ tıklamadır (Pan/Fly değil)!
                                if delta < 6.0 {
                                    if let Some(win) = self.window.clone() {
                                        let sz = win.inner_size();
                                        let screen_w = sz.width as f32;
                                        let screen_h = sz.height as f32;
                                        let layout = crate::ui::DockLayout::with_custom_sizes(
                                            screen_w,
                                            screen_h,
                                            self.editor.dock_left_w,
                                            self.editor.dock_right_w,
                                            self.editor.dock_bottom_h,
                                        );
                                        let vp_x = layout.vp_x;
                                        let vp_w = layout.vp_w;
                                        let vp_canvas_y = layout.vp_canvas_y;
                                        let vp_canvas_h = layout.vp_canvas_h;
                                        let mouse = self.editor.mouse_pos;

                                        // Viewport tuval sınırları içindeyse
                                        if mouse.x >= vp_x
                                            && mouse.x <= vp_x + vp_w
                                            && mouse.y >= vp_canvas_y
                                            && mouse.y <= vp_canvas_y + vp_canvas_h
                                        {
                                            // Tıklanan yerde varlık var mı kontrol et
                                            let mut target_ent = None;
                                            if self.editor.workspace == WorkspaceMode::ThreeD {
                                                let aspect = vp_w / vp_canvas_h.max(1.0);
                                                let render_cam =
                                                    self.editor.camera_3d.to_render_camera(aspect);
                                                let vp_mat = render_cam.view_projection();
                                                let ndc_x = ((mouse.x - vp_x) / vp_w) * 2.0 - 1.0;
                                                let ndc_y = 1.0
                                                    - ((mouse.y - vp_canvas_y) / vp_canvas_h) * 2.0;
                                                let inv_vp = vp_mat.inverse();
                                                let p_near_h = inv_vp
                                                    * glam::Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
                                                let p_far_h = inv_vp
                                                    * glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

                                                if p_near_h.w.abs() > 1e-6 && p_far_h.w.abs() > 1e-6
                                                {
                                                    let p_near = p_near_h.truncate() / p_near_h.w;
                                                    let p_far = p_far_h.truncate() / p_far_h.w;
                                                    let ray_dir = (p_far - p_near).normalize();
                                                    let ray_3d = rugeng_physics::Ray3D::new(
                                                        p_near, ray_dir, 1000.0,
                                                    );
                                                    let mut min_ray_dist = f32::INFINITY;

                                                    let entities_3d: Vec<(
                                                        Entity,
                                                        Transform3D,
                                                        Option<Mesh3DComponent>,
                                                    )> = self
                                                        .registry
                                                        .entities::<Transform3D>()
                                                        .into_iter()
                                                        .filter_map(|e| {
                                                            let t3 = self
                                                                .registry
                                                                .get::<Transform3D>(e)
                                                                .copied()?;
                                                            let m3 = self
                                                                .registry
                                                                .get::<Mesh3DComponent>(e)
                                                                .cloned();
                                                            Some((e, t3, m3))
                                                        })
                                                        .collect();

                                                    for (ent, t3, m3) in &entities_3d {
                                                        let mut entity_hit_dist = None;
                                                        if let Some(mesh_comp) = m3 {
                                                            if mesh_comp.mesh_name == "sphere" {
                                                                let radius = (t3
                                                                    .scale
                                                                    .x
                                                                    .max(t3.scale.y)
                                                                    .max(t3.scale.z)
                                                                    * 0.5)
                                                                    .max(0.2);
                                                                let sphere = rugeng_physics::BoundingSphere::new(t3.translation, radius);
                                                                if let Some(hit) = rugeng_physics::ray_intersect_sphere(&ray_3d, &sphere) {
                                                                    entity_hit_dist = Some(hit.dist);
                                                                }
                                                            } else {
                                                                let half_ext = (t3.scale * 0.5)
                                                                    .max(glam::Vec3::splat(0.2));
                                                                let aabb = rugeng_physics::AABB3D::from_center_half_extents(t3.translation, half_ext);
                                                                if let Some(hit) = rugeng_physics::ray_intersect_aabb_3d(&ray_3d, &aabb) {
                                                                    entity_hit_dist = Some(hit.dist);
                                                                }
                                                            }
                                                        } else {
                                                            let half_ext = (t3.scale * 0.5)
                                                                .max(glam::Vec3::splat(0.2));
                                                            let aabb = rugeng_physics::AABB3D::from_center_half_extents(t3.translation, half_ext);
                                                            if let Some(hit) = rugeng_physics::ray_intersect_aabb_3d(&ray_3d, &aabb) {
                                                                    entity_hit_dist = Some(hit.dist);
                                                                }
                                                        }

                                                        if let Some(dist) = entity_hit_dist {
                                                            if dist < min_ray_dist {
                                                                min_ray_dist = dist;
                                                                target_ent = Some(*ent);
                                                            }
                                                        }
                                                    }
                                                }

                                                let mut closest_d = 40.0;
                                                for ent in self.registry.entities::<Transform3D>() {
                                                    if target_ent.is_some() {
                                                        break;
                                                    }
                                                    if let Some(t3) =
                                                        self.registry.get::<Transform3D>(ent)
                                                    {
                                                        let clip = vp_mat
                                                            * glam::Vec4::new(
                                                                t3.translation.x,
                                                                t3.translation.y,
                                                                t3.translation.z,
                                                                1.0,
                                                            );
                                                        if clip.w > 0.05 {
                                                            let ndc_x = clip.x / clip.w;
                                                            let ndc_y = clip.y / clip.w;
                                                            let sx =
                                                                vp_x + (ndc_x * 0.5 + 0.5) * vp_w;
                                                            let sy = vp_canvas_y
                                                                + (1.0 - (ndc_y * 0.5 + 0.5))
                                                                    * vp_canvas_h;
                                                            let d = (mouse - Vec2::new(sx, sy))
                                                                .length();
                                                            if d < closest_d {
                                                                closest_d = d;
                                                                target_ent = Some(ent);
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                let cam = &self.editor.camera;
                                                let vp_center = Vec2::new(
                                                    vp_x + vp_w * 0.5,
                                                    vp_canvas_y + vp_canvas_h * 0.5,
                                                );
                                                let world_m =
                                                    (mouse - vp_center) / cam.zoom + cam.pos;
                                                self.registry.view1_ref::<Transform2D, _>(
                                                    |e, tr| {
                                                        let half = tr.size * 0.5;
                                                        if world_m.x >= tr.pos.x - half.x
                                                            && world_m.x <= tr.pos.x + half.x
                                                            && world_m.y >= tr.pos.y - half.y
                                                            && world_m.y <= tr.pos.y + half.y
                                                        {
                                                            target_ent = Some(e);
                                                        }
                                                    },
                                                );
                                            }

                                            // Context Menu öğelerini oluştur
                                            let items = if let Some(e) = target_ent {
                                                self.editor.selected_entity = Some(e);
                                                vec![
                                                    ContextMenuItem {
                                                        label: "[D] Varligi Cogalt".to_string(),
                                                        shortcut: "Ctrl+D".to_string(),
                                                        action_id: "duplicate".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[P] Prefab Olarak Kaydet"
                                                            .to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "save_prefab".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[X] Varligi Sil".to_string(),
                                                        shortcut: "Del".to_string(),
                                                        action_id: "delete".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[-] Ayirici".to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[Z+] Z-Index Artir (+1)"
                                                            .to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "z_up".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[Z-] Z-Index Azalt (-1)"
                                                            .to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "z_down".to_string(),
                                                    },
                                                ]
                                            } else if self.editor.workspace == WorkspaceMode::ThreeD
                                            {
                                                vec![
                                                    ContextMenuItem {
                                                        label: "[3] 3D Kup (Cube)".to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_cube_3d".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[3] 3D Kure (Sphere)".to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_sphere_3d".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[3] 3D Duzlem (Plane)".to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_plane_3d".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[3] 3D Silindir (Cylinder)"
                                                            .to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_cylinder_3d".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[3] 3D Koni (Cone)".to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_cone_3d".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[3] 3D Piramit (Pyramid)"
                                                            .to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_pyramid_3d".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[3] 3D Kapsul (Capsule)"
                                                            .to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_capsule_3d".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[3] 3D Sprite (2.5D)".to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_sprite_3d".to_string(),
                                                    },
                                                ]
                                            } else {
                                                vec![
                                                    ContextMenuItem {
                                                        label: "[+] 2D Sprite (Gemi)".to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_ship".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[+] 2D Dusman (Bug)".to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_enemy".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[+] 2D Can / Kalp".to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_heart".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[+] 2D Enerji Kuresi".to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_orb".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[+] 2D Karo Haritasi".to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_tilemap".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[+] 2D Metin (Text2D)".to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_text".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[-] Ayirici".to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[3] 3D Kup (Cube)".to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_cube_3d".to_string(),
                                                    },
                                                    ContextMenuItem {
                                                        label: "[3] 3D Kure (Sphere)".to_string(),
                                                        shortcut: "".to_string(),
                                                        action_id: "add_sphere_3d".to_string(),
                                                    },
                                                ]
                                            };

                                            self.editor.context_menu = Some(ContextMenu {
                                                screen_pos: mouse,
                                                target_entity: target_ent,
                                                items,
                                            });
                                            self.audio.play("sfx_blip");
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if button == MouseButton::Middle {
                    if self.editor.workspace == WorkspaceMode::ThreeD {
                        self.editor.camera_3d.is_panning = pressed;
                    } else {
                        self.editor.camera.is_panning = pressed;
                    }
                } else if button == MouseButton::Left {
                    self.editor.is_mouse_down = pressed;
                    if pressed {
                        let w_opt = self.window.clone();
                        if let Some(win) = w_opt {
                            let sz = win.inner_size();
                            let had_rename = self.editor.renaming.is_some();
                            let dc = self.is_double_click();
                            self.handle_mouse_click(
                                self.editor.mouse_pos,
                                sz.width as f32,
                                sz.height as f32,
                                dc,
                            );
                            // Çift tıkla rename başladıysa IME'yi aç (Türkçe giriş)
                            if !had_rename && self.editor.renaming.is_some() {
                                if let Some(w) = self.window.as_ref() {
                                    w.set_ime_allowed(true);
                                }
                            }
                        }
                    } else {
                        // Sürükleme işlemi bittiğinde konum değiştiyse UndoAction::MoveEntity kaydet
                        if let (Some(e), Some(old_tr)) = (
                            self.editor.selected_entity,
                            self.editor.drag_start_transform.take(),
                        ) {
                            if let Some(new_tr) = self.registry.get::<Transform2D>(e).copied() {
                                if (new_tr.pos - old_tr.pos).length_squared() > 1e-4
                                    || (new_tr.size - old_tr.size).length_squared() > 1e-4
                                    || (new_tr.rotation - old_tr.rotation).abs() > 1e-4
                                {
                                    self.editor.undo_stack.push(UndoAction::MoveEntity {
                                        entity: e,
                                        old_transform: old_tr,
                                        new_transform: new_tr,
                                    });
                                    self.editor.redo_stack.clear();
                                    self.editor.is_scene_dirty = true;
                                }
                            }
                        }

                        // Kutu Seçimi Tamamlama (Bırakıldığında içindeki varlıkları seç)
                        if let Some(box_sel) = self.editor.box_selection.take() {
                            let min_x = box_sel.start_world.x.min(box_sel.current_world.x);
                            let max_x = box_sel.start_world.x.max(box_sel.current_world.x);
                            let min_y = box_sel.start_world.y.min(box_sel.current_world.y);
                            let max_y = box_sel.start_world.y.max(box_sel.current_world.y);

                            let mut in_box = Vec::new();
                            self.registry.view1_ref::<Transform2D, _>(|e, tr| {
                                if tr.pos.x >= min_x
                                    && tr.pos.x <= max_x
                                    && tr.pos.y >= min_y
                                    && tr.pos.y <= max_y
                                {
                                    in_box.push(e);
                                }
                            });
                            if let Some(&first) = in_box.first() {
                                self.editor.selected_entity = Some(first);
                                self.editor.logs.push(LogMessage {
                                    level: "INFO",
                                    text: format!("Kutu Secimi: {} varlik secildi.", in_box.len()),
                                });
                            }
                            self.editor.selected_entities = in_box;
                        }

                        self.editor.drag_mode = DragMode::None;
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => (pos.y as f32) * 0.05,
                };
                if self.editor.workspace == WorkspaceMode::ThreeD {
                    self.editor.camera_3d.dolly(y * 0.8);
                } else if y > 0.0 {
                    self.editor.camera.zoom = (self.editor.camera.zoom * 1.15).min(5.0);
                } else if y < 0.0 {
                    self.editor.camera.zoom = (self.editor.camera.zoom / 1.15).max(0.2);
                }
            }
            WindowEvent::Ime(winit::event::Ime::Commit(txt)) => {
                // IBus/IME ile Türkçe karakter girişi (rename modunda).
                if let Some(r) = self.editor.renaming.as_mut() {
                    for ch in txt.chars() {
                        r.insert(ch);
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        text,
                        ..
                    },
                ..
            } => {
                let pressed = state == ElementState::Pressed;

                // --- Satır içi yeniden adlandırma modu: tüm kısayolları bloklar ---
                if self.editor.renaming.is_some() {
                    if pressed {
                        match key {
                            KeyCode::Enter => {
                                self.commit_rename();
                                if let Some(w) = self.window.as_ref() {
                                    w.set_ime_allowed(false);
                                }
                            }
                            KeyCode::Escape => {
                                self.cancel_rename();
                                if let Some(w) = self.window.as_ref() {
                                    w.set_ime_allowed(false);
                                }
                            }
                            KeyCode::Backspace => {
                                if let Some(r) = self.editor.renaming.as_mut() {
                                    r.backspace();
                                }
                            }
                            KeyCode::Delete => {
                                if let Some(r) = self.editor.renaming.as_mut() {
                                    r.delete_forward();
                                }
                            }
                            KeyCode::ArrowLeft => {
                                if let Some(r) = self.editor.renaming.as_mut() {
                                    r.move_caret(-1);
                                }
                            }
                            KeyCode::ArrowRight => {
                                if let Some(r) = self.editor.renaming.as_mut() {
                                    r.move_caret(1);
                                }
                            }
                            KeyCode::Home => {
                                if let Some(r) = self.editor.renaming.as_mut() {
                                    r.home();
                                }
                            }
                            KeyCode::End => {
                                if let Some(r) = self.editor.renaming.as_mut() {
                                    r.end();
                                }
                            }
                            _ => {
                                // X11/yol-üstü tuşlama: yazılabilir karakter akışı
                                if let Some(txt) = text.as_ref() {
                                    for ch in txt.chars() {
                                        if let Some(r) = self.editor.renaming.as_mut() {
                                            r.insert(ch);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    return;
                }

                let scancode = match key {
                    KeyCode::KeyA => 0x1E,
                    KeyCode::KeyD => 0x20,
                    KeyCode::KeyW => 0x11,
                    KeyCode::KeyS => 0x1F,
                    KeyCode::KeyE => 0x12, // 3D Dikey Yukarı
                    KeyCode::KeyQ => 0x10, // 3D Dikey Aşağı
                    KeyCode::ShiftLeft | KeyCode::ShiftRight => 0x2A, // Turbo Hız
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

                // Control tuşu durumunu takip et
                if matches!(key, KeyCode::ControlLeft | KeyCode::ControlRight) {
                    self.editor.is_ctrl_pressed = pressed;
                }

                // 3D Modunda Shift tuşuna basılı tutulduğunda turbo hızı ayarla
                if matches!(key, KeyCode::ShiftLeft | KeyCode::ShiftRight) {
                    self.editor.camera_3d.is_turbo = pressed;
                }

                if !pressed {
                    return;
                }

                // 3D modunda ve sağ tık (flycam) aktifken Q/E/W/R araç kısayollarını tetikleme
                if self.editor.workspace == WorkspaceMode::ThreeD && self.editor.camera_3d.is_flying
                {
                    return;
                }

                // Ctrl kısayolları öncelikli
                if self.editor.is_ctrl_pressed {
                    match key {
                        KeyCode::KeyD | KeyCode::KeyC => {
                            self.duplicate_selected_entity();
                            return;
                        }
                        KeyCode::KeyZ => {
                            self.undo();
                            return;
                        }
                        KeyCode::KeyY => {
                            self.redo();
                            return;
                        }
                        KeyCode::KeyT => {
                            self.new_scene();
                            return;
                        }
                        KeyCode::KeyK => {
                            let path = self.editor.current_scene_path.clone();
                            self.save_current_scene(&path);
                            return;
                        }
                        KeyCode::KeyL => {
                            let path = self.editor.current_scene_path.clone();
                            self.load_scene_from_file(&path);
                            return;
                        }
                        _ => {}
                    }
                }

                match key {
                    KeyCode::KeyQ => {
                        self.editor.active_tool = ViewportTool::Select;
                        self.editor.logs.push(LogMessage {
                            level: "INFO",
                            text: "Arac: Secim (Q)".to_string(),
                        });
                    }
                    KeyCode::KeyW => {
                        self.editor.active_tool = ViewportTool::Move;
                        self.editor.logs.push(LogMessage {
                            level: "INFO",
                            text: "Arac: Tasima (W)".to_string(),
                        });
                    }
                    KeyCode::KeyE => {
                        self.editor.active_tool = ViewportTool::Rotate;
                        self.editor.logs.push(LogMessage {
                            level: "INFO",
                            text: "Arac: Dondurme (E)".to_string(),
                        });
                    }
                    KeyCode::KeyR => {
                        self.editor.active_tool = ViewportTool::Scale;
                        self.editor.logs.push(LogMessage {
                            level: "INFO",
                            text: "Arac: Boyutlandirma (R)".to_string(),
                        });
                    }
                    KeyCode::KeyF => {
                        if let Some(e) = self.editor.selected_entity {
                            if self.editor.workspace == WorkspaceMode::ThreeD {
                                if let Some(tr3) = self.registry.get::<Transform3D>(e) {
                                    self.editor.camera_3d.eye =
                                        tr3.translation + glam::Vec3::new(0.0, 2.0, 5.0);
                                    self.editor.camera_3d.target = tr3.translation;
                                    self.editor.logs.push(LogMessage {
                                        level: "INFO",
                                        text: format!(
                                            "3D Kamera Dugum #{}:{} uzerine odaklandi (F).",
                                            e.index(),
                                            e.generation()
                                        ),
                                    });
                                }
                            } else if let Some(tr) = self.registry.get::<Transform2D>(e) {
                                self.editor.camera.pos = tr.pos;
                                self.editor.logs.push(LogMessage {
                                    level: "INFO",
                                    text: format!(
                                        "Kamera Dugum #{}:{} uzerine odaklandi (F).",
                                        e.index(),
                                        e.generation()
                                    ),
                                });
                            }
                        }
                    }
                    KeyCode::KeyS => {
                        self.editor.snap_to_grid = !self.editor.snap_to_grid;
                        let msg = if self.editor.snap_to_grid {
                            "Izgara Yapismasi (Snap) Acildi (16px)."
                        } else {
                            "Izgara Yapismasi (Snap) Kapatildi."
                        };
                        self.editor.logs.push(LogMessage {
                            level: "INFO",
                            text: msg.to_string(),
                        });
                    }
                    KeyCode::KeyZ => self.undo(),
                    KeyCode::KeyY => self.redo(),
                    KeyCode::KeyT => self.new_scene(),
                    KeyCode::KeyD | KeyCode::KeyC => self.duplicate_selected_entity(),
                    KeyCode::KeyK => {
                        let path = self.editor.current_scene_path.clone();
                        self.save_current_scene(&path);
                    }
                    KeyCode::KeyL => {
                        let path = self.editor.current_scene_path.clone();
                        self.load_scene_from_file(&path);
                    }
                    KeyCode::KeyN => self.spawn_preset_entity("ship"),
                    KeyCode::KeyX => self.trigger_shake_and_explosion(),
                    KeyCode::F2 => {
                        self.start_rename();
                        if let Some(w) = self.window.as_ref() {
                            w.set_ime_allowed(true);
                        }
                    }
                    KeyCode::Delete | KeyCode::Backspace => self.delete_selected_entity(),
                    KeyCode::Escape => event_loop.exit(),
                    _ => {}
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

                self.update_simulation(frame_delta);

                self.fps_counter += 1;
                if self.fps_timer.elapsed().as_secs_f32() >= 1.0 {
                    self.fps = self.fps_counter;
                    println!(
                        "[Godot Workbench] Kare: {:.2} ms | Varlik: {} | FPS: {}",
                        self.frame_time_ms,
                        self.registry.len(),
                        self.fps
                    );
                    self.fps_counter = 0;
                    self.fps_timer = Instant::now();
                }

                self.frame_count += 1;
                if std::env::var("AUTO_TEST").is_ok() && self.frame_count >= 120 {
                    println!(
                        "[AUTO_TEST] Godot Workbench testi basariyla tamamlandi. Kare: {}",
                        self.frame_count
                    );
                    event_loop.exit();
                    std::process::exit(0);
                }

                self.render_ui_and_scene(size.width as f32, size.height as f32);
                window.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.clone() {
            window.request_redraw();
        }
    }
}

fn main() {
    println!("============================================================");
    println!(" Project Ember — Godot 4 Tasarımlı Motor Çalışma Alanı");
    println!(" Üst Menü, Sol Sahne & Dosya Ağacı, Tuval ve Tam Boy Denetçi Aktif.");
    println!("============================================================");
    println!("Kontroller:");
    println!("- Viewport Araçları: Q (Seç), W (Taşı), E (Döndür), R (Boyutlandır)");
    println!("- Nesne Manipülasyonu: Fare sol tuşla tut-sürükle (taşı), 4 mavi köşe kutusuyla boyutlandır, sarı yön ucuyla döndür");
    println!("- Kamera Gezinme: Fare sağ / orta tuşla sürükleyerek kaydır (pan), tekerlek ile yakınlaş/uzaklaş (zoom)");
    println!(
        "- Üst Menü: Mod Değiştirici (2D/3D/Script/Game) ve Oynatma Butonları ([>], [||], [#])"
    );
    println!("- Sol Üst: Sahne Ağacı [Scene/Import], [2D Scene] kök düğüm ekleme");
    println!("- Sol Alt: Dosya Sistemi [FileSystem/History], res:// dosya tarayıcısı");
    println!("- Sağ Alan: Tam boy [Inspector/Signals] denetçi (X/Y, Rotation R-/R+, Scale W-/W+)");
    println!("- Kısayollar: N (Yeni Düğüm), Delete (Sil), ESC (Çıkış)");

    let event_loop = EventLoop::new().expect("EventLoop olusturulamadi");
    let mut app = App::new();
    let _ = event_loop.run_app(&mut app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rugeng_core::{NodeName, TileData};
    use rugeng_render::font;

    #[test]
    fn test_editor_tilemap_coordinates_world_to_grid() {
        let map = Tilemap2DComponent::new(32.0);
        assert_eq!(map.world_to_grid(Vec2::new(10.0, 20.0)), (0, 0));
        assert_eq!(map.world_to_grid(Vec2::new(32.0, 64.0)), (1, 2));
        assert_eq!(map.world_to_grid(Vec2::new(63.9, 95.9)), (1, 2));
        assert_eq!(map.world_to_grid(Vec2::new(-10.0, -20.0)), (-1, -1));
        assert_eq!(map.world_to_grid(Vec2::new(-32.0, -32.0)), (-1, -1));
    }

    #[test]
    fn test_editor_tilemap_brush_palette_and_placement() {
        let mut map = Tilemap2DComponent::new(32.0);
        let palette = [
            (TileType::Grass, false),
            (TileType::Stone, true),
            (TileType::Metal, true),
            (TileType::Lava, false),
            (TileType::Water, false),
        ];

        for (idx, (tt, expected_solid)) in palette.iter().enumerate() {
            let gx = idx as i32;
            let gy = 0;
            map.set_tile(gx, gy, *tt, tt.is_default_solid());
            let tile = map.get_tile(gx, gy).expect("Karo bulunamadi");
            assert_eq!(tile.tile_type, *tt);
            assert_eq!(tile.solid, *expected_solid);
            assert_eq!(tile.color, tt.default_color());
        }

        assert_eq!(map.count(), 5);
    }

    #[test]
    fn test_editor_tilemap_undo_redo() {
        let mut registry = Registry::new();
        registry.register_component::<Transform2D>();
        registry.register_component::<Tilemap2DComponent>();

        let e = registry.create();
        registry.emplace(
            e,
            Transform2D {
                pos: Vec2::ZERO,
                size: Vec2::splat(32.0),
                rotation: 0.0,
            },
        );
        registry.emplace(e, Tilemap2DComponent::new(32.0));

        let mut undo_stack: Vec<UndoAction> = Vec::new();
        let mut redo_stack: Vec<UndoAction> = Vec::new();

        // 1. Fırça ile (2, 3) konumuna Taş Karo yerleştir
        let gx = 2;
        let gy = 3;
        let old_tile = registry
            .get::<Tilemap2DComponent>(e)
            .unwrap()
            .get_tile(gx, gy)
            .copied();
        let new_tile_data = TileData {
            tile_type: TileType::Stone,
            color: TileType::Stone.default_color(),
            solid: true,
            one_way: false,
        };
        let new_tile = Some(new_tile_data);

        registry
            .get_mut::<Tilemap2DComponent>(e)
            .unwrap()
            .set_tile_ex(
                gx,
                gy,
                new_tile_data.tile_type,
                new_tile_data.solid,
                new_tile_data.one_way,
            );
        undo_stack.push(UndoAction::TileChange {
            entity: e,
            gx,
            gy,
            old_tile,
            new_tile,
        });

        assert_eq!(registry.get::<Tilemap2DComponent>(e).unwrap().count(), 1);

        // 2. Undo: Karo silinmeli
        if let Some(UndoAction::TileChange {
            entity,
            gx,
            gy,
            old_tile,
            new_tile,
        }) = undo_stack.pop()
        {
            let map = registry.get_mut::<Tilemap2DComponent>(entity).unwrap();
            match old_tile {
                Some(t) => map.set_tile_ex(gx, gy, t.tile_type, t.solid, t.one_way),
                None => {
                    map.remove_tile(gx, gy);
                }
            }
            redo_stack.push(UndoAction::TileChange {
                entity,
                gx,
                gy,
                old_tile,
                new_tile,
            });
        }

        assert_eq!(registry.get::<Tilemap2DComponent>(e).unwrap().count(), 0);

        // 3. Redo: Karo tekrar yerleştirilmeli
        if let Some(UndoAction::TileChange {
            entity,
            gx,
            gy,
            old_tile,
            new_tile,
        }) = redo_stack.pop()
        {
            let map = registry.get_mut::<Tilemap2DComponent>(entity).unwrap();
            match new_tile {
                Some(t) => map.set_tile_ex(gx, gy, t.tile_type, t.solid, t.one_way),
                None => {
                    map.remove_tile(gx, gy);
                }
            }
            undo_stack.push(UndoAction::TileChange {
                entity,
                gx,
                gy,
                old_tile,
                new_tile,
            });
        }

        assert_eq!(registry.get::<Tilemap2DComponent>(e).unwrap().count(), 1);
        let restored = registry
            .get::<Tilemap2DComponent>(e)
            .unwrap()
            .get_tile(gx, gy)
            .copied()
            .unwrap();
        assert_eq!(restored.tile_type, TileType::Stone);
    }

    #[test]
    fn test_editor_tilemap_eraser_and_clear() {
        let mut map = Tilemap2DComponent::new(32.0);
        map.set_tile(0, 0, TileType::Grass, false);
        map.set_tile(1, 1, TileType::Stone, true);
        assert_eq!(map.count(), 2);

        // Silgi ile (0, 0) kaldır
        let removed = map.remove_tile(0, 0);
        assert!(removed.is_some());
        assert_eq!(map.count(), 1);
        assert!(map.get_tile(0, 0).is_none());
        assert!(map.get_tile(1, 1).is_some());

        // Hepsini temizle
        map.clear();
        assert_eq!(map.count(), 0);
    }

    #[test]
    fn test_editor_tilemap_selection_prioritization() {
        let mut registry = Registry::new();
        registry.register_component::<Transform2D>();
        registry.register_component::<Tilemap2DComponent>();

        let tm1 = registry.create();
        registry.emplace(
            tm1,
            Transform2D {
                pos: Vec2::ZERO,
                size: Vec2::splat(32.0),
                rotation: 0.0,
            },
        );
        registry.emplace(tm1, Tilemap2DComponent::new(32.0));

        let tm2 = registry.create();
        registry.emplace(
            tm2,
            Transform2D {
                pos: Vec2::ZERO,
                size: Vec2::splat(16.0),
                rotation: 0.0,
            },
        );
        registry.emplace(tm2, Tilemap2DComponent::new(16.0));

        // tm2 seçiliyken get_or_create_tilemap_entity tm2'yi seçmeli
        let selected_entity = Some(tm2);
        let target = if let Some(sel) = selected_entity {
            if registry.has::<Tilemap2DComponent>(sel) {
                sel
            } else {
                tm1
            }
        } else {
            tm1
        };
        assert_eq!(target, tm2);
        assert_eq!(
            registry
                .get::<Tilemap2DComponent>(target)
                .unwrap()
                .tile_size,
            16.0
        );
    }

    #[test]
    fn test_editor_theme_palette() {
        let theme = EditorTheme::dark();
        assert_eq!(theme.bg_canvas, [35, 35, 35, 255]);
        assert_eq!(theme.bg_panel, [40, 40, 40, 255]);
        assert_eq!(theme.bg_panel_alt, [32, 32, 32, 255]);
        assert_eq!(theme.tab_active, [48, 48, 48, 255]);
        assert_eq!(theme.tab_inactive, [32, 32, 32, 255]);
        assert_eq!(theme.border, [26, 26, 26, 255]);
        assert_eq!(theme.accent, [74, 158, 255, 255]);
        assert_eq!(theme.text_primary, [238, 238, 238, 255]);
        assert_eq!(theme.text_secondary, [168, 168, 168, 255]);
    }

    #[test]
    fn test_procedural_icons_generation() {
        let mut instances = Vec::new();
        let color = [255, 255, 255, 255];

        draw_icon_play(&mut instances, 0.0, 0.0, 12.0, color);
        assert!(!instances.is_empty(), "Play ikonu instance üretmeli");
        let play_count = instances.len();

        draw_icon_pause(&mut instances, 0.0, 0.0, 12.0, color);
        assert!(
            instances.len() > play_count,
            "Pause ikonu instance üretmeli"
        );
        let pause_count = instances.len();

        draw_icon_stop(&mut instances, 0.0, 0.0, 12.0, color);
        assert!(
            instances.len() > pause_count,
            "Stop ikonu instance üretmeli"
        );
        let stop_count = instances.len();

        draw_icon_step(&mut instances, 0.0, 0.0, 12.0, color);
        assert!(instances.len() > stop_count, "Step ikonu instance üretmeli");
        let step_count = instances.len();

        draw_icon_select(&mut instances, 0.0, 0.0, color);
        draw_icon_move(&mut instances, 0.0, 0.0, color);
        draw_icon_rotate(&mut instances, 0.0, 0.0, color);
        draw_icon_scale(&mut instances, 0.0, 0.0, color);
        draw_icon_brush(&mut instances, 0.0, 0.0, color);
        draw_icon_eraser(&mut instances, 0.0, 0.0, color);

        assert!(
            instances.len() > step_count + 15,
            "Tüm UI ikonları başarıyla çizilmeli"
        );
    }

    #[test]
    fn test_simulation_step_and_stop_rollback() {
        let mut app = App::new();
        let e = app.registry.create();
        let initial_pos = Vec2::new(10.0, 20.0);
        app.registry.emplace(
            e,
            Transform2D {
                pos: initial_pos,
                size: Vec2::splat(32.0),
                rotation: 0.0,
            },
        );
        app.registry.emplace(
            e,
            Velocity2D {
                vel: Vec2::new(60.0, 0.0),
            },
        );

        // Simülasyonu başlat (Snapshot otomatik alınır)
        app.start_simulation();

        // 1 kare (1/60 saniye) adım ilerlet
        app.step_simulation_single_frame(1.0 / 60.0);

        let new_pos = app.registry.get::<Transform2D>(e).unwrap().pos;
        assert_eq!(new_pos.x, 11.0, "1 kare sonra x konumu 10.0 -> 11.0 olmalı");

        // Stop: Snapshot'a geri dön
        app.stop_simulation();

        assert_eq!(
            app.registry.get::<Transform2D>(e).unwrap().pos,
            initial_pos,
            "Stop sonrası nesne ilk konumuna dönmeli"
        );
    }

    #[test]
    fn test_editor_simulation_3d_state_rollback() {
        let mut app = App::new();
        let e3d = app.registry.create();
        let initial_trans = glam::Vec3::new(3.0, 5.0, -2.0);
        let initial_rot = glam::Quat::IDENTITY;
        app.registry.emplace(
            e3d,
            Transform3D {
                translation: initial_trans,
                rotation: initial_rot,
                scale: glam::Vec3::ONE,
            },
        );

        // 1. Simülasyonu başlat (2D ve 3D snapshot otomatik alınır)
        app.start_simulation();

        // 2. Simülasyon adımı işlet (3D varlık döner ve ötelenir)
        app.step_simulation_single_frame(1.0 / 60.0);

        let simulated_tr = *app.registry.get::<Transform3D>(e3d).unwrap();
        assert_ne!(
            simulated_tr.translation, initial_trans,
            "Simulasyon sonrasi 3D oteleme degismeli"
        );
        assert_ne!(
            simulated_tr.rotation, initial_rot,
            "Simulasyon sonrasi 3D rotasyon degismeli"
        );

        // 3. Stop: app.stop_simulation() ile 3D durumu geri yükle
        app.stop_simulation();

        let restored_tr = *app.registry.get::<Transform3D>(e3d).unwrap();
        assert_eq!(
            restored_tr.translation, initial_trans,
            "Stop sonrasi 3D nesne ilk konumuna geri donmeli"
        );
        assert_eq!(
            restored_tr.rotation, initial_rot,
            "Stop sonrasi 3D nesne ilk rotasyonuna geri donmeli"
        );
    }

    #[test]
    fn test_editor_export_pak_generation() {
        let mut app = App::new();
        let initial_count = app.registry.entities::<Transform2D>().len()
            + app.registry.entities::<Transform3D>().len();

        let e = app.registry.create();
        app.registry.emplace(
            e,
            Transform3D {
                translation: glam::Vec3::new(1.0, 2.0, 3.0),
                rotation: glam::Quat::IDENTITY,
                scale: glam::Vec3::ONE,
            },
        );
        app.registry.emplace(
            e,
            rugeng_core::Mesh3DComponent::with_pbr("cube", [1.0, 1.0, 1.0, 1.0], 0.5, 0.1),
        );

        let temp_pak = "/tmp/test_export_stage18.pak";
        let res = app.export_project_pak(temp_pak);
        assert!(res.is_ok(), "PAK export basarili olmali: {:?}", res);

        // PAK dosyasını PakReader ile oku ve doğrula
        let reader =
            rugeng_assets::PakReader::from_file(temp_pak).expect("PAK dosyasi okunabilmeli");
        let scene_bin = reader
            .get_file("scenes/main.bin")
            .expect("scenes/main.bin pakette bulunmali");
        assert!(!scene_bin.is_empty(), "Sahne binary verisi bos olmamali");

        let vs = rugeng_assets::VersionedScene::from_bytes(scene_bin)
            .expect("Binary sahne ayristirilabilmeli");
        assert_eq!(
            vs.entities.len(),
            initial_count + 1,
            "Var olan ve yeni eklenen 3D varliklar paketlenmis olmali"
        );

        // Eklenen varlığın transform3d özelliklerini doğrula
        let found = vs.entities.iter().find(|item| {
            item.transform_3d
                .map(|t| t.translation.x == 1.0 && t.translation.y == 2.0)
                .unwrap_or(false)
        });
        assert!(found.is_some(), "Eklenen 3D varlik pakette bulunmali");

        let _ = std::fs::remove_file(temp_pak);
    }

    #[test]
    fn test_toolbar_and_palette_layout_no_overflow() {
        // 1. Her araç butonunun metni buton sınırları içine sığmalı ve taşmamalı
        for &(_tool, label, btn_w) in &VIEWPORT_TOOLS {
            let txt_scale = if label.len() > 6 { 0.9 } else { 0.95 };
            let tw = font::text_width(label, txt_scale);
            let right_edge = 16.0 + tw; // 16.0 icon ve padding ofseti
            assert!(
                right_edge <= btn_w,
                "Araç butonu '{}' taşma yapıyor: right_edge ({}) > btn_w ({})",
                label,
                right_edge,
                btn_w
            );
        }

        // 2. Her karo butonunun metni buton sınırları içine sığmalı
        for &(label, _tt, btn_w) in &TILE_PALETTE_ITEMS {
            let txt_scale = if label.len() > 3 { 0.95 } else { 1.0 };
            let txt_offset = if label.len() <= 2 {
                11.0
            } else if label.len() == 3 {
                8.0
            } else {
                6.0
            };
            let tw = font::text_width(label, txt_scale);
            let right_edge = txt_offset + tw;
            assert!(
                right_edge <= btn_w,
                "Karo butonu '{}' taşma yapıyor: right_edge ({}) > btn_w ({})",
                label,
                right_edge,
                btn_w
            );
        }

        // 3. 1280x720 çözünürlükte (vp_w = 700px), Araçlar + Palet Snap butonuna ve Inspector paneline taşmamalı
        let tools_w: f32 = VIEWPORT_TOOLS.iter().map(|t| t.2 + 4.0).sum();
        let palette_w: f32 = TILE_PALETTE_ITEMS.iter().map(|p| p.2 + 4.0).sum();
        let total_left = tools_w + 8.0 + palette_w;
        let snap_start = 700.0 - 74.0; // vp_w - 74.0
        assert!(
            total_left <= snap_start,
            "Araç çubuğu ve karo paleti Snap butonuna çarpıyor: total_left ({}) > snap_start ({})",
            total_left,
            snap_start
        );
    }

    #[test]
    fn test_editor_3d_camera_orbit_and_pan() {
        let mut cam3d = types::EditorCamera3D::default();
        let initial_eye = cam3d.eye;
        let initial_yaw = cam3d.yaw;
        let initial_pitch = cam3d.pitch;

        // Orbit simülasyonu
        cam3d.yaw += 0.2;
        cam3d.pitch = (cam3d.pitch + 0.1).clamp(-1.5, 1.5);
        assert_ne!(cam3d.yaw, initial_yaw);
        assert_ne!(cam3d.pitch, initial_pitch);

        // Dolly Zoom (mouse wheel)
        cam3d.dolly(-2.0);
        let dist = (cam3d.eye - cam3d.target).length();
        assert!(dist > 1.0);

        // Pan
        let right = cam3d.right();
        let up = cam3d.up();
        cam3d.eye += right * 1.5 + up * 0.5;
        cam3d.target += right * 1.5 + up * 0.5;
        assert_ne!(cam3d.eye, initial_eye);

        let render_cam = cam3d.to_render_camera(16.0 / 9.0);
        let vp = render_cam.view_projection();
        assert!(!vp.is_nan());
    }

    #[test]
    fn test_editor_versioned_scene_save_load_3d() {
        let mut app = App::new();
        let e3d = app.registry.create();
        app.registry.emplace(
            e3d,
            Transform3D {
                translation: glam::Vec3::new(5.0, 2.5, -3.0),
                rotation: glam::Quat::IDENTITY,
                scale: glam::Vec3::splat(1.5),
            },
        );
        app.registry.emplace(
            e3d,
            Mesh3DComponent::with_pbr("sphere", [1.0, 0.5, 0.2, 1.0], 0.35, 0.8),
        );

        let temp_path = "/tmp/test_rugeng_stage8_stage9_scene.json";
        app.save_current_scene(temp_path);

        // Sahneyi temizle ve tekrar yükle
        app.load_scene_from_file(temp_path);

        let entities_3d = app.registry.entities::<Transform3D>();
        assert!(!entities_3d.is_empty(), "3D varliklar yuklenmeli");

        let found = entities_3d
            .into_iter()
            .find(|&e| {
                app.registry
                    .get::<Mesh3DComponent>(e)
                    .map(|m| m.mesh_name == "sphere" && (m.roughness - 0.35).abs() < 1e-4)
                    .unwrap_or(false)
            })
            .expect("Kaydedilip yuklenen 3D kure varligi bulunamadi");

        let loaded_tr = app.registry.get::<Transform3D>(found).unwrap();
        assert_eq!(loaded_tr.translation.x, 5.0);
        assert_eq!(loaded_tr.translation.y, 2.5);
        assert_eq!(loaded_tr.translation.z, -3.0);
        assert_eq!(loaded_tr.scale.x, 1.5);

        let loaded_mesh = app.registry.get::<Mesh3DComponent>(found).unwrap();
        assert_eq!(loaded_mesh.mesh_name, "sphere");
        assert_eq!(loaded_mesh.roughness, 0.35);
        assert_eq!(loaded_mesh.metallic, 0.8);

        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_spawn_3d_preset_and_components() {
        let mut app = App::new();
        let initial_count = app.registry.entities::<Transform3D>().len();

        app.spawn_preset_entity("cube_3d");

        let entities = app.registry.entities::<Transform3D>();
        assert_eq!(entities.len(), initial_count + 1);
        let e = *entities.last().unwrap();
        let mesh = app
            .registry
            .get::<Mesh3DComponent>(e)
            .expect("Mesh3DComponent olmali");
        assert_eq!(mesh.mesh_name, "cube");
        assert_eq!(mesh.roughness, 0.5);
        assert_eq!(mesh.metallic, 0.1);

        app.spawn_preset_entity("sphere_3d");
        let entities2 = app.registry.entities::<Transform3D>();
        assert_eq!(entities2.len(), initial_count + 2);

        app.spawn_preset_entity("cylinder_3d");
        app.spawn_preset_entity("cone_3d");
        app.spawn_preset_entity("pyramid_3d");
        app.spawn_preset_entity("capsule_3d");
        app.spawn_preset_entity("sprite_3d");

        let entities_final = app.registry.entities::<Transform3D>();
        assert_eq!(entities_final.len(), initial_count + 7);
    }

    #[test]
    fn test_editor_3d_flycam_controls() {
        let mut app = App::new();
        app.editor.workspace = WorkspaceMode::ThreeD;

        let initial_eye = app.editor.camera_3d.eye;
        app.editor.camera_3d.is_flying = true;
        app.editor.camera_3d.is_turbo = true;

        assert_eq!(app.editor.camera_3d.effective_speed(), 8.0 * 2.5);

        // move_up (W) simülasyonu -> fwd yönünde ilerleme
        app.action_map.update_key_event(0x11, true); // W
        app.update_simulation(0.1);
        assert!(app.editor.camera_3d.eye != initial_eye);
        app.action_map.update_key_event(0x11, false);

        // fly_up (E) simülasyonu -> Y dikey yükselme
        let eye_before_up = app.editor.camera_3d.eye;
        app.action_map.update_key_event(0x12, true); // E
        app.update_simulation(0.1);
        assert!(
            app.editor.camera_3d.eye.y > eye_before_up.y,
            "E tusu dikey yukari ucurmali"
        );
        app.action_map.update_key_event(0x12, false);

        // fly_down (Q) simülasyonu -> Y dikey alçalma
        let eye_before_down = app.editor.camera_3d.eye;
        app.action_map.update_key_event(0x10, true); // Q
        app.update_simulation(0.1);
        assert!(
            app.editor.camera_3d.eye.y < eye_before_down.y,
            "Q tusu dikey asagi indirmeli"
        );
        app.action_map.update_key_event(0x10, false);
    }

    #[test]
    fn test_editor_3d_gizmo_projection() {
        let mut app = App::new();
        app.editor.workspace = WorkspaceMode::ThreeD;
        app.spawn_preset_entity("cube_3d");

        let e = app.registry.entities::<Transform3D>()[0];
        app.editor.selected_entity = Some(e);

        let render_cam = app.editor.camera_3d.to_render_camera(16.0 / 9.0);
        let vp_mat = render_cam.view_projection();
        assert!(!vp_mat.is_nan());

        let t3 = app.registry.get::<Transform3D>(e).unwrap();
        let clip =
            vp_mat * glam::Vec4::new(t3.translation.x, t3.translation.y, t3.translation.z, 1.0);
        assert!(
            clip.w > 0.0,
            "Kamera onundeki nesne pozitif w degerine sahip olmali"
        );
    }

    #[test]
    fn test_editor_3d_picking_and_selection() {
        let mut app = App::new();
        app.editor.workspace = WorkspaceMode::ThreeD;
        app.spawn_preset_entity("sphere_3d");

        let e = *app.registry.entities::<Transform3D>().last().unwrap();
        app.editor.selected_entity = None;
        assert_eq!(app.editor.selected_entity, None);

        // Ekran boyutları ve viewport hesaplaması
        let screen_w = 1280.0;
        let screen_h = 720.0;
        let left_w = 260.0;
        let right_w = 300.0;
        let bottom_h = 160.0;
        let menu_h = 30.0;
        let scene_tab_h = 26.0;
        let vp_tool_h = 24.0;
        let vp_canvas_y = menu_h + scene_tab_h + vp_tool_h;
        let vp_x = left_w;
        let vp_w = screen_w - left_w - right_w;
        let vp_canvas_h: f32 = screen_h - bottom_h - vp_canvas_y;

        let aspect = vp_w / vp_canvas_h.max(1.0);
        let render_cam = app.editor.camera_3d.to_render_camera(aspect);
        let vp_mat = render_cam.view_projection();

        let t3 = app.registry.get::<Transform3D>(e).copied().unwrap();
        let clip =
            vp_mat * glam::Vec4::new(t3.translation.x, t3.translation.y, t3.translation.z, 1.0);
        assert!(clip.w > 0.05);
        let ndc_x = clip.x / clip.w;
        let ndc_y = clip.y / clip.w;
        let sx = vp_x + (ndc_x * 0.5 + 0.5) * vp_w;
        let sy = vp_canvas_y + (1.0 - (ndc_y * 0.5 + 0.5)) * vp_canvas_h;

        // Varlığın ekran projeksiyon noktasına tıkla
        app.handle_mouse_click(Vec2::new(sx, sy), screen_w, screen_h, false);
        assert_eq!(
            app.editor.selected_entity,
            Some(e),
            "3D varligin ekran koordinatindaki tiklama varligi secmelidir"
        );

        // Boş alana tıklandığında seçimin kaldırıldığını doğrula
        app.handle_mouse_click(
            Vec2::new(vp_x + 5.0, vp_canvas_y + 5.0),
            screen_w,
            screen_h,
            false,
        );
        assert_eq!(
            app.editor.selected_entity, None,
            "Bos alana tiklama 3D varlik secimini kaldirmalidir"
        );
    }

    #[test]
    fn test_editor_3d_gizmo_drag_and_actions() {
        let mut app = App::new();
        app.editor.workspace = WorkspaceMode::ThreeD;
        app.spawn_preset_entity("cube_3d");

        let e = app.registry.entities::<Transform3D>()[0];
        app.editor.selected_entity = Some(e);
        let orig_pos = app.registry.get::<Transform3D>(e).unwrap().translation;

        // 1. Move Tool Drag (Translate3D on X axis)
        app.editor.active_tool = ViewportTool::Move;
        app.editor.drag_mode = DragMode::Translate3D {
            axis: 0,
            start_translation: orig_pos,
            start_mouse: Vec2::new(100.0, 100.0),
        };

        // Simüle edilmiş fare sürüklemesi (m = 150.0, 100.0)
        let m = Vec2::new(150.0, 100.0);
        let mouse_delta = m - Vec2::new(100.0, 100.0);
        if let Some(t3) = app.registry.get_mut::<Transform3D>(e) {
            let amount = mouse_delta.dot(Vec2::X) * 0.03;
            t3.translation = orig_pos + glam::Vec3::X * amount;
        }

        let new_pos = app.registry.get::<Transform3D>(e).unwrap().translation;
        assert!(
            new_pos.x > orig_pos.x,
            "Translate3D X ekseninde konumu artirmali"
        );

        // 2. Rotate Tool Drag (Rotate3D on Y axis / Yaw)
        app.editor.active_tool = ViewportTool::Rotate;
        let orig_rot = app.registry.get::<Transform3D>(e).unwrap().rotation;
        app.editor.drag_mode = DragMode::Rotate3D {
            axis: 1,
            start_rotation: orig_rot,
            start_mouse: Vec2::new(100.0, 100.0),
        };

        let rot_delta = 0.5;
        if let Some(t3) = app.registry.get_mut::<Transform3D>(e) {
            let delta_quat = glam::Quat::from_axis_angle(glam::Vec3::Y, rot_delta);
            t3.rotation = delta_quat * orig_rot;
        }
        let new_rot = app.registry.get::<Transform3D>(e).unwrap().rotation;
        assert_ne!(new_rot, orig_rot, "Rotate3D dondurme uygulamali");

        // 3. Scale Tool Drag (Scale3D on Z axis)
        app.editor.active_tool = ViewportTool::Scale;
        let orig_scale = app.registry.get::<Transform3D>(e).unwrap().scale;
        app.editor.drag_mode = DragMode::Scale3D {
            axis: 2,
            start_scale: orig_scale,
            start_mouse: Vec2::new(100.0, 100.0),
        };

        if let Some(t3) = app.registry.get_mut::<Transform3D>(e) {
            t3.scale = glam::Vec3::new(orig_scale.x, orig_scale.y, orig_scale.z * 1.5);
        }
        let new_scale = app.registry.get::<Transform3D>(e).unwrap().scale;
        assert_eq!(
            new_scale.z,
            orig_scale.z * 1.5,
            "Scale3D olcekleme uygulamali"
        );
    }

    #[test]
    fn test_dock_layout_and_splitter_drag() {
        let mut app = App::new();
        let screen_w = 1280.0;
        let screen_h = 720.0;

        let initial_layout = DockLayout::with_custom_sizes(
            screen_w,
            screen_h,
            app.editor.dock_left_w,
            app.editor.dock_right_w,
            app.editor.dock_bottom_h,
        );

        // 1. Sol splitter tespiti ve sürükleme testi
        let splitter_hit_pos = Vec2::new(initial_layout.left_w, 200.0);
        assert!(initial_layout.is_over_left_splitter(splitter_hit_pos));

        app.editor.drag_mode = DragMode::SplitterLeft {
            start_left_w: app.editor.dock_left_w,
            start_mouse_x: splitter_hit_pos.x,
        };

        // Fareyi sağa sürükle (+40px)
        let m = Vec2::new(splitter_hit_pos.x + 40.0, 200.0);
        if let DragMode::SplitterLeft {
            start_left_w,
            start_mouse_x,
        } = app.editor.drag_mode
        {
            let delta = m.x - start_mouse_x;
            let max_w = (screen_w * 0.45f32).max(160.0);
            app.editor.dock_left_w = (start_left_w + delta).clamp(160.0, max_w);
        }
        assert_eq!(app.editor.dock_left_w, 300.0);

        // 2. Alt splitter tespiti ve sürükleme testi
        let bottom_y = screen_h - app.editor.dock_bottom_h;
        let bot_hit_pos = Vec2::new(500.0, bottom_y);
        let updated_layout = DockLayout::with_custom_sizes(
            screen_w,
            screen_h,
            app.editor.dock_left_w,
            app.editor.dock_right_w,
            app.editor.dock_bottom_h,
        );
        assert!(updated_layout.is_over_bottom_splitter(bot_hit_pos));

        app.editor.drag_mode = DragMode::SplitterBottom {
            start_bottom_h: app.editor.dock_bottom_h,
            start_mouse_y: bot_hit_pos.y,
        };

        // Fareyi yukarı sürükle (-30px) -> bottom_h artmalı
        let m_bot = Vec2::new(500.0, bot_hit_pos.y - 30.0);
        if let DragMode::SplitterBottom {
            start_bottom_h,
            start_mouse_y,
        } = app.editor.drag_mode
        {
            let delta = start_mouse_y - m_bot.y;
            let max_h = (screen_h * 0.5f32).max(80.0);
            app.editor.dock_bottom_h = (start_bottom_h + delta).clamp(80.0, max_h);
        }
        assert_eq!(app.editor.dock_bottom_h, 190.0);

        // 3. Sağ splitter tespiti ve sürükleme testi
        let default_layout = DockLayout::new(screen_w, screen_h);
        let right_hit_pos = Vec2::new(default_layout.insp_x, 200.0);
        assert!(default_layout.is_over_right_splitter(right_hit_pos));

        app.editor.drag_mode = DragMode::SplitterRight {
            start_right_w: app.editor.dock_right_w,
            start_mouse_x: right_hit_pos.x,
        };

        // Fareyi sola sürükle (-50px) -> right_w artmalı
        let m_right = Vec2::new(right_hit_pos.x - 50.0, 200.0);
        if let DragMode::SplitterRight {
            start_right_w,
            start_mouse_x,
        } = app.editor.drag_mode
        {
            let delta = start_mouse_x - m_right.x;
            let max_w = (screen_w * 0.45f32).max(180.0);
            app.editor.dock_right_w = (start_right_w + delta).clamp(180.0, max_w);
        }
        assert_eq!(app.editor.dock_right_w, 350.0);

        // 4. Küçük ekran / aşırı boyut kenetleme (clamping) ve asgari viewport garantisi
        let small_layout = DockLayout::with_custom_sizes(640.0, 480.0, 500.0, 500.0, 400.0);
        assert!(
            small_layout.vp_w >= 100.0,
            "Viewport asgari 100px genislik korumali"
        );
        assert!(
            small_layout.vp_canvas_h >= 100.0,
            "Viewport asgari 100px yukseklik korumali"
        );

        // 5. Sürükleme bitişi
        app.editor.drag_mode = DragMode::None;
        assert_eq!(app.editor.drag_mode, DragMode::None);
    }

    #[test]
    fn test_editor_3d_right_click_raycast_picking() {
        let mut app = App::new();
        app.editor.workspace = WorkspaceMode::ThreeD;
        app.spawn_preset_entity("sphere_3d");

        let e = *app.registry.entities::<Transform3D>().last().unwrap();
        app.editor.selected_entity = None;
        assert_eq!(app.editor.selected_entity, None);

        let screen_w = 1280.0;
        let screen_h = 720.0;
        let layout = DockLayout::with_custom_sizes(
            screen_w,
            screen_h,
            app.editor.dock_left_w,
            app.editor.dock_right_w,
            app.editor.dock_bottom_h,
        );

        let aspect = layout.vp_w / layout.vp_canvas_h.max(1.0);
        let render_cam = app.editor.camera_3d.to_render_camera(aspect);
        let vp_mat = render_cam.view_projection();

        let t3 = app.registry.get::<Transform3D>(e).copied().unwrap();
        let clip =
            vp_mat * glam::Vec4::new(t3.translation.x, t3.translation.y, t3.translation.z, 1.0);
        assert!(clip.w > 0.05);
        let ndc_x = clip.x / clip.w;
        let ndc_y = clip.y / clip.w;
        let sx = layout.vp_x + (ndc_x * 0.5 + 0.5) * layout.vp_w;
        let sy = layout.vp_canvas_y + (1.0 - (ndc_y * 0.5 + 0.5)) * layout.vp_canvas_h;
        assert!(sx >= layout.vp_x && sx <= layout.vp_x + layout.vp_w);
        assert!(sy >= layout.vp_canvas_y && sy <= layout.vp_canvas_y + layout.vp_canvas_h);

        // 3D Raycasting ile sağ tıklama seçimi doğrula
        let inv_vp = vp_mat.inverse();
        let p_near_h = inv_vp * glam::Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
        let p_far_h = inv_vp * glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);
        assert!(p_near_h.w.abs() > 1e-6);

        let p_near = p_near_h.truncate() / p_near_h.w;
        let p_far = p_far_h.truncate() / p_far_h.w;
        let ray_dir = (p_far - p_near).normalize();
        let ray_3d = rugeng_physics::Ray3D::new(p_near, ray_dir, 1000.0);
        let sphere = rugeng_physics::BoundingSphere::new(t3.translation, 0.5);
        let hit = rugeng_physics::ray_intersect_sphere(&ray_3d, &sphere);
        assert!(hit.is_some(), "Kureye yonelen isin kesismelidir");
        assert!(hit.unwrap().dist > 0.0);
    }

    #[test]
    fn test_z_index_sorting_and_2_5d_depth() {
        let mut app = App::new();

        // 2 adet 2D varlık oluştur: biri arka plan (z = -2), diğeri ön plan (z = 5)
        let bg_entity = app.registry.create();
        app.registry.emplace(
            bg_entity,
            Transform2D {
                pos: Vec2::new(50.0, 50.0),
                size: Vec2::new(100.0, 100.0),
                rotation: 0.0,
            },
        );
        app.registry.emplace(
            bg_entity,
            SpriteColor {
                color: [100, 100, 100, 255],
            },
        );
        app.registry.emplace(bg_entity, ZIndex2D::new(-2));

        let fg_entity = app.registry.create();
        app.registry.emplace(
            fg_entity,
            Transform2D {
                pos: Vec2::new(60.0, 60.0),
                size: Vec2::new(32.0, 32.0),
                rotation: 0.0,
            },
        );
        app.registry.emplace(
            fg_entity,
            SpriteColor {
                color: [255, 0, 0, 255],
            },
        );
        app.registry.emplace(fg_entity, ZIndex2D::new(5));

        // 1. Sıralama doğrulaması
        let mut items: Vec<(Entity, i32)> = Vec::new();
        app.registry
            .view2_ref_ref::<Transform2D, SpriteColor, _>(|e, _tr, _spr| {
                let z = app.registry.get::<ZIndex2D>(e).map(|zi| zi.z).unwrap_or(0);
                items.push((e, z));
            });
        items.sort_by_key(|item| item.1);

        assert_eq!(items.len(), 5); // Standart sahnedeki 3 varlık (z=0) + bizim 2 varlık (-2 ve 5)
        assert_eq!(items.first().unwrap().0, bg_entity);
        assert_eq!(items.first().unwrap().1, -2);
        assert_eq!(items.last().unwrap().0, fg_entity);
        assert_eq!(items.last().unwrap().1, 5);

        // 2. Sahne kaydetme ve geri yükleme (Serialization round-trip)
        let temp_scene_path = "/tmp/test_rugeng_z_index_scene.scene";
        app.save_current_scene(temp_scene_path);
        app.load_scene_from_file(temp_scene_path);

        // Geri yüklenen varlıkların Z-Index değerlerini teyit et
        let mut found_bg = false;
        let mut found_fg = false;
        app.registry
            .view2_ref_ref::<Transform2D, SpriteColor, _>(|e, tr, _spr| {
                let zi = app.registry.get::<ZIndex2D>(e).map(|z| z.z);
                if (tr.pos.x - 50.0).abs() < 1e-3 {
                    assert_eq!(zi, Some(-2));
                    found_bg = true;
                } else if (tr.pos.x - 60.0).abs() < 1e-3 {
                    assert_eq!(zi, Some(5));
                    found_fg = true;
                }
            });
        assert!(
            found_bg && found_fg,
            "Z-Index'li varliklar basariyla serilestirilip yuklenmeli"
        );

        let _ = std::fs::remove_file(temp_scene_path);
    }

    #[test]
    fn test_context_menu_and_add_menu() {
        let mut app = App::new();

        // 1. Top Menubar Add Dropdown Testi:
        app.editor.active_menu = Some(OpenMenu::Add);
        // Menü tıklaması simülasyonu: Add menüsü x=142, drop_y=30, item_h=24
        // 0. eleman: add_ship (y = 30 + 4 + 12 = 46)
        let initial_count = app.registry.entities::<Transform2D>().len();
        app.handle_mouse_click(Vec2::new(160.0, 46.0), 1280.0, 720.0, false);

        assert_eq!(
            app.editor.active_menu, None,
            "Tiklama sonrasi acilir menu kapanmali"
        );
        let new_count = app.registry.entities::<Transform2D>().len();
        assert_eq!(new_count, initial_count + 1, "Add Ship varligi olusturmali");

        // 2. Context Menu (Sağ Tık Menüsü) Testi:
        // Boşlukta açılan Context Menu simülasyonu
        let ctx = ContextMenu {
            screen_pos: Vec2::new(400.0, 300.0),
            target_entity: None,
            items: vec![
                ContextMenuItem {
                    label: "[+] 2D Dusman (Bug)".to_string(),
                    shortcut: "".to_string(),
                    action_id: "add_enemy".to_string(),
                },
                ContextMenuItem {
                    label: "[3] 3D Kup (Cube)".to_string(),
                    shortcut: "".to_string(),
                    action_id: "add_cube_3d".to_string(),
                },
            ],
        };
        app.editor.context_menu = Some(ctx.clone());

        // Context Menu 0. öğesine sol tık (pos_x=400, pos_y=300, y=312)
        app.handle_mouse_click(Vec2::new(450.0, 312.0), 1280.0, 720.0, false);
        assert!(
            app.editor.context_menu.is_none(),
            "Tiklama sonrasi context menu kapanmali"
        );
        let enemy_count = app.registry.entities::<Transform2D>().len();
        assert_eq!(
            enemy_count,
            new_count + 1,
            "add_enemy ile yeni varlik olusmali"
        );

        // 3. Varlık üzerindeki Context Menu (Duplicate, Delete, Z-Index) Testi:
        let test_e = app.registry.create();
        app.registry.emplace(
            test_e,
            Transform2D {
                pos: Vec2::new(100.0, 100.0),
                size: Vec2::new(40.0, 40.0),
                rotation: 0.0,
            },
        );
        app.registry.emplace(test_e, ZIndex2D::new(0));

        let ent_ctx = ContextMenu {
            screen_pos: Vec2::new(500.0, 300.0),
            target_entity: Some(test_e),
            items: vec![ContextMenuItem {
                label: "[Z+] Z-Index Artir (+1)".to_string(),
                shortcut: "".to_string(),
                action_id: "z_up".to_string(),
            }],
        };
        app.editor.context_menu = Some(ent_ctx);
        app.handle_mouse_click(Vec2::new(550.0, 312.0), 1280.0, 720.0, false);

        let z = app.registry.get::<ZIndex2D>(test_e).unwrap().z;
        assert_eq!(z, 1, "Context menu z_up Z-Index'i artirmali");
    }

    #[test]
    fn test_rename_commit_undo_redo_and_disk_roundtrip() {
        let mut app = App::new();
        app.spawn_preset_entity("ship");

        let mut ents = Vec::new();
        app.registry
            .view1_ref::<rugeng_core::Transform2D, _>(|e, _t| ents.push(e));
        let e = *ents.last().expect("sahnede Transform2D düğüm olmalı");
        app.editor.selected_entity = Some(e);

        // F2: üretik adla düzenleme modu açılır
        app.start_rename();
        assert_eq!(
            app.editor.renaming.as_ref().map(|r| r.buffer.as_str()),
            Some(format!("Node2D_{}", e.index()).as_str())
        );

        // Tamponu temizle (F2 mevcut adı önden yazar) ve yeni adı yaz
        while app.editor.renaming.as_mut().unwrap().backspace() {}
        for c in "Oyuncu 1".chars() {
            app.editor.renaming.as_mut().unwrap().insert(c);
        }
        app.commit_rename();
        assert!(app.editor.renaming.is_none());
        assert_eq!(
            app.registry.get::<NodeName>(e).map(|n| n.name.as_str()),
            Some("Oyuncu 1")
        );
        assert!(
            matches!(
                app.editor.undo_stack.last(),
                Some(crate::types::UndoAction::NodeNameChanged { .. })
            ),
            "commit undo yığınına NodeNameChanged yazmalı"
        );

        // Ctrl+Z: ad kalkar; Ctrl+Y: ad geri gelir
        app.undo();
        assert!(app.registry.get::<NodeName>(e).is_none());
        app.redo();
        assert_eq!(
            app.registry.get::<NodeName>(e).map(|n| n.name.as_str()),
            Some("Oyuncu 1")
        );

        // Disk turu: özel ad dosyaya yazılır, yüklemede NodeName döner
        let path = "/tmp/test_rugeng_rename_flow.scene";
        app.save_current_scene(path);
        app.load_scene_from_file(path);
        let mut found = false;
        app.registry
            .view1_ref::<NodeName, _>(|_e, n| found |= n.name == "Oyuncu 1");
        assert!(found, "özel ad kaydet/yükle turundan sağ çıkmalı");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_rename_empty_and_cancel_leave_no_trace() {
        let mut app = App::new();
        let e = {
            let mut ents = Vec::new();
            app.registry
                .view1_ref::<rugeng_core::Transform2D, _>(|e, _t| ents.push(e));
            *ents.last().unwrap()
        };
        app.editor.selected_entity = Some(e);
        let undo_len = app.editor.undo_stack.len();

        // Boş tamponla Enter: hiçbir şey yazılmamalı
        app.start_rename();
        while app.editor.renaming.as_mut().unwrap().backspace() {}
        app.commit_rename();
        assert!(app.registry.get::<NodeName>(e).is_none());
        assert_eq!(app.editor.undo_stack.len(), undo_len);

        // Esc ile vazgeç: ad değişmemeli
        app.start_rename();
        for c in "XX".chars() {
            app.editor.renaming.as_mut().unwrap().insert(c);
        }
        app.cancel_rename();
        assert!(app.editor.renaming.is_none());
        assert!(app.registry.get::<NodeName>(e).is_none());
        assert_eq!(app.editor.undo_stack.len(), undo_len);
    }

    #[test]
    fn test_undo_after_scene_reload_does_not_panic_on_dead_entity() {
        let mut app = App::new();
        app.spawn_preset_entity("ship");
        let e = {
            let mut ents = Vec::new();
            app.registry
                .view1_ref::<rugeng_core::Transform2D, _>(|e, _t| ents.push(e));
            *ents.last().unwrap()
        };
        app.editor.selected_entity = Some(e);
        app.start_rename();
        // mevcut tamponu temizleyip yeni ad yaz
        while app.editor.renaming.as_mut().unwrap().backspace() {}
        for c in "Hayalet".chars() {
            app.editor.renaming.as_mut().unwrap().insert(c);
        }
        app.commit_rename();

        // Sahneyi yeniden yükle: e artık ölü. Undo panic ATMAMALI.
        let path = "/tmp/test_rugeng_rename_dead.scene";
        app.save_current_scene(path);
        app.load_scene_from_file(path);
        app.undo(); // ölü entity'ye dokunan kol — guard olmasa emplace assert'le patlardı
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_duplicate_selected_entity_with_offset_and_undo_redo() {
        let mut app = App::new();
        app.spawn_preset_entity("ship");
        let orig_e = {
            let mut ents = Vec::new();
            app.registry
                .view1_ref::<rugeng_core::Transform2D, _>(|e, _t| ents.push(e));
            *ents.last().unwrap()
        };
        app.editor.selected_entity = Some(orig_e);
        app.registry
            .emplace(orig_e, NodeName::new("PlayerVessel".to_string()));
        let orig_pos = app.registry.get::<Transform2D>(orig_e).unwrap().pos;

        // Çoğalt (Duplicate / Ctrl+D)
        app.duplicate_selected_entity();

        let dup_e = app.editor.selected_entity.expect("Klon seçili olmalı");
        assert_ne!(dup_e, orig_e);
        let dup_pos = app.registry.get::<Transform2D>(dup_e).unwrap().pos;
        assert_eq!(dup_pos, orig_pos + glam::Vec2::new(16.0, 16.0));
        assert_eq!(
            app.registry.get::<NodeName>(dup_e).map(|n| n.name.as_str()),
            Some("PlayerVessel (Kopya)")
        );

        // Undo: Çoğaltılan silinir
        app.undo();
        assert!(!app.registry.is_alive(dup_e));

        // Redo: Yeniden oluşturulur
        app.redo();
        let redone_e = app
            .editor
            .selected_entity
            .expect("Redo sonrası seçili olmalı");
        assert!(app.registry.is_alive(redone_e));
        assert_eq!(
            app.registry.get::<Transform2D>(redone_e).unwrap().pos,
            orig_pos + glam::Vec2::new(16.0, 16.0)
        );
    }

    #[test]
    fn test_prefab_save_and_load_scene_flow() {
        let mut app = App::new();
        app.spawn_preset_entity("ship");
        let root = {
            let mut ents = Vec::new();
            app.registry
                .view1_ref::<rugeng_core::Transform2D, _>(|e, _t| ents.push(e));
            *ents.last().unwrap()
        };
        app.editor.selected_entity = Some(root);
        app.registry
            .emplace(root, NodeName::new("BossShip".to_string()));

        let prefab_path = "/tmp/test_boss_ship.prefab";
        assert!(app.save_selected_as_prefab(prefab_path).is_ok());

        // Prefab'ı sahneye yükle
        let loaded_ent = app
            .load_prefab_to_scene(prefab_path)
            .expect("Prefab yüklenemedi");
        assert!(app.registry.is_alive(loaded_ent));
        assert_eq!(
            app.registry
                .get::<NodeName>(loaded_ent)
                .map(|n| n.name.as_str()),
            Some("BossShip")
        );

        let _ = std::fs::remove_file(prefab_path);
    }

    #[test]
    fn test_mouse_click_caret_positioning_during_rename() {
        let mut app = App::new();
        // İlk varlığı al (App::new sahnesindeki ilk varlık)
        let first_ent = {
            let mut ents = Vec::new();
            app.registry
                .view1_ref::<rugeng_core::Transform2D, _>(|e, _t| ents.push(e));
            ents[0]
        };
        app.editor.selected_entity = Some(first_ent);
        app.registry
            .emplace(first_ent, NodeName::new("PlayerShip".to_string()));

        // F2 rename modunu başlat
        app.start_rename();
        assert!(app.editor.renaming.is_some());
        let ren = app.editor.renaming.as_ref().unwrap();
        assert_eq!(ren.buffer, "PlayerShip");
        assert_eq!(ren.caret, "PlayerShip".chars().count()); // Başlangıçta sonda (10)

        // Ağaç satırının dikey koordinatını hesapla (ilk düğüm)
        let filter_y = 30.0 + 26.0;
        let root_y = filter_y + 26.0;
        let b1_y = root_y + 16.0;
        let b5_y = b1_y + (24.0 * 4.0);
        let act_bar_y = b5_y + 24.0;
        let row_y = act_bar_y + 26.0;

        // Metin başlangıcı (28.0 px) üzerine tıklama -> caret 0 olmalı
        app.handle_mouse_click(glam::Vec2::new(28.0, row_y + 10.0), 1280.0, 720.0, false);
        assert!(app.editor.renaming.is_some());
        assert_eq!(app.editor.renaming.as_ref().unwrap().caret, 0);

        // Yaklaşık 4. karakter (Player|Ship) koordinatına tıklama -> caret ~4 olmalı
        let char_step = (5.0 + 2.0) * 1.2; // 8.4 px
        let target_x = 28.0 + (char_step * 4.0);
        app.handle_mouse_click(
            glam::Vec2::new(target_x, row_y + 10.0),
            1280.0,
            720.0,
            false,
        );
        assert_eq!(app.editor.renaming.as_ref().unwrap().caret, 4);

        // Metnin çok sağına tıklama -> metin uzunluğuna (10) kenetlenmeli
        app.handle_mouse_click(glam::Vec2::new(200.0, row_y + 10.0), 1280.0, 720.0, false);
        assert_eq!(app.editor.renaming.as_ref().unwrap().caret, 10);
    }

    #[test]
    fn test_import_gltf_scene_and_pbr_mapping() {
        let mut app = App::new();
        let sample_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/models/sample.glb"
        );
        let ents = app
            .import_gltf_scene(sample_path)
            .expect("sample.glb sahneye aktarılabilmeli");
        assert_eq!(ents.len(), 1, "1 submesh varlığı oluşturulmalı");

        let e = ents[0];
        assert!(app.registry.has::<rugeng_core::Transform3D>(e));
        assert!(app.registry.has::<rugeng_core::Mesh3DComponent>(e));

        let m3 = app.registry.get::<rugeng_core::Mesh3DComponent>(e).unwrap();
        assert_eq!(m3.color, [0.2, 0.8, 0.4, 1.0]);
        assert!((m3.roughness - 0.3).abs() < 1e-4);
        assert!((m3.metallic - 0.7).abs() < 1e-4);
    }
}
