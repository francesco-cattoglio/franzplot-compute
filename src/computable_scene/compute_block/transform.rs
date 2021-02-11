use crate::shader_processing::*;
use super::{ProcessedMap, ProcessingResult};
use super::{ComputeBlock, BlockCreationError, BlockId, Dimensions};
use crate::rendering::model::MODEL_CHUNK_VERTICES;
use crate::rendering::{StandardVertexData, GLSL_STANDARD_VERTEX_STRUCT};

const LOCAL_SIZE_X: usize = 16;
const LOCAL_SIZE_Y: usize = 16;

#[derive(Debug)]
pub struct TransformBlockDescriptor {
    pub geometry: Option<BlockId>,
    pub matrix: Option<BlockId>,
}

impl TransformBlockDescriptor {
    pub fn make_block(self, device: &wgpu::Device, processed_blocks: &ProcessedMap) -> ProcessingResult {
        Ok(ComputeBlock::Transform(TransformData::new(device, processed_blocks, self)?))
    }
}

pub struct TransformData {
    pub out_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
    dispatch_sizes: (usize, usize),
}

impl TransformData {
    pub fn new(device: &wgpu::Device, processed_blocks: &ProcessedMap, descriptor: TransformBlockDescriptor) -> Result<Self, BlockCreationError> {
        let geometry_id = descriptor.geometry.ok_or(BlockCreationError::InputMissing(" This Transform node \n is missing the Geometry input "))?;
        let found_element = processed_blocks.get(&geometry_id).ok_or(BlockCreationError::InternalError("Transform Geometry input does not exist in the block map"))?;
        let geometry_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;

        let matrix_id = descriptor.matrix.ok_or(BlockCreationError::InputMissing(" This Transform node \n is missing the Matrix input "))?;
        let found_element = processed_blocks.get(&matrix_id).ok_or(BlockCreationError::InternalError("Transform Matrix input does not exist in the block map"))?;
        let matrix_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;

        let (geometry_dim, geometry_buffer) = match geometry_block {
            ComputeBlock::Point(data) => (data.out_dim.clone(), &data.out_buffer),
            ComputeBlock::Curve(data) => (data.out_dim.clone(), &data.out_buffer),
            ComputeBlock::Surface(data) => (data.out_dim.clone(), &data.out_buffer),
            ComputeBlock::Transform(data) => (data.out_dim.clone(), &data.out_buffer),
            ComputeBlock::Prefab(data) => (data.out_dim.clone(), &data.out_buffer),
            _ => return Err(BlockCreationError::InputInvalid("the first input provided to the Transform is not a Geometry"))
        };
        let (matrix_dim, matrix_buffer) = match matrix_block {
            ComputeBlock::Matrix(data) => (data.out_dim.clone(), &data.out_buffer),
            _ => return Err(BlockCreationError::InputInvalid("the second input provided to the Transform is not a Matrix"))
        };
        let out_dim: Dimensions;
        let out_buffer: wgpu::Buffer;
        let compute_pipeline: wgpu::ComputePipeline;
        let compute_bind_group: wgpu::BindGroup;
        let dispatch_sizes: (usize, usize);
        let vec4_size = 4 * std::mem::size_of::<f32>();
        let vertex_size = std::mem::size_of::<StandardVertexData>();
        // This massive match statement handles the 9 different possible combinations
        // of geometries and matrices being applied to them.
        // Some of these cases are really simple (usually this is true when the matrix is a non-parametric one).
        // While others can be quite convoluted. However this match statement should be easy to understand.
        // One important detail, some of these arms have if conditions to check if the parameter
        // used in the matrix is the same used in the geometry. Make sure to keep them in the
        // correct order (i.e: they must be before the match with no if condition)
        match (geometry_dim, matrix_dim) {
            (Dimensions::D0, Dimensions::D0) => {
                out_dim = Dimensions::D0;
                out_buffer = out_dim.create_storage_buffer(vec4_size, &device);
                dispatch_sizes = (1, 1);
                let (pipeline, bind_group) = Self::transform_0d_0d(
                    device,
                    geometry_buffer,
                    matrix_buffer,
                    &out_buffer,
                    )?;
                compute_pipeline = pipeline;
                compute_bind_group = bind_group;
            },
            (Dimensions::D0, Dimensions::D1(mat_param)) => {
                out_dim = Dimensions::D1(mat_param.clone());
                out_buffer = out_dim.create_storage_buffer(vec4_size, &device);
                dispatch_sizes = (mat_param.size/LOCAL_SIZE_X, 1);
                let (pipeline, bind_group) = Self::transform_0d_up1(
                    device,
                    geometry_buffer,
                    matrix_buffer,
                    &out_buffer,
                    )?;
                compute_pipeline = pipeline;
                compute_bind_group = bind_group;
            },
            (Dimensions::D1(geo_param), Dimensions::D0) => {
                out_dim = Dimensions::D1(geo_param.clone());
                out_buffer = out_dim.create_storage_buffer(vec4_size, &device);
                dispatch_sizes = (geo_param.size/LOCAL_SIZE_X, 1);
                let (pipeline, bind_group) = Self::transform_1d_1d(
                    device,
                    geometry_buffer,
                    matrix_buffer,
                    &out_buffer,
                    )?;
                compute_pipeline = pipeline;
                compute_bind_group = bind_group;
            },
            (Dimensions::D1(geo_param), Dimensions::D1(mat_param)) if geo_param.is_equal(&mat_param)? => {
                out_dim = Dimensions::D1(geo_param.clone());
                out_buffer = out_dim.create_storage_buffer(vec4_size, &device);
                dispatch_sizes = (geo_param.size/LOCAL_SIZE_X, 1);
                let (pipeline, bind_group) = Self::transform_1d_multi(
                    device,
                    geometry_buffer,
                    matrix_buffer,
                    &out_buffer,
                    )?;
                compute_pipeline = pipeline;
                compute_bind_group = bind_group;
            },
            (Dimensions::D1(geo_param), Dimensions::D1(mat_param)) => {
                out_dim = Dimensions::D2(geo_param.clone(), mat_param.clone());
                out_buffer = out_dim.create_storage_buffer(vec4_size, &device);
                dispatch_sizes = (geo_param.size/LOCAL_SIZE_X, mat_param.size/LOCAL_SIZE_Y);
                let (pipeline, bind_group) = Self::transform_1d_up2(
                    device,
                    geometry_buffer,
                    matrix_buffer,
                    &out_buffer,
                    )?;
                compute_pipeline = pipeline;
                compute_bind_group = bind_group;
            },
            (Dimensions::D2(geo_p1, geo_p2), Dimensions::D0) => {
                out_dim = Dimensions::D2(geo_p1.clone(), geo_p2.clone());
                out_buffer = out_dim.create_storage_buffer(vec4_size, &device);
                dispatch_sizes = (geo_p1.size/LOCAL_SIZE_X, geo_p2.size/LOCAL_SIZE_Y);
                let (pipeline, bind_group) = Self::transform_2d_2d(
                    device,
                    geometry_buffer,
                    matrix_buffer,
                    &out_buffer,
                    )?;
                compute_pipeline = pipeline;
                compute_bind_group = bind_group;
            },
            (Dimensions::D2(geo_p1, geo_p2), Dimensions::D1(mat_param)) if geo_p1.is_equal(&mat_param)? => {
                out_dim = Dimensions::D2(geo_p1.clone(), geo_p2.clone());
                out_buffer = out_dim.create_storage_buffer(vec4_size, &device);
                dispatch_sizes = (geo_p1.size/LOCAL_SIZE_X, geo_p2.size/LOCAL_SIZE_Y);
                let (pipeline, bind_group) = Self::transform_2d_same_param(
                    device,
                    geometry_buffer,
                    matrix_buffer,
                    &out_buffer,
                    1
                    )?;
                compute_pipeline = pipeline;
                compute_bind_group = bind_group;
            },
            (Dimensions::D2(geo_p1, geo_p2), Dimensions::D1(mat_param)) if geo_p2.is_equal(&mat_param)? => {
                out_dim = Dimensions::D2(geo_p1.clone(), geo_p2.clone());
                out_buffer = out_dim.create_storage_buffer(vec4_size, &device);
                dispatch_sizes = (geo_p1.size/LOCAL_SIZE_X, geo_p2.size/LOCAL_SIZE_Y);
                let (pipeline, bind_group) = Self::transform_2d_same_param(
                    device,
                    geometry_buffer,
                    matrix_buffer,
                    &out_buffer,
                    2
                    )?;
                compute_pipeline = pipeline;
                compute_bind_group = bind_group;
            },
            (Dimensions::D2(_geo_p1, _geo_p2), Dimensions::D1(_mat_param)) => {
                return Err(BlockCreationError::InputInvalid(" this operation would create \n an object with three parameters, \n which is not supported "));
            },
            (Dimensions::D3(vertex_count, prefab_id), Dimensions::D0) => {
                out_dim = Dimensions::D3(vertex_count, prefab_id);
                out_buffer = out_dim.create_storage_buffer(vertex_size, &device);
                dispatch_sizes = (vertex_count/MODEL_CHUNK_VERTICES, 1);
                let (pipeline, bind_group) = Self::transform_3d_3d(
                    device,
                    geometry_buffer,
                    matrix_buffer,
                    &out_buffer,
                    )?;
                compute_pipeline = pipeline;
                compute_bind_group = bind_group;
            },
            (Dimensions::D3(_, _), Dimensions::D1(_)) => {
                return Err(BlockCreationError::InternalError(" applying a parametric transform \n to a primitive geometry \n is not allowed "));
            },
            (_, Dimensions::D2(_, _)) => {
                return Err(BlockCreationError::InternalError("the input Matrix has 2 parameters"));
            },
            (_, Dimensions::D3(_, _)) => {
                return Err(BlockCreationError::InternalError("the input Matrix has dimension equal to 3"));
            },
        }
        Ok(Self {
            compute_pipeline,
            compute_bind_group,
            dispatch_sizes,
            out_dim,
            out_buffer,
        })
    }

    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor{
                label: Some("transform compute pass")
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.dispatch(self.dispatch_sizes.0 as u32, self.dispatch_sizes.1 as u32, 1);
    }

    fn transform_0d_0d(device: &wgpu::Device, in_buff: &wgpu::Buffer, in_matrix: &wgpu::Buffer, out_buff: &wgpu::Buffer) -> CompilationResult {
        let shader_source = format!(r##"
#version 450
layout(local_size_x = 1, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputPoint {{
    vec4 in_buff;
}};

layout(set = 0, binding = 1) buffer InputMatrix {{
    mat4 in_matrix;
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff;
}};

void main() {{
    out_buff = in_matrix * in_buff;
}}
"##);
        //println!("debug info for 1d->1d transform shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer: in_buff,
        });
        // add descriptor for matrix
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer: in_matrix,
        });
        bindings.push(CustomBindDescriptor {
            position: 2,
            buffer: out_buff,
        });
        compile_compute_shader(device, shader_source.as_str(), &bindings, None, Some("Transform"))
    }

    fn transform_0d_up1(device: &wgpu::Device, in_buff: &wgpu::Buffer, in_matrix: &wgpu::Buffer, out_buff: &wgpu::Buffer,
                     ) -> CompilationResult {
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputCurve {{
    vec4 in_point;
}};

layout(set = 0, binding = 1) buffer InputMatrix {{
    mat4 in_matrix[];
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff[];
}};

void main() {{
    // the output index should be the same as the common 2D -> 2D transform
    uint index = gl_GlobalInvocationID.x;
    // while the index used for accessing the inputs are the global invocation id for x and y
    out_buff[index] = in_matrix[index] * in_point;
}}
"##, dimx=LOCAL_SIZE_X);
        //println!("debug info for 1d->1d transform shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer: in_buff,
        });
        // add descriptor for matrix
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer: in_matrix,
        });
        bindings.push(CustomBindDescriptor {
            position: 2,
            buffer: out_buff,
        });
        compile_compute_shader(device, shader_source.as_str(), &bindings, None, Some("Transform"))
    }

    fn transform_1d_1d(device: &wgpu::Device, in_buff: &wgpu::Buffer, in_matrix: &wgpu::Buffer, out_buff: &wgpu::Buffer) -> CompilationResult {
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputCurve {{
    vec4 in_buff[];
}};

layout(set = 0, binding = 1) buffer InputMatrix {{
    mat4 in_matrix;
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff[];
}};

void main() {{
    uint index = gl_GlobalInvocationID.x;
    out_buff[index] = in_matrix * in_buff[index];
}}
"##, dimx=LOCAL_SIZE_X);
        //println!("debug info for 1d->1d transform shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer: in_buff,
        });
        // add descriptor for matrix
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer: in_matrix,
        });
        bindings.push(CustomBindDescriptor {
            position: 2,
            buffer: out_buff,
        });
        compile_compute_shader(device, shader_source.as_str(), &bindings, None, Some("Transform"))
    }

    fn transform_1d_multi(device: &wgpu::Device, in_buff: &wgpu::Buffer, in_matrix: &wgpu::Buffer, out_buff: &wgpu::Buffer) -> CompilationResult {
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputCurve {{
    vec4 in_buff[];
}};

layout(set = 0, binding = 1) buffer InputMatrix {{
    mat4 in_matrix[];
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff[];
}};

void main() {{
    uint index = gl_GlobalInvocationID.x;
    out_buff[index] = in_matrix[index] * in_buff[index];
}}
"##, dimx=LOCAL_SIZE_X);
        //println!("debug info for 1d multi 1d transform shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer: in_buff,
        });
        // add descriptor for matrix
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer: in_matrix,
        });
        bindings.push(CustomBindDescriptor {
            position: 2,
            buffer: out_buff,
        });
        compile_compute_shader(device, shader_source.as_str(), &bindings, None, Some("Transform"))
    }

    fn transform_2d_2d(device: &wgpu::Device, in_buff: &wgpu::Buffer, in_matrix: &wgpu::Buffer, out_buff: &wgpu::Buffer) -> CompilationResult {
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = {dimy}) in;

layout(set = 0, binding = 0) buffer InputSurface {{
    vec4 in_buff[];
}};

layout(set = 0, binding = 1) buffer InputMatrix {{
    mat4 in_matrix;
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff[];
}};

void main() {{
    // the only difference between the 1d->1d and the 2d->2d shader is the local_sizes and the indexing
    uint par1_idx = gl_GlobalInvocationID.x;
    uint par2_idx = gl_GlobalInvocationID.y;
    uint index = gl_GlobalInvocationID.x + gl_NumWorkGroups.x * gl_WorkGroupSize.x * gl_GlobalInvocationID.y;
    out_buff[index] = in_matrix * in_buff[index];
}}
"##, dimx=LOCAL_SIZE_X, dimy=LOCAL_SIZE_Y);
        //println!("debug info for 1d->1d transform shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer: in_buff,
        });
        // add descriptor for matrix
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer: in_matrix,
        });
        bindings.push(CustomBindDescriptor {
            position: 2,
            buffer: out_buff,
        });
        compile_compute_shader(device, shader_source.as_str(), &bindings, None, Some("Transform"))
    }

    fn transform_3d_3d(device: &wgpu::Device, in_buff: &wgpu::Buffer, in_matrix: &wgpu::Buffer, out_buff: &wgpu::Buffer) -> CompilationResult {
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

{vertex_struct}

layout(set = 0, binding = 0) buffer InputSurface {{
    Vertex in_buff[];
}};

layout(set = 0, binding = 1) buffer InputMatrix {{
    mat4 in_matrix;
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    Vertex out_buff[];
}};

void main() {{
    // in 3d->3d we operate on both the positions AND the normals.
    uint idx = gl_GlobalInvocationID.x;
    // positions get multiplied by the matrix
    vec4 in_position = in_buff[idx].position;
    out_buff[idx].position = in_matrix * in_position;
    // normals get multiplied by the inverse transpose of the matrix,
    // PROVIDED THE MATRIX IS NOT SINGULAR
    // TODO: possible optimization: precompute inverse transpose
    // directly in the matrix compute block (for 0D matrices only)
    float determinant = determinant(in_matrix);
    if (determinant > 1e-6) {{
        out_buff[idx].normal = normalize(transpose(inverse(in_matrix))*in_buff[idx].normal);
    }} else {{
        // this is wrong, but at least it won't produce undefined garbage results
        out_buff[idx].normal = in_buff[idx].normal;
    }}
    out_buff[idx].uv_coords = in_buff[idx].uv_coords;
    out_buff[idx]._padding = in_buff[idx]._padding;
}}
"##, vertex_struct=GLSL_STANDARD_VERTEX_STRUCT, dimx=MODEL_CHUNK_VERTICES);
        //println!("debug info for 1d->1d transform shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer: in_buff,
        });
        // add descriptor for matrix
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer: in_matrix,
        });
        bindings.push(CustomBindDescriptor {
            position: 2,
            buffer: out_buff,
        });
        compile_compute_shader(device, shader_source.as_str(), &bindings, None, Some("Transform"))
    }

    fn transform_1d_up2(device: &wgpu::Device, in_buff: &wgpu::Buffer, in_matrix: &wgpu::Buffer, out_buff: &wgpu::Buffer) -> CompilationResult {
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = {dimy}) in;

layout(set = 0, binding = 0) buffer InputCurve {{
    vec4 in_buff[];
}};

layout(set = 0, binding = 1) buffer InputMatrix {{
    mat4 in_matrix[];
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff[];
}};

void main() {{
    // the output index should be the same as the common 2D -> 2D transform
    uint par1_idx = gl_GlobalInvocationID.x;
    uint par2_idx = gl_GlobalInvocationID.y;
    uint index = gl_GlobalInvocationID.x + gl_NumWorkGroups.x * gl_WorkGroupSize.x * gl_GlobalInvocationID.y;
    // while the index used for accessing the inputs are the global invocation id for x and y
    out_buff[index] = in_matrix[par2_idx] * in_buff[par1_idx];
}}
"##, dimx=LOCAL_SIZE_X, dimy=LOCAL_SIZE_Y);
        //println!("debug info for 1d->1d transform shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer: in_buff,
        });
        // add descriptor for matrix
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer: in_matrix,
        });
        bindings.push(CustomBindDescriptor {
            position: 2,
            buffer: out_buff,
        });
        compile_compute_shader(device, shader_source.as_str(), &bindings, None, Some("Transform"))
    }

    fn transform_2d_same_param(device: &wgpu::Device, in_buff: &wgpu::Buffer, in_matrix: &wgpu::Buffer, out_buff: &wgpu::Buffer,
        which_param: u32) -> CompilationResult {
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = {dimy}) in;

layout(set = 0, binding = 0) buffer InputSurface {{
    vec4 in_buff[];
}};

layout(set = 0, binding = 1) buffer InputMatrix {{
    mat4 in_matrix[];
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff[];
}};

void main() {{
    uint index_1 = gl_GlobalInvocationID.x;
    uint index_2 = gl_GlobalInvocationID.y;
    // the only difference between the 1d->1d and the 2d->2d shader is the local_sizes and the indexing
    uint index = gl_GlobalInvocationID.x + gl_NumWorkGroups.x * gl_WorkGroupSize.x * gl_GlobalInvocationID.y;
    out_buff[index] = in_matrix[index_{which_idx}] * in_buff[index];
}}
"##, dimx=LOCAL_SIZE_X, dimy=LOCAL_SIZE_Y, which_idx=which_param);
        //println!("debug info for 1d->1d transform shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer: in_buff,
        });
        // add descriptor for matrix
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer: in_matrix,
        });
        bindings.push(CustomBindDescriptor {
            position: 2,
            buffer: out_buff,
        });
        compile_compute_shader(device, shader_source.as_str(), &bindings, None, Some("Transform"))
    }
}
