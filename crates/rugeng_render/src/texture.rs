//! 2D Doku Atlası ve Varlık Yükleyici (Texture Atlas & Asset Pipeline).
//!
//! Tek bir büyük GPU dokusu (Atlas) içinde birden fazla sprite'ı paketler,
//! 16-bit hassasiyetli UV koordinatları üretir ve tek `vkCmdDraw` içinde
//! dokulu ve dokusuz sprite'ların kesintisiz çizilmesini sağlar.

use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};
use gpu_allocator::MemoryLocation;
use std::collections::HashMap;

use crate::context::VulkanContext;

#[derive(Clone, Copy, Debug)]
pub struct AtlasSubRegion {
    pub rect: [u32; 4],      // [x, y, w, h] piksel
    pub uv_min: [f32; 2],    // [u_min, v_min] 0.0 .. 1.0
    pub uv_size: [f32; 2],   // [u_size, v_size] 0.0 .. 1.0
    pub uv_min_packed: u32,  // GLSL unpackUnorm2x16 için paketlenmiş
    pub uv_size_packed: u32, // GLSL unpackUnorm2x16 için paketlenmiş
}

pub struct TextureAtlas {
    pub width: u32,
    pub height: u32,
    pub image: vk::Image,
    pub allocation: Option<Allocation>,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub regions: HashMap<String, AtlasSubRegion>,
    current_x: u32,
    current_y: u32,
    row_height: u32,
}

impl TextureAtlas {
    pub fn new(ctx: &VulkanContext, width: u32, height: u32) -> Result<Self, String> {
        let device = &ctx.device;

        // 1. VkImage oluşturulması
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let image = unsafe {
            device
                .create_image(&image_info, None)
                .map_err(|e| format!("Atlas VkImage olusturulamadi: {e}"))?
        };

        // 2. GPU Memory tahsisi
        let requirements = unsafe { device.get_image_memory_requirements(image) };
        let mut alloc = ctx.allocator.lock().unwrap();
        let allocation = alloc
            .allocate(&AllocationCreateDesc {
                name: "TextureAtlas_Image",
                requirements,
                location: MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| format!("Atlas bellek tahsis edilemedi: {e}"))?;

        unsafe {
            device
                .bind_image_memory(image, allocation.memory(), allocation.offset())
                .map_err(|e| format!("Atlas bellek baglanamadi: {e}"))?;
        }
        drop(alloc);

        // 3. VkImageView
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let image_view = unsafe {
            device
                .create_image_view(&view_info, None)
                .map_err(|e| format!("Atlas ImageView olusturulamadi: {e}"))?
        };

        // 4. VkSampler (Bilinear filtering + clamp to edge)
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::NEAREST) // Piksel sanatı için keskin NEAREST filtre
            .min_filter(vk::Filter::NEAREST)
            .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .max_anisotropy(1.0)
            .min_lod(0.0)
            .max_lod(0.0);

        let sampler = unsafe {
            device
                .create_sampler(&sampler_info, None)
                .map_err(|e| format!("Atlas Sampler olusturulamadi: {e}"))?
        };

        // 5. İlk layout geçişi: UNDEFINED -> SHADER_READ_ONLY_OPTIMAL
        ctx.execute_immediate(|cmd| unsafe {
            let barrier = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
                .src_access_mask(vk::AccessFlags2::NONE)
                .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                .dst_access_mask(vk::AccessFlags2::SHADER_READ)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image(image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            let dep_info =
                vk::DependencyInfo::default().image_memory_barriers(std::slice::from_ref(&barrier));
            device.cmd_pipeline_barrier2(cmd, &dep_info);
        })?;

        let mut atlas = Self {
            width,
            height,
            image,
            allocation: Some(allocation),
            image_view,
            sampler,
            regions: HashMap::new(),
            current_x: 0,
            current_y: 0,
            row_height: 0,
        };

        // Dokusuz çizimler için sol üstte 4x4 opak beyaz piksel tahsis et
        let white_pixels = [255u8; 4 * 4 * 4];
        atlas.allocate_and_upload(ctx, "__white__", 4, 4, &white_pixels)?;

        // Temel örnek procedürel sprite'ları yükle
        atlas.load_builtin_procedural_sprites(ctx)?;

        Ok(atlas)
    }

    /// Yeni bir sprite için yer ayırır ve piksel verisini GPU atlasına kopyalar
    pub fn allocate_and_upload(
        &mut self,
        ctx: &VulkanContext,
        name: &str,
        w: u32,
        h: u32,
        rgba_pixels: &[u8],
    ) -> Result<AtlasSubRegion, String> {
        assert_eq!(rgba_pixels.len(), (w * h * 4) as usize);

        if w > self.width || h > self.height {
            return Err(format!(
                "Gorsel boyutlari ({w}x{h}) atlas boyutlarini ({width}x{height}) asiyor!",
                width = self.width,
                height = self.height
            ));
        }

        // Raf (Shelf) paketleme algoritması
        if self.current_x + w > self.width {
            self.current_x = 0;
            self.current_y += self.row_height + 2; // 2 piksel padding
            self.row_height = 0;
        }

        if self.current_y + h > self.height {
            return Err("TextureAtlas bellegi doldu!".to_string());
        }

        let alloc_x = self.current_x;
        let alloc_y = self.current_y;

        self.current_x += w + 2;
        self.row_height = self.row_height.max(h);

        // UV Koordinatları (0.0 .. 1.0)
        let u_min = alloc_x as f32 / self.width as f32;
        let v_min = alloc_y as f32 / self.height as f32;
        let u_size = w as f32 / self.width as f32;
        let v_size = h as f32 / self.height as f32;

        let u_min_u16 = (u_min.clamp(0.0, 1.0) * 65535.0).round() as u32;
        let v_min_u16 = (v_min.clamp(0.0, 1.0) * 65535.0).round() as u32;
        let u_size_u16 = (u_size.clamp(0.0, 1.0) * 65535.0).round() as u32;
        let v_size_u16 = (v_size.clamp(0.0, 1.0) * 65535.0).round() as u32;

        let uv_min_packed = (v_min_u16 << 16) | (u_min_u16 & 0xFFFF);
        let uv_size_packed = (v_size_u16 << 16) | (u_size_u16 & 0xFFFF);

        let sub_region = AtlasSubRegion {
            rect: [alloc_x, alloc_y, w, h],
            uv_min: [u_min, v_min],
            uv_size: [u_size, v_size],
            uv_min_packed,
            uv_size_packed,
        };

        // GPU'ya staging buffer üzerinden aktar
        let device = &ctx.device;
        let buffer_size = rgba_pixels.len() as u64;

        let buf_info = vk::BufferCreateInfo::default()
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let staging_buf = unsafe {
            device
                .create_buffer(&buf_info, None)
                .map_err(|e| format!("Staging buffer olusturulamadi: {e}"))?
        };

        let reqs = unsafe { device.get_buffer_memory_requirements(staging_buf) };
        let mut alloc = ctx.allocator.lock().unwrap();
        let staging_alloc = alloc
            .allocate(&AllocationCreateDesc {
                name: "Atlas_Staging_Buffer",
                requirements: reqs,
                location: MemoryLocation::CpuToGpu,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| format!("Staging allocation basarisiz: {e}"))?;

        unsafe {
            device
                .bind_buffer_memory(staging_buf, staging_alloc.memory(), staging_alloc.offset())
                .map_err(|e| e.to_string())?;
        }

        let mapped_ptr = staging_alloc.mapped_ptr().unwrap().as_ptr() as *mut u8;
        unsafe {
            std::ptr::copy_nonoverlapping(rgba_pixels.as_ptr(), mapped_ptr, rgba_pixels.len());
        }

        // Komut tamponu ile staging -> image kopyalama ve senkronizasyon
        ctx.execute_immediate(|cmd| unsafe {
            // 1. Layout: SHADER_READ_ONLY_OPTIMAL -> TRANSFER_DST_OPTIMAL
            let barrier_to_dst = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                .src_access_mask(vk::AccessFlags2::SHADER_READ)
                .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                .old_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .image(self.image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            let dep_to_dst = vk::DependencyInfo::default()
                .image_memory_barriers(std::slice::from_ref(&barrier_to_dst));
            device.cmd_pipeline_barrier2(cmd, &dep_to_dst);

            // 2. Buffer to Image Copy
            let copy_region = vk::BufferImageCopy::default()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image_offset(vk::Offset3D {
                    x: alloc_x as i32,
                    y: alloc_y as i32,
                    z: 0,
                })
                .image_extent(vk::Extent3D {
                    width: w,
                    height: h,
                    depth: 1,
                });

            device.cmd_copy_buffer_to_image(
                cmd,
                staging_buf,
                self.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[copy_region],
            );

            // 3. Layout: TRANSFER_DST_OPTIMAL -> SHADER_READ_ONLY_OPTIMAL
            let barrier_to_read = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                .dst_access_mask(vk::AccessFlags2::SHADER_READ)
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image(self.image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            let dep_to_read = vk::DependencyInfo::default()
                .image_memory_barriers(std::slice::from_ref(&barrier_to_read));
            device.cmd_pipeline_barrier2(cmd, &dep_to_read);
        })?;

        // Staging kaynağını serbest bırak
        unsafe {
            device.destroy_buffer(staging_buf, None);
        }
        let _ = alloc.free(staging_alloc);
        drop(alloc);

        self.regions.insert(name.to_string(), sub_region);
        Ok(sub_region)
    }

    /// Mevcut bir bölgenin piksel verisini yerinde (in-place) günceller (Hot Reload için)
    pub fn update_existing_region(
        &self,
        ctx: &VulkanContext,
        name: &str,
        rgba_pixels: &[u8],
    ) -> Result<(), String> {
        let region = self
            .regions
            .get(name)
            .ok_or_else(|| format!("Bolge bulunamadi: {name}"))?;

        let alloc_x = region.rect[0];
        let alloc_y = region.rect[1];
        let w = region.rect[2];
        let h = region.rect[3];

        if rgba_pixels.len() != (w * h * 4) as usize {
            return Err(format!(
                "Piksel boyutu uyusmuyor: beklenen {} bayt, gelen {} bayt",
                w * h * 4,
                rgba_pixels.len()
            ));
        }

        let device = &ctx.device;
        let buffer_size = rgba_pixels.len() as u64;

        let buf_info = ash::vk::BufferCreateInfo::default()
            .size(buffer_size)
            .usage(ash::vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(ash::vk::SharingMode::EXCLUSIVE);

        let staging_buf = unsafe {
            device
                .create_buffer(&buf_info, None)
                .map_err(|e| format!("Staging buffer olusturulamadi: {e}"))?
        };

        let reqs = unsafe { device.get_buffer_memory_requirements(staging_buf) };
        let mut alloc = ctx.allocator.lock().unwrap();
        let staging_alloc = alloc
            .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: "Atlas_HotReload_Staging_Buffer",
                requirements: reqs,
                location: gpu_allocator::MemoryLocation::CpuToGpu,
                linear: true,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| format!("Staging allocation basarisiz: {e}"))?;

        unsafe {
            device
                .bind_buffer_memory(staging_buf, staging_alloc.memory(), staging_alloc.offset())
                .map_err(|e| e.to_string())?;
        }

        let mapped_ptr = staging_alloc.mapped_ptr().unwrap().as_ptr() as *mut u8;
        unsafe {
            std::ptr::copy_nonoverlapping(rgba_pixels.as_ptr(), mapped_ptr, rgba_pixels.len());
        }

        ctx.execute_immediate(|cmd| unsafe {
            let barrier_to_dst = ash::vk::ImageMemoryBarrier2::default()
                .src_stage_mask(ash::vk::PipelineStageFlags2::FRAGMENT_SHADER)
                .src_access_mask(ash::vk::AccessFlags2::SHADER_READ)
                .dst_stage_mask(ash::vk::PipelineStageFlags2::TRANSFER)
                .dst_access_mask(ash::vk::AccessFlags2::TRANSFER_WRITE)
                .old_layout(ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .new_layout(ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .image(self.image)
                .subresource_range(ash::vk::ImageSubresourceRange {
                    aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            let dep_to_dst = ash::vk::DependencyInfo::default()
                .image_memory_barriers(std::slice::from_ref(&barrier_to_dst));
            device.cmd_pipeline_barrier2(cmd, &dep_to_dst);

            let copy_region = ash::vk::BufferImageCopy::default()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(ash::vk::ImageSubresourceLayers {
                    aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image_offset(ash::vk::Offset3D {
                    x: alloc_x as i32,
                    y: alloc_y as i32,
                    z: 0,
                })
                .image_extent(ash::vk::Extent3D {
                    width: w,
                    height: h,
                    depth: 1,
                });

            device.cmd_copy_buffer_to_image(
                cmd,
                staging_buf,
                self.image,
                ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[copy_region],
            );

            let barrier_to_read = ash::vk::ImageMemoryBarrier2::default()
                .src_stage_mask(ash::vk::PipelineStageFlags2::TRANSFER)
                .src_access_mask(ash::vk::AccessFlags2::TRANSFER_WRITE)
                .dst_stage_mask(ash::vk::PipelineStageFlags2::FRAGMENT_SHADER)
                .dst_access_mask(ash::vk::AccessFlags2::SHADER_READ)
                .old_layout(ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image(self.image)
                .subresource_range(ash::vk::ImageSubresourceRange {
                    aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            let dep_to_read = ash::vk::DependencyInfo::default()
                .image_memory_barriers(std::slice::from_ref(&barrier_to_read));
            device.cmd_pipeline_barrier2(cmd, &dep_to_read);
        })?;

        unsafe {
            device.destroy_buffer(staging_buf, None);
        }
        let _ = alloc.free(staging_alloc);
        drop(alloc);

        Ok(())
    }

    /// Harici bir PNG bayt dizisini çözüp GPU doku atlasına yerleştirir.
    pub fn load_png_bytes(
        &mut self,
        ctx: &VulkanContext,
        name: &str,
        png_bytes: &[u8],
    ) -> Result<AtlasSubRegion, String> {
        let (w, h, rgba) = decode_png(png_bytes)?;
        self.allocate_and_upload(ctx, name, w, h, &rgba)
    }

    /// Dosya sisteminden bir PNG dosyasını okuyup GPU doku atlasına yerleştirir.
    pub fn load_png_file(
        &mut self,
        ctx: &VulkanContext,
        name: &str,
        path: impl AsRef<std::path::Path>,
    ) -> Result<AtlasSubRegion, String> {
        let bytes = std::fs::read(path.as_ref())
            .map_err(|e| format!("PNG dosyasi okunamadi ({}): {e}", path.as_ref().display()))?;
        self.load_png_bytes(ctx, name, &bytes)
    }

    pub fn get_region(&self, name: &str) -> Option<&AtlasSubRegion> {
        self.regions.get(name)
    }

    /// Dahili procedürel 2D sprite setini üretir (Gemi, Düşman, Mermi, Kalp, Yıldız)
    pub fn load_builtin_procedural_sprites(&mut self, ctx: &VulkanContext) -> Result<(), String> {
        // 1. Oyuncu Uzay Gemisi (32x32)
        let mut ship = vec![0u8; 32 * 32 * 4];
        for y in 0i32..32 {
            for x in 0i32..32 {
                let dx = (x - 16).abs();
                let dy = y;
                let idx = ((y * 32 + x) * 4) as usize;
                // Üçgen gövde
                if dy >= 4 && dx <= (dy / 2) && dy < 28 {
                    ship[idx] = 71; // R
                    ship[idx + 1] = 140; // G
                    ship[idx + 2] = 220; // B
                    ship[idx + 3] = 255; // A
                }
                // Kokpit camı
                if (10..=18).contains(&dy) && dx <= 3 {
                    ship[idx] = 220;
                    ship[idx + 1] = 245;
                    ship[idx + 2] = 255;
                    ship[idx + 3] = 255;
                }
                // İtici alevleri
                if (26..32).contains(&dy) && dx <= 4 {
                    ship[idx] = 255;
                    ship[idx + 1] = 140;
                    ship[idx + 2] = 40;
                    ship[idx + 3] = 255;
                }
            }
        }
        self.allocate_and_upload(ctx, "player_ship", 32, 32, &ship)?;

        // 2. Düşman Yaratık (32x32)
        let mut enemy = vec![0u8; 32 * 32 * 4];
        for y in 0i32..32 {
            for x in 0i32..32 {
                let dist_sq = (x - 16).pow(2) + (y - 16).pow(2);
                let idx = ((y * 32 + x) * 4) as usize;
                if dist_sq < 144 {
                    enemy[idx] = 228;
                    enemy[idx + 1] = 70;
                    enemy[idx + 2] = 70;
                    enemy[idx + 3] = 255;
                }
                // Gözler
                if (y == 12 || y == 13) && (x == 11 || x == 12 || x == 19 || x == 20) {
                    enemy[idx] = 255;
                    enemy[idx + 1] = 240;
                    enemy[idx + 2] = 40;
                    enemy[idx + 3] = 255;
                }
            }
        }
        self.allocate_and_upload(ctx, "enemy_bug", 32, 32, &enemy)?;

        // 3. Parlayan Enerji Küresi (16x16)
        let mut orb = vec![0u8; 16 * 16 * 4];
        for y in 0..16 {
            for x in 0..16 {
                let dist = ((x as f32 - 7.5).powi(2) + (y as f32 - 7.5).powi(2)).sqrt();
                let idx = ((y * 16 + x) * 4) as usize;
                if dist < 7.0 {
                    let alpha = ((1.0 - (dist / 7.0)) * 255.0) as u8;
                    orb[idx] = 255;
                    orb[idx + 1] = 230;
                    orb[idx + 2] = 80;
                    orb[idx + 3] = alpha;
                }
            }
        }
        self.allocate_and_upload(ctx, "energy_orb", 16, 16, &orb)?;

        // 4. Can Kalbi (16x16)
        let mut heart = vec![0u8; 16 * 16 * 4];
        for y in 0..16 {
            for x in 0..16 {
                let idx = ((y * 16 + x) * 4) as usize;
                let is_heart = ((4..=6).contains(&y)
                    && ((2..=6).contains(&x) || (9..=13).contains(&x)))
                    || ((7..=9).contains(&y) && (1..=14).contains(&x))
                    || ((10..=11).contains(&y) && (3..=12).contains(&x))
                    || ((12..=13).contains(&y) && (5..=10).contains(&x))
                    || (y == 14 && (7..=8).contains(&x));
                if is_heart {
                    heart[idx] = 240;
                    heart[idx + 1] = 50;
                    heart[idx + 2] = 80;
                    heart[idx + 3] = 255;
                }
            }
        }
        self.allocate_and_upload(ctx, "heart_icon", 16, 16, &heart)?;

        Ok(())
    }

    pub fn destroy(&mut self, device: &ash::Device, allocator: &mut Allocator) {
        unsafe {
            device.destroy_sampler(self.sampler, None);
            device.destroy_image_view(self.image_view, None);
            device.destroy_image(self.image, None);
            if let Some(alloc) = self.allocation.take() {
                let _ = allocator.free(alloc);
            }
        }
    }
}

/// PNG formatındaki bayt dizisini RGBA8 ham piksellerine ve (genişlik, yükseklik) boyutuna çözer.
pub fn decode_png(png_bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let img = image::load_from_memory_with_format(png_bytes, image::ImageFormat::Png)
        .map_err(|e| format!("PNG decode hatasi: {e}"))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return Err("PNG gorsel boyutlari sifir olamaz".to_string());
    }
    Ok((w, h, img.into_raw()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_invalid_png_returns_err() {
        let bad_bytes = [0x89, b'P', b'N', b'G', 0x00, 0x00];
        let res = decode_png(&bad_bytes);
        assert!(res.is_err());
    }

    #[test]
    fn test_decode_valid_synthetic_png() {
        // 2x2 boyutunda sentetik RGBA piksel oluştur ve PNG baytlarına dönüştür
        let img = image::RgbaImage::from_raw(
            2,
            2,
            vec![
                255, 0, 0, 255, // Kırmızı
                0, 255, 0, 255, // Yeşil
                0, 0, 255, 255, // Mavi
                255, 255, 0, 255, // Sarı
            ],
        )
        .unwrap();

        let mut png_bytes = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .unwrap();

        // decode_png ile çöz
        let (w, h, rgba) = decode_png(&png_bytes).expect("PNG cozumlenemedi");
        assert_eq!(w, 2);
        assert_eq!(h, 2);
        assert_eq!(rgba.len(), 16);
        assert_eq!(&rgba[0..4], &[255, 0, 0, 255]);
        assert_eq!(&rgba[4..8], &[0, 255, 0, 255]);
        assert_eq!(&rgba[8..12], &[0, 0, 255, 255]);
        assert_eq!(&rgba[12..16], &[255, 255, 0, 255]);
    }
}
