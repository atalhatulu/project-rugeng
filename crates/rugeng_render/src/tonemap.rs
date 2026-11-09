//! Tam Ekran ACES Filmik Ton Eşleme ve Gama Düzeltme Hattı (Tonemapping Pipeline).
//! HDR offscreen hedefini (`R16G16B16A16_SFLOAT`) alıp swapchain formatına dönüştürür.

use ash::vk;
use bytemuck::{Pod, Zeroable};

pub const TONEMAP_VERT_SPV: &[u8] = include_bytes!("../shaders/tonemap.vert.spv");
pub const TONEMAP_FRAG_SPV: &[u8] = include_bytes!("../shaders/tonemap.frag.spv");

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct TonemapPushConstants {
    pub exposure: f32,
    pub gamma: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

impl Default for TonemapPushConstants {
    fn default() -> Self {
        Self {
            exposure: 1.0,
            gamma: 2.2,
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

pub struct TonemapPipeline {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set: vk::DescriptorSet,
    pub sampler: vk::Sampler,
}

impl TonemapPipeline {
    pub fn new(
        device: &ash::Device,
        output_format: vk::Format,
        hdr_image_view: vk::ImageView,
    ) -> Result<Self, String> {
        // 1. Descriptor Set Layout (Binding 0: HDR Texture sampler)
        let binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);

        let layout_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(std::slice::from_ref(&binding));

        let descriptor_set_layout = unsafe {
            device
                .create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| format!("Tonemap DescriptorSetLayout olusturulamadi: {e}"))?
        };

        // 2. Pipeline Layout (Push Constants: 16 bayt)
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(std::mem::size_of::<TonemapPushConstants>() as u32);

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(std::slice::from_ref(&descriptor_set_layout))
            .push_constant_ranges(std::slice::from_ref(&push_constant_range));

        let layout = unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .map_err(|e| format!("Tonemap PipelineLayout olusturulamadi: {e}"))?
        };

        // 3. Shader Modülleri
        let vert_spv = ash::util::read_spv(&mut std::io::Cursor::new(TONEMAP_VERT_SPV))
            .map_err(|e| format!("Tonemap vert shader okunamadi: {e}"))?;
        let frag_spv = ash::util::read_spv(&mut std::io::Cursor::new(TONEMAP_FRAG_SPV))
            .map_err(|e| format!("Tonemap frag shader okunamadi: {e}"))?;

        let vert_module = unsafe {
            let info = vk::ShaderModuleCreateInfo::default().code(&vert_spv);
            device
                .create_shader_module(&info, None)
                .map_err(|e| format!("Tonemap vert modulu olusturulamadi: {e}"))?
        };
        let frag_module = unsafe {
            let info = vk::ShaderModuleCreateInfo::default().code(&frag_spv);
            device
                .create_shader_module(&info, None)
                .map_err(|e| format!("Tonemap frag modulu olusturulamadi: {e}"))?
        };

        let entry = c"main";
        let stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_module)
                .name(entry),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_module)
                .name(entry),
        ];

        // Boş Vertex Input State (Vertices generated entirely inside vertex shader via gl_VertexIndex)
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // Derinlik testi kapalı (Tam ekran post-process)
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(false)
            .depth_write_enable(false);

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false);

        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(std::slice::from_ref(&color_blend_attachment));

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let color_attachment_formats = [output_format];
        let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&color_attachment_formats);

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .depth_stencil_state(&depth_stencil)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state_info)
            .layout(layout)
            .push_next(&mut rendering_info);

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|(_, e)| format!("Tonemap Pipeline olusturulamadi: {e}"))?[0]
        };

        unsafe {
            device.destroy_shader_module(vert_module, None);
            device.destroy_shader_module(frag_module, None);
        }

        // Sampler oluştur
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .max_lod(0.0)
            .min_lod(0.0);

        let sampler = unsafe {
            device
                .create_sampler(&sampler_info, None)
                .map_err(|e| format!("Tonemap Sampler olusturulamadi: {e}"))?
        };

        // Descriptor Pool ve Set oluştur
        let pool_size = vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(4);

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(std::slice::from_ref(&pool_size))
            .max_sets(2);

        let descriptor_pool = unsafe {
            device
                .create_descriptor_pool(&pool_info, None)
                .map_err(|e| format!("Tonemap DescriptorPool olusturulamadi: {e}"))?
        };

        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(std::slice::from_ref(&descriptor_set_layout));

        let descriptor_set = unsafe {
            device
                .allocate_descriptor_sets(&alloc_info)
                .map_err(|e| format!("Tonemap DescriptorSet allocate basarisiz: {e}"))?[0]
        };

        let image_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(hdr_image_view)
            .sampler(sampler);

        let write = vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&image_info));

        unsafe {
            device.update_descriptor_sets(&[write], &[]);
        }

        Ok(Self {
            pipeline,
            layout,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_set,
            sampler,
        })
    }

    /// Yeni HDR Image View ile descriptor set'i günceller (Boyut değişimlerinde)
    pub fn update_hdr_view(&self, device: &ash::Device, new_hdr_view: vk::ImageView) {
        let image_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(new_hdr_view)
            .sampler(self.sampler);

        let write = vk::WriteDescriptorSet::default()
            .dst_set(self.descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&image_info));

        unsafe {
            device.update_descriptor_sets(&[write], &[]);
        }
    }

    /// Tam ekran tonemapping çizimini icra eder (Full-screen pass)
    pub fn draw_tonemap(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        extent: vk::Extent2D,
        exposure: f32,
        gamma: f32,
    ) {
        unsafe {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);

            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            device.cmd_set_viewport(cmd, 0, &[viewport]);

            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            };
            device.cmd_set_scissor(cmd, 0, &[scissor]);

            let push_data = TonemapPushConstants {
                exposure,
                gamma,
                _pad0: 0.0,
                _pad1: 0.0,
            };
            let push_bytes = bytemuck::bytes_of(&push_data);
            device.cmd_push_constants(
                cmd,
                self.layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                push_bytes,
            );

            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.layout,
                0,
                &[self.descriptor_set],
                &[],
            );

            // 3 köşe çiz (tam ekran üçgeni)
            device.cmd_draw(cmd, 3, 1, 0, 0);
        }
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_sampler(self.sampler, None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            device.destroy_pipeline_layout(self.layout, None);
            device.destroy_pipeline(self.pipeline, None);
        }
    }
}
