//! 2D Instanced Quad Render Pipeline (Vulkan 1.3 Dynamic Rendering).
//!
//! 32-baytlık `QuadInstance` yapısı, persistent-mapped frames-in-flight ring upload buffer
//! ve tek bir `vkCmdDraw(6, N)` çağrısıyla 50k-200k sprite çizimi sağlar.

use ash::vk;
use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;
use std::ptr::NonNull;

pub const VERT_SPV: &[u8] = include_bytes!("../shaders/quad.vert.spv");
pub const FRAG_SPV: &[u8] = include_bytes!("../shaders/quad.frag.spv");

pub const MAX_QUADS_PER_FRAME: usize = 500_000;
pub const QUAD_INSTANCE_SIZE: usize = std::mem::size_of::<QuadInstance>(); // 32 bayt
pub const BUFFER_SIZE_PER_FRAME: usize = MAX_QUADS_PER_FRAME * QUAD_INSTANCE_SIZE; // 16 MB

/// 32-Baytlık Sıkıştırılmış QuadInstance
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct QuadInstance {
    pub pos: [f32; 2],     // 8 bayt (x, y)
    pub size: [f32; 2],    // 8 bayt (width, height)
    pub uv_rect: [u32; 2], // 8 bayt (uv_min_packed, uv_size_packed)
    pub color: u32,        // 4 bayt (paketlenmiş RGBA8)
    pub tex_id: u32,       // 4 bayt (atlas slot: 0=dokusuz, 1+=atlas)
}

const _: () = assert!(
    std::mem::size_of::<QuadInstance>() == 32,
    "QuadInstance 32 bayt olmalidir!"
);

impl QuadInstance {
    #[inline(always)]
    pub fn pack_tex_and_rotation(tex_id: u32, rotation: f32) -> u32 {
        let norm = rotation.rem_euclid(std::f32::consts::TAU);
        let quant = ((norm / std::f32::consts::TAU) * 65535.0).round() as u32;
        (tex_id & 0xFFFF) | ((quant & 0xFFFF) << 16)
    }

    #[inline(always)]
    pub fn new(pos: [f32; 2], size: [f32; 2], color_rgba: [u8; 4], tex_id: u32) -> Self {
        let color = u32::from_le_bytes(color_rgba);
        Self {
            pos,
            size,
            uv_rect: [0, 0],
            color,
            tex_id,
        }
    }

    #[inline(always)]
    pub fn new_rotated(
        pos: [f32; 2],
        size: [f32; 2],
        color_rgba: [u8; 4],
        tex_id: u32,
        rotation: f32,
    ) -> Self {
        let color = u32::from_le_bytes(color_rgba);
        Self {
            pos,
            size,
            uv_rect: [0, 0],
            color,
            tex_id: Self::pack_tex_and_rotation(tex_id, rotation),
        }
    }

    #[inline(always)]
    pub fn with_uv(
        pos: [f32; 2],
        size: [f32; 2],
        uv_rect: [u32; 2],
        color_rgba: [u8; 4],
        tex_id: u32,
    ) -> Self {
        let color = u32::from_le_bytes(color_rgba);
        Self {
            pos,
            size,
            uv_rect,
            color,
            tex_id,
        }
    }

    #[inline(always)]
    pub fn with_uv_rotated(
        pos: [f32; 2],
        size: [f32; 2],
        uv_rect: [u32; 2],
        color_rgba: [u8; 4],
        tex_id: u32,
        rotation: f32,
    ) -> Self {
        let color = u32::from_le_bytes(color_rgba);
        Self {
            pos,
            size,
            uv_rect,
            color,
            tex_id: Self::pack_tex_and_rotation(tex_id, rotation),
        }
    }
}

pub struct RingBufferFrame {
    pub buffer: vk::Buffer,
    pub allocation: Allocation,
    pub mapped_ptr: NonNull<u8>,
    pub descriptor_set: vk::DescriptorSet,
}

pub struct Pipeline2D {
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub frames: Vec<RingBufferFrame>,
    pub allocator: std::sync::Arc<std::sync::Mutex<Allocator>>,
}

impl Pipeline2D {
    pub fn new(
        device: &ash::Device,
        allocator: std::sync::Arc<std::sync::Mutex<Allocator>>,
        color_format: vk::Format,
        frames_in_flight: usize,
        atlas_view: vk::ImageView,
        atlas_sampler: vk::Sampler,
    ) -> Result<Self, String> {
        Self::new_with_depth(
            device,
            allocator,
            color_format,
            None,
            frames_in_flight,
            atlas_view,
            atlas_sampler,
        )
    }

    pub fn new_with_depth(
        device: &ash::Device,
        allocator: std::sync::Arc<std::sync::Mutex<Allocator>>,
        color_format: vk::Format,
        depth_format: Option<vk::Format>,
        frames_in_flight: usize,
        atlas_view: vk::ImageView,
        atlas_sampler: vk::Sampler,
    ) -> Result<Self, String> {
        // 1. Descriptor Set Layout (Binding 0: SSBO InstanceBuffer, Binding 1: Sampler u_atlas)
        let bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

        // SAFETY: Descriptor set layout oluşturulması
        let descriptor_set_layout = unsafe {
            device
                .create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| format!("DescriptorSetLayout olusturulamadi: {e}"))?
        };

        // 2. Pipeline Layout (Push Constant: mat4 view_proj, 64 bayt)
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<Mat4>() as u32);

        let set_layouts = [descriptor_set_layout];
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(std::slice::from_ref(&push_constant_range));

        // SAFETY: Pipeline layout oluşturulması
        let pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .map_err(|e| format!("PipelineLayout olusturulamadi: {e}"))?
        };

        // 3. Shader Modülleri (4-bayt hizalı güvenli SPIR-V yüklemesi)
        let vert_spv = ash::util::read_spv(&mut std::io::Cursor::new(VERT_SPV))
            .map_err(|e| format!("Vertex shader yuklenemedi: {e}"))?;
        let frag_spv = ash::util::read_spv(&mut std::io::Cursor::new(FRAG_SPV))
            .map_err(|e| format!("Fragment shader yuklenemedi: {e}"))?;

        // SAFETY: SPIR-V bytecode'larından shader modülleri oluşturulması
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

        // 4. Vertex Input (SSBO ve procedural quad kullanıldığından boş input)
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default();

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
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // Alpha Blending (src * alpha + dst * (1 - alpha))
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

        // Vulkan 1.3 Dynamic Rendering Attachment Bilgisi
        let color_attachment_formats = [color_format];
        let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&color_attachment_formats);

        if let Some(depth_fmt) = depth_format {
            rendering_info = rendering_info.depth_attachment_format(depth_fmt);
        }

        // Depth Stencil State: Derinlik tamponu ile birlikte 2D/UI çizilirken derinlik testi ve yazma kapalıdır.
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_compare_op(vk::CompareOp::ALWAYS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        let mut pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .push_next(&mut rendering_info);

        if depth_format.is_some() {
            pipeline_info = pipeline_info.depth_stencil_state(&depth_stencil);
        }

        // SAFETY: Graphics pipeline oluşturulması
        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|(_, e)| format!("GraphicsPipeline olusturulamadi: {e}"))?[0]
        };

        // Shader modüllerini temizle
        unsafe {
            device.destroy_shader_module(vert_module, None);
            device.destroy_shader_module(frag_module, None);
        }

        // 5. Descriptor Pool ve Ring Buffer Frame'leri
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(frames_in_flight as u32),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(frames_in_flight as u32),
        ];

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(frames_in_flight as u32);

        // SAFETY: Descriptor pool oluşturulması
        let descriptor_pool = unsafe {
            device
                .create_descriptor_pool(&pool_info, None)
                .map_err(|e| format!("DescriptorPool olusturulamadi: {e}"))?
        };

        let mut frames = Vec::with_capacity(frames_in_flight);

        for _ in 0..frames_in_flight {
            // Buffer oluştur
            let buffer_info = vk::BufferCreateInfo::default()
                .size(BUFFER_SIZE_PER_FRAME as u64)
                .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            // SAFETY: Buffer oluşturulması
            let buffer = unsafe {
                device
                    .create_buffer(&buffer_info, None)
                    .map_err(|e| format!("Storage buffer olusturulamadi: {e}"))?
            };

            // gpu-allocator ile persistent mapped tahsis
            let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
            let allocation = {
                let mut alloc = allocator.lock().unwrap();
                alloc
                    .allocate(&AllocationCreateDesc {
                        name: "2D Quad Ring Buffer",
                        requirements,
                        location: MemoryLocation::CpuToGpu,
                        linear: true,
                        allocation_scheme: AllocationScheme::GpuAllocatorManaged,
                    })
                    .map_err(|e| format!("Bellek tahsisi basarisiz: {e}"))?
            };

            // SAFETY: Bellek bağlama
            unsafe {
                device
                    .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                    .map_err(|e| format!("Buffer bellegi baglanamadi: {e}"))?;
            }

            let mapped_ptr = allocation
                .mapped_ptr()
                .ok_or("Buffer persistent mapped olarak eslenemedi")?
                .cast::<u8>();

            // Descriptor set tahsisi
            let set_layouts = [descriptor_set_layout];
            let alloc_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&set_layouts);

            // SAFETY: Descriptor set tahsisi ve güncellemesi
            let descriptor_set = unsafe {
                let set = device
                    .allocate_descriptor_sets(&alloc_info)
                    .map_err(|e| format!("DescriptorSet tahsis edilemedi: {e}"))?[0];

                let buffer_info = vk::DescriptorBufferInfo::default()
                    .buffer(buffer)
                    .offset(0)
                    .range(BUFFER_SIZE_PER_FRAME as u64);

                let image_info = vk::DescriptorImageInfo::default()
                    .sampler(atlas_sampler)
                    .image_view(atlas_view)
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

                let writes = [
                    vk::WriteDescriptorSet::default()
                        .dst_set(set)
                        .dst_binding(0)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(std::slice::from_ref(&buffer_info)),
                    vk::WriteDescriptorSet::default()
                        .dst_set(set)
                        .dst_binding(1)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(std::slice::from_ref(&image_info)),
                ];

                device.update_descriptor_sets(&writes, &[]);
                set
            };

            frames.push(RingBufferFrame {
                buffer,
                allocation,
                mapped_ptr,
                descriptor_set,
            });
        }

        Ok(Self {
            pipeline_layout,
            pipeline,
            descriptor_set_layout,
            descriptor_pool,
            frames,
            allocator,
        })
    }

    /// CPU verisini persistent-mapped ring buffer'a tek bir kopyalama ile aktarır.
    ///
    /// # Safety
    /// Bu kareye ait GPU fence'i geçmiş olmalıdır (karenin tamamlanmış olduğu garanti edilir).
    #[inline]
    pub unsafe fn upload_instances(
        &mut self,
        frame_index: usize,
        instances: &[QuadInstance],
    ) -> usize {
        let count = instances.len().min(MAX_QUADS_PER_FRAME);
        if count == 0 {
            return 0;
        }

        let frame = &self.frames[frame_index];
        let dst = frame.mapped_ptr.as_ptr() as *mut QuadInstance;
        std::ptr::copy_nonoverlapping(instances.as_ptr(), dst, count);
        count
    }

    /// Instanced quad çizimini kaydeder: tek bir `vkCmdDraw(6, N)`
    pub fn draw(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame_index: usize,
        count: u32,
        view_proj: Mat4,
        viewport_extent: vk::Extent2D,
    ) {
        if count == 0 {
            return;
        }

        let frame = &self.frames[frame_index];

        // Dynamic Viewport ve Scissor
        let viewport = vk::Viewport::default()
            .x(0.0)
            .y(0.0)
            .width(viewport_extent.width as f32)
            .height(viewport_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = vk::Rect2D::default().extent(viewport_extent);

        // SAFETY: 2D çizim komutları kaydı
        unsafe {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
            device.cmd_set_viewport(cmd, 0, &[viewport]);
            device.cmd_set_scissor(cmd, 0, &[scissor]);

            // Push Constant (Camera2D View-Proj matrisi)
            let cols = view_proj.to_cols_array();
            let pc_bytes: &[u8] = bytemuck::bytes_of(&cols);
            device.cmd_push_constants(
                cmd,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                pc_bytes,
            );

            // Descriptor set (InstanceBuffer SSBO)
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[frame.descriptor_set],
                &[],
            );

            // TEK DRAW CALL: 6 vertex, N instance!
            device.cmd_draw(cmd, 6, count, 0, 0);
        }
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        // SAFETY: Kaynakların temizlenmesi
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            let mut alloc = self.allocator.lock().unwrap();
            for frame in self.frames.drain(..) {
                device.destroy_buffer(frame.buffer, None);
                let _ = alloc.free(frame.allocation);
            }
        }
    }
}
