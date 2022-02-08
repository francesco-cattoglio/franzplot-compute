pub mod camera;
pub mod model;
pub mod texture;
pub mod scene_renderer;

pub use scene_renderer::SceneRenderer;

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
    const VERTEX_ATTR_ARRAY: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x2, 3 => Float32x2];
    fn vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::VERTEX_ATTR_ARRAY,
        }
    }
}

unsafe impl bytemuck::Pod for StandardVertexData {}
unsafe impl bytemuck::Zeroable for StandardVertexData {}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WireframeVertexData {
    position: [f32; 3],
    color: [u8; 4],
}

impl WireframeVertexData {
    const VERTEX_ATTR_ARRAY: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Unorm8x4];
    fn vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::VERTEX_ATTR_ARRAY,
        }
    }
}

unsafe impl bytemuck::Pod for WireframeVertexData {}
unsafe impl bytemuck::Zeroable for WireframeVertexData {}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct BillboardVertexData {
    position: [f32; 2],
    offset: [f32; 3],
    color: [u8; 4],
}

impl BillboardVertexData {
    const VERTEX_ATTR_ARRAY: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x3, 2 => Unorm8x4];
    pub fn vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::VERTEX_ATTR_ARRAY,
        }
    }
}

unsafe impl bytemuck::Pod for BillboardVertexData {}
unsafe impl bytemuck::Zeroable for BillboardVertexData {}

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
pub const SWAPCHAIN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;


