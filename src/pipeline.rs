use std::ffi::CStr;

use ash::{vk, Device};
use memoffset::offset_of;

use crate::mesh::Vertex;

pub unsafe fn shader_stage_create_info<'a>(
    stage: vk::ShaderStageFlags,
    shader_module: vk::ShaderModule,
) -> vk::PipelineShaderStageCreateInfoBuilder<'a> {
    vk::PipelineShaderStageCreateInfo::builder()
        .stage(stage)
        .module(shader_module)
        .name(CStr::from_bytes_with_nul_unchecked(b"main\0"))
}

unsafe fn vertex_input_state_create_info<'a>() -> (
    Vec<vk::VertexInputAttributeDescription>,
    Vec<vk::VertexInputBindingDescription>,
) {
    let main_binding = vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(std::mem::size_of::<Vertex>() as u32)
        .input_rate(vk::VertexInputRate::VERTEX);

    let position_attr = vk::VertexInputAttributeDescription::builder()
        .binding(0)
        .location(0)
        .format(vk::Format::R32G32B32A32_SFLOAT)
        .offset(offset_of!(Vertex, position) as u32);

    let color_attr = vk::VertexInputAttributeDescription::builder()
        .binding(0)
        .location(1)
        .format(vk::Format::R32G32B32A32_SFLOAT)
        .offset(offset_of!(Vertex, color) as u32);

    let attributes = vec![color_attr.build(), position_attr.build()];
    let bindings = vec![main_binding.build()];
    (attributes, bindings)
}

unsafe fn input_assembly_create_info<'a>(
    topology: vk::PrimitiveTopology,
) -> vk::PipelineInputAssemblyStateCreateInfoBuilder<'a> {
    vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(topology)
        .primitive_restart_enable(false)
}

unsafe fn rasterization_state_create_info<'a>(
    polygon_mode: vk::PolygonMode,
) -> vk::PipelineRasterizationStateCreateInfoBuilder<'a> {
    vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(polygon_mode)
        .line_width(1.0f32)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::CLOCKWISE)
        .depth_bias_enable(false)
        .depth_bias_constant_factor(0.0f32)
        .depth_bias_clamp(0.0f32)
        .depth_bias_slope_factor(0.0f32)
}

unsafe fn multisampling_state_create_info<'a>() -> vk::PipelineMultisampleStateCreateInfoBuilder<'a>
{
    vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1)
        .min_sample_shading(1.0f32)
        .sample_mask(&[])
        .alpha_to_coverage_enable(false)
        .alpha_to_one_enable(false)
}

unsafe fn color_blend_attachment_state<'a>() -> vk::PipelineColorBlendAttachmentStateBuilder<'a> {
    vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(false)
}

pub fn build_pipeline(
    device: &Device,
    render_pass: vk::RenderPass,
    shaders: &Vec<vk::PipelineShaderStageCreateInfo>,
    layout: vk::PipelineLayout,
    viewport: vk::Viewport,
    scissor: vk::Rect2D,
) -> vk::Pipeline {
    unsafe {
        let viewports = [viewport];
        let scissors = [scissor];
        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewport_count(1)
            .viewports(&viewports)
            .scissor_count(1)
            .scissors(&scissors);

        let color_attachment = color_blend_attachment_state();
        let attachments = [color_attachment.build()];
        let color_blending = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op(vk::LogicOp::COPY)
            .attachments(&attachments);

        let (attrs, bindings) = vertex_input_state_create_info();
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&attrs)
            .vertex_binding_descriptions(&bindings);

        let input_assembly = input_assembly_create_info(vk::PrimitiveTopology::TRIANGLE_LIST);
        let rasterization = rasterization_state_create_info(vk::PolygonMode::FILL);
        let multisampling = multisampling_state_create_info();

        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(shaders)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .color_blend_state(&color_blending)
            .rasterization_state(&rasterization)
            .multisample_state(&multisampling)
            .layout(layout)
            .render_pass(render_pass)
            .base_pipeline_handle(vk::Pipeline::null());

        let pipelines = &[pipeline_info.build()];
        device
            .create_graphics_pipelines(vk::PipelineCache::null(), pipelines, None)
            .unwrap()[0]
    }
}
