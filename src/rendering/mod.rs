pub mod camera;
pub mod texture;
pub mod scene_renderer;

// TODO: just copy-paste the entire scene_renderer in here? Or move all these constants
// in that module and pub use the useful ones
pub use scene_renderer::SceneRenderer;

// BEWARE: whenever you do any change at the following structure, also remember to modify
// the corresponding VertexStateDescriptor that is used at pipeline creation stage
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StandardVertexData {
    position: [f32; 4],
    normal: [f32; 4],
    uv_coords: [f32; 2],
    _padding: [f32; 2],
}

pub const GLSL_STANDARD_VERTEX_STRUCT: & str = r##"
struct Vertex {
    vec4 position;
    vec4 normal;
    vec2 uv_coords;
    vec2 _padding;
};
"##;

unsafe impl bytemuck::Pod for StandardVertexData {}
unsafe impl bytemuck::Zeroable for StandardVertexData {}

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const SWAPCHAIN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

pub const TEXTURE_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor =
wgpu::BindGroupLayoutDescriptor {
    entries: &[
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            count: None,
            visibility: wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::SampledTexture {
                multisampled: false,
                component_type: wgpu::TextureComponentType::Float,
                dimension: wgpu::TextureViewDimension::D2,
            },
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            count: None,
            visibility: wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::Sampler { comparison: false },
        },
    ],
    label: Some("texture bind group layout"),
};

pub const DEPTH_BUFFER_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor =
wgpu::BindGroupLayoutDescriptor {
    entries: &[
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            count: None,
            visibility: wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::SampledTexture {
                multisampled: false,
                component_type: wgpu::TextureComponentType::Float,
                dimension: wgpu::TextureViewDimension::D2,
            },
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            count: None,
            visibility: wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::Sampler { comparison: true },
        },
    ],
    label: Some("texture bind group layout"),
};

pub const CAMERA_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor =
wgpu::BindGroupLayoutDescriptor {
    entries: &[
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            count: None,
            visibility: wgpu::ShaderStage::VERTEX,
            ty: wgpu::BindingType::UniformBuffer {
                dynamic: false,
                min_binding_size: None,
            },
        },
    ],
    label: Some("camera bind group layout"),
};

pub const PICKING_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor =
wgpu::BindGroupLayoutDescriptor {
    entries: &[
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            count: None,
            visibility: wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::StorageBuffer {
                readonly: false,
                dynamic: false,
                min_binding_size: None,
            },
        },
    ],
    label: Some("object picking bind group layout"),
};

pub fn create_solid_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    // shader compiling
    let mut shader_compiler = shaderc::Compiler::new().unwrap();
    let vert_src = include_str!("solid.vert");
    let frag_src = include_str!("solid.frag");
    let vert_spirv = shader_compiler.compile_into_spirv(vert_src, shaderc::ShaderKind::Vertex, "solid.vert", "main", None).unwrap();
    let frag_spirv = shader_compiler.compile_into_spirv(frag_src, shaderc::ShaderKind::Fragment, "solid.frag", "main", None).unwrap();
    let vert_data = wgpu::util::make_spirv(vert_spirv.as_binary_u8());
    let frag_data = wgpu::util::make_spirv(frag_spirv.as_binary_u8());
    let vert_module = device.create_shader_module(vert_data);
    let frag_module = device.create_shader_module(frag_data);

    let camera_bind_layout = device.create_bind_group_layout(&CAMERA_LAYOUT_DESCRIPTOR);
    let picking_bind_layout = device.create_bind_group_layout(&PICKING_LAYOUT_DESCRIPTOR);
    let texture_bind_layout = device.create_bind_group_layout(&TEXTURE_LAYOUT_DESCRIPTOR);
    let render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            push_constant_ranges: &[],
            bind_group_layouts: &[&camera_bind_layout, &picking_bind_layout, &texture_bind_layout]
        });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
        layout: Some(&render_pipeline_layout),
        label: None,
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vert_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &frag_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            clamp_depth: false,
            cull_mode: wgpu::CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor{
            format: SWAPCHAIN_FORMAT,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL
        }],
        depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilStateDescriptor {
                front: wgpu::StencilStateFaceDescriptor::IGNORE,
                back: wgpu::StencilStateFaceDescriptor::IGNORE,
                read_mask: 0,
                write_mask: 0,
            }
        }),
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: &[
                wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<StandardVertexData>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float4, 1 => Float4, 2 => Float2],
                },
            ],
        },
    })

}

