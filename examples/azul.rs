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
use vulkano::format::ClearValue;

use image::{ImageBuffer, Rgba};

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

    let iter = (0 .. 1024 * 1024 * 4).map(|_| 0u8);
    let buf = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, iter).expect("failed to create buffer");

    let mut builder = AutoCommandBufferBuilder::new(device.clone(), queue.family()).unwrap();
    builder
        .clear_color_image(image.clone(), ClearValue::Float([0.0, 0.0, 1.0, 1.0])).unwrap()
        .copy_image_to_buffer(image.clone(), buf.clone()).unwrap();
        
    let command_buffer = builder.build().unwrap();

    let finished = command_buffer.execute(queue.clone()).unwrap();
    finished.then_signal_fence_and_flush().unwrap().wait(None).unwrap();

    let buffer_content = buf.read().unwrap();

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &buffer_content[..]).unwrap();

    image.save("image.png").unwrap();

}


