macro_rules! shader_bindings_impl {
    (
        $(#[meta:meta])*
        $vis:vis struct $struct_name:ident {
            $( $field_vis:vis $field:ident : $type_of:ty = $decl:expr),+ $(,)?
        }

        $($fvis:vis fn $fname:ident($($fargs:tt)*) $(-> $ftype:ty)? $fblk:block)*

    ) => {
        $vis struct $struct_name {
            $( $field_vis $field: $type_of),+
        }

        impl $struct_name {
            $vis fn new(device: &wgpu::Device) -> Self {
                Self {
                    $( $field: BufferBinding::new(device, $decl), )+
                }
            }

            fn to_vec(&self) -> Vec<&dyn Binding> {
                vec![$( &self.$field ),+]
            }

            $( $fvis fn $fname ($($fargs)*) $(-> $ftype)? $fblk )*
        }

    }
}

trait Binding {
    fn bind(&self) -> wgpu::BindingResource;
    fn stage(&self, queue: &wgpu::Queue);
    fn as_wgsl_str(&self) -> &str;
}

pub struct BufferBinding<T> {
    data: T,
    decl: &'static str,
    buffer: wgpu::Buffer,
}

impl<T: bytemuck::Pod + Default> BufferBinding<T> {
    fn new(device: &wgpu::Device, decl: &'static str) -> Self {
        Self {
            decl,
            data: T::default(),
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: std::mem::size_of::<T>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        }
    }

    pub fn update(&mut self, q: &wgpu::Queue, new: T) {
        self.data = new;
        self.stage(q);
    }
}

impl<T: bytemuck::Pod> Binding for BufferBinding<T> {
    fn bind(&self) -> wgpu::BindingResource {
        wgpu::BindingResource::Buffer(wgpu::Buffer::as_entire_buffer_binding(&self.buffer))
    }

    fn stage(&self, queue: &wgpu::Queue) {
        let data = bytemuck::bytes_of(&self.data).to_vec();
        queue.write_buffer(&self.buffer, 0, &data);
    }

    fn as_wgsl_str(&self) -> &str {
        self.decl
    }
}

shader_bindings_impl! {
    pub struct ShaderBindings {
        pub time: BufferBinding<f32> = "var<uniform> Time: f32",
        pub resolution: BufferBinding<[f32; 2]> = "var<uniform> Resolution: vec2<f32>",
        pub mouse: BufferBinding<[f32; 2]> = "var<uniform> Mouse: vec2<f32>",
    }

    pub fn create_bind_group_layout(&self, device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &self
                .to_vec()
                .iter()
                .enumerate()
                .map(|(i, _)| wgpu::BindGroupLayoutEntry {
                    binding: i as _,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                })
                .collect::<Vec<_>>(),
        })
    }

    pub fn create_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        let layout = &self.create_bind_group_layout(device);
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

    pub fn as_wgsl_string(&self) -> String {
        self.to_vec()
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let decl = b.as_wgsl_str();
                if decl.is_empty() {
                    String::new()
                } else {
                    format!("@group(0) @binding({i}) {decl};\n")
                }
            })
            .collect()
    }
}
