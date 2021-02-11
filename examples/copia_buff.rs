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
        Device::new(physical, &Features::none(), &DeviceExtensions::none(),
                    [(queue_family, 1.0)].iter().cloned()).expect("failed to create a device")
    };

    let queue = queues.next().unwrap();

    let source_content = 0 .. 64;
    let source = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, source_content).expect("failed to create buffer");

    let dest_content = (0 .. 64).map(|_| 0);
    let dest = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, dest_content).expect("failed to create buffer");

    let mut builder = AutoCommandBufferBuilder::new(device.clone(), queue.family()).unwrap();
    builder.copy_buffer(source.clone(), dest.clone()).unwrap();

    let command_buffer = builder.build().unwrap();
    let finished = command_buffer.execute(queue.clone()).unwrap();

    finished.then_signal_fence_and_flush().unwrap().wait(None).unwrap();

    let src_content = source.read().unwrap();
    let des_content = dest.read().unwrap();

    println!("{:?}",&*src_content);
    println!("{:?}",&*des_content);

}

