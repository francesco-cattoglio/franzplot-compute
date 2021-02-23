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

