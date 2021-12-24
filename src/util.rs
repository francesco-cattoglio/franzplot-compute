use crate::rust_gui;
use crate::device_manager;
use crate::rendering::texture;
use crate::rendering::model;
use crate::state::Action;
use crate::state::State;

use std::future::Future;
pub struct Executor {
    #[cfg(not(target_arch = "wasm32"))]
    pool: futures::executor::ThreadPool,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            pool: futures::executor::ThreadPool::new().unwrap(),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn execut<F: Future<Output = ()> + Send + 'static>(&self, f: F) {
        self.pool.spawn_ok(f);
    }
    #[cfg(target_arch = "wasm32")]
    pub fn execut<F: Future<Output = ()> + 'static>(&self, f: F) {
        wasm_bindgen_futures::spawn_local(f);
    }
}

pub fn create_storage_buffer(device: &wgpu::Device, buffer_size: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        mapped_at_creation: false,
        size: buffer_size as wgpu::BufferAddress,
        // Beware:copy and map are only needed when debugging/inspecting
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::MAP_READ,
    })
}


pub fn load_imgui_masks<P: AsRef<std::path::Path>>(manager: &device_manager::Manager, renderer: &mut imgui_wgpu::Renderer, files: &[P]) -> rust_gui::MaskIds {
    files.iter()
        .map(|path| {
            let texture = texture::Texture::thumbnail(&manager.device, &manager.queue, path, None).unwrap();
            renderer.textures.insert(texture.into())
        })
        .collect::<Vec<_>>() // make it into a vector
        .try_into() // and then turn it into an array
        .unwrap() // panic if dimensions don't match
}

pub fn load_imgui_materials<P: AsRef<std::path::Path>>(manager: &device_manager::Manager, renderer: &mut imgui_wgpu::Renderer, files: &[P]) -> rust_gui::MaterialIds {
    files.iter()
        .map(|path| {
            let texture = texture::Texture::thumbnail(&manager.device, &manager.queue, path, None).unwrap();
            renderer.textures.insert(texture.into())
        })
        .collect()
}

pub fn imgui_model_names<P: AsRef<std::path::Path>>(files: &[P]) -> Vec<imgui::ImString> {
    files.iter()
        .map(|path| {
            let file_stem = path.as_ref().file_stem().unwrap().to_str().unwrap();
            imgui::ImString::new(file_stem)
        })
        .collect()
}

pub fn load_masks<P: AsRef<std::path::Path>>(manager: &device_manager::Manager, files: &[P]) -> texture::Masks {
    files.iter()
        .map(|path| {
            texture::Texture::load(&manager.device, &manager.queue, path, None).unwrap()
        })
        .collect::<Vec<_>>() // make it into a vector
        .try_into() // and then turn it into an array
        .unwrap() // panic if dimensions don't match
}

pub fn load_materials<P: AsRef<std::path::Path>>(manager: &device_manager::Manager, files: &[P]) -> texture::Materials {
    files.iter()
        .map(|path| {
            texture::Texture::load(&manager.device, &manager.queue, path, None).unwrap()
        })
        .collect()
}

pub fn load_models<P: AsRef<std::path::Path>>(device: &wgpu::Device, files: &[P]) -> Vec<model::Model> {
    files.iter()
        .map(|path| {
            let obj_contents = obj::Obj::load(path).unwrap();
            model::Model::from_obj(device, &obj_contents.data)
        })
        .collect()
}

use winit::event::MouseScrollDelta;

// handling sensitivity is a bit messy, due to the fact that every platform handles things
// differently, and as a bonus, the MouseScrollDelta reported by a WindowEvent can be different
// from the MouseScrollDelta in a DeviceEvent.
#[cfg(target_os = "windows")]
pub fn compute_scroll(delta: MouseScrollDelta, sensitivity: f32) -> f32 {
    match delta {
        MouseScrollDelta::LineDelta(_x, y) => {
            sensitivity * y.cbrt()
        },
        MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition {y, ..}) => {
            sensitivity * y as f32
        }
    }
}

#[cfg(target_os = "macos")]
pub fn compute_scroll(delta: MouseScrollDelta, sensitivity: f32) -> f32 {
    match delta {
        MouseScrollDelta::LineDelta(_x, y) => {
            sensitivity * y.cbrt()
        },
        MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition {y, ..}) => {
            sensitivity * y as f32
        }
    }
}

#[cfg(target_os = "linux")]
pub fn compute_scroll(delta: MouseScrollDelta, sensitivity: f32) -> f32 {
    match delta {
        MouseScrollDelta::LineDelta(_x, y) => {
            sensitivity * y
        },
        MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition {y, ..}) => {
            sensitivity * y as f32
        }
    }
}

pub fn compute_scene_zoom(delta: MouseScrollDelta, sensitivity: f32) -> f32 {
    let coeff = -1.0;
    coeff * compute_scroll(delta, sensitivity)
}

pub fn compute_graph_zoom(delta: MouseScrollDelta, sensitivity: f32) -> f32 {
    let coeff = 1.0;
    coeff * compute_scroll(delta, sensitivity)
}

pub trait FourBytes {
    fn from_bytes(bytes: [u8; 4]) -> Self;
}

impl FourBytes for f32 {
    fn from_bytes(bytes: [u8; 4]) -> Self {
        f32::from_ne_bytes(bytes)
    }
}

impl FourBytes for i32 {
    fn from_bytes(bytes: [u8; 4]) -> Self {
        i32::from_ne_bytes(bytes)
    }
}

// maps a buffer, waits for it to be available, and copies its contents into a new Vec<T>
#[allow(unused)]
pub fn copy_buffer_as<T: FourBytes>(buffer: &wgpu::Buffer, device: &wgpu::Device) -> Vec<T> {
    use futures::executor::block_on;
    let future_result = buffer.slice(..).map_async(wgpu::MapMode::Read);
    device.poll(wgpu::Maintain::Wait);
    block_on(future_result).unwrap();
    let mapped_buffer = buffer.slice(..).get_mapped_range();
    let data: &[u8] = &mapped_buffer;
    use std::convert::TryInto;
    // Since contents are got in bytes, this converts these bytes back to f32
    let result: Vec<T> = data
        .chunks_exact(4)
        .map(|b| T::from_bytes(b.try_into().unwrap()))
        .skip(0)
        .step_by(1)
        .collect();
    // With the current interface, we have to make sure all mapped views are
    // dropped before we unmap the buffer.
    drop(mapped_buffer);
    buffer.unmap();

    result
}

/// support struct for copying textures to png
struct BufferDimensions {
    width: usize,
    height: usize,
    unpadded_bytes_per_row: usize,
    padded_bytes_per_row: usize,
}

impl BufferDimensions {
    fn new(width: usize, height: usize) -> Self {
        let bytes_per_pixel = std::mem::size_of::<u32>();
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
        Self {
            width,
            height,
            unpadded_bytes_per_row,
            padded_bytes_per_row,
        }
    }
}

pub fn create_graph_png<P: AsRef<std::path::Path>>(state: &mut State, output_path: &P,
                                                   window: &winit::window::Window, platform: &mut imgui_winit_support::WinitPlatform, renderer: &mut imgui_wgpu::Renderer,
                                                   rust_gui: &mut rust_gui::Gui, imgui: &mut imgui::Context, logical_size: winit::dpi::LogicalSize::<f32>) {
    let texture_size = wgpu::Extent3d {
        height: state.app.manager.size.height,
        width: state.app.manager.size.width,
        depth_or_array_layers: 1,
    };

    let output_texture = super::rendering::texture::Texture::create_screenshot_texture(&state.app.manager.device, texture_size, 1);

    // use the acquired frame for a rendering pass, which will clear the screen and render the gui
    let mut encoder: wgpu::CommandEncoder =
        state.app.manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[wgpu::RenderPassColorAttachment {
            view: &output_texture.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: true,
            },
        }],
        depth_stencil_attachment: None,
    });

    // actual imgui rendering
    // run it twice because we imgui needs to resize itself.
    let executor = Executor::new();
    {
        let ui = imgui.frame();
        let _requested_logical_rectangle = rust_gui.render(&ui, [logical_size.width, logical_size.height], state, &executor);
    }
    let ui = imgui.frame();
    let _requested_logical_rectangle = rust_gui.render(&ui, [logical_size.width, logical_size.height], state, &executor);
    // after calling the gui render function we know if we need to render the scene or not

    platform.prepare_render(&ui, window);
    renderer
        .render(ui.render(), &state.app.manager.queue, &state.app.manager.device, &mut rpass)
        .expect("Imgui rendering failed");

    drop(rpass); // dropping the render pass is required for the encoder.finish() command

    // submit the framebuffer rendering pass
    state.app.manager.queue.submit(Some(encoder.finish()));

    let buffer_dimensions = BufferDimensions::new(texture_size.width as usize, texture_size.height as usize);
    // The output buffer lets us retrieve the data as an array
    let png_buffer = state.app.manager.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let command_buffer = {
        use std::num::NonZeroU32;

        let mut encoder = state.app.manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        // Copy the data from the texture to the buffer
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &output_texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &png_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(NonZeroU32::new(buffer_dimensions.padded_bytes_per_row as u32).unwrap()),
                    rows_per_image: None,
                },
            },
            texture_size,
        );

        encoder.finish()
    };

    state.app.manager.queue.submit(Some(command_buffer));

    let buffer_slice = png_buffer.slice(..);
    let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

    // Poll the device in a blocking manner so that our future resolves.
    // In an actual application, `device.poll(...)` should
    // be called in an event loop or on another thread.
    state.app.manager.device.poll(wgpu::Maintain::Wait);
    // If a file system is available, write the buffer as a PNG
    let has_file_system_available = cfg!(not(target_arch = "wasm32"));
    if !has_file_system_available {
        return;
    }

    use futures::executor::block_on;
    block_on(buffer_future).unwrap();
    let padded_buffer = buffer_slice.get_mapped_range();
    let mut padded_vector = Vec::<u8>::new();
    for chunk in padded_buffer.chunks_exact(4).into_iter() {
        padded_vector.push(chunk[2]);
        padded_vector.push(chunk[1]);
        padded_vector.push(chunk[0]);
        padded_vector.push(chunk[3]);
    }

    let mut png_encoder = png::Encoder::new(
        std::fs::File::create(output_path).unwrap(),
        buffer_dimensions.width as u32,
        buffer_dimensions.height as u32,
    );
    png_encoder.set_depth(png::BitDepth::Eight);
    png_encoder.set_color(png::ColorType::RGBA);
    let mut png_writer = png_encoder
        .write_header()
        .unwrap()
        .into_stream_writer_with_size(buffer_dimensions.unpadded_bytes_per_row);

    // from the padded_buffer we write just the unpadded bytes into the image
    use std::io::Write;
    for chunk in padded_vector.chunks(buffer_dimensions.padded_bytes_per_row) {
        png_writer
            .write(&chunk[..buffer_dimensions.unpadded_bytes_per_row])
            .unwrap();
    }
    png_writer.finish().unwrap();

    // With the current interface, we have to make sure all mapped views are
    // dropped before we unmap the buffer.
    drop(padded_buffer);

    png_buffer.unmap();
}

pub fn create_scene_png<P: AsRef<std::path::Path>>(state: &mut State, output_path: &P) {
    let height = 1080;
    let width = 1920;
    let texture_size = wgpu::Extent3d {
        height,
        width,
        depth_or_array_layers: 1,
    };
    let output_texture = super::rendering::texture::Texture::create_output_texture(&state.app.manager.device, texture_size, 1);
    let processing_result = state.process(Action::ProcessUserState());
    if let Err(error) = processing_result {
            println!("Warning: errors detected in the scene: {}", error);
    }

    let render_request = Action::RenderScene(texture_size, &output_texture.view);
    state.process(render_request).expect("failed to render the scene due to an unknown error");

    let buffer_dimensions = BufferDimensions::new(width as usize, height as usize);
    // The output buffer lets us retrieve the data as an array
    let png_buffer = state.app.manager.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let command_buffer = {
        use std::num::NonZeroU32;

        let mut encoder = state.app.manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        // Copy the data from the texture to the buffer
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &output_texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &png_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(NonZeroU32::new(buffer_dimensions.padded_bytes_per_row as u32).unwrap()),
                    rows_per_image: None,
                },
            },
            texture_size,
        );

        encoder.finish()
    };

    state.app.manager.queue.submit(Some(command_buffer));

    let buffer_slice = png_buffer.slice(..);
    let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

    // Poll the device in a blocking manner so that our future resolves.
    // In an actual application, `device.poll(...)` should
    // be called in an event loop or on another thread.
    state.app.manager.device.poll(wgpu::Maintain::Wait);
    // If a file system is available, write the buffer as a PNG
    let has_file_system_available = cfg!(not(target_arch = "wasm32"));
    if !has_file_system_available {
        return;
    }

    use futures::executor::block_on;
    block_on(buffer_future).unwrap();
    let padded_buffer = buffer_slice.get_mapped_range();

    let mut png_encoder = png::Encoder::new(
        std::fs::File::create(output_path).unwrap(),
        buffer_dimensions.width as u32,
        buffer_dimensions.height as u32,
    );
    png_encoder.set_depth(png::BitDepth::Eight);
    png_encoder.set_color(png::ColorType::RGBA);
    let mut png_writer = png_encoder
        .write_header()
        .unwrap()
        .into_stream_writer_with_size(buffer_dimensions.unpadded_bytes_per_row);

    // from the padded_buffer we write just the unpadded bytes into the image
    use std::io::Write;
    for chunk in padded_buffer.chunks(buffer_dimensions.padded_bytes_per_row) {
        png_writer
            .write(&chunk[..buffer_dimensions.unpadded_bytes_per_row])
            .unwrap();
    }
    png_writer.finish().unwrap();

    // With the current interface, we have to make sure all mapped views are
    // dropped before we unmap the buffer.
    drop(padded_buffer);

    png_buffer.unmap();
}

