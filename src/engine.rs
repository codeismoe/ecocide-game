use gpu_allocator::vulkan::*;
use std::borrow::Cow;
use std::os::raw::c_char;
use std::{cell::RefCell, ffi::CStr};

use ash::{
    extensions::{
        ext::DebugUtils,
        khr::{Surface, Swapchain},
    },
    vk, Device, Entry, Instance,
};

use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use crate::mesh::{triangle_mesh, MeshBuffer};
use crate::pipeline::{build_pipeline, shader_stage_create_info};

pub struct VkEngine {
    pub window: Window,
    pub event_loop: RefCell<EventLoop<()>>,

    pub entry: Entry,
    pub instance: Instance,
    pub debug_utils_loader: DebugUtils,
    pub debug_callback: vk::DebugUtilsMessengerEXT,

    pub device: Device,
    pub pdevice: vk::PhysicalDevice,
    pub queue_family_index: u32,
    pub present_queue: vk::Queue,

    pub command_pool: vk::CommandPool,
    pub command_buffer: vk::CommandBuffer,

    pub surface_loader: Surface,
    pub surface: vk::SurfaceKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,

    pub swapchain_loader: Swapchain,
    pub swapchain: vk::SwapchainKHR,
    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,

    pub framebuffers: Vec<vk::Framebuffer>,
    pub render_pass: vk::RenderPass,

    pub present_semaphore: vk::Semaphore,
    pub render_semaphore: vk::Semaphore,
    pub render_fence: vk::Fence,

    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,

    pub compiler: shaderc::Compiler,
    pub allocator: Option<Allocator>,
    pub meshes: MeshBuffer,
}

impl VkEngine {
    pub fn new() -> Self {
        unsafe {
            let event_loop = EventLoop::new();
            let window = WindowBuilder::new()
                .with_title("Ecocide")
                .with_inner_size(winit::dpi::LogicalSize::new(800.0f64, 600.0f64))
                .build(&event_loop)
                .unwrap();
            let entry = Entry::linked();
            let app_name = CStr::from_bytes_with_nul_unchecked(b"Ecocide\0");
            let layer_names = [CStr::from_bytes_with_nul_unchecked(
                b"VK_LAYER_KHRONOS_validation\0",
            )];
            let layers_names_raw: Vec<*const c_char> = layer_names
                .iter()
                .map(|raw_name| raw_name.as_ptr())
                .collect();

            let mut extension_names = ash_window::enumerate_required_extensions(&window)
                .unwrap()
                .to_vec();

            extension_names.push(DebugUtils::name().as_ptr());
            // extension_names.push(CStr::from_bytes_with_nul_unchecked(b"VK_KHR_device_group\0").as_ptr());

            let appinfo = vk::ApplicationInfo::builder()
                .application_name(app_name)
                .application_version(0)
                .engine_name(app_name)
                .engine_version(0)
                .api_version(vk::make_api_version(0, 1, 3, 0));

            let create_info = vk::InstanceCreateInfo::builder()
                .application_info(&appinfo)
                .enabled_layer_names(&layers_names_raw)
                .enabled_extension_names(&extension_names)
                .flags(vk::InstanceCreateFlags::default());

            let instance: Instance = entry
                .create_instance(&create_info, None)
                .expect("Instance creation error");

            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(vulkan_debug_callback));

            let debug_utils_loader = DebugUtils::new(&entry, &instance);
            let debug_callback = debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap();

            let surface = ash_window::create_surface(&entry, &instance, &window, None).unwrap();

            let pdevices = instance
                .enumerate_physical_devices()
                .expect("Physical Device Error");

            let surface_loader = Surface::new(&entry, &instance);
            let (pdevice, queue_family_index) = pdevices
                .iter()
                .find_map(|pdevice| {
                    instance
                        .get_physical_device_queue_family_properties(*pdevice)
                        .iter()
                        .enumerate()
                        .find_map(|(index, info)| {
                            let supports_graphic_and_surface =
                                info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                    && surface_loader
                                        .get_physical_device_surface_support(
                                            *pdevice,
                                            index as u32,
                                            surface,
                                        )
                                        .unwrap();
                            if supports_graphic_and_surface {
                                Some((*pdevice, index))
                            } else {
                                None
                            }
                        })
                })
                .expect("Couldn't find suitable device.");
            let queue_family_index = queue_family_index as u32;
            let device_extension_names_raw = [Swapchain::name().as_ptr()];
            let features = vk::PhysicalDeviceFeatures {
                shader_clip_distance: 1,
                ..Default::default()
            };
            let priorities = [1.0];

            let queue_info = vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .queue_priorities(&priorities);

            let device_create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(std::slice::from_ref(&queue_info))
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&features);

            let device: Device = instance
                .create_device(pdevice, &device_create_info, None)
                .unwrap();
            let swapchain_loader = Swapchain::new(&instance, &device);
            let present_queue = device.get_device_queue(queue_family_index as u32, 0);
            let (
                swapchain,
                surface_resolution,
                surface_format,
                present_images,
                present_image_views,
            ) = create_swapchain(
                &device,
                &pdevice,
                &surface_loader,
                &surface,
                &swapchain_loader,
            );

            let pool_create_info = vk::CommandPoolCreateInfo::builder()
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(queue_family_index);

            let command_pool = device.create_command_pool(&pool_create_info, None).unwrap();

            let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_buffer_count(1)
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY);

            let command_buffer = device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .unwrap()[0];

            let render_pass = create_render_pass(&device, surface_format);
            let framebuffers = create_framebuffers(
                &device,
                &present_image_views,
                surface_resolution,
                render_pass,
            );

            let semaphore_create_info = vk::SemaphoreCreateInfo::default();

            let present_semaphore = device
                .create_semaphore(&semaphore_create_info, None)
                .unwrap();
            let render_semaphore = device
                .create_semaphore(&semaphore_create_info, None)
                .unwrap();
            let render_fence = device
                .create_fence(
                    &vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED),
                    None,
                )
                .unwrap();

            let compiler = shaderc::Compiler::new().unwrap();

            let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder()
                .flags(vk::PipelineLayoutCreateFlags::empty())
                .set_layouts(&[])
                .push_constant_ranges(&[]);
            let pipeline_layout = device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .unwrap();
            let shaders = vec![
                compile_shader(
                    &device,
                    &compiler,
                    "assets/shaders/triangle.frag",
                    shaderc::ShaderKind::Fragment,
                ),
                compile_shader(
                    &device,
                    &compiler,
                    "assets/shaders/triangle.vert",
                    shaderc::ShaderKind::Vertex,
                ),
            ];
            let shader_info = vec![
                shader_stage_create_info(vk::ShaderStageFlags::FRAGMENT, shaders[0]).build(),
                shader_stage_create_info(vk::ShaderStageFlags::VERTEX, shaders[1]).build(),
            ];
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: surface_resolution,
            };
            let viewport = vk::Viewport {
                x: 0f32,
                y: 0f32,
                width: surface_resolution.width as f32,
                height: surface_resolution.height as f32,
                min_depth: 0.0f32,
                max_depth: 1.0f32,
            };
            let pipeline = build_pipeline(
                &device,
                render_pass,
                &shader_info,
                pipeline_layout,
                viewport,
                scissor,
            );
            for shader in shaders {
                device.destroy_shader_module(shader, None)
            }

            let mut allocator = Allocator::new(&AllocatorCreateDesc {
                instance: instance.clone(),
                device: device.clone(),
                physical_device: pdevice,
                debug_settings: Default::default(),
                buffer_device_address: false,
            })
            .unwrap();

            let meshes = triangle_mesh(&device, &mut allocator);
            VkEngine {
                window,
                event_loop: RefCell::new(event_loop),
                entry,
                instance,
                debug_utils_loader,
                debug_callback,
                device,
                queue_family_index,
                pdevice,
                surface_loader,
                present_queue,
                command_pool,
                command_buffer,
                surface,
                surface_format,
                surface_resolution,
                swapchain,
                swapchain_loader,
                present_images,
                present_image_views,
                framebuffers,
                render_pass,
                render_fence,
                render_semaphore,
                present_semaphore,
                pipeline_layout,
                pipeline,
                compiler,
                allocator: Some(allocator),
                meshes,
            }
        }
    }

    pub fn draw(&self) {
        unsafe {
            self.device
                .wait_for_fences(&[self.render_fence], true, 1000000000)
                .unwrap();
            self.device.reset_fences(&[self.render_fence]).unwrap();
            let (swapchain_index, _suboptimal) = self
                .swapchain_loader
                .acquire_next_image(
                    self.swapchain,
                    1000000000,
                    self.present_semaphore,
                    vk::Fence::null(),
                )
                .unwrap();
            self.device
                .reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty())
                .unwrap();
            let command_begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device
                .begin_command_buffer(self.command_buffer, &command_begin_info)
                .unwrap();

            let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.render_pass)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.surface_resolution,
                })
                .framebuffer(self.framebuffers[swapchain_index as usize])
                .clear_values(&[vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0f32, 0f32, 0f32, 1f32],
                    },
                }]);
            self.device.cmd_begin_render_pass(
                self.command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
            self.device.cmd_bind_pipeline(
                self.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
            let buffers = [self.meshes.buffer];
            let offets = [0];
            self.device
                .cmd_bind_vertex_buffers(self.command_buffer, 0, &buffers, &offets);

            self.device.cmd_draw(self.command_buffer, 3, 1, 0, 0);

            self.device.cmd_end_render_pass(self.command_buffer);
            self.device.end_command_buffer(self.command_buffer).unwrap();

            let submit_info = vk::SubmitInfo::builder()
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                .wait_semaphores(&[self.present_semaphore])
                .signal_semaphores(&[self.render_semaphore])
                .command_buffers(&[self.command_buffer])
                .build();

            self.device
                .queue_submit(self.present_queue, &[submit_info], self.render_fence)
                .expect("Queue Submit Failure");

            let swapchains = &[self.swapchain];
            let wait_semaphores = &[self.render_semaphore];
            let image_indices = &[swapchain_index];
            let present_info = vk::PresentInfoKHR::builder()
                .swapchains(swapchains)
                .wait_semaphores(wait_semaphores)
                .image_indices(image_indices);

            self.swapchain_loader
                .queue_present(self.present_queue, &present_info)
                .expect("Issue Presenting");
        }
    }
}

impl Drop for VkEngine {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            self.device.destroy_buffer(self.meshes.buffer, None);

            let meshes = std::mem::take(&mut self.meshes.meshes);
            if let (Some(ms), Some(mut alloc)) = (meshes, std::mem::take(&mut self.allocator)) {
                for mesh in ms.into_iter() {
                    alloc.free(mesh.allocation).unwrap();
                }
            }
            drop(std::mem::take(&mut self.allocator));
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_pipeline(self.pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            for &image_view in self.present_image_views.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            for &framebuffer in self.framebuffers.iter() {
                self.device.destroy_framebuffer(framebuffer, None);
            }
            self.device.destroy_render_pass(self.render_pass, None);
            self.device.destroy_semaphore(self.present_semaphore, None);
            self.device.destroy_semaphore(self.render_semaphore, None);
            self.device.destroy_fence(self.render_fence, None);

            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_callback, None);
            self.instance.destroy_instance(None);
        }
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number: i32 = callback_data.message_id_number as i32;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    println!(
        "{:?}:\n{:?} [{} ({})] : {}\n",
        message_severity,
        message_type,
        message_id_name,
        &message_id_number.to_string(),
        message,
    );

    vk::FALSE
}

unsafe fn create_swapchain(
    device: &Device,
    pdevice: &vk::PhysicalDevice,
    surface_loader: &Surface,
    surface: &vk::SurfaceKHR,
    swapchain_loader: &Swapchain,
) -> (
    vk::SwapchainKHR,
    vk::Extent2D,
    vk::SurfaceFormatKHR,
    Vec<vk::Image>,
    Vec<vk::ImageView>,
) {
    let surface_format = surface_loader
        .get_physical_device_surface_formats(*pdevice, *surface)
        .unwrap()[0];
    let surface_capabilities = surface_loader
        .get_physical_device_surface_capabilities(*pdevice, *surface)
        .unwrap();
    let mut desired_image_count = surface_capabilities.min_image_count + 1;
    if surface_capabilities.max_image_count > 0
        && desired_image_count > surface_capabilities.max_image_count
    {
        desired_image_count = surface_capabilities.max_image_count;
    }

    let surface_resolution = match surface_capabilities.current_extent.width {
        std::u32::MAX => vk::Extent2D {
            width: 800,
            height: 600,
        },
        _ => surface_capabilities.current_extent,
    };
    let pre_transform = if surface_capabilities
        .supported_transforms
        .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
    {
        vk::SurfaceTransformFlagsKHR::IDENTITY
    } else {
        surface_capabilities.current_transform
    };
    let present_modes = surface_loader
        .get_physical_device_surface_present_modes(*pdevice, *surface)
        .unwrap();
    let present_mode = present_modes
        .iter()
        .cloned()
        .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO);

    let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
        .surface(*surface)
        .min_image_count(desired_image_count)
        .image_color_space(surface_format.color_space)
        .image_format(surface_format.format)
        .image_extent(surface_resolution)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(pre_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .image_array_layers(1);

    let swapchain = swapchain_loader
        .create_swapchain(&swapchain_create_info, None)
        .unwrap();

    let present_images = swapchain_loader.get_swapchain_images(swapchain).unwrap();
    let present_image_views = present_images
        .iter()
        .map(|&image| {
            let create_view_info = vk::ImageViewCreateInfo::builder()
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::R,
                    g: vk::ComponentSwizzle::G,
                    b: vk::ComponentSwizzle::B,
                    a: vk::ComponentSwizzle::A,
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image(image);
            device.create_image_view(&create_view_info, None).unwrap()
        })
        .collect();
    (
        swapchain,
        surface_resolution,
        surface_format,
        present_images,
        present_image_views,
    )
}

unsafe fn create_framebuffers(
    device: &Device,
    present_image_views: &Vec<vk::ImageView>,
    surface_resolution: vk::Extent2D,
    render_pass: vk::RenderPass,
) -> Vec<vk::Framebuffer> {
    present_image_views
        .iter()
        .map(|&image_view| {
            device
                .create_framebuffer(
                    &vk::FramebufferCreateInfo::builder()
                        .render_pass(render_pass)
                        .attachments(&[image_view])
                        .width(surface_resolution.width)
                        .height(surface_resolution.height)
                        .layers(1),
                    None,
                )
                .unwrap()
        })
        .collect()
}

unsafe fn create_render_pass(
    device: &Device,
    surface_format: vk::SurfaceFormatKHR,
) -> vk::RenderPass {
    let color_attachment = [vk::AttachmentDescription {
        format: surface_format.format,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
        ..Default::default()
    }];
    let color_attachment_ref = [vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    }];

    let subpass = [vk::SubpassDescription {
        pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
        color_attachment_count: 1,
        p_color_attachments: color_attachment_ref.as_ptr(),
        ..Default::default()
    }];

    let renderpass = vk::RenderPassCreateInfo::builder()
        .attachments(&color_attachment)
        .subpasses(&subpass);

    device.create_render_pass(&renderpass, None).unwrap()
}

fn compile_shader(
    device: &Device,
    compiler: &shaderc::Compiler,
    file: &str,
    kind: shaderc::ShaderKind,
) -> vk::ShaderModule {
    let source = std::fs::read_to_string(file).unwrap();
    let artifact = compiler
        .compile_into_spirv(source.as_str(), kind, file, "main", None)
        .unwrap();

    let create_info = vk::ShaderModuleCreateInfo::builder().code(artifact.as_binary());
    unsafe { device.create_shader_module(&create_info, None).unwrap() }
}
