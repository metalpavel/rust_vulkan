mod app_data;
mod app_defines;
mod command_buffer;
mod framebuffer;
mod instance;
mod logical_device;
mod physical_device;
mod pipeline;
mod queue_family;
mod swapchain;
mod sync;
mod vertex_buffer;
mod image;
mod descriptor;

use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use nalgebra_glm as glm;
use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::window as vk_window;
use winit::window::{Window};
use std::time::Instant;
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;

use vulkanalia::vk::ExtDebugUtilsExtension;
use vulkanalia::vk::KhrSurfaceExtension;
use vulkanalia::vk::KhrSwapchainExtension;

lazy_static! {
    static ref VERTICES: Vec<vertex_buffer::Vertex> = vec![
        vertex_buffer::Vertex::new(glm::vec3(-0.5, -0.5, 0.0),glm::vec3(1.0, 0.0, 0.0)),
        vertex_buffer::Vertex::new(glm::vec3(0.5, -0.5, 0.0), glm::vec3(0.0, 1.0, 0.0)),
        vertex_buffer::Vertex::new(glm::vec3(0.5, 0.5, 0.0), glm::vec3(0.0, 0.0, 1.0)),
        vertex_buffer::Vertex::new(glm::vec3(-0.5, 0.5, 0.0), glm::vec3(1.0, 1.0, 1.0)),

        vertex_buffer::Vertex::new(glm::vec3(-0.5, -0.5, 1.0),glm::vec3(1.0, 1.0, 0.0)),
        vertex_buffer::Vertex::new(glm::vec3(0.5, -0.5, 1.0), glm::vec3(0.0, 1.0, 1.0)),
        vertex_buffer::Vertex::new(glm::vec3(0.5, 0.5, 1.0), glm::vec3(1.0, 0.0, 1.0)),
        vertex_buffer::Vertex::new(glm::vec3(-0.5, 0.5, 1.0), glm::vec3(1.0, 1.0, 1.0)),
    ];

    static ref INDICES: Vec<u16> = vec![
        // bottom flipped
        0, 1, 2, 2, 3, 0, // bottom
        4, 5, 6, 6, 7, 4, // top
        0, 1, 5, 5, 4, 0, // left
        2, 3, 7, 7, 6, 2, // right
        1, 2, 6, 6, 5, 1, // front
        // back flipped
        3, 0, 4, 4, 7, 3, // back
    ];
}

#[derive(Clone, Debug)]
pub struct App {
    entry: Entry,
    instance: Instance,
    data: app_data::Data,
    device: Device,
    frame: usize,
    pub resized: bool,
    start: Instant,
}

impl App {
    pub unsafe fn create(window: &Window) -> Result<Self> {
        let loader = LibloadingLoader::new(LIBRARY)?;
        let entry = Entry::new(loader).map_err(|b| anyhow!("{}", b))?;
        let mut data = app_data::Data::default();
        let instance = instance::create(window, &entry, &mut data)?;

        data.surface = vk_window::create_surface(&instance, &window, &window)?;

        physical_device::pick_physical_device(&instance, &mut data)?;

        let device = logical_device::create(&instance, &mut data)?;

        swapchain::create(window, &instance, &device, &mut data)?;

        swapchain::create_swapchain_image_views(&device, &mut data)?;

        pipeline::create_render_pass(&instance, &device, &mut data)?;

        descriptor::create_descriptor_set_layout(&device, &mut data)?;

        pipeline::create_pipeline(&device, &mut data)?;

        command_buffer::create_command_pool(&instance, &device, &mut data)?;

        swapchain::create_depth_objects(&instance, &device, &mut data)?;

        framebuffer::create(&device, &mut data)?;

        vertex_buffer::create(&instance, &device, &mut data, &VERTICES)?;
        vertex_buffer::create_index_buffer(&instance, &device, &mut data, &INDICES)?;

        vertex_buffer::create_uniform_buffers(&instance, &device, &mut data)?;

        descriptor::create_descriptor_pool(&device, &mut data)?;
        descriptor::create_descriptor_sets(&device, &mut data)?;

        command_buffer::create_command_buffers(&device, &mut data, INDICES.len() as u32)?;

        sync::create_sync_objects(&device, &mut data)?;

        Ok(Self {entry, instance, data, device, frame: 0, resized: false, start: Instant::now() })
    }

    pub unsafe fn render(&mut self, window: &Window) -> Result<()> {
        let in_flight_fence = self.data.in_flight_fences[self.frame];

        self.device
            .wait_for_fences(&[in_flight_fence], true, u64::max_value())?;

        let result = self.device.acquire_next_image_khr(
            self.data.swapchain,
            u64::max_value(),
            self.data.image_available_semaphores[self.frame],
            vk::Fence::null(),
        );

        let image_index = match result {
            Ok((image_index, _)) => image_index as usize,
            Err(vk::ErrorCode::OUT_OF_DATE_KHR) => return self.recreate_swapchain(window),
            Err(e) => return Err(anyhow!(e)),
        };

        let image_in_flight = self.data.images_in_flight[image_index];
        if !image_in_flight.is_null() {
            self.device
                .wait_for_fences(&[image_in_flight], true, u64::max_value())?;
        }

        self.data.images_in_flight[image_index] = in_flight_fence;

        self.update_uniform_buffer(image_index)?;

        let wait_semaphores = &[self.data.image_available_semaphores[self.frame]];
        let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = &[self.data.command_buffers[image_index]];
        let signal_semaphores = &[self.data.render_finished_semaphores[self.frame]];
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_stages)
            .command_buffers(command_buffers)
            .signal_semaphores(signal_semaphores);

        self.device.reset_fences(&[in_flight_fence])?;

        self.device
            .queue_submit(self.data.graphics_queue, &[submit_info], in_flight_fence)?;

        let swapchains = &[self.data.swapchain];
        let image_indices = &[image_index as u32];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(signal_semaphores)
            .swapchains(swapchains)
            .image_indices(image_indices);

        let result = self.device.queue_present_khr(self.data.present_queue, &present_info);
        let changed = result == Ok(vk::SuccessCode::SUBOPTIMAL_KHR) || result == Err(vk::ErrorCode::OUT_OF_DATE_KHR);
        if self.resized || changed {
            self.resized = false;
            self.recreate_swapchain(window)?;
        } else if let Err(e) = result {
            return Err(anyhow!(e));
        }

        self.frame = (self.frame + 1) % app_defines::MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }

    unsafe fn recreate_swapchain(&mut self, window: &Window) -> Result<()> {
        self.device.device_wait_idle()?;
        self.destroy_swapchain();

        swapchain::create(window, &self.instance, &self.device, &mut self.data)?;
        swapchain::create_swapchain_image_views(&self.device, &mut self.data)?;

        pipeline::create_render_pass(&self.instance, &self.device, &mut self.data)?;
        pipeline::create_pipeline(&self.device, &mut self.data)?;

        swapchain::create_depth_objects(&self.instance, &self.device, &mut self.data)?;

        framebuffer::create(&self.device, &mut self.data)?;

        vertex_buffer::create_uniform_buffers(&self.instance, &self.device, &mut self.data)?;

        descriptor::create_descriptor_pool(&self.device, &mut self.data)?;
        descriptor::create_descriptor_sets(&self.device, &mut self.data)?;

        command_buffer::create_command_buffers(&self.device, &mut self.data,  INDICES.len() as u32)?;

        self.data.images_in_flight.resize(self.data.swapchain_images.len(), vk::Fence::null());

        Ok(())
    }

    pub unsafe fn destroy(&mut self) {
        self.device.device_wait_idle().unwrap();

        self.destroy_swapchain();

        self.data.in_flight_fences.iter().for_each(|f| self.device.destroy_fence(*f, None));
        self.data.render_finished_semaphores.iter().for_each(|s| self.device.destroy_semaphore(*s, None));
        self.data.image_available_semaphores.iter().for_each(|s| self.device.destroy_semaphore(*s, None));
        self.device.free_memory(self.data.index_buffer_memory, None);
        self.device.destroy_buffer(self.data.index_buffer, None);
        self.device.free_memory(self.data.vertex_buffer_memory, None);
        self.device.destroy_buffer(self.data.vertex_buffer, None);
        self.device.destroy_command_pool(self.data.command_pool, None);
        self.device.destroy_descriptor_set_layout(self.data.descriptor_set_layout, None);
        self.device.destroy_device(None);
        self.instance.destroy_surface_khr(self.data.surface, None);

        if app_defines::VALIDATION_ENABLED {
            self.instance.destroy_debug_utils_messenger_ext(self.data.messenger, None);
        }

        self.instance.destroy_instance(None);
    }

    unsafe fn destroy_swapchain(&mut self) {
        self.device.free_command_buffers(self.data.command_pool, &self.data.command_buffers);
        self.device.destroy_descriptor_pool(self.data.descriptor_pool, None);
        self.data.uniform_buffers_memory.iter().for_each(|m| self.device.free_memory(*m, None));
        self.data.uniform_buffers.iter().for_each(|b| self.device.destroy_buffer(*b, None));
        self.device.destroy_image_view(self.data.depth_image_view, None);
        self.device.free_memory(self.data.depth_image_memory, None);
        self.device.destroy_image(self.data.depth_image, None);
        self.data.framebuffers.iter().for_each(|f| self.device.destroy_framebuffer(*f, None));
        self.device.destroy_pipeline(self.data.pipeline, None);
        self.device.destroy_pipeline_layout(self.data.pipeline_layout, None);
        self.device.destroy_render_pass(self.data.render_pass, None);
        self.data.swapchain_image_views.iter().for_each(|v| self.device.destroy_image_view(*v, None));
        self.device.destroy_swapchain_khr(self.data.swapchain, None);
    }

    unsafe fn update_uniform_buffer(&self, image_index: usize) -> Result<()> {
        // MVP

        let time = self.start.elapsed().as_secs_f32();

        let model = glm::rotate(
            &glm::identity(),
            time * glm::radians(&glm::vec1(90.0))[0],
            &glm::vec3(0.0, 0.0, 1.0),
        );

        let view = glm::look_at(
            &glm::vec3(2.0, 2.0, 2.0),
            &glm::vec3(0.0, 0.0, 0.0),
            &glm::vec3(0.0, 0.0, 1.0),
        );

        let mut proj = glm::perspective_rh_zo(
            self.data.swapchain_extent.width as f32 / self.data.swapchain_extent.height as f32,
            glm::radians(&glm::vec1(45.0))[0],
            0.1,
            10.0,
        );

        proj[(1, 1)] *= -1.0;

        let ubo = vertex_buffer::UniformBufferObject { model, view, proj };

        // Copy

        let memory = self.device.map_memory(
            self.data.uniform_buffers_memory[image_index],
            0,
            size_of::<vertex_buffer::UniformBufferObject>() as u64,
            vk::MemoryMapFlags::empty(),
        )?;

        memcpy(&ubo, memory.cast(), 1);

        self.device.unmap_memory(self.data.uniform_buffers_memory[image_index]);

        Ok(())
    }
}
