use ash::{vk, Device};
use gpu_allocator::vulkan::*;

#[derive(Clone)]
pub struct Vertex {
    pub position: cgmath::Vector3<f32>,
    pub color: cgmath::Vector3<f32>,
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub allocation: Allocation,
}

pub struct MeshBuffer {
    pub buffer: vk::Buffer,
    pub meshes: Option<Vec<Mesh>>,
}

pub fn triangle_mesh<'a>(device: &Device, allocator: &mut Allocator) -> MeshBuffer {
    let triangle = vec![
        Vertex {
            position: cgmath::vec3(0.0, -1.0, 0.0),
            color: cgmath::vec3(0.0, 1.0, 0.0),
        },
        Vertex {
            position: cgmath::vec3(1.0, 1.0, 0.0),
            color: cgmath::vec3(1.0, 0.0, 0.0),
        },
        Vertex {
            position: cgmath::vec3(-1.0, 1.0, 0.0),
            color: cgmath::vec3(0.0, 0.0, 1.0),
        },
    ];
    let buffer_info = vk::BufferCreateInfo::builder()
        .size(4096)
        .usage(vk::BufferUsageFlags::VERTEX_BUFFER);
    let buffer = unsafe { device.create_buffer(&buffer_info, None) }.unwrap();
    let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
    let allocation = allocator
        .allocate(&AllocationCreateDesc {
            name: "Triangle",
            requirements,
            location: gpu_allocator::MemoryLocation::CpuToGpu,
            linear: true,
        })
        .unwrap();
    unsafe {
        device
            .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
            .unwrap()
    };
    unsafe {
        let ptr = allocation.mapped_ptr().unwrap().cast::<Vertex>().as_ptr();
        std::ptr::copy_nonoverlapping(triangle.as_ptr(), ptr, std::mem::size_of::<Vertex>() * 3);
    }
    MeshBuffer {
        buffer,
        meshes: Some(vec![Mesh {
            vertices: triangle,
            allocation,
        }]),
    }
}
