use crate::util::uniform_buffer_size;
// Thx to https://github.com/compute-toys/wgpu-compute-toy/blob/b0d8c41a1885e7a13d4882a1f02d5df26305ec6b/src/bind.rs#L39
// for idea and overall understanting

trait Binding {
    fn bind(&self) -> wgpu::BindingResource;
    fn stage(&self, queue: &wgpu::Queue);
    fn binding_type(&self) -> &wgpu::BindingType;
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Time(pub f32);

pub struct BufferBinding<T> {
    pub data: T,
    decl: String,
    serialize: Box<dyn Fn(&T) -> Vec<u8>>,
    buffer: wgpu::Buffer,
    binding_type: wgpu::BindingType,
    bind: Box<dyn for<'a> Fn(&'a wgpu::Buffer) -> wgpu::BufferBinding<'a>>,
}

impl<T> Binding for BufferBinding<T> {
    fn bind(&self) -> wgpu::BindingResource {
        wgpu::BindingResource::Buffer((self.bind)(&self.buffer))
    }

    fn stage(&self, queue: &wgpu::Queue) {
        let data = (self.serialize)(&self.data);
        queue.write_buffer(&self.buffer, 0, &data);
    }

    fn binding_type(&self) -> &wgpu::BindingType {
        &self.binding_type
    }
}

pub struct ShaderBindings {
    pub time: BufferBinding<Time>,
}

impl ShaderBindings {
    pub fn new(device: &wgpu::Device) -> Self {
        let binding_type = wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        };

        Self {
            time: BufferBinding {
                decl: "var<uniform> time: Time".into(),
                data: Time(0.),
                serialize: Box::new(|d| bytemuck::bytes_of(d).to_vec()),
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: uniform_buffer_size::<Time>(),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                binding_type,
                bind: Box::new(wgpu::Buffer::as_entire_buffer_binding),
            },
        }
    }

    pub fn create_bind_group_layout(&self, device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &self
                .to_vec()
                .iter()
                .enumerate()
                .map(|(i, b)| wgpu::BindGroupLayoutEntry {
                    binding: i as _,
                    visibility: wgpu::ShaderStages::all(),
                    ty: *b.binding_type(),
                    count: None,
                })
                .collect::<Vec<_>>(),
        })
    }

    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &self
                .to_vec()
                .iter()
                .enumerate()
                .map(|(i, b)| wgpu::BindGroupEntry {
                    binding: i as _,
                    resource: b.bind(),
                })
                .collect::<Vec<_>>(),
        })
    }

    pub fn stage(&self, queue: &wgpu::Queue) {
        self.time.stage(queue);
    }

    fn to_vec(&self) -> Vec<&dyn Binding> {
        vec![&self.time]
    }
}
