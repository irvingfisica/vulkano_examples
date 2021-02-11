use vulkano::instance::Instance;
use vulkano::instance::InstanceExtensions;
use vulkano::instance::PhysicalDevice;
use vulkano::device::Device;
use vulkano::device::DeviceExtensions;
use vulkano::device::Features;
use vulkano::buffer::BufferUsage;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::CommandBuffer;
use vulkano::sync::GpuFuture;
use vulkano::format::Format;
use vulkano::image::Dimensions;
use vulkano::image::StorageImage;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::pipeline_layout::PipelineLayoutAbstract;
use vulkano::pipeline::ComputePipeline;

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

    let image = StorageImage::new(device.clone(), Dimensions::Dim2d { width: 1024, height: 1024},
                    Format::R8G8B8A8Unorm, Some(queue.family())).unwrap();

    let shader = cs::Shader::load(device.clone()).expect("failed to create shader module");

    let compute_pipeline = Arc::new(
        ComputePipeline::new(device.clone(), &shader.main_entry_point(), &(), None)
                .expect("failed to create compute pipeline"));

    let layout = compute_pipeline.layout().descriptor_set_layout(0).unwrap();
    let set = Arc::new(
        PersistentDescriptorSet::start(layout.clone())
        .add_image(image.clone()).unwrap()
        .build().unwrap(),
    );

    let iter = (0 .. 1024 * 1024 * 4).map(|_| 0u8);
    let buf = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, iter).expect("failed to create buffer");

    let mut builder = AutoCommandBufferBuilder::new(device.clone(), queue.family()).unwrap();
    builder
        .dispatch([1024 / 8, 1024 / 8, 1], compute_pipeline.clone(), set.clone(), ()).unwrap()
        .copy_image_to_buffer(image.clone(), buf.clone()).unwrap();
        
    let command_buffer = builder.build().unwrap();

    let finished = command_buffer.execute(queue.clone()).unwrap();
    finished.then_signal_fence_and_flush().unwrap().wait(None).unwrap();

    let buffer_content = buf.read().unwrap();

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &buffer_content[..]).unwrap();

    image.save("image.png").unwrap();

}

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        src: "
#version 450

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;

void main() {
    vec2 norm_coordinates = (gl_GlobalInvocationID.xy + vec2(0.5)) / vec2(imageSize(img));
    vec2 c = (norm_coordinates - vec2(0.5)) * 2.0 - vec2(1.0, 0.0);

    vec2 z = vec2(0.0, 0.0);
    float i;
    for (i=0.0; i<1.0; i += 0.005) {
        z = vec2(
            z.x * z.x - z.y * z.y + c.x,
            z.y * z.x + z.x * z.y + c.y
        );

        if (length(z) > 4.0) {
            break;
        }
    }

    vec4 to_write = vec4(vec3(i), 1.0);
    imageStore(img, ivec2(gl_GlobalInvocationID.xy), to_write);
}"
    }
}


