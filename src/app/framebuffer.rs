use super::app_data;

use anyhow::{Result};
use vulkanalia::prelude::v1_0::*;

pub unsafe fn create(device: &Device, data: &mut app_data::Data) -> Result<()> {
    data.framebuffers = data.swapchain_image_views.iter()
        .map(|i| {
            let attachments = &[*i];
            let create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(data.render_pass)
                .attachments(attachments)
                .width(data.swapchain_extent.width)
                .height(data.swapchain_extent.height)
                .layers(1);

            device.create_framebuffer(&create_info, None)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(())
}