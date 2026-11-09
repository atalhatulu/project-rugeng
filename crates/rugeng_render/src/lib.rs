//! Rugeng Vulkan 1.3 Render Katmanı.
//! Dynamic Rendering, Synchronization2, 2D Instanced Quad Pipeline ve Persistent Ring Upload Buffer.

pub mod camera_3d;
pub mod context;
pub mod depth;
pub mod font;
pub mod gpu_mesh;
pub mod icons;
pub mod mesh;
pub mod pipeline_2d;
pub mod pipeline_3d;
pub mod shadow;
pub mod texture;
pub mod theme;

pub use camera_3d::{AmbientLight, Camera3D, DirectionalLight, OrbitCamera3D, SpringArm3D};
pub use context::{Swapchain, VulkanContext, FRAMES_IN_FLIGHT};
pub use depth::{DepthBuffer, DirectionalShadowMap, DEPTH_FORMAT};
pub use font::TrueTypeFont;
pub use gpu_mesh::GpuMesh3D;
pub use icons::*;
pub use mesh::{Mesh3D, Vertex3D};
pub use pipeline_2d::{
    Pipeline2D, QuadInstance, BUFFER_SIZE_PER_FRAME, MAX_QUADS_PER_FRAME, QUAD_INSTANCE_SIZE,
};
pub use pipeline_3d::{Pipeline3D, PushConstants3D};
pub use shadow::{create_shadow_sampler, ShadowPipeline3D, ShadowPushConstants};
pub use texture::{decode_png, AtlasSubRegion, TextureAtlas};
pub use theme::Theme;

#[cfg(test)]
mod tests {
    use glam::{Mat4, Vec4};

    #[test]
    fn test_orthographic_rh_mapping() {
        let screen_w = 1280.0;
        let screen_h = 720.0;

        // Vulkan NDC standard: top is -1.0, bottom is +1.0
        let mat = Mat4::orthographic_rh(0.0, screen_w, 0.0, screen_h, -1.0, 1.0);
        let p_top = mat * Vec4::new(0.0, 0.0, 0.0, 1.0);
        let p_bot = mat * Vec4::new(0.0, 720.0, 0.0, 1.0);
        assert!(
            (p_top.y - (-1.0)).abs() < 1e-5,
            "Vulkan NDC tepe noktasi -1.0 olmalidir"
        );
        assert!(
            (p_bot.y - 1.0).abs() < 1e-5,
            "Vulkan NDC taban noktasi +1.0 olmalidir"
        );
    }
}
