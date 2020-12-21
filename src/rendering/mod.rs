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
    // maybe we could pack the color and the 2d indices in here!
    // the color can be packed in [u8; 4] and passed in as Uchar4Norm,
    // while the two indices could simply be [u16; 2]
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

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WireframeVertexData {
    position: [f32; 3],
    color: [u8; 4],
}

pub const GLSL_WIREFRAME_VERTEX_STRUCT: & str = r##"
struct Vertex {
    vec3 position;
    vec4 color;
};
"##;

unsafe impl bytemuck::Pod for WireframeVertexData {}
unsafe impl bytemuck::Zeroable for WireframeVertexData {}


pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const SWAPCHAIN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;


