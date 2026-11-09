//! Vulkan 1.3 3D Graphics Pipeline (Dynamic Rendering, Depth Test, Blinn-Phong Shading).

use ash::vk;
use bytemuck::{Pod, Zeroable};
use glam::Mat4;

use crate::camera_3d::{AmbientLight, Camera3D, DirectionalLight};
use crate::gpu_mesh::GpuMesh3D;
use crate::mesh::Vertex3D;

pub const VERT_3D_SPV: &[u8] = include_bytes!("../shaders/mesh_3d.vert.spv");
pub const FRAG_3D_SPV: &[u8] = include_bytes!("../shaders/mesh_3d.frag.spv");

/// Push Constants (256 bayt: mat4 vp, mat4 model, mat4 light_space, vec4 light_dir_amb, vec4 light_col, vec4 cam_pos, vec4 pbr_params, u32 bone_count + 3x u32 pad)
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PushConstants3D {
    pub view_proj: [f32; 16],            // 64 bayt
    pub model: [f32; 16],                // 64 bayt
    pub light_space: [f32; 16],          // 64 bayt (Işık uzayı VP matrisi)
    pub light_dir_ambient: [f32; 4],     // 16 bayt (xyz: dir, w: ambient)
    pub light_color_shininess: [f32; 4], // 16 bayt (xyz: color, w: shininess/pbr)
    pub camera_pos_flags: [f32; 4],      // 16 bayt (xyz: pos, w: has_texture)
    pub pbr_params: [f32; 3],            // 12 bayt (x: roughness, y: metallic, z: has_mr_texture)
    pub bone_count: u32,                 // 4 bayt (ciltleme kemik sayısı: 0 ise statik meş)
}

pub struct Pipeline3D {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub dummy_descriptor_set: vk::DescriptorSet,
    pub dummy_buffer: vk::Buffer,
    pub dummy_memory: vk::DeviceMemory,
}

impl Pipeline3D {
    pub fn new(
        device: &ash::Device,
        color_format: vk::Format,
        depth_format: vk::Format,
        default_image_view: vk::ImageView,
        default_sampler: vk::Sampler,
    ) -> Result<Self, String> {
        // 1. Descriptor Set Layout
        // Binding 0: albedo texture (FRAGMENT)
        // Binding 1: metallic-roughness texture (FRAGMENT)
        // Binding 2: shadow map (FRAGMENT)
        // Binding 3: joint matrices SSBO (VERTEX)
        let bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(3)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX),
        ];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

        // SAFETY: Descriptor set layout oluşturulması
        let descriptor_set_layout = unsafe {
            device
                .create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| format!("DescriptorSetLayout3D olusturulamadi: {e}"))?
        };

        // 2. Pipeline Layout (Push Constant: 256 bayt)
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(std::mem::size_of::<PushConstants3D>() as u32);

        let set_layouts = [descriptor_set_layout];
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(std::slice::from_ref(&push_constant_range));

        // SAFETY: Pipeline Layout oluşturulması
        let layout = unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .map_err(|e| format!("PipelineLayout3D olusturulamadi: {e}"))?
        };

        // 3. Shader Modülleri
        let vert_spv = ash::util::read_spv(&mut std::io::Cursor::new(VERT_3D_SPV))
            .map_err(|e| format!("Vertex 3D shader yuklenemedi: {e}"))?;
        let frag_spv = ash::util::read_spv(&mut std::io::Cursor::new(FRAG_3D_SPV))
            .map_err(|e| format!("Fragment 3D shader yuklenemedi: {e}"))?;

        let vert_module = unsafe {
            let create_info = vk::ShaderModuleCreateInfo::default().code(&vert_spv);
            device
                .create_shader_module(&create_info, None)
                .map_err(|e| e.to_string())?
        };

        let frag_module = unsafe {
            let create_info = vk::ShaderModuleCreateInfo::default().code(&frag_spv);
            device
                .create_shader_module(&create_info, None)
                .map_err(|e| e.to_string())?
        };

        let entry_name = c"main";
        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_module)
                .name(entry_name),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_module)
                .name(entry_name),
        ];

        // 4. Vertex Input Binding ve Attributes (Vertex3D: pos, normal, uv, color)
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
            // normal: location 1, vec3 (R32G32B32_SFLOAT)
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(12),
            // uv: location 2, vec2 (R32G32_SFLOAT)
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(24),
            // color: location 3, vec4 (R32G32B32A32_SFLOAT)
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(3)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(32),
            // joint_indices: location 4, uvec4 (R16G16B16A16_UINT)
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(4)
                .format(vk::Format::R16G16B16A16_UINT)
                .offset(48),
            // joint_weights: location 5, vec4 (R32G32B32A32_SFLOAT)
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(5)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(56),
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

        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // Depth Stencil State (Aktif Depth Test & Depth Write)
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD);

        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(std::slice::from_ref(&color_blend_attachment));

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        // Dynamic Rendering bilgisi
        let color_attachment_formats = [color_format];
        let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&color_attachment_formats)
            .depth_attachment_format(depth_format);

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .depth_stencil_state(&depth_stencil)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state_info)
            .layout(layout)
            .push_next(&mut rendering_info);

        // SAFETY: 3D Graphics Pipeline oluşturulması
        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|(_, e)| format!("GraphicsPipeline3D olusturulamadi: {e}"))?[0]
        };

        unsafe {
            device.destroy_shader_module(vert_module, None);
            device.destroy_shader_module(frag_module, None);
        }

        // 5. Descriptor Pool ve Varsayılan Doku Seti
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(24),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(8),
        ];

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(8);

        let descriptor_pool = unsafe {
            device
                .create_descriptor_pool(&pool_info, None)
                .map_err(|e| format!("DescriptorPool3D olusturulamadi: {e}"))?
        };

        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(std::slice::from_ref(&descriptor_set_layout));

        let dummy_descriptor_set = unsafe {
            device
                .allocate_descriptor_sets(&alloc_info)
                .map_err(|e| format!("DescriptorSet3D allocate basarisiz: {e}"))?[0]
        };

        // Varsayılan doku bilgisini bağla (Binding 0: Albedo, Binding 1: Metallic-Roughness, Binding 2: Shadow Map)
        let image_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(default_image_view)
            .sampler(default_sampler);

        // Dummy SSBO oluştur (128 Mat4 = 8192 bayt)
        let dummy_buffer_info = vk::BufferCreateInfo::default()
            .size(8192)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let dummy_buffer = unsafe {
            device
                .create_buffer(&dummy_buffer_info, None)
                .map_err(|e| format!("Dummy joint buffer olusturulamadi: {e}"))?
        };

        let mem_reqs = unsafe { device.get_buffer_memory_requirements(dummy_buffer) };
        let mem_prop = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_reqs.size)
            .memory_type_index(0);

        // SAFETY: Dummy joint buffer belleği (geçici host visible tahsis)
        let dummy_memory = unsafe {
            device
                .allocate_memory(&mem_prop, None)
                .map_err(|e| format!("Dummy joint memory allocate basarisiz: {e}"))?
        };

        unsafe {
            device
                .bind_buffer_memory(dummy_buffer, dummy_memory, 0)
                .map_err(|e| format!("Dummy joint memory baglanamadi: {e}"))?;
        }

        let buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(dummy_buffer)
            .offset(0)
            .range(8192);

        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(dummy_descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&image_info)),
            vk::WriteDescriptorSet::default()
                .dst_set(dummy_descriptor_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&image_info)),
            vk::WriteDescriptorSet::default()
                .dst_set(dummy_descriptor_set)
                .dst_binding(2)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&image_info)),
            vk::WriteDescriptorSet::default()
                .dst_set(dummy_descriptor_set)
                .dst_binding(3)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(std::slice::from_ref(&buffer_info)),
        ];

        unsafe {
            device.update_descriptor_sets(&writes, &[]);
        }

        Ok(Self {
            pipeline,
            layout,
            descriptor_set_layout,
            descriptor_pool,
            dummy_descriptor_set,
            dummy_buffer,
            dummy_memory,
        })
    }

    /// 3D Mesh çizimi gerçekleştirir
    #[allow(clippy::too_many_arguments)]
    pub fn draw_mesh(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        camera: &Camera3D,
        mesh: &GpuMesh3D,
        model_matrix: Mat4,
        sun: &DirectionalLight,
        ambient: &AmbientLight,
        has_texture: bool,
        extent: vk::Extent2D,
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

            // Push Constants hazırla
            let vp = camera.view_projection();
            let light_space = sun.light_space_matrix(glam::Vec3::ZERO, 15.0, 12.0, 0.1, 40.0);
            let push_data = PushConstants3D {
                view_proj: vp.to_cols_array(),
                model: model_matrix.to_cols_array(),
                light_space: light_space.to_cols_array(),
                light_dir_ambient: [
                    sun.direction.x,
                    sun.direction.y,
                    sun.direction.z,
                    ambient.intensity,
                ],
                light_color_shininess: [sun.color[0], sun.color[1], sun.color[2], 32.0],
                camera_pos_flags: [
                    camera.eye.x,
                    camera.eye.y,
                    camera.eye.z,
                    if has_texture { 1.0 } else { 0.0 },
                ],
                pbr_params: [0.5, 0.0, 0.0],
                bone_count: 0,
            };

            let push_bytes = bytemuck::bytes_of(&push_data);
            device.cmd_push_constants(
                cmd,
                self.layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                push_bytes,
            );

            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.layout,
                0,
                &[self.dummy_descriptor_set],
                &[],
            );

            mesh.draw(device, cmd);
        }
    }

    /// PBR parametreleri (Roughness, Metallic) ile 3D Mesh çizimi gerçekleştirir (Varsayılan tam ekran)
    #[allow(clippy::too_many_arguments)]
    pub fn draw_mesh_pbr(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        camera: &Camera3D,
        mesh: &GpuMesh3D,
        model_matrix: Mat4,
        sun: &DirectionalLight,
        ambient: &AmbientLight,
        roughness: f32,
        metallic: f32,
        has_texture: bool,
        extent: vk::Extent2D,
    ) {
        self.draw_mesh_pbr_viewport(
            device,
            cmd,
            camera,
            mesh,
            model_matrix,
            sun,
            ambient,
            roughness,
            metallic,
            has_texture,
            0.0,
            0.0,
            extent.width as f32,
            extent.height as f32,
        );
    }

    /// PBR parametreleri ile belirtilen Viewport / Scissor dikdörtgeninde 3D Mesh çizimi gerçekleştirir
    #[allow(clippy::too_many_arguments)]
    pub fn draw_mesh_pbr_viewport(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        camera: &Camera3D,
        mesh: &GpuMesh3D,
        model_matrix: Mat4,
        sun: &DirectionalLight,
        ambient: &AmbientLight,
        roughness: f32,
        metallic: f32,
        has_texture: bool,
        vp_x: f32,
        vp_y: f32,
        vp_w: f32,
        vp_h: f32,
    ) {
        self.draw_mesh_pbr_viewport_ext(
            device,
            cmd,
            camera,
            mesh,
            model_matrix,
            sun,
            ambient,
            roughness,
            metallic,
            has_texture,
            false,
            None,
            vp_x,
            vp_y,
            vp_w,
            vp_h,
        );
    }

    /// PBR materyal ve doku seti ile detaylı Viewport / Scissor çizimi
    #[allow(clippy::too_many_arguments)]
    pub fn draw_mesh_pbr_viewport_ext(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        camera: &Camera3D,
        mesh: &GpuMesh3D,
        model_matrix: Mat4,
        sun: &DirectionalLight,
        ambient: &AmbientLight,
        roughness: f32,
        metallic: f32,
        has_texture: bool,
        has_mr_texture: bool,
        descriptor_set: Option<vk::DescriptorSet>,
        vp_x: f32,
        vp_y: f32,
        vp_w: f32,
        vp_h: f32,
    ) {
        unsafe {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);

            let viewport = vk::Viewport {
                x: vp_x,
                y: vp_y + vp_h,
                width: vp_w,
                height: -vp_h,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            device.cmd_set_viewport(cmd, 0, &[viewport]);

            let scissor = vk::Rect2D {
                offset: vk::Offset2D {
                    x: vp_x.max(0.0) as i32,
                    y: vp_y.max(0.0) as i32,
                },
                extent: vk::Extent2D {
                    width: vp_w.max(1.0) as u32,
                    height: vp_h.max(1.0) as u32,
                },
            };
            device.cmd_set_scissor(cmd, 0, &[scissor]);

            let vp = camera.view_projection();
            let light_space = sun.light_space_matrix(glam::Vec3::ZERO, 15.0, 12.0, 0.1, 40.0);
            let push_data = PushConstants3D {
                view_proj: vp.to_cols_array(),
                model: model_matrix.to_cols_array(),
                light_space: light_space.to_cols_array(),
                light_dir_ambient: [
                    sun.direction.x,
                    sun.direction.y,
                    sun.direction.z,
                    ambient.intensity,
                ],
                light_color_shininess: [sun.color[0], sun.color[1], sun.color[2], 32.0],
                camera_pos_flags: [
                    camera.eye.x,
                    camera.eye.y,
                    camera.eye.z,
                    if has_texture { 1.0 } else { 0.0 },
                ],
                pbr_params: [roughness, metallic, if has_mr_texture { 1.0 } else { 0.0 }],
                bone_count: 0,
            };

            let push_bytes = bytemuck::bytes_of(&push_data);
            device.cmd_push_constants(
                cmd,
                self.layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                push_bytes,
            );

            let active_set = descriptor_set.unwrap_or(self.dummy_descriptor_set);
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.layout,
                0,
                &[active_set],
                &[],
            );

            mesh.draw(device, cmd);
        }
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_buffer(self.dummy_buffer, None);
            device.free_memory(self.dummy_memory, None);
        }
    }
}
