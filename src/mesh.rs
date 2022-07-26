use ash::{vk, Device};
use cgmath::Vector3;
use gpu_allocator::vulkan::*;
use tobj::GPU_LOAD_OPTIONS;

#[derive(Clone)]
#[repr(C)]
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
    pub vertex_count: u32,
    pub meshes: Option<Vec<Mesh>>,
}

pub fn monkey_mesh(device: &Device, allocator: &mut Allocator) -> MeshBuffer {
    let (models, _) =
        tobj::load_obj("assets/monkey_flat.obj", &GPU_LOAD_OPTIONS).expect("Could not load monkey");

    let mut vertices: Vec<Vertex> = Vec::new();
    let mesh = &models[0].mesh;
    let positions = &mesh.positions;
    let normals = &mesh.normals;

    for idx in mesh.indices.iter() {
        let i = *idx as usize;
        vertices.push(Vertex {
            position: Vector3::new(positions[i * 3], positions[i * 3 + 1], positions[i * 3 + 2]),
            color: Vector3::new(normals[i * 3], normals[i * 3 + 1], normals[i * 3 + 2]),
        });
    }

    let buffer_info = vk::BufferCreateInfo::builder()
        .size(10000 * 12)
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
        std::ptr::copy_nonoverlapping(
            vertices.as_ptr(),
            ptr,
            std::mem::size_of::<Vertex>() * vertices.len(),
        );
    };

    MeshBuffer {
        buffer,
        vertex_count: (vertices.len()) as u32,
        meshes: Some(vec![Mesh {
            vertices,
            allocation,
        }]),
    }
}

// pub fn triangle_mesh(device: &Device, allocator: &mut Allocator) -> MeshBuffer {
//     let triangle = vec![
//         Vertex {
//             position: cgmath::vec3(0.0, -1.0, 0.0),
//             color: cgmath::vec3(0.0, 1.0, 0.0),
//         },
//         Vertex {
//             position: cgmath::vec3(1.0, 1.0, 0.0),
//             color: cgmath::vec3(1.0, 0.0, 0.0),
//         },
//         Vertex {
//             position: cgmath::vec3(-1.0, 1.0, 0.0),
//             color: cgmath::vec3(0.0, 0.0, 1.0),
//         },
//     ];
//     let buffer_info = vk::BufferCreateInfo::builder()
//         .size(4096)
//         .usage(vk::BufferUsageFlags::VERTEX_BUFFER);
//     let buffer = unsafe { device.create_buffer(&buffer_info, None) }.unwrap();
//     let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
//     let allocation = allocator
//         .allocate(&AllocationCreateDesc {
//             name: "Triangle",
//             requirements,
//             location: gpu_allocator::MemoryLocation::CpuToGpu,
//             linear: true,
//         })
//         .unwrap();
//     unsafe {
//         device
//             .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
//             .unwrap()
//     };
//     unsafe {
//         let ptr = allocation.mapped_ptr().unwrap().cast::<Vertex>().as_ptr();
//         std::ptr::copy_nonoverlapping(triangle.as_ptr(), ptr, std::mem::size_of::<Vertex>() * 3);
//     }
//     MeshBuffer {
//         buffer,
//         meshes: Some(vec![Mesh {
//             vertices: triangle,
//             allocation,
//         }]),
//     }
// }
