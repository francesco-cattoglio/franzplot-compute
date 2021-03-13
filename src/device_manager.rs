use winit::window::Window;

use crate::rendering::SWAPCHAIN_FORMAT;

pub struct Manager {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub surface: wgpu::Surface,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
}


#[cfg(target_os = "windows")]
const DEFAULT_BACKEND: wgpu::BackendBit = wgpu::BackendBit::DX12;

#[cfg(target_os = "macos")]
const DEFAULT_BACKEND: wgpu::BackendBit = wgpu::BackendBit::METAL;

#[cfg(target_os = "linux")]
const DEFAULT_BACKEND: wgpu::BackendBit = wgpu::BackendBit::VULKAN;

impl Manager {
    pub fn new(window: &Window, trace_path: Option<&std::path::Path>, backend_override: Option<wgpu::BackendBit>) -> Self {
        use futures::executor::block_on;
        let instance = wgpu::Instance::new(backend_override.unwrap_or(DEFAULT_BACKEND));

        let (size, surface) = unsafe {
            let size = window.inner_size();
            let surface = instance.create_surface(window);
            (size, surface)
        };

        let adapter_future = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance, // TODO: investigate, this could be useful!
                compatible_surface: Some(&surface),
            }
        );
        let adapter = block_on(adapter_future).expect("unable to open an adapter");

        let device_future = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("requested device"),
                features: wgpu::Features::empty(),
                limits: wgpu::Limits {
                    max_storage_buffers_per_shader_stage: 6, // TODO: we need to make sure that every possible GPU supports this
                    .. Default::default()
                },
            },
            trace_path,
        );
        let (device, queue) = block_on(device_future).expect("unable to get a device and a queue");

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: SWAPCHAIN_FORMAT,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = Self::create_swapchain(&device, &surface, &sc_desc);

        Self {
            device,
            instance,
            queue,
            size,
            surface,
            swap_chain,
            sc_desc,
        }
    }

    pub fn get_frame(&mut self) -> Option<wgpu::SwapChainFrame> {
        // get the framebuffer frame. We might need to re-create the swapchain if for some
        // reason our current one is outdated
        let maybe_frame = self.swap_chain.get_current_frame();
        match maybe_frame {
                Ok(swapchain_frame) => {
                    Some(swapchain_frame)
                }
                Err(wgpu::SwapChainError::Outdated) => {
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
                Err(wgpu::SwapChainError::OutOfMemory) => {
                    panic!("Out Of Memory error in frame rendering");
                }
                Err(wgpu::SwapChainError::Timeout) => {
                    panic!("Timeout error in frame rendering");
                }
                Err(wgpu::SwapChainError::Lost) => {
                    panic!("Frame Lost error in frame rendering");
                }
        }
    }

    #[allow(unused)]
    pub fn update_swapchain(&mut self, window: &Window) {
        let size = window.inner_size();
        let swapchain_descriptor = wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Fifo,
            };
        self.swap_chain = Self::create_swapchain(&self.device, &self.surface, &swapchain_descriptor);
    }

    fn create_swapchain(device: &wgpu::Device, surface: &wgpu::Surface, swapchain_descriptor: &wgpu::SwapChainDescriptor) -> wgpu::SwapChain {
        device.create_swap_chain(surface, swapchain_descriptor)
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.sc_desc.width = size.width as u32;
        self.sc_desc.height = size.height as u32;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

}
