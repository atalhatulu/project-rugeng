//! 3D GPU Mesh Tamponu (Vertex & Index Buffer).

use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};
use gpu_allocator::MemoryLocation;
use std::sync::{Arc, Mutex};

use crate::mesh::{Mesh3D, Vertex3D};

pub struct GpuMesh3D {
    pub vertex_buffer: vk::Buffer,
    pub vertex_allocation: Option<Allocation>,
    pub vertex_count: u32,

    pub index_buffer: Option<vk::Buffer>,
    pub index_allocation: Option<Allocation>,
    pub index_count: u32,
}

impl GpuMesh3D {
    /// CPU tarafındaki Mesh3D nesnesini GPU belleğine yükler.
    pub fn from_mesh(
        device: &ash::Device,
        allocator: Arc<Mutex<Allocator>>,
        mesh: &Mesh3D,
    ) -> Result<Self, String> {
        if mesh.vertices.is_empty() {
            return Err("Mesh tepe noktalari bos".to_string());
        }

        // 1. Vertex Buffer
        let vertex_bytes_size = (mesh.vertices.len() * std::mem::size_of::<Vertex3D>()) as u64;
        let v_buffer_info = vk::BufferCreateInfo::default()
            .size(vertex_bytes_size)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let vertex_buffer = unsafe {
            device
                .create_buffer(&v_buffer_info, None)
                .map_err(|e| format!("Vertex buffer olusturulamadi: {e}"))?
        };

        let v_requirements = unsafe { device.get_buffer_memory_requirements(vertex_buffer) };
        let mut alloc = allocator.lock().unwrap();
        let mut vertex_allocation = alloc
            .allocate(&AllocationCreateDesc {
                name: "GpuMesh3D_VertexBuffer",
                requirements: v_requirements,
                location: MemoryLocation::CpuToGpu,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| format!("Vertex buffer bellek tahsis edilemedi: {e}"))?;

        // Veriyi belleğe kopyala
        let v_slice = vertex_allocation
            .mapped_slice_mut()
            .ok_or_else(|| "Vertex buffer mapped_slice alinamadi".to_string())?;
        let src_bytes = bytemuck::cast_slice(&mesh.vertices);
        v_slice[..src_bytes.len()].copy_from_slice(src_bytes);

        unsafe {
            device
                .bind_buffer_memory(
                    vertex_buffer,
                    vertex_allocation.memory(),
                    vertex_allocation.offset(),
                )
                .map_err(|e| format!("Vertex buffer bellek baglanamadi: {e}"))?;
        }

        // 2. Index Buffer (varsa)
        let (index_buffer, index_allocation, index_count) = if !mesh.indices.is_empty() {
            let index_bytes_size = (mesh.indices.len() * std::mem::size_of::<u32>()) as u64;
            let i_buffer_info = vk::BufferCreateInfo::default()
                .size(index_bytes_size)
                .usage(vk::BufferUsageFlags::INDEX_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            let ib = unsafe {
                device
                    .create_buffer(&i_buffer_info, None)
                    .map_err(|e| format!("Index buffer olusturulamadi: {e}"))?
            };

            let i_requirements = unsafe { device.get_buffer_memory_requirements(ib) };
            let mut i_allocation = alloc
                .allocate(&AllocationCreateDesc {
                    name: "GpuMesh3D_IndexBuffer",
                    requirements: i_requirements,
                    location: MemoryLocation::CpuToGpu,
                    linear: true,
                    allocation_scheme: AllocationScheme::GpuAllocatorManaged,
                })
                .map_err(|e| format!("Index buffer bellek tahsis edilemedi: {e}"))?;

            let i_slice = i_allocation
                .mapped_slice_mut()
                .ok_or_else(|| "Index buffer mapped_slice alinamadi".to_string())?;
            let idx_bytes = bytemuck::cast_slice(&mesh.indices);
            i_slice[..idx_bytes.len()].copy_from_slice(idx_bytes);

            unsafe {
                device
                    .bind_buffer_memory(ib, i_allocation.memory(), i_allocation.offset())
                    .map_err(|e| format!("Index buffer bellek baglanamadi: {e}"))?;
            }

            (Some(ib), Some(i_allocation), mesh.indices.len() as u32)
        } else {
            (None, None, 0)
        };

        drop(alloc);

        Ok(Self {
            vertex_buffer,
            vertex_allocation: Some(vertex_allocation),
            vertex_count: mesh.vertices.len() as u32,
            index_buffer,
            index_allocation,
            index_count,
        })
    }

    /// Komut tamponuna çizim komutunu kaydeder.
    pub fn draw(&self, device: &ash::Device, cmd: vk::CommandBuffer) {
        unsafe {
            device.cmd_bind_vertex_buffers(cmd, 0, &[self.vertex_buffer], &[0]);
            if let Some(ib) = self.index_buffer {
                device.cmd_bind_index_buffer(cmd, ib, 0, vk::IndexType::UINT32);
                device.cmd_draw_indexed(cmd, self.index_count, 1, 0, 0, 0);
            } else {
                device.cmd_draw(cmd, self.vertex_count, 1, 0, 0);
            }
        }
    }

    pub fn destroy(&mut self, device: &ash::Device, allocator: &mut Allocator) {
        unsafe {
            device.destroy_buffer(self.vertex_buffer, None);
            if let Some(ib) = self.index_buffer {
                device.destroy_buffer(ib, None);
            }
        }
        if let Some(alloc) = self.vertex_allocation.take() {
            let _ = allocator.free(alloc);
        }
        if let Some(alloc) = self.index_allocation.take() {
            let _ = allocator.free(alloc);
        }
    }
}
