//! 3D Derinlik Tamponu (Depth Buffer: D32_SFLOAT) Yönetimi.

use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};
use gpu_allocator::MemoryLocation;
use std::sync::{Arc, Mutex};

pub const DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;

pub struct DepthBuffer {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub allocation: Option<Allocation>,
    pub width: u32,
    pub height: u32,
    pub format: vk::Format,
}

impl DepthBuffer {
    pub fn new(
        device: &ash::Device,
        allocator: Arc<Mutex<Allocator>>,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let format = DEPTH_FORMAT;

        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width: width.max(1),
                height: height.max(1),
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        // SAFETY: Depth VkImage oluşturulması
        let image = unsafe {
            device
                .create_image(&image_info, None)
                .map_err(|e| format!("Depth VkImage olusturulamadi: {e}"))?
        };

        // GPU Allocator ile bellek tahsisi
        let requirements = unsafe { device.get_image_memory_requirements(image) };
        let mut alloc = allocator.lock().unwrap();
        let allocation = alloc
            .allocate(&AllocationCreateDesc {
                name: "DepthBuffer_D32_SFLOAT",
                requirements,
                location: MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| format!("Depth bellek tahsis edilemedi: {e}"))?;

        // SAFETY: Belleğin resme bağlanması
        unsafe {
            device
                .bind_image_memory(image, allocation.memory(), allocation.offset())
                .map_err(|e| format!("Depth bellek baglanamadi: {e}"))?;
        }
        drop(alloc);

        // Depth ImageView oluşturulması
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        // SAFETY: Depth ImageView oluşturulması
        let view = unsafe {
            device
                .create_image_view(&view_info, None)
                .map_err(|e| format!("Depth ImageView olusturulamadi: {e}"))?
        };

        Ok(Self {
            image,
            view,
            allocation: Some(allocation),
            width: width.max(1),
            height: height.max(1),
            format,
        })
    }

    pub fn destroy(&mut self, device: &ash::Device, allocator: &mut Allocator) {
        unsafe {
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
        }
        if let Some(alloc) = self.allocation.take() {
            let _ = allocator.free(alloc);
        }
    }
}

/// Yönlü Işık İçin Özel Gölge Haritası (Directional Shadow Map).
/// Derinlik eki (DEPTH_STENCIL_ATTACHMENT) ve shader örnekleme (SAMPLED) özelliklerine sahiptir.
pub struct DirectionalShadowMap {
    pub image: vk::Image,
    pub view: vk::ImageView, // TYPE_2D_ARRAY (4 layers) for shader sampling
    pub layer_views: [vk::ImageView; 4], // TYPE_2D per cascade for depth render attachments
    pub allocation: Option<Allocation>,
    pub resolution: u32,
    pub format: vk::Format,
}

impl DirectionalShadowMap {
    pub const DEFAULT_RESOLUTION: u32 = 2048;
    pub const CASCADE_COUNT: u32 = 4;

    pub fn new(
        device: &ash::Device,
        allocator: Arc<Mutex<Allocator>>,
        resolution: u32,
    ) -> Result<Self, String> {
        let format = DEPTH_FORMAT;
        let res = resolution.max(256);

        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width: res,
                height: res,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(Self::CASCADE_COUNT)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        // SAFETY: Shadow Map VkImage oluşturulması
        let image = unsafe {
            device
                .create_image(&image_info, None)
                .map_err(|e| format!("ShadowMap VkImage olusturulamadi: {e}"))?
        };

        let requirements = unsafe { device.get_image_memory_requirements(image) };
        let mut alloc = allocator.lock().unwrap();
        let allocation = alloc
            .allocate(&AllocationCreateDesc {
                name: "ShadowMap_CSM_D32_SFLOAT",
                requirements,
                location: MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| format!("ShadowMap bellek tahsis edilemedi: {e}"))?;

        // SAFETY: Belleğin resme bağlanması
        unsafe {
            device
                .bind_image_memory(image, allocation.memory(), allocation.offset())
                .map_err(|e| format!("ShadowMap bellek baglanamadi: {e}"))?;
        }
        drop(alloc);

        // 1. Shader örnekleme için TYPE_2D_ARRAY (4 katman birden)
        let array_view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D_ARRAY)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: Self::CASCADE_COUNT,
            });

        let view = unsafe {
            device
                .create_image_view(&array_view_info, None)
                .map_err(|e| format!("ShadowMap Array ImageView olusturulamadi: {e}"))?
        };

        // 2. Her bir kaskadın derinlik eki olarak renderlanması için tekil TYPE_2D görünümleri
        let mut layer_views = [vk::ImageView::null(); 4];
        for (i, layer_view) in layer_views.iter_mut().enumerate() {
            let single_view_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::DEPTH,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: i as u32,
                    layer_count: 1,
                });

            *layer_view = unsafe {
                device
                    .create_image_view(&single_view_info, None)
                    .map_err(|e| format!("ShadowMap Layer {} ImageView olusturulamadi: {e}", i))?
            };
        }

        Ok(Self {
            image,
            view,
            layer_views,
            allocation: Some(allocation),
            resolution: res,
            format,
        })
    }

    pub fn destroy(&mut self, device: &ash::Device, allocator: &mut Allocator) {
        unsafe {
            for lv in self.layer_views {
                if lv != vk::ImageView::null() {
                    device.destroy_image_view(lv, None);
                }
            }
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
        }
        if let Some(alloc) = self.allocation.take() {
            let _ = allocator.free(alloc);
        }
    }
}

pub const HDR_COLOR_FORMAT: vk::Format = vk::Format::R16G16B16A16_SFLOAT;

/// HDR Offscreen Render Hedefi (R16G16B16A16_SFLOAT Renk + D32_SFLOAT Derinlik)
pub struct HdrRenderTarget {
    pub color_image: vk::Image,
    pub color_view: vk::ImageView,
    pub color_alloc: Option<Allocation>,
    pub depth_buffer: DepthBuffer,
    pub width: u32,
    pub height: u32,
    pub color_format: vk::Format,
}

impl HdrRenderTarget {
    pub fn new(
        device: &ash::Device,
        allocator: Arc<Mutex<Allocator>>,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let w = width.max(1);
        let h = height.max(1);
        let color_format = HDR_COLOR_FORMAT;

        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(color_format)
            .extent(vk::Extent3D {
                width: w,
                height: h,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        // SAFETY: HDR VkImage oluşturulması
        let color_image = unsafe {
            device
                .create_image(&image_info, None)
                .map_err(|e| format!("HDR color image olusturulamadi: {e}"))?
        };

        let requirements = unsafe { device.get_image_memory_requirements(color_image) };
        let mut alloc = allocator.lock().unwrap();
        let color_alloc = alloc
            .allocate(&AllocationCreateDesc {
                name: "HdrRenderTarget_R16G16B16A16_SFLOAT",
                requirements,
                location: MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| format!("HDR color bellek tahsis edilemedi: {e}"))?;

        // SAFETY: HDR bellek bağlama
        unsafe {
            device
                .bind_image_memory(color_image, color_alloc.memory(), color_alloc.offset())
                .map_err(|e| format!("HDR color bellek baglanamadi: {e}"))?;
        }
        drop(alloc);

        let view_info = vk::ImageViewCreateInfo::default()
            .image(color_image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(color_format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let color_view = unsafe {
            device
                .create_image_view(&view_info, None)
                .map_err(|e| format!("HDR color ImageView olusturulamadi: {e}"))?
        };

        let depth_buffer = DepthBuffer::new(device, Arc::clone(&allocator), w, h)?;

        Ok(Self {
            color_image,
            color_view,
            color_alloc: Some(color_alloc),
            depth_buffer,
            width: w,
            height: h,
            color_format,
        })
    }

    /// HDR rengi COLOR_ATTACHMENT durumuna geçirir
    pub fn transition_to_color_attachment(&self, device: &ash::Device, cmd: vk::CommandBuffer) {
        let barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
            .src_access_mask(vk::AccessFlags2::SHADER_READ)
            .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image(self.color_image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let dependency =
            vk::DependencyInfo::default().image_memory_barriers(std::slice::from_ref(&barrier));
        unsafe {
            device.cmd_pipeline_barrier2(cmd, &dependency);
        }
    }

    /// HDR rengi tonemap geçişi için SHADER_READ durumuna geçirir
    pub fn transition_to_shader_read(&self, device: &ash::Device, cmd: vk::CommandBuffer) {
        let barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
            .dst_access_mask(vk::AccessFlags2::SHADER_READ)
            .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image(self.color_image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let dependency =
            vk::DependencyInfo::default().image_memory_barriers(std::slice::from_ref(&barrier));
        unsafe {
            device.cmd_pipeline_barrier2(cmd, &dependency);
        }
    }

    pub fn destroy(&mut self, device: &ash::Device, allocator: &mut Allocator) {
        unsafe {
            device.destroy_image_view(self.color_view, None);
            device.destroy_image(self.color_image, None);
        }
        if let Some(alloc) = self.color_alloc.take() {
            let _ = allocator.free(alloc);
        }
        self.depth_buffer.destroy(device, allocator);
    }
}
