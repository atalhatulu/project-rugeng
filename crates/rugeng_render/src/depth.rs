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
    pub view: vk::ImageView,
    pub allocation: Option<Allocation>,
    pub resolution: u32,
    pub format: vk::Format,
}

impl DirectionalShadowMap {
    pub const DEFAULT_RESOLUTION: u32 = 2048;

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
            .array_layers(1)
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
                name: "ShadowMap_D32_SFLOAT",
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

        // Shadow Map ImageView oluşturulması
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

        let view = unsafe {
            device
                .create_image_view(&view_info, None)
                .map_err(|e| format!("ShadowMap ImageView olusturulamadi: {e}"))?
        };

        Ok(Self {
            image,
            view,
            allocation: Some(allocation),
            resolution: res,
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
