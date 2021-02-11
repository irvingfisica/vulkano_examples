use vulkano::instance::Instance;
use vulkano::instance::InstanceExtensions;
use vulkano::instance::PhysicalDevice;
use vulkano::device::Device;
use vulkano::device::DeviceExtensions;
use vulkano::device::Features;
use vulkano::buffer::BufferUsage;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::framebuffer::Framebuffer;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::CommandBuffer;
use vulkano::sync::GpuFuture;
use vulkano::format::Format;
use vulkano::image::Dimensions;
use vulkano::image::StorageImage;
use vulkano::command_buffer::SubpassContents;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::framebuffer::Subpass;
use vulkano::command_buffer::DynamicState;
use vulkano::pipeline::viewport::Viewport;

use image::{ImageBuffer, Rgba};

use std::sync::Arc;

fn main() {

    let instance = Instance::new(None, &InstanceExtensions::none(), None).expect("failed to create an instance");

    for physical_device in PhysicalDevice::enumerate(&instance) {
        println!("Available device: {}", physical_device.name());
    }

    let mut iter = PhysicalDevice::enumerate(&instance);

    let physical = iter.next().expect("no device available");
    println!("Selected device: {}", physical.name());

    for family in physical.queue_families() {
        println!("Found a queue family with {:?} queue(s), id: {:?}", family.queues_count(), family.id());
        println!("It supports graphics: {:?}", family.supports_graphics());
        println!("It supports compute: {:?}", family.supports_compute());
        println!("It supports transfers explicitly: {:?}", family.explicitly_supports_transfers());
        println!("It supports sparse binding: {:?}", family.supports_sparse_binding());
    }

    let queue_family = physical.queue_families()
        .find(|&q| q.supports_graphics() & q.supports_compute())
        .expect("Couldn't find a queue family");
    println!("Selected queue family: {}", queue_family.id());

    let (device, mut queues) = {

        let device_ext = DeviceExtensions {
            khr_storage_buffer_storage_class: true,
            .. DeviceExtensions::none()
        };

        Device::new(physical, &Features::none(), &device_ext,
                    [(queue_family, 1.0)].iter().cloned()).expect("failed to create a device")
    };

    let queue = queues.next().unwrap();

    let vertex1 = Vertex { position: [-0.5, -0.5 ]};
    let vertex2 = Vertex { position: [ 0.0,  0.5 ]};
    let vertex3 = Vertex { position: [ 0.5, -0.25]};

    let vertex_buffer = CpuAccessibleBuffer::from_iter(
                            device.clone(), 
                            BufferUsage::all(), 
                            false, 
                            vec![vertex1, vertex2, vertex3].into_iter()
                        ).unwrap();

    let render_pass = Arc::new(vulkano::single_pass_renderpass!(device.clone(),
                        attachments: {
                            color: {
                                load: Clear,
                                store: Store,
                                format: Format::R8G8B8A8Unorm,
                                samples: 1,
                            }
                        },
                        pass: {
                            color: [color],
                            depth_stencil: {}
                        }
                ).unwrap());

    let image = StorageImage::new(device.clone(), Dimensions::Dim2d { width: 1024, height: 1024},
                    Format::R8G8B8A8Unorm, Some(queue.family())).unwrap();

    let iter = (0 .. 1024 * 1024 * 4).map(|_| 0u8);
    let buf = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, iter).expect("failed to create buffer");

    let framebuffer = Arc::new(Framebuffer::start(render_pass.clone())
                        .add(image.clone()).unwrap()
                        .build().unwrap()
                    );

    let dynamic_state = DynamicState {
        viewports: Some(vec![Viewport {
            origin: [0.0, 0.0],
            dimensions: [1024.0, 1024.0],
            depth_range: 0.0 .. 1.0,
        }]),
        .. DynamicState::none()
    };

    let vs = vs::Shader::load(device.clone()).expect("failed to create vertex shader module");
    let fs = fs::Shader::load(device.clone()).expect("failed to create fragment shader module");

    let pipeline = Arc::new(GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(vs.main_entry_point(), ())
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap()
        );

    let mut builder = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family()).unwrap();
    builder
        .begin_render_pass(framebuffer.clone(), SubpassContents::Inline, vec![[0.0, 0.0, 1.0, 1.0].into()])
        .unwrap()

        .draw(pipeline.clone(), &dynamic_state, vertex_buffer.clone(), (), ())
        .unwrap()

        .end_render_pass()
        .unwrap()

        .copy_image_to_buffer(image.clone(), buf.clone())
        .unwrap();
        
    let command_buffer = builder.build().unwrap();

    let finished = command_buffer.execute(queue.clone()).unwrap();
    finished.then_signal_fence_and_flush().unwrap().wait(None).unwrap();

    let buffer_content = buf.read().unwrap();

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &buffer_content[..]).unwrap();

    image.save("triangle.png").unwrap();

}

#[derive(Default, Copy, Clone)]
struct Vertex {
    position: [f32;2],
}

vulkano::impl_vertex!(Vertex, position);

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
#version 450

layout(location = 0) in vec2 position;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
}
"
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
#version 450

layout(location = 0) out vec4 f_color;

void main() {
    f_color = vec4(1.0, 0.0, 0.0, 1.0);
}
"
    }
}


