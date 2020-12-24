use crate::rust_gui;
use crate::device_manager;
use crate::rendering::texture;

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

