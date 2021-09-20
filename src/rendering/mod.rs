pub mod camera;
pub mod model;
pub mod texture;
pub mod scene_renderer;

// TODO: just copy-paste the entire scene_renderer in here? Or move all these constants
// in that module and pub use the useful ones
pub use scene_renderer::SceneRenderer;

// BEWARE: whenever you do any change at the following structure, also remember to modify
// the corresponding VertexStateDescriptor that is used at pipeline creation stage
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct StandardVertexData {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub uv_coords: [f32; 2],
    // maybe we could pack the color and the 2d indices in here!
    // the color can be packed in [u8; 4] and passed in as Uchar4Norm,
    // while the two indices could simply be [u16; 2]
    pub _padding: [f32; 2],
}

impl StandardVertexData {
    fn vertex_attribute_array() -> [wgpu::VertexAttribute; 4] {
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x2, 3 => Sint32x2]
    }
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

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WireframeVertexData {
    position: [f32; 4],
    color: [u8; 4],
}

#[allow(unused)]
pub const GLSL_WIREFRAME_VERTEX_STRUCT: & str = r##"
struct Vertex {
    vec3 position;
    vec4 color;
};
"##;

unsafe impl bytemuck::Pod for WireframeVertexData {}
unsafe impl bytemuck::Zeroable for WireframeVertexData {}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct BillboardVertexData {
    position: [f32; 2],
    offset: [f32; 3],
    color: [u8; 4],
}

#[allow(unused)]
pub const GLSL_BILLBOARD_VERTEX_STRUCT: & str = r##"
struct Vertex {
    vec2 position;
    vec3 offset;
    vec4 color;
};
"##;

unsafe impl bytemuck::Pod for BillboardVertexData {}
unsafe impl bytemuck::Zeroable for BillboardVertexData {}


pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
pub const SWAPCHAIN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;


