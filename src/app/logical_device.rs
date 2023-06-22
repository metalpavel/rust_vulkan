use super::app_data;
use super::app_defines;
use super::queue_family;

use anyhow::{Result};
use std::collections::HashSet;
use vulkanalia::prelude::v1_0::*;

pub unsafe fn create(instance: &Instance, data: &mut app_data::Data) -> Result<Device> {
    // Queue Create Infos

    let indices = queue_family::QueueFamilyIndices::get(instance, data, data.physical_device)?;

    let mut unique_indices = HashSet::new();
    unique_indices.insert(indices.graphics);
    unique_indices.insert(indices.present);

    let queue_priorities = &[1.0];
    let queue_infos = unique_indices
        .iter()
        .map(|i| {
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(*i)
                .queue_priorities(queue_priorities)
        })
        .collect::<Vec<_>>();

    // Layers

    let layers = if app_defines::VALIDATION_ENABLED {
        vec![app_defines::VALIDATION_LAYER.as_ptr()]
    } else {
        vec![]
    };

    // Extensions

    let extensions = app_defines::DEVICE_EXTENSIONS.iter().map(|n| n.as_ptr()).collect::<Vec<_>>();

    // Features

    let features = vk::PhysicalDeviceFeatures::builder();

    // Create

    let info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .enabled_features(&features);

    let device = instance.create_device(data.physical_device, &info, None)?;

    // Queues

    data.graphics_queue = device.get_device_queue(indices.graphics, 0);
    data.present_queue = device.get_device_queue(indices.present, 0);

    Ok(device)
}
