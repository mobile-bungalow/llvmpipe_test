// we're all hungry and itching to get back to our hotplates at home
mod boilerplates;
use bytemuck::{Pod, Zeroable, bytes_of};
use image::RgbaImage;
use wgpu::*;

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable)]
struct PushConstants {
    redness: f32,
    _pad: f32,
    resolution: [f32; 2],
}

// Draw something!
pub fn render_toy(redness: f32, resolution: [f32; 2]) -> RgbaImage {
    let (device, queue) = boilerplates::set_up_wgpu();

    let source = wgpu::include_wgsl!("demo.wgsl");
    let module = device.create_shader_module(source);

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: None,
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::WriteOnly,
                format: TextureFormat::Rgba8Unorm,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[PushConstantRange {
            stages: ShaderStages::COMPUTE,
            range: 0..16,
        }],
    });

    let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    let width = resolution[0] as u32;
    let height = resolution[1] as u32;

    let tex = device.create_texture(&TextureDescriptor {
        label: None,
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let view = tex.create_view(&TextureViewDescriptor::default());

    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::TextureView(&view),
        }],
    });

    let mut cb = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

    {
        let mut pass = cb.begin_compute_pass(&ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });

        let pc = PushConstants {
            redness,
            _pad: 0.0,
            resolution,
        };

        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_push_constants(0, bytes_of(&pc));
        pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
    }

    let buffer = device.create_buffer(&BufferDescriptor {
        label: None,
        size: (width * height * 4) as u64,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    cb.copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture: &tex,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit([cb.finish()]);

    let slice = buffer.slice(..);
    slice.map_async(MapMode::Read, |_| {});

    device
        .poll(PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .unwrap();

    let data = slice.get_mapped_range().to_vec();
    RgbaImage::from_raw(width, height, data).unwrap()
}

#[cfg(test)]
mod test {
    use crate::render_toy;

    #[test]
    fn snapshot() {
        let actual = render_toy(0.5, [512., 512.]);

        let png_bytes = include_bytes!("fixture.png");
        let expected = image::load_from_memory(png_bytes)
            .expect("Failed to decode PNG")
            .to_rgba8();

        assert_eq!(actual.dimensions(), expected.dimensions());

        for (a, e) in actual.pixels().zip(expected.pixels()) {
            for i in 0..4 {
                let diff = (a.0[i] as i16 - e.0[i] as i16).abs();
                assert!(diff <= 1, "pixel diff too large: {diff}");
            }
        }
    }
}
