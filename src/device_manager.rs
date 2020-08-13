
pub struct DeviceManager {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl DeviceManager {
    pub fn new() -> Self {
        use futures::executor::block_on;

        let adapter_future = wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::Default,
            compatible_surface: None,
            },
            wgpu::BackendBit::PRIMARY,
        );
        let adapter = block_on(adapter_future).expect("unable to open an adapter");

        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: Default::default(),
        }));

        Self {
            device,
            queue
        }
    }

}
