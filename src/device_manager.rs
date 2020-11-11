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

impl Manager {
    pub fn new(window: &Window) -> Self {
        use futures::executor::block_on;
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

        let (size, surface) = unsafe {
            let size = window.inner_size();
            let surface = instance.create_surface(window);
            (size, surface)
        };

        let adapter_future = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            }
        );
        let adapter = block_on(adapter_future).expect("unable to open an adapter");

        let device_future = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: adapter.features(),
                limits: Default::default(),
                shader_validation: true,
            },
            None
        );
        let (device, queue) = block_on(device_future).expect("unable to get a device and a queue");

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
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

    pub fn update_swapchain(&mut self, window: &Window) {
        let size = window.inner_size();
        let swapchain_descriptor = wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
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
