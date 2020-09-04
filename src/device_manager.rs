
pub struct Manager {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Manager {
    pub fn new() -> Self {
        use futures::executor::block_on;

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

        let adapter_future = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: None,
            }
        );
        let adapter = block_on(adapter_future).expect("unable to open an adapter");

        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            features: adapter.features(),
            limits: Default::default(),
            shader_validation: true,
        },
        None)).expect("unable to get a device and a queue");

        Self {
            device,
            instance,
            queue
        }
    }

}
