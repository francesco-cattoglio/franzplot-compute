use crate::rust_gui;
use crate::device_manager;
use crate::rendering::texture;
use crate::rendering::model;

pub fn load_imgui_masks<P: AsRef<std::path::Path>>(manager: &device_manager::Manager, renderer: &mut imgui_wgpu::Renderer, files: &[P]) -> rust_gui::MaskIds {
    use std::convert::TryInto;
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
    use std::convert::TryInto;
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
//TODO: DRY? We might want to unify the two compute_graph_zoom and compute_scene_zoom functions
#[cfg(target_os = "macos")]
// On MacOS:
// - the mouse wheel reports as LineDelta with fractional floats,
// and the value accelerates quickly depending on how many lines one
// scrolled. (one tick starts as 0.1, can easily go up to 10.1 for
// EACH tick on a scroll. "Compressing" with a sqrt or by ^0.3333 might
// be a good idea!)
// - the scroll pad reports as a PhysicalPosition, and has inertia!
pub fn compute_scene_zoom(delta: MouseScrollDelta, mouse_sensitivity: f32, touchpad_sensitivity: f32) -> f32 {
    // Since we want to give the user reasonable numbers shown as sensitivity settings,
    // a hidden coefficient helps with keeping the numbers all in the same range
    let coeff = 0.1;
    match delta {
        MouseScrollDelta::LineDelta(_x, y) => {
            // mouse got scrolled, due to the way this gets reported, we want to squash it
            // as close as possible to 1 by applying a cube root, then multiplying it
            // by the sensitivity
            coeff * mouse_sensitivity * y.cbrt()
        },
        MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition {y, ..}) => {
            // scrolling on the touch pad reports some more normal values, so we just
            // need to convert to float and multiplying by the sensitivity
            coeff * touchpad_sensitivity * y as f32
        }
    }
}

#[cfg(target_os = "macos")]
pub fn compute_graph_zoom(delta: MouseScrollDelta, mouse_sensitivity: f32, touchpad_sensitivity: f32) -> f32 {
    // Since we want to give the user reasonable numbers shown as sensitivity settings,
    // a hidden coefficient helps with keeping the numbers all in the same range
    let coeff = 0.2;
    match delta {
        MouseScrollDelta::LineDelta(_x, y) => {
            // mouse got scrolled, due to the way this gets reported, we want to squash it
            // as close as possible to 1 by applying a cube root, then multiplying it
            // by the sensitivity
            -coeff * mouse_sensitivity * y.cbrt()
        },
        MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition {y, ..}) => {
            // scrolling on the touch pad reports some more normal values, so we just
            // need to convert to float and multiplying by the sensitivity
            -coeff * touchpad_sensitivity * y as f32
        }
    }
}

// On my Linux setup (Arch, X11):
// - the mouse wheel reports as LineDelta with "integral" floats.
// - the touch pad reports as LineDelta with real floats.
// - both values are really similar though,
#[cfg(target_os = "linux")]
pub fn compute_scene_zoom(delta: MouseScrollDelta, mouse_sensitivity: f32, touchpad_sensitivity: f32) -> f32 {
    // Since we want to give the user reasonable numbers shown as sensitivity settings,
    // a hidden coefficient helps with keeping the numbers all in the same range
    let coeff = 0.1;
    match delta {
        MouseScrollDelta::LineDelta(_x, y) => {
            // to differentiate between mouse and touchpad, compute the fractional part and check
            // if it exists or not
            let frac = y.fract();
            if frac.abs() < 42.0*std::f32::EPSILON {
                // no fractional part, mouse input!
                coeff * mouse_sensitivity * y
            } else {
                coeff * touchpad_sensitivity * y
            }
        },
        MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition {y, ..}) => {
            // this never gets reported on my linux, but if it ever happens we can just assume
            // it is from touchpad
            coeff * touchpad_sensitivity * y as f32
        }
    }
}

#[cfg(target_os = "linux")]
pub fn compute_graph_zoom(delta: MouseScrollDelta, mouse_sensitivity: f32, touchpad_sensitivity: f32) -> f32 {
    // Since we want to give the user reasonable numbers shown as sensitivity settings,
    // a hidden coefficient helps with keeping the numbers all in the same range
    let coeff = 0.2;
    match delta {
        MouseScrollDelta::LineDelta(_x, y) => {
            // to differentiate between mouse and touchpad, compute the fractional part and check
            // if it exists or not
            let frac = y.fract();
            if frac.abs() < 42.0*std::f32::EPSILON {
                // no fractional part, mouse input!
                -coeff * mouse_sensitivity * y
            } else {
                -coeff * touchpad_sensitivity * y
            }
        },
        MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition {y, ..}) => {
            // this never gets reported on my linux, but if it ever happens we can just assume
            // it is from touchpad
            -coeff * touchpad_sensitivity * y as f32
        }
    }
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

use crate::state::State;
pub fn create_png<P: AsRef<std::path::Path>>(state: &mut State, output_path: &P) {
    let height = 1080;
    let width = 1920;
    let texture_size = wgpu::Extent3d {
        height,
        width,
        depth: 1,
    };
    state.app.update_depth_buffer(texture_size);
    state.app.update_projection_matrix(texture_size);
    let output_texture = super::rendering::texture::Texture::create_output_texture(&state.app.manager.device, texture_size, 1);
    state.app.update_scene(&output_texture.view);

    let buffer_dimensions = BufferDimensions::new(width as usize, height as usize);
    // The output buffer lets us retrieve the data as an array
    let png_buffer = state.app.manager.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height) as u64,
        usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
        mapped_at_creation: false,
    });

    let command_buffer = {
        let mut encoder = state.app.manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        // Copy the data from the texture to the buffer
        encoder.copy_texture_to_buffer(
            wgpu::TextureCopyView {
                texture: &output_texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::BufferCopyView {
                buffer: &png_buffer,
                layout: wgpu::TextureDataLayout {
                    offset: 0,
                    bytes_per_row: buffer_dimensions.padded_bytes_per_row as u32,
                    rows_per_image: 0,
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

