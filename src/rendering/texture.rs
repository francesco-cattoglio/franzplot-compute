use image::GenericImageView;
use anyhow::Result;

use crate::rendering::DEPTH_FORMAT;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler
}

impl Texture {

    pub fn load<P: AsRef<std::path::Path>>(device: &wgpu::Device, queue: &wgpu::Queue, path: P, label: &str) -> anyhow::Result<Self> {
        let img = image::open(path)?;
        Self::from_image(device, queue, &img, Some(label))
    }

    pub fn create_depth_texture(device: &wgpu::Device, swapchain_desc: &wgpu::SwapChainDescriptor, label: &str) -> Self {
        let size = wgpu::Extent3d {
            width: swapchain_desc.width,
            height: swapchain_desc.height,
            depth: 1
        };

        let descriptor = wgpu::TextureDescriptor {
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            size,
            format: DEPTH_FORMAT,
            mip_level_count: 1,
            label: Some(label),
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT
                | wgpu::TextureUsage::SAMPLED
                | wgpu::TextureUsage::COPY_SRC
            };

        let texture = device.create_texture(&descriptor);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor { // 4.
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            anisotropy_clamp: None,
            label: None,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare: Some(wgpu::CompareFunction::LessEqual), // 5.
        });


        Self {
            texture,
            view,
            sampler
        }
    }

    #[allow(unused)]
    pub fn from_bytes(device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8], label: &str) -> Result<Self> {
        let img = image::load_from_memory(bytes).unwrap();
        Self::from_image(device, queue, &img, Some(label))
    }

    pub fn from_image(device: &wgpu::Device, queue: &wgpu::Queue, img: &image::DynamicImage, tex_label: Option<&str>) -> Result<Self> {
        let diffuse_rgba = img.to_rgba();
        let image_size = img.dimensions();

        let wsize = wgpu::Extent3d {
            depth: 1,
            height: image_size.1,
            width: image_size.0,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor{
            size: wsize,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            label: tex_label,
        });

        queue.write_texture(
            // Tells wgpu where to copy the pixel data
            wgpu::TextureCopyView {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            // The actual pixel data
            bytemuck::cast_slice(&diffuse_rgba),
            // The layout of the texture
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * image_size.0,
                rows_per_image: image_size.1,
            },
            wsize,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            anisotropy_clamp: None,
            label: None,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare: None,
        });

        Ok(Texture{sampler, texture, view: texture_view})
    }

}

