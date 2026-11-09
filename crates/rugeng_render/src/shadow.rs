//! Gölge Haritalama (Shadow Mapping) ve Yönlü Işık Derinlik Hattı.
//!
//! Vulkan 1.3 Dynamic Rendering ile `D32_SFLOAT` derinlik tamponuna render alır.
//! Sadece derinlik yazıldığı için renk eki içermez (renksiz pipeline).

use ash::vk;
use bytemuck::{Pod, Zeroable};

use crate::depth::DirectionalShadowMap;
use crate::mesh::Vertex3D;

pub const SHADOW_VERT_SPV: &[u8] = include_bytes!("../shaders/shadow.vert.spv");

/// Gölge geçişi için minimal Push Constants (LightSpaceViewProj * Model).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ShadowPushConstants {
    pub light_space_mvp: [f32; 16], // 64 bayt (LightSpaceMatrix * ModelMatrix)
}

/// Sadece derinlik yazan gölge render pipeline'ı.
pub struct ShadowPipeline3D {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
}

impl ShadowPipeline3D {
    pub fn new(device: &ash::Device, depth_format: vk::Format) -> Result<Self, String> {
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<ShadowPushConstants>() as u32);

        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .push_constant_ranges(std::slice::from_ref(&push_constant_range));

        let layout = unsafe {
            device
                .create_pipeline_layout(&layout_info, None)
                .map_err(|e| format!("ShadowPipelineLayout olusturulamadi: {e}"))?
        };

        let vert_spv = ash::util::read_spv(&mut std::io::Cursor::new(SHADOW_VERT_SPV))
            .map_err(|e| format!("Shadow vertex shader okunamadi: {e}"))?;

        let vert_module = unsafe {
            let create_info = vk::ShaderModuleCreateInfo::default().code(&vert_spv);
            device
                .create_shader_module(&create_info, None)
                .map_err(|e| format!("Shadow shader modulu olusturulamadi: {e}"))?
        };

        let entry_name = c"main";
        let shader_stages = [vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_module)
            .name(entry_name)];

        // Vertex Input Binding & Attributes (Sadece konum yeterlidir ancak Vertex3D ile birebir uyumlu stride kullanıyoruz)
        let binding_description = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Vertex3D>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX);

        let attribute_descriptions = [
            // pos: location 0, vec3 (R32G32B32_SFLOAT)
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0),
        ];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(std::slice::from_ref(&binding_description))
            .vertex_attribute_descriptions(&attribute_descriptions);

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        // Rasterization: Derinlik eğim bias'ı (slope bias) donanımsal olarak uygulanabilir
        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(true)
            .depth_bias_constant_factor(1.5)
            .depth_bias_slope_factor(2.5);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        // Dynamic rendering: Renk eki yok, sadece derinlik eki var
        let mut rendering_info =
            vk::PipelineRenderingCreateInfo::default().depth_attachment_format(depth_format);

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .depth_stencil_state(&depth_stencil)
            .dynamic_state(&dynamic_state_info)
            .layout(layout)
            .push_next(&mut rendering_info);

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|(_, e)| format!("ShadowPipeline3D olusturulamadi: {e}"))?[0]
        };

        unsafe {
            device.destroy_shader_module(vert_module, None);
        }

        Ok(Self { pipeline, layout })
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
        }
    }
}

/// Gölge haritası yardımcı fonksiyonları ve senkronizasyon araçları.
impl DirectionalShadowMap {
    /// Gölge haritasını derinlik eki (`DEPTH_STENCIL_ATTACHMENT_OPTIMAL`) durumuna geçirir.
    pub fn transition_to_attachment(&self, device: &ash::Device, cmd: vk::CommandBuffer) {
        let barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(
                vk::PipelineStageFlags2::FRAGMENT_SHADER
                    | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
            )
            .src_access_mask(
                vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
            )
            .dst_stage_mask(
                vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                    | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
            )
            .dst_access_mask(vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .image(self.image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: Self::CASCADE_COUNT,
            });

        let dependency =
            vk::DependencyInfo::default().image_memory_barriers(std::slice::from_ref(&barrier));
        unsafe {
            device.cmd_pipeline_barrier2(cmd, &dependency);
        }
    }

    /// Gölge haritasını gölgelendirici okuma (`SHADER_READ_ONLY_OPTIMAL`) durumuna geçirir.
    pub fn transition_to_shader_read(&self, device: &ash::Device, cmd: vk::CommandBuffer) {
        let barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS)
            .src_access_mask(vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
            .dst_access_mask(vk::AccessFlags2::SHADER_READ)
            .old_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image(self.image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: Self::CASCADE_COUNT,
            });

        let dependency =
            vk::DependencyInfo::default().image_memory_barriers(std::slice::from_ref(&barrier));
        unsafe {
            device.cmd_pipeline_barrier2(cmd, &dependency);
        }
    }
}

/// Gölge örnekleyici (Shadow Sampler) oluşturucu
pub fn create_shadow_sampler(device: &ash::Device) -> Result<vk::Sampler, String> {
    let sampler_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_BORDER)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_BORDER)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_BORDER)
        .border_color(vk::BorderColor::FLOAT_OPAQUE_WHITE)
        .compare_enable(false)
        .anisotropy_enable(false)
        .max_lod(0.0)
        .min_lod(0.0);

    unsafe {
        device
            .create_sampler(&sampler_info, None)
            .map_err(|e| format!("Shadow Sampler olusturulamadi: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use crate::camera_3d::DirectionalLight;
    use glam::{Vec3, Vec4};

    #[test]
    fn test_light_space_matrix_ortho_bounds() {
        let sun = DirectionalLight::new(Vec3::new(0.5, 1.0, 0.5), [1.0, 1.0, 1.0], 1.0);
        let scene_center = Vec3::new(0.0, 0.0, 0.0);
        let distance = 25.0;
        let ortho_size = 15.0; // [-15, 15] kutusu
        let z_near = 0.1;
        let z_far = 50.0;

        let light_space = sun.light_space_matrix(scene_center, distance, ortho_size, z_near, z_far);

        // Sahne AABB köşeleri: [-5, 5] küpü
        let aabb_corners = [
            Vec3::new(-5.0, -5.0, -5.0),
            Vec3::new(5.0, -5.0, -5.0),
            Vec3::new(-5.0, 5.0, -5.0),
            Vec3::new(5.0, 5.0, -5.0),
            Vec3::new(-5.0, -5.0, 5.0),
            Vec3::new(5.0, -5.0, 5.0),
            Vec3::new(-5.0, 5.0, 5.0),
            Vec3::new(5.0, 5.0, 5.0),
        ];

        for &corner in &aabb_corners {
            let p_clip = light_space * Vec4::new(corner.x, corner.y, corner.z, 1.0);
            let ndc = p_clip.truncate() / p_clip.w;

            // X ve Y [-1.0, 1.0] aralığında olmalı (ortho_size 15 > 5 * sqrt(3) ~ 8.66)
            assert!(
                ndc.x >= -1.0 && ndc.x <= 1.0,
                "Köşe {} X sinirinda kalmali: {}",
                corner,
                ndc.x
            );
            assert!(
                ndc.y >= -1.0 && ndc.y <= 1.0,
                "Köşe {} Y sinirinda kalmali: {}",
                corner,
                ndc.y
            );
            // Vulkan Derinliği [0.0, 1.0] aralığında olmalı
            assert!(
                ndc.z >= 0.0 && ndc.z <= 1.0,
                "Köşe {} Z derinlik araliginda kalmali: {}",
                corner,
                ndc.z
            );
        }
    }

    #[test]
    fn test_shadow_bias_no_peter_panning() {
        // Sentetik düzlem testi: Eğim duyarlı bias (slope-scaled bias) hesaplama mantığı
        // Formül: base_bias * (1.0 - cos_theta) ile minimum base_bias * 0.2
        let base_bias = 0.0025f32;

        // 1. Işığa tam dik düzlem (N dot L = 1.0, cos_theta = 1.0)
        let n_perp = Vec3::Y;
        let l_perp = Vec3::Y;
        let cos_perp = n_perp.dot(l_perp).max(0.0);
        let bias_perp = (base_bias * (1.0 - cos_perp)).max(base_bias * 0.2);

        // Peter-panning olmaması için bias çok yüksek olmamalı
        assert!(bias_perp <= 0.001);
        assert!(bias_perp >= 0.0005);

        // 2. Eğimli yüzey (N dot L = 0.5, cos_theta = 0.5)
        let cos_grazing = 0.5f32;
        let bias_grazing = (base_bias * (1.0 - cos_grazing)).max(base_bias * 0.2);
        assert!(bias_grazing > bias_perp);
        assert!(bias_grazing <= base_bias);

        // 3. Çok dik/teğet ışık (N dot L = 0.05)
        let cos_tangent = 0.05f32;
        let bias_tangent = (base_bias * (1.0 - cos_tangent)).max(base_bias * 0.2);
        assert!(bias_tangent > bias_grazing);
        assert!(bias_tangent <= base_bias);
    }
}
