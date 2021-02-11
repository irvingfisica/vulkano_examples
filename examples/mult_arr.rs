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
use std::sync::Arc;
use vulkano::pipeline::ComputePipeline;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::PipelineLayoutAbstract;

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

    let data_iter = 0 .. 65536;
    let data_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, data_iter)
                        .expect("failed to create bufer");

    let shader = cs::Shader::load(device.clone()).expect("failed to create shader module");

    let compute_pipeline = Arc::new(ComputePipeline::new(device.clone(), &shader.main_entry_point(), &(), None)
                            .expect("failed to create compute pipeline"));

    let layout = compute_pipeline.layout().descriptor_set_layout(0).unwrap();
    let set = Arc::new(PersistentDescriptorSet::start(layout.clone())
                .add_buffer(data_buffer.clone()).unwrap().build().unwrap());

    let mut builder = AutoCommandBufferBuilder::new(device.clone(), queue.family()).unwrap();
    builder.dispatch([1024,1,1], compute_pipeline.clone(), set.clone(), ()).unwrap();
    let command_buffer = builder.build().unwrap();

    let finished = command_buffer.execute(queue.clone()).unwrap();

    finished.then_signal_fence_and_flush().unwrap()
            .wait(None).unwrap();

    let content = data_buffer.read().unwrap();
    for (n, val) in content.iter().enumerate() {
        // println!("n: {}, val: {}", n, *val);
        assert_eq!(*val, n as u32 * 12);
    }

    println!("Everything succeeded!");
}

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        src: "
#version 450

layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;

layout(set = 0, binding = 0) buffer Data {
    uint data[];
} buf;

void main() {
    uint idx = gl_GlobalInvocationID.x;
    buf.data[idx] *= 12;
}"

    }
}

