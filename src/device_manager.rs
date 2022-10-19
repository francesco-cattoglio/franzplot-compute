use winit::window::Window;

use crate::rendering::SWAPCHAIN_FORMAT;

pub struct Manager {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub sample_count: u32,
}


#[cfg(target_os = "windows")]
const DEFAULT_BACKEND: wgpu::Backends = wgpu::Backends::DX12;

#[cfg(target_os = "macos")]
const DEFAULT_BACKEND: wgpu::Backends = wgpu::Backends::METAL;

#[cfg(target_os = "linux")]
const DEFAULT_BACKEND: wgpu::Backends = wgpu::Backends::VULKAN;

impl Manager {
    pub fn new(trace_path: Option<&std::path::Path>, backend_override: Option<wgpu::Backends>) -> Self {
        use futures::executor::block_on;
        let sample_count = match backend_override {
            Some(wgpu::Backends::GL) => 1,
            _ => 4,
        };
        let instance = wgpu::Instance::new(backend_override.unwrap_or(DEFAULT_BACKEND));

        let adapter_future = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower, // TODO: HighPower caused an issue on at least one AMD discrete GPU card
                force_fallback_adapter: false, // TODO: what is this?
                compatible_surface: None, // We do not request this adapter to be compatible with
                                          // a specific surface, we will check it after we are done
                                          // with initialization of the Manager
            }
        );
        let adapter = block_on(adapter_future).expect("unable to open an adapter");

        let device_future = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("requested device"),
                features: wgpu::Features::MAPPABLE_PRIMARY_BUFFERS, // TODO: we need to disable this in the future!
                // features: wgpu::Features::empty(),
                limits: wgpu::Limits {
                    max_storage_buffers_per_shader_stage: 6, // TODO: we need to make sure that every possible GPU supports this
                    max_compute_workgroup_size_x: 256,
                    max_compute_invocations_per_workgroup: 256,
                    .. Default::default()
                },
            },
            trace_path,
        );
        let (device, queue) = block_on(device_future).expect("unable to get a device and a queue");

        Self {
            device,
            instance,
            queue,
            sample_count,
        }
    }

}
