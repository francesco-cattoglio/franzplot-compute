use image::{GenericImageView, Pixel};

use std::num::NonZeroU32;

#[derive(Debug)]
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
    pub size: wgpu::Extent3d,
}

impl Texture {
    pub fn load<P: AsRef<std::path::Path>>(device: &wgpu::Device, queue: &wgpu::Queue, path: P, label: Option<&str>) -> anyhow::Result<Self> {
        let img = image::open(path)?;
        Self::from_image(device, queue, &img, label)
    }

    pub fn thumbnail<P: AsRef<std::path::Path>>(device: &wgpu::Device, queue: &wgpu::Queue, path: P, label: Option<&str>) -> anyhow::Result<Self> {
        let img = image::open(path)?;
        let thumb = img.thumbnail_exact(64, 64);
        Self::from_image(device, queue, &thumb, label)
    }

    pub fn create_depth_texture(device: &wgpu::Device, size: wgpu::Extent3d, sample_count: u32) -> Self {
        let descriptor = wgpu::TextureDescriptor {
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            size,
            format: super::DEPTH_FORMAT,
            mip_level_count: 1,
            label: Some("Depth texture"),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
            };

        let texture = device.create_texture(&descriptor);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            anisotropy_clamp: 1,
            label: None,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: Some(wgpu::CompareFunction::LessEqual), // 5.
            border_color: None,
        });

        let texture_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: sample_count > 1,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    count: None,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                },
            ],
            label: Some("texture bind group layout"),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Depth texture bind group"),
            layout: &texture_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            texture,
            view,
            bind_group,
            size,
        }
    }

    pub fn create_screenshot_texture(device: &wgpu::Device, size: wgpu::Extent3d, sample_count: u32) -> Self {
        let descriptor = wgpu::TextureDescriptor {
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            size,
            format: super::SWAPCHAIN_FORMAT,
            mip_level_count: 1,
            label: Some("Scene output texture"),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
            };

        let texture = device.create_texture(&descriptor);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            anisotropy_clamp: 1,
            label: None,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: None,
            border_color: None,
        });

        let texture_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: sample_count > 1,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    count: None,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                },
            ],
            label: Some("texture bind group layout"),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Depth texture bind group"),
            layout: &texture_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            texture,
            view,
            bind_group,
            size,
        }
    }

    pub fn create_output_texture(device: &wgpu::Device, size: wgpu::Extent3d, sample_count: u32) -> Self {
        let descriptor = wgpu::TextureDescriptor {
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            size,
            format: super::SCENE_FORMAT,
            mip_level_count: 1,
            label: Some("Scene output texture"),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
            };

        let texture = device.create_texture(&descriptor);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            anisotropy_clamp: 1,
            label: None,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: None,
            border_color: None,
        });

        let texture_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: sample_count > 1,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    count: None,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                },
            ],
            label: Some("texture bind group layout"),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Depth texture bind group"),
            layout: &texture_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            texture,
            view,
            bind_group,
            size,
        }
    }

    pub fn default_bind_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    count: None,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                },
            ],
            label: Some("default texture bind group layout"),
        })
    }

    pub fn from_image(device: &wgpu::Device, queue: &wgpu::Queue, image: &image::DynamicImage, tex_label: Option<&str>) -> anyhow::Result<Self> {
        let image_size = image.dimensions();
        let size = wgpu::Extent3d {
            depth_or_array_layers: 1,
            height: image_size.1,
            width: image_size.0,
        };

        // depending on the image color, we create a texture with a different format
        let texture = match image.color() {
            image::ColorType::L8 | image::ColorType::L16 => {
                let grayscale_img = image.to_luma8();
                let texture = device.create_texture(&wgpu::TextureDescriptor{
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::R8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    label: tex_label,
                    view_formats: &[],
                });

                queue.write_texture(
                    // Tells wgpu where to copy the pixel data
                    wgpu::ImageCopyTexture {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    // The actual pixel data
                    bytemuck::cast_slice(&grayscale_img),
                    // The layout of the texture
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(std::mem::size_of::<u8>() as u32 * image_size.0),
                        rows_per_image: Some(image_size.1),
                    },
                    size,
                );
                texture
            },
            image::ColorType::Rgb8 | image::ColorType::Rgba8 => {
                // TODO: Srgb or not?
                let mut color_img = image.to_rgba8();
                for pixel in color_img.pixels_mut() {
                    // Premultiply color by alpha, as is required by egui
                    let factor = pixel[3] as f32 / 255.0;
                    pixel[0] = (pixel[0] as f32 * factor) as u8;
                    pixel[1] = (pixel[1] as f32 * factor) as u8;
                    pixel[2] = (pixel[2] as f32 * factor) as u8;
                }
                let texture = device.create_texture(&wgpu::TextureDescriptor{
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    label: tex_label,
                    view_formats: &[],
                });

                queue.write_texture(
                    // Tells wgpu where to copy the pixel data
                    wgpu::ImageCopyTexture {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    // The actual pixel data
                    bytemuck::cast_slice(&color_img),
                    // The layout of the texture
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(std::mem::size_of::<u32>() as u32 * image_size.0),
                        rows_per_image: Some(image_size.1),
                    },
                    size,
                );
                texture
            },
            _ => unimplemented!("image format not yet supported"),
        };

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            anisotropy_clamp: 1,
            label: None,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: None,
            border_color: None,
        });

        let texture_bind_layout = Self::default_bind_layout(device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Depth texture bind group"),
            layout: &texture_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Ok(Texture{
            texture,
            view: texture_view,
            bind_group,
            size,
        })
    }

}

