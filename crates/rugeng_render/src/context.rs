//! Vulkan 1.3 Donanım Katmanı (RHI Context).
//!
//! Dynamic Rendering ve Synchronization2 standartlarını kullanır.
//! RenderPass ve Framebuffer külfeti olmadan, swapchain ve resize yönetimi sağlar.

use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub const FRAMES_IN_FLIGHT: usize = 2;

pub struct Swapchain {
    pub loader: ash::khr::swapchain::Device,
    pub handle: vk::SwapchainKHR,
    pub format: vk::Format,
    pub extent: vk::Extent2D,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
}

pub struct VulkanContext {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub surface_loader: ash::khr::surface::Instance,
    pub surface: vk::SurfaceKHR,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub graphics_queue: vk::Queue,
    pub graphics_queue_family: u32,
    pub swapchain: Swapchain,
    pub command_pool: vk::CommandPool,
    pub command_buffers: [vk::CommandBuffer; FRAMES_IN_FLIGHT],
    pub image_available_semaphores: [vk::Semaphore; FRAMES_IN_FLIGHT],
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: [vk::Fence; FRAMES_IN_FLIGHT],
    pub current_frame: usize,
    pub resized: bool,
    pub validation_error_count: std::sync::Arc<std::sync::atomic::AtomicU32>,
    pub allocator:
        std::mem::ManuallyDrop<std::sync::Arc<std::sync::Mutex<gpu_allocator::vulkan::Allocator>>>,
}

impl VulkanContext {
    pub fn new<W: HasWindowHandle + HasDisplayHandle>(
        window: &W,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let entry = unsafe {
            ash::Entry::load().map_err(|e| format!("Vulkan kutuphanesi yuklenemedi: {e}"))?
        };

        // 1. Instance Kurulumu
        let display_handle = window.display_handle().map_err(|e| e.to_string())?.as_raw();
        let window_handle = window.window_handle().map_err(|e| e.to_string())?.as_raw();

        let extension_names = ash_window::enumerate_required_extensions(display_handle)
            .map_err(|e| format!("Pencere uzantilari alinamadi: {e}"))?
            .to_vec();

        let available_layers = unsafe {
            entry
                .enumerate_instance_layer_properties()
                .unwrap_or_default()
        };
        let has_validation = available_layers.iter().any(|l| {
            let name = unsafe { std::ffi::CStr::from_ptr(l.layer_name.as_ptr()) };
            name.to_bytes() == b"VK_LAYER_KHRONOS_validation"
        });

        let mut layer_names = Vec::new();
        if has_validation {
            println!(
                "[Vulkan] VK_LAYER_KHRONOS_validation katmani tespit edildi ve aktiflestirildi."
            );
            layer_names.push(c"VK_LAYER_KHRONOS_validation".as_ptr());
        } else {
            println!("[Vulkan] VK_LAYER_KHRONOS_validation bulunamadi, katmansiz devam ediliyor.");
        }

        let app_info = vk::ApplicationInfo::default()
            .application_name(c"Rugeng Engine")
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(c"Ember")
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::make_api_version(0, 1, 3, 0));

        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names)
            .enabled_layer_names(&layer_names);

        // SAFETY: Instance oluşturulması
        let instance = unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .map_err(|e| format!("Vulkan Instance olusturulamadi: {e}"))?
        };

        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);
        // SAFETY: Pencere yuzeyinin olusturulmasi
        let surface = unsafe {
            ash_window::create_surface(&entry, &instance, display_handle, window_handle, None)
                .map_err(|e| format!("Surface olusturulamadi: {e}"))?
        };

        // 2. Physical Device Seçimi (Discrete Önceliği, ENGINE_GPU desteği)
        // SAFETY: Fiziksel aygıtların sorgulanması
        let physical_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .map_err(|e| format!("Fiziksel aygitlar sorgulanamadi: {e}"))?
        };

        let env_gpu_filter = std::env::var("ENGINE_GPU").ok();
        println!("[Vulkan] Sistemde bulunan grafik aygitlari:");

        let mut candidate_gpus = Vec::new();

        for (device_idx, &pdev) in physical_devices.iter().enumerate() {
            // SAFETY: Aygıt özelliklerinin sorgulanması
            let props = unsafe { instance.get_physical_device_properties(pdev) };
            let raw_name = unsafe { std::ffi::CStr::from_ptr(props.device_name.as_ptr()) };
            let device_name = raw_name.to_str().unwrap_or("Bilinmeyen GPU").to_string();
            let device_type = props.device_type;

            let type_str = match device_type {
                vk::PhysicalDeviceType::DISCRETE_GPU => "Ayrık GPU (DISCRETE)",
                vk::PhysicalDeviceType::INTEGRATED_GPU => "Tümleşik GPU (INTEGRATED)",
                vk::PhysicalDeviceType::VIRTUAL_GPU => "Sanal GPU (VIRTUAL)",
                vk::PhysicalDeviceType::CPU => "Yazılımsal CPU",
                _ => "Diğer",
            };

            println!(
                "  [{}] {} | Tip: {} | API: {}.{}.{}",
                device_idx,
                device_name,
                type_str,
                vk::api_version_major(props.api_version),
                vk::api_version_minor(props.api_version),
                vk::api_version_patch(props.api_version)
            );

            // Kuyruk ailesi desteğini kontrol et
            let queue_families =
                unsafe { instance.get_physical_device_queue_family_properties(pdev) };
            let mut supported_family = None;

            for (idx, qf) in queue_families.iter().enumerate() {
                let supports_graphics = qf.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                let supports_present = unsafe {
                    surface_loader
                        .get_physical_device_surface_support(pdev, idx as u32, surface)
                        .unwrap_or(false)
                };

                if supports_graphics && supports_present {
                    supported_family = Some(idx as u32);
                    break;
                }
            }

            if let Some(queue_family) = supported_family {
                // Öncelik puanı (Discrete > Integrated > Other)
                let score = match device_type {
                    vk::PhysicalDeviceType::DISCRETE_GPU => 1000,
                    vk::PhysicalDeviceType::INTEGRATED_GPU => 500,
                    vk::PhysicalDeviceType::VIRTUAL_GPU => 250,
                    _ => 10,
                };
                candidate_gpus.push((pdev, device_name, device_type, queue_family, score));
            }
        }

        if candidate_gpus.is_empty() {
            return Err(
                "Grafik ve sunum kuyruguna sahip uygun Vulkan aygiti bulunamadi".to_string(),
            );
        }

        // ENGINE_GPU filtre kontrolü
        let selected = if let Some(filter) = &env_gpu_filter {
            candidate_gpus
                .iter()
                .find(|(_, name, _, _, _)| name.to_lowercase().contains(&filter.to_lowercase()))
                .or_else(|| candidate_gpus.first())
        } else {
            // En yüksek puanlı GPU'yu seç (Discrete GPU öncelikli)
            candidate_gpus
                .iter()
                .max_by_key(|(_, _, _, _, score)| *score)
        };

        let (physical_device, selected_name, selected_type, selected_queue_family, _) =
            selected.cloned().unwrap();

        println!(
            "[Vulkan] Aktif Secilen Aygit: {} ({:?})",
            selected_name, selected_type
        );

        // 3. Mantıksal Aygıt (Device) ve Vulkan 1.3 Özellikleri
        let queue_priorities = [1.0f32];
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(selected_queue_family)
            .queue_priorities(&queue_priorities);

        let device_extensions = [ash::khr::swapchain::NAME.as_ptr()];

        // Vulkan 1.3 Dynamic Rendering ve Synchronization2
        let mut vulkan_13_features = vk::PhysicalDeviceVulkan13Features::default()
            .dynamic_rendering(true)
            .synchronization2(true);

        let mut physical_device_features2 =
            vk::PhysicalDeviceFeatures2::default().push_next(&mut vulkan_13_features);

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_create_info))
            .enabled_extension_names(&device_extensions)
            .push_next(&mut physical_device_features2);

        // SAFETY: Vulkan 1.3 mantıksal aygıt oluşturulması
        let device = unsafe {
            instance
                .create_device(physical_device, &device_create_info, None)
                .map_err(|e| format!("Vulkan Logical Device olusturulamadi: {e}"))?
        };

        // SAFETY: Kuyruğun alınması
        let graphics_queue = unsafe { device.get_device_queue(selected_queue_family, 0) };

        // 4. Swapchain
        let swapchain_loader = ash::khr::swapchain::Device::new(&instance, &device);
        let swapchain = Self::create_swapchain_inner(
            &instance,
            &surface_loader,
            surface,
            physical_device,
            &device,
            &swapchain_loader,
            width,
            height,
            vk::SwapchainKHR::null(),
        )?;

        // 5. Command Pool & Buffers
        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(selected_queue_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        // SAFETY: Komut havuzunun oluşturulması
        let command_pool = unsafe {
            device
                .create_command_pool(&pool_info, None)
                .map_err(|e| format!("Command pool olusturulamadi: {e}"))?
        };

        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(FRAMES_IN_FLIGHT as u32);

        // SAFETY: Komut tamponlarının tahsis edilmesi
        let command_buffers_vec = unsafe {
            device
                .allocate_command_buffers(&alloc_info)
                .map_err(|e| format!("Command buffer tahsis edilemedi: {e}"))?
        };
        let command_buffers = [command_buffers_vec[0], command_buffers_vec[1]];

        // 6. Senkronizasyon Nesneleri
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        let mut image_available_semaphores = [vk::Semaphore::null(); FRAMES_IN_FLIGHT];
        let mut in_flight_fences = [vk::Fence::null(); FRAMES_IN_FLIGHT];

        for i in 0..FRAMES_IN_FLIGHT {
            // SAFETY: Semafor ve fence oluşturulması
            unsafe {
                image_available_semaphores[i] = device
                    .create_semaphore(&semaphore_info, None)
                    .map_err(|e| e.to_string())?;
                in_flight_fences[i] = device
                    .create_fence(&fence_info, None)
                    .map_err(|e| e.to_string())?;
            }
        }

        let mut render_finished_semaphores = Vec::with_capacity(swapchain.images.len());
        for _ in 0..swapchain.images.len() {
            let sem = unsafe {
                device
                    .create_semaphore(&semaphore_info, None)
                    .map_err(|e| e.to_string())?
            };
            render_finished_semaphores.push(sem);
        }

        let allocator = {
            let desc = gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.clone(),
                device: device.clone(),
                physical_device,
                debug_settings: Default::default(),
                buffer_device_address: false,
                allocation_sizes: Default::default(),
            };
            let alloc = gpu_allocator::vulkan::Allocator::new(&desc)
                .map_err(|e| format!("GPU Allocator baslatilamadi: {e}"))?;
            std::sync::Arc::new(std::sync::Mutex::new(alloc))
        };

        Ok(Self {
            entry,
            instance,
            surface_loader,
            surface,
            physical_device,
            device,
            graphics_queue,
            graphics_queue_family: selected_queue_family,
            swapchain,
            command_pool,
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            current_frame: 0,
            resized: false,
            validation_error_count: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
            allocator: std::mem::ManuallyDrop::new(allocator),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn create_swapchain_inner(
        _instance: &ash::Instance,
        surface_loader: &ash::khr::surface::Instance,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
        device: &ash::Device,
        swapchain_loader: &ash::khr::swapchain::Device,
        width: u32,
        height: u32,
        old_swapchain: vk::SwapchainKHR,
    ) -> Result<Swapchain, String> {
        // SAFETY: Yüzey yeteneklerinin sorgulanması
        let caps = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface)
                .map_err(|e| e.to_string())?
        };

        // SAFETY: Yüzey formatlarının sorgulanması
        let formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(physical_device, surface)
                .map_err(|e| e.to_string())?
        };

        let format = formats
            .iter()
            .find(|f| {
                (f.format == vk::Format::B8G8R8A8_UNORM || f.format == vk::Format::R8G8B8A8_UNORM)
                    && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .or_else(|| {
                formats.iter().find(|f| {
                    f.format == vk::Format::B8G8R8A8_SRGB
                        && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
                })
            })
            .unwrap_or(&formats[0]);

        let extent = if caps.current_extent.width != u32::MAX {
            caps.current_extent
        } else {
            vk::Extent2D {
                width: width.clamp(caps.min_image_extent.width, caps.max_image_extent.width),
                height: height.clamp(caps.min_image_extent.height, caps.max_image_extent.height),
            }
        };

        let mut image_count = caps.min_image_count + 1;
        if caps.max_image_count > 0 && image_count > caps.max_image_count {
            image_count = caps.max_image_count;
        }

        // Present Mode Seçimi:
        // RUGENG_PRESENT_MODE=IMMEDIATE veya MAILBOX ise öncelikle onu seç; yoksa FIFO.
        let present_modes = unsafe {
            surface_loader
                .get_physical_device_surface_present_modes(physical_device, surface)
                .unwrap_or_default()
        };
        let target_mode_env = std::env::var("RUGENG_PRESENT_MODE").unwrap_or_default();
        let chosen_present_mode = match target_mode_env.to_uppercase().as_str() {
            "IMMEDIATE" if present_modes.contains(&vk::PresentModeKHR::IMMEDIATE) => {
                vk::PresentModeKHR::IMMEDIATE
            }
            "MAILBOX" if present_modes.contains(&vk::PresentModeKHR::MAILBOX) => {
                vk::PresentModeKHR::MAILBOX
            }
            _ => {
                if present_modes.contains(&vk::PresentModeKHR::MAILBOX) {
                    vk::PresentModeKHR::MAILBOX
                } else if present_modes.contains(&vk::PresentModeKHR::IMMEDIATE) {
                    vk::PresentModeKHR::IMMEDIATE
                } else {
                    vk::PresentModeKHR::FIFO
                }
            }
        };

        let swapchain_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(caps.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(chosen_present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain);

        // SAFETY: SwapchainKHR oluşturulması
        let handle = unsafe {
            swapchain_loader
                .create_swapchain(&swapchain_info, None)
                .map_err(|e| format!("Swapchain olusturulamadi: {e}"))?
        };

        // SAFETY: Swapchain resimlerinin sorgulanması
        let images = unsafe {
            swapchain_loader
                .get_swapchain_images(handle)
                .map_err(|e| format!("Swapchain resimleri alinamadi: {e}"))?
        };

        let mut image_views = Vec::with_capacity(images.len());
        for &img in &images {
            let view_info = vk::ImageViewCreateInfo::default()
                .image(img)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format.format)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            // SAFETY: ImageView oluşturulması
            let view = unsafe {
                device
                    .create_image_view(&view_info, None)
                    .map_err(|e| format!("ImageView olusturulamadi: {e}"))?
            };
            image_views.push(view);
        }

        Ok(Swapchain {
            loader: swapchain_loader.clone(),
            handle,
            format: format.format,
            extent,
            images,
            image_views,
        })
    }

    /// Pencere yeniden boyutlandırıldığında swapchain'i yeniden oluşturur
    pub fn recreate_swapchain(&mut self, width: u32, height: u32) -> Result<(), String> {
        if width == 0 || height == 0 {
            return Ok(()); // Simge durumuna küçülmüş
        }

        // SAFETY: GPU işlemlerinin bitmesini bekle
        unsafe {
            self.device.device_wait_idle().map_err(|e| e.to_string())?;
        }

        // Eski view'ları temizle
        for &view in &self.swapchain.image_views {
            unsafe {
                self.device.destroy_image_view(view, None);
            }
        }

        let old_handle = self.swapchain.handle;
        let new_swapchain = Self::create_swapchain_inner(
            &self.instance,
            &self.surface_loader,
            self.surface,
            self.physical_device,
            &self.device,
            &self.swapchain.loader,
            width,
            height,
            old_handle,
        )?;

        // Eski swapchain'i serbest bırak
        unsafe {
            self.swapchain.loader.destroy_swapchain(old_handle, None);
        }

        for &sem in &self.render_finished_semaphores {
            unsafe {
                self.device.destroy_semaphore(sem, None);
            }
        }
        self.render_finished_semaphores.clear();
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        for _ in 0..new_swapchain.images.len() {
            let sem = unsafe {
                self.device
                    .create_semaphore(&semaphore_info, None)
                    .map_err(|e| e.to_string())?
            };
            self.render_finished_semaphores.push(sem);
        }

        self.swapchain = new_swapchain;
        self.resized = false;
        Ok(())
    }

    /// Kare başlangıcı: Fence bekleme, sonraki resmi alma ve komut tamponunu başlatma
    pub fn begin_frame(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<Option<(u32, vk::CommandBuffer)>, String> {
        let fence = self.in_flight_fences[self.current_frame];

        // SAFETY: Önceki karenin bitmesini bekle ve fence'i sıfırla
        unsafe {
            self.device
                .wait_for_fences(&[fence], true, u64::MAX)
                .map_err(|e| e.to_string())?;
        }

        if self.resized {
            self.recreate_swapchain(width, height)?;
            return Ok(None);
        }

        let image_available = self.image_available_semaphores[self.current_frame];
        let acquire_result = unsafe {
            self.swapchain.loader.acquire_next_image(
                self.swapchain.handle,
                u64::MAX,
                image_available,
                vk::Fence::null(),
            )
        };

        let image_index = match acquire_result {
            Ok((idx, _suboptimal)) => idx,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain(width, height)?;
                return Ok(None);
            }
            Err(e) => return Err(format!("acquire_next_image basarisiz: {e}")),
        };

        // SAFETY: Fence'i sıfırla ve komut tamponunu kayda aç
        unsafe {
            self.device
                .reset_fences(&[fence])
                .map_err(|e| e.to_string())?;
            let cmd = self.command_buffers[self.current_frame];
            self.device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())
                .map_err(|e| e.to_string())?;

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device
                .begin_command_buffer(cmd, &begin_info)
                .map_err(|e| e.to_string())?;

            Ok(Some((image_index, cmd)))
        }
    }

    /// Dynamic Rendering kapsamını başlatır ve ekranı temizler
    pub fn begin_rendering(&self, cmd: vk::CommandBuffer, image_index: u32, clear_color: [f32; 4]) {
        self.begin_rendering_with_depth(cmd, image_index, clear_color, None);
    }

    /// Dynamic Rendering kapsamını başlatır; opsiyonel derinlik eki (depth attachment) ile renk ve derinliği temizler.
    pub fn begin_rendering_with_depth(
        &self,
        cmd: vk::CommandBuffer,
        image_index: u32,
        clear_color: [f32; 4],
        depth_image_view: Option<vk::ImageView>,
    ) {
        let image = self.swapchain.images[image_index as usize];
        let image_view = self.swapchain.image_views[image_index as usize];

        // 1. Pipeline Barrier 2: UNDEFINED -> COLOR_ATTACHMENT_OPTIMAL
        let barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags2::empty())
            .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let dependency_info =
            vk::DependencyInfo::default().image_memory_barriers(std::slice::from_ref(&barrier));

        // SAFETY: Sync2 pipeline barrier
        unsafe {
            self.device.cmd_pipeline_barrier2(cmd, &dependency_info);
        }

        // 2. Dynamic Rendering: vkCmdBeginRendering
        let clear_val = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: clear_color,
            },
        };

        let color_attachment = vk::RenderingAttachmentInfo::default()
            .image_view(image_view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(clear_val);

        let depth_clear = vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        };

        let mut rendering_info = vk::RenderingInfo::default()
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain.extent,
            })
            .layer_count(1)
            .color_attachments(std::slice::from_ref(&color_attachment));

        let depth_attachment = depth_image_view.map(|depth_view| {
            vk::RenderingAttachmentInfo::default()
                .image_view(depth_view)
                .image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(depth_clear)
        });

        if let Some(ref da) = depth_attachment {
            rendering_info = rendering_info.depth_attachment(da);
        }

        // SAFETY: Dynamic rendering başlatılması
        unsafe {
            self.device.cmd_begin_rendering(cmd, &rendering_info);
        }
    }

    /// Dynamic Rendering kapsamını sonlandırır ve Swapchain Present durumuna geçirir
    pub fn end_rendering(&self, cmd: vk::CommandBuffer, image_index: u32) {
        let image = self.swapchain.images[image_index as usize];

        // SAFETY: Dynamic rendering sonlandırılması
        unsafe {
            self.device.cmd_end_rendering(cmd);
        }

        // Pipeline Barrier 2: COLOR_ATTACHMENT_OPTIMAL -> PRESENT_SRC_KHR
        let present_barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE)
            .dst_access_mask(vk::AccessFlags2::empty())
            .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let present_dep = vk::DependencyInfo::default()
            .image_memory_barriers(std::slice::from_ref(&present_barrier));

        // SAFETY: Sync2 present barrier
        unsafe {
            self.device.cmd_pipeline_barrier2(cmd, &present_dep);
        }
    }

    /// Boş ekranı Dynamic Rendering ve Synchronization2 ile temizler
    pub fn draw_clear(&self, cmd: vk::CommandBuffer, image_index: u32, clear_color: [f32; 4]) {
        self.begin_rendering(cmd, image_index, clear_color);
        self.end_rendering(cmd, image_index);
    }

    /// Kare bitişi: Queue submit ve present
    pub fn end_frame(
        &mut self,
        image_index: u32,
        cmd: vk::CommandBuffer,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        let fence = self.in_flight_fences[self.current_frame];
        let image_available = self.image_available_semaphores[self.current_frame];
        let render_finished = self.render_finished_semaphores[image_index as usize];

        // SAFETY: Komut kaydını bitir ve kuyruğa gönder
        unsafe {
            self.device
                .end_command_buffer(cmd)
                .map_err(|e| e.to_string())?;

            let wait_semaphores = [image_available];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let signal_semaphores = [render_finished];
            let command_buffers = [cmd];

            let submit_info = vk::SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores);

            self.device
                .queue_submit(self.graphics_queue, &[submit_info], fence)
                .map_err(|e| e.to_string())?;

            // Present işlemi
            let swapchains = [self.swapchain.handle];
            let image_indices = [image_index];

            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&signal_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);

            let present_res = self
                .swapchain
                .loader
                .queue_present(self.graphics_queue, &present_info);
            match present_res {
                Ok(suboptimal) => {
                    if suboptimal || self.resized {
                        self.recreate_swapchain(width, height)?;
                    }
                }
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    self.recreate_swapchain(width, height)?;
                }
                Err(e) => return Err(format!("queue_present basarisiz: {e}")),
            }
        }

        self.current_frame = (self.current_frame + 1) % FRAMES_IN_FLIGHT;
        Ok(())
    }

    /// Tek seferlik komut tamponu kaydeder, graphics queue'ya gönderir ve tamamlanmasını bekler
    pub fn execute_immediate<F: FnOnce(vk::CommandBuffer)>(&self, f: F) -> Result<(), String> {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        unsafe {
            let cmd = self
                .device
                .allocate_command_buffers(&alloc_info)
                .map_err(|e| format!("Immediate cmd tahsis edilemedi: {e}"))?[0];

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            self.device
                .begin_command_buffer(cmd, &begin_info)
                .map_err(|e| format!("Immediate cmd baslatilamadi: {e}"))?;

            f(cmd);

            self.device
                .end_command_buffer(cmd)
                .map_err(|e| format!("Immediate cmd sonlandirilamadi: {e}"))?;

            let submit_info = vk::SubmitInfo::default().command_buffers(std::slice::from_ref(&cmd));

            self.device
                .queue_submit(self.graphics_queue, &[submit_info], vk::Fence::null())
                .map_err(|e| format!("Immediate cmd submit basarisiz: {e}"))?;

            self.device
                .queue_wait_idle(self.graphics_queue)
                .map_err(|e| format!("Queue bekleme basarisiz: {e}"))?;

            self.device.free_command_buffers(self.command_pool, &[cmd]);
        }
        Ok(())
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        // SAFETY: Kaynakların serbest bırakılması
        unsafe {
            let _ = self.device.device_wait_idle();

            for i in 0..FRAMES_IN_FLIGHT {
                self.device
                    .destroy_semaphore(self.image_available_semaphores[i], None);
                self.device.destroy_fence(self.in_flight_fences[i], None);
            }

            for &sem in &self.render_finished_semaphores {
                self.device.destroy_semaphore(sem, None);
            }

            self.device.destroy_command_pool(self.command_pool, None);

            for &view in &self.swapchain.image_views {
                self.device.destroy_image_view(view, None);
            }
            self.swapchain
                .loader
                .destroy_swapchain(self.swapchain.handle, None);

            // KRİTİK: Logical device yok edilmeden önce allocator açıkça serbest bırakılır (Use-After-Free önlenir)
            std::mem::ManuallyDrop::drop(&mut self.allocator);

            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);

            let err_count = self
                .validation_error_count
                .load(std::sync::atomic::Ordering::Relaxed);
            println!(
                "[Vulkan] Oturum sonlandi. Toplam Validation Hatasi: {}",
                err_count
            );
        }
    }
}
