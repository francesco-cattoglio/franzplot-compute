use winit::window::Window;

use crate::rendering::SWAPCHAIN_FORMAT;

pub struct Manager {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub surface: wgpu::Surface,
    pub config: wgpu::SurfaceConfiguration,
}


#[cfg(target_os = "windows")]
const DEFAULT_BACKEND: wgpu::Backends = wgpu::Backends::DX12;

#[cfg(target_os = "macos")]
const DEFAULT_BACKEND: wgpu::Backends = wgpu::Backends::METAL;

#[cfg(target_os = "linux")]
const DEFAULT_BACKEND: wgpu::Backends = wgpu::Backends::VULKAN;

impl Manager {
    pub fn new(window: &Window, trace_path: Option<&std::path::Path>, backend_override: Option<wgpu::Backends>) -> Self {
        use futures::executor::block_on;
        let instance = wgpu::Instance::new(backend_override.unwrap_or(DEFAULT_BACKEND));

        let (size, surface) = unsafe {
            let size = window.inner_size();
            let surface = instance.create_surface(window);
            (size, surface)
        };

        let adapter_future = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower, // TODO: HighPower caused an issue on at least one AMD discrete GPU card
                force_fallback_adapter: false, // TODO: what is this?
                compatible_surface: Some(&surface),
            }
        );
        let adapter = block_on(adapter_future).expect("unable to open an adapter");

        let device_future = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("requested device"),
                features: wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
                limits: wgpu::Limits {
                    max_storage_buffers_per_shader_stage: 6, // TODO: we need to make sure that every possible GPU supports this
                    .. Default::default()
                },
            },
            trace_path,
        );
        let (device, queue) = block_on(device_future).expect("unable to get a device and a queue");

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: SWAPCHAIN_FORMAT,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        Self {
            device,
            instance,
            queue,
            size,
            surface,
            config,
        }
    }

    pub fn get_frame(&mut self) -> Option<wgpu::SurfaceTexture> {
        // get the framebuffer frame. We might need to re-create the swapchain if for some
        // reason our current one is outdated
        let maybe_frame = self.surface.get_current_texture();
        match maybe_frame {
                Ok(surface_frame) => {
                    Some(surface_frame)
                }
                Err(wgpu::SurfaceError::Outdated) => {
                    // This interesting thing happens when we just resized the window but due to a
                    // race condition the winit ResizeEvent has not fired just yet. We might resize
                    // the swapchain here, but doing so would leave the app in a borked state:
                    // imgui needs to be notified about the resize as well, otherwise it will run
                    // a scissor test on a framebuffer of a different physical size and the
                    // validation layer will panic. The best course of action is doing nothing at
                    // all, the problem will fix itself on the next frame, when the Resized event
                    // fires.
                    None
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    panic!("Out Of Memory error in frame rendering");
                }
                Err(wgpu::SurfaceError::Timeout) => {
                    println!("Warning: timeout error in frame rendering!");
                    None
                }
                Err(wgpu::SurfaceError::Lost) => {
                    println!("Warning: frame Lost error in frame rendering");
                    None
                }
        }
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let height = size.height as u32;
        let width = size.width as u32;
        if height >= 8 && width >= 8 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

}
