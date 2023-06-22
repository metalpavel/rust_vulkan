use super::app_data;
use super::app_defines;
use super::queue_family;
use super::swapchain;

use anyhow::{anyhow, Result};
use std::collections::HashSet;
use log::*;
use vulkanalia::prelude::v1_0::*;

pub unsafe fn pick_physical_device(instance: &Instance, data: &mut app_data::Data) -> Result<()> {
    for physical_device in instance.enumerate_physical_devices()? {
        let properties = instance.get_physical_device_properties(physical_device);

        if let Err(error) = check_physical_device(instance, data, physical_device) {
            warn!("Skipping physical device (`{}`): {}", properties.device_name, error);
        } else {
            info!("Selected physical device (`{}`).", properties.device_name);
            data.physical_device = physical_device;
            return Ok(());
        }
    }

    Err(anyhow!("Failed to find suitable physical device."))
}

unsafe fn check_physical_device(instance: &Instance, data: &app_data::Data, physical_device: vk::PhysicalDevice) -> Result<()> {
    queue_family::QueueFamilyIndices::get(instance, data, physical_device)?;
    check_physical_device_extensions(instance, physical_device)?;

    let support = swapchain::SwapchainSupport::get(instance, data, physical_device)?;
    if support.formats.is_empty() || support.present_modes.is_empty() {
        return Err(anyhow!("Insufficient swapchain support."));
    }

    Ok(())
}

unsafe fn check_physical_device_extensions(instance: &Instance, physical_device: vk::PhysicalDevice) -> Result<()> {
    let extensions = instance.enumerate_device_extension_properties(physical_device, None)?
        .iter().map(|e| e.extension_name).collect::<HashSet<_>>();

    if app_defines::DEVICE_EXTENSIONS.iter().all(|e| extensions.contains(e)) {
        Ok(())
    } else {
        Err(anyhow!("Missing required device extensions."))
    }
}