#![allow(unused_imports)]
pub mod bottom_shelf;
pub mod dock_layout;
pub mod inspector;
pub mod scene_tree;
pub mod top_bar;
pub mod viewport;
pub mod vulkan_render;

pub use dock_layout::DockLayout;

use crate::App;
use rugeng_render::pipeline_2d::QuadInstance;
use std::time::Instant;

impl App {
    pub(crate) fn render_ui_and_scene(&mut self, screen_w: f32, screen_h: f32) {
        let render_start = Instant::now();
        if self.context.is_none() || self.pipeline_2d.is_none() {
            return;
        }

        self.arena.reset();
        let mut instances = Vec::with_capacity(8192);

        let layout = DockLayout::with_custom_sizes(
            screen_w,
            screen_h,
            self.editor.dock_left_w,
            self.editor.dock_right_w,
            self.editor.dock_bottom_h,
        );

        // 1. Orta Alan: Viewport (Sahne sekmeleri, araç çubuğu, tuval ve 3D gizmo projeksiyonları)
        self.render_viewport(&mut instances, &layout);

        // 2. Sol Sütun: Sahne Ağacı ve Dosya Sistemi (Scene Tree & FileSystem)
        self.render_scene_tree_and_filesystem(&mut instances, &layout);

        // 3. Alt Panel: Konsol ve Hata Ayıklama (Output & Debugger)
        self.render_bottom_shelf(&mut instances, &layout);

        // 4. Sağ Sütun: Denetçi (Inspector & Component Cards)
        self.render_inspector(&mut instances, &layout);

        // 5. Üst Menü ve Açılır Pencereler (Top Bar & Popups)
        self.render_top_bar(&mut instances, &layout);

        // 6. Vulkan 1.3 Dynamic Rendering Katmanı
        self.render_vulkan_frame(&instances, &layout);

        self.frame_time_ms = render_start.elapsed().as_secs_f32() * 1000.0;
    }
}
