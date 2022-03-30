pub mod functions;
pub mod allocator;
pub mod math;

const NEUTRAL_ARROW_VECTOR : Vector3<f32> = Vector3{x:0.0,y:0.0,z:1.0};

use std::{sync::mpsc::{Sender, Receiver}};

use allocator::{Allocator, BufferAndAllocation};
use ash::{Entry, Instance, extensions::khr::{Surface, Swapchain}, vk::{SurfaceKHR, SwapchainKHR, ImageView, PhysicalDevice, RenderPass, ShaderModule, Framebuffer, DescriptorSetLayout, PipelineLayout, PipelineCache, DescriptorPool, DescriptorSet, Pipeline, Fence, CommandPool, Queue, CommandBuffer, PipelineStageFlags, SubmitInfo, StructureType, PresentInfoKHR, Extent2D}, Device};
use cgmath::{Matrix4, Vector3, Matrix, Quaternion, InnerSpace};
use functions::{image::ImageAndView, device::QueueInfo, synchronization::Synchronizer, buffer::UniformBufferObject, swapchain::SwapchainInfo, vertex::INSTANCE_BUFFERS};
use math::{UniformBuffer, camera::Camera, ModelMatrix};
use rayon::{ThreadPoolBuilder, ThreadPool};
use winit::{event_loop::{EventLoop, ControlFlow}, window::Window, event::{Event, WindowEvent, StartCause, VirtualKeyCode, DeviceEvent, MouseScrollDelta, MouseButton, ElementState}, dpi::PhysicalSize};
#[cfg(target_os="linux")]
use winit::platform::{unix::EventLoopExtUnix, run_return::EventLoopExtRunReturn};
#[cfg(target_os="windows")]
use winit::platform::{run_return::EventLoopExtRunReturn,windows::EventLoopExtWindows};
const MAX_FRAMES_IN_FLIGHT : usize = 2;

pub struct Renderer{
    sender : Sender<RenderTask>,
    //receiver : Receiver<RenderResult>,
    receiver_shutdown : Receiver<RenderResult>,
    thread_pool : ThreadPool,
}
impl Renderer{
    pub fn new(debug : bool) -> Self{
        let thread_pool = ThreadPoolBuilder::new().build().expect("Failed to create threadpool");
        let (sender, receiver_render_thread) = std::sync::mpsc::channel();
        //let (sender_render_thread, receiver) = std::sync::mpsc::channel();
        let (shutdown_sender, receiver_shutdown) = std::sync::mpsc::channel();
        //Start the renderer on another thread
        thread_pool.spawn(move ||{
            println!("Created render thread");
            let mut event_loop : EventLoop<()> = EventLoop::new_any_thread();
            let window = Window::new(&event_loop).expect("Failed to create render window");
            let mut renderer = RenderOnThread::new(&window, debug);
            event_loop.run_return(|event,_,control_flow|{
                match receiver_render_thread.try_recv(){
                    Ok(task) => {
                        match task{
                            RenderTask::Draw=>{}
                            RenderTask::UpdateObjects(objects)=>{
                                renderer.set_matrixes(objects);
                                //sender_render_thread.send(RenderResult::Success).unwrap();
                            }
                        }
                    }
                    Err(_)=>{}
                };
                match event{
                    Event::WindowEvent{event,window_id:_}=>{
                        match event{
                            WindowEvent::CloseRequested=>{*control_flow=ControlFlow::Exit;}
                            WindowEvent::KeyboardInput{device_id:_, is_synthetic:_, input}=>{
                                if input.virtual_keycode == Some(VirtualKeyCode::F10) && input.state == ElementState::Pressed{
                                    renderer.allocator.dump_contents();
                                }
                                else if input.virtual_keycode == Some(VirtualKeyCode::N) && input.state == ElementState::Pressed{
                                    shutdown_sender.send(RenderResult::NextStep).unwrap();
                                }
                            }
                            WindowEvent::MouseWheel{delta, .. }=>{
                                match delta{
                                    MouseScrollDelta::LineDelta(_,y) =>{
                                        renderer.camera.mouse_zoom(y);
                                    }
                                    MouseScrollDelta::PixelDelta(delta)=>{
                                        renderer.camera.mouse_zoom(delta.y as f32);
                                    }
                                }
                            }
                            WindowEvent::MouseInput{button,state,..} => {
                                match button{
                                    MouseButton::Left =>{
                                        match state{
                                            ElementState::Pressed=>{renderer.camera.left_mouse_button_pressed = true}
                                            ElementState::Released=>{renderer.camera.left_mouse_button_pressed = false}
                                        }
                                    }
                                    _=>{}
                                }
                            }
                            _=>{}
                        }
                    }
                    Event::RedrawRequested(_) => {
                        let resized = renderer.draw();
                        if resized{renderer.recreate_swapchain(window.inner_size());}
                    }
                    Event::MainEventsCleared => {
                        window.request_redraw();
                    }
                    Event::NewEvents(start) =>{
                        match start{
                            StartCause::Init=>{*control_flow=ControlFlow::Poll}
                            _=>{}
                        }
                    }
                    Event::DeviceEvent{device_id:_,event}=>{
                        match event{
                            DeviceEvent::MouseMotion{delta}=>{
                                renderer.camera.mouse_movement(delta);
                            }
                            DeviceEvent::MouseWheel{delta} => {
                                match delta{
                                    MouseScrollDelta::LineDelta(_,y) =>{
                                        renderer.camera.mouse_zoom(y);
                                    }
                                    MouseScrollDelta::PixelDelta(delta)=>{
                                        renderer.camera.mouse_zoom(delta.y as f32);
                                    }
                                }
                            }
                            _=>{}
                        }
                    }
                    _=>{}
                }
            });
            drop(renderer);
            println!("Destroying render thread");
            shutdown_sender.send(RenderResult::Shutdown).unwrap();
        });
        return Self{
            sender,/*receiver,*/thread_pool,receiver_shutdown,
        }
    }
    pub fn transform_grid(&self, grid : Vec<Vec<Vec<([f32;3],[f32;3])>>>){
        let matrices = self.thread_pool.install(||{
            return RenderTask::UpdateObjects(grid_to_matrices(grid));
        });
        match self.sender.send(matrices){Ok(_)=>{}Err(_)=>{}};
    }
    pub fn await_request(&self) -> RenderResult{
        return self.receiver_shutdown.recv().unwrap();
    }
}
pub enum RenderTask{
    Draw,
    UpdateObjects(Vec<ModelMatrix>),
}
pub enum RenderResult{
    NextStep,
    Shutdown,
}
struct RenderOnThread{
    _entry : Entry,
    instance : Instance,
    surface_loader : Surface,
    surface : SurfaceKHR,
    physical_device : PhysicalDevice,
    queue_info : QueueInfo,
    device : Device,
    swapchain_loader : Swapchain,
    swapchain : SwapchainKHR,
    swapchain_info : SwapchainInfo,
    swapchain_image_views : Vec<ImageView>,
    allocator : Allocator,
    depth_image : ImageAndView,
    render_pass : RenderPass,
    framebuffers : Vec<Framebuffer>,
    uniform_buffer : UniformBufferObject,
    descriptor_set_layout : DescriptorSetLayout,
    descriptor_pool : DescriptorPool,
    descriptor_sets : Vec<DescriptorSet>,
    pipeline_layout : PipelineLayout,
    pipeline_cache : PipelineCache,
    shaders : Vec<ShaderModule>,
    pipelines : Vec<Pipeline>,
    graphics_command_pool : CommandPool,
    graphics_queue : Queue,
    vertex_buffers : Vec<(u32,BufferAndAllocation)>,
    drawing_command_buffers : Vec<CommandBuffer>,
    synchronizer : Synchronizer,
    camera : Camera,
}
impl RenderOnThread{
    pub fn new(window : &Window, debug : bool) -> Self{
        let entry = unsafe{Entry::load()}.expect("Failed to load Vulkan drivers");
        let instance = unsafe{functions::instance::create_instance(&entry, window, debug)};
        let surface_loader = Surface::new(&entry, &instance);
        let surface = unsafe{ash_window::create_surface(&entry, &instance, window, None)}.expect("Failed to create window");
        let physical_device = functions::device::get_device_handle(&instance, &surface_loader, &surface);
        let device_limits = unsafe{instance.get_physical_device_properties(physical_device)}.limits;
        let queue_info = functions::device::QueueInfo::new(&instance, &surface_loader, &surface, physical_device);
        let device = unsafe{functions::device::create_device(&instance, physical_device, &queue_info)};
        let swapchain_loader = Swapchain::new(&instance, &device);
        let swapchain_info = functions::swapchain::SwapchainInfo::new(&instance, physical_device, &surface_loader, surface, window.inner_size());
        let swapchain = unsafe{functions::swapchain::create_swapchain(&swapchain_loader, &swapchain_info, &queue_info, surface)};
        let swapchain_images = unsafe{swapchain_loader.get_swapchain_images(swapchain)}.expect("Failed to get swapchain images");
        let swapchain_image_views = unsafe{functions::image::create_swapchain_image_views(&device, &swapchain_images, swapchain_info.surface_format)};
        let mut allocator = unsafe{allocator::Allocator::new(&instance, physical_device, device.clone())};
        let depth_image = unsafe{functions::image::create_depth_image(&device, &mut allocator, swapchain_info.extent, swapchain_info.depth_format)};
        let render_pass = unsafe{functions::render_pass::create_render_pass(&device, swapchain_info.surface_format, swapchain_info.depth_format)};
        let framebuffers = unsafe{functions::framebuffer::create_framebuffers(&device, &swapchain_image_views, depth_image.view, render_pass, swapchain_info.extent)};
        let uniform_buffer = unsafe{functions::buffer::create_uniform_buffers(&device, &mut allocator, swapchain_image_views.len() as u32, &device_limits)};
        let descriptor_set_layout = unsafe{functions::descriptor::create_descriptor_set_layout(&device)};
        let pipeline_layout = unsafe{functions::pipeline::create_pipeline_layout(&device, &descriptor_set_layout)};
        let pipeline_cache = unsafe{functions::pipeline::create_pipeline_cache(&device)};
        let descriptor_pool = unsafe{functions::descriptor::create_descriptor_pool(&device, swapchain_image_views.len() as u32)};
        let descriptor_sets = unsafe{functions::descriptor::create_descriptor_sets(&device, descriptor_set_layout, descriptor_pool, swapchain_image_views.len() as u32, uniform_buffer.buffer.buffer, &device_limits)};
        let synchronizer = unsafe{Synchronizer::new(&device, swapchain_image_views.len() as u32)};
        let shaders = unsafe{functions::shader::load_shaders(&device)};
        let pipelines = unsafe{functions::pipeline::create_pipelines(&device, pipeline_cache, pipeline_layout, render_pass, &shaders, swapchain_info.extent)};
        let graphics_command_pool = unsafe{functions::command::create_command_pool(&device, queue_info.graphics_family)};
        let graphics_queue = unsafe{device.get_device_queue(queue_info.graphics_family, 0)};
        let instance_positions = ModelMatrix::get_default();
        let vertex_buffers = unsafe{functions::vertex::create_vertex_buffers(&device, &mut allocator, graphics_command_pool, graphics_queue, instance_positions)};
        let drawing_command_buffers = unsafe{functions::command::create_drawing_command_buffers(&device, graphics_command_pool, pipeline_layout, &pipelines, render_pass, &framebuffers, &descriptor_sets, &vertex_buffers, swapchain_info.extent)};
        let camera = Camera::new(swapchain_info.extent);
        return Self{
            _entry:entry,instance,surface_loader,surface,physical_device,queue_info,device,swapchain_loader,swapchain,swapchain_image_views,allocator,depth_image,
            render_pass,shaders,framebuffers,uniform_buffer,descriptor_set_layout,pipeline_layout,pipeline_cache,descriptor_pool,descriptor_sets,pipelines,synchronizer,
            graphics_queue,graphics_command_pool,vertex_buffers,drawing_command_buffers,camera,swapchain_info,
        }
    }
    ///Draw a new image to the screen
    pub fn draw(&mut self) -> bool{ 
        let wait_fences = [self.synchronizer.in_flight_fences[self.synchronizer.current_frame]];
        unsafe{
            self.device.wait_for_fences(&wait_fences, true, u64::MAX).expect("Failed to wait for fences");
        }
        let (image_index, suboptimal) = match unsafe{self.swapchain_loader.acquire_next_image(self.swapchain, u64::MAX, self.synchronizer.image_available_semaphores[self.synchronizer.current_frame], Fence::null())}{
            Ok(tuple)=>{tuple}
            Err(error)=>{
                match error{
                    ash::vk::Result::ERROR_OUT_OF_DATE_KHR=>{return true}
                    _=>{panic!("Failed to draw frame");}
                }
            }
        };
        self.camera.update();
        unsafe{self.update_uniform_buffer(image_index, self.camera.matrix)};
        let wait_semaphores = [self.synchronizer.image_available_semaphores[self.synchronizer.current_frame]];
        let wait_stages = [PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.synchronizer.render_finished_semaphores[self.synchronizer.current_frame]];
        let submit_infos = [SubmitInfo {
            s_type: StructureType::SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: wait_semaphores.len() as u32,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            p_wait_dst_stage_mask: wait_stages.as_ptr(),
            command_buffer_count: 1,
            p_command_buffers: &self.drawing_command_buffers[image_index as usize],
            signal_semaphore_count: signal_semaphores.len() as u32,
            p_signal_semaphores: signal_semaphores.as_ptr(),
        }];
        unsafe{self.device.reset_fences(&wait_fences)}.expect("Failed to reset fences");
        unsafe{self.device.queue_submit(self.graphics_queue, &submit_infos, self.synchronizer.in_flight_fences[self.synchronizer.current_frame])}.expect("Failed to submit queue");
        let swapchains = [self.swapchain];
        let present_info = PresentInfoKHR {
            s_type: StructureType::PRESENT_INFO_KHR,
            p_next: std::ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: signal_semaphores.as_ptr(),
            swapchain_count: 1,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: &image_index,
            p_results: std::ptr::null_mut(),
        };
        self.synchronizer.current_frame = (self.synchronizer.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
        match unsafe{self.swapchain_loader.queue_present(self.graphics_queue, &present_info)}{
            Ok(sub) => {return sub||suboptimal}
            Err(error) => {
                match error{
                    ash::vk::Result::ERROR_OUT_OF_DATE_KHR => {return true}
                    _=>{panic!("Queue present failed")}
                }
            }
        };
    }
    ///Recreate the image to render to (swapchain), necessary if the window gets resized
    pub fn recreate_swapchain(&mut self, window_size : PhysicalSize<u32>){
        unsafe{self.destroy_swapchain()}
        self.swapchain_info = functions::swapchain::SwapchainInfo::new(&self.instance, self.physical_device, &self.surface_loader, self.surface, window_size);
        self.swapchain = unsafe{functions::swapchain::create_swapchain(&self.swapchain_loader, &self.swapchain_info, &self.queue_info, self.surface)};
        let swapchain_images = unsafe{self.swapchain_loader.get_swapchain_images(self.swapchain).expect("Failed to get swapchain images")};
        self.swapchain_image_views = unsafe{functions::image::create_swapchain_image_views(&self.device, &swapchain_images, self.swapchain_info.surface_format)};
        self.depth_image = unsafe{functions::image::create_depth_image(&self.device, &mut self.allocator, self.swapchain_info.extent, self.swapchain_info.depth_format)};
        self.framebuffers = unsafe{functions::framebuffer::create_framebuffers(&self.device, &self.swapchain_image_views, self.depth_image.view, self.render_pass, self.swapchain_info.extent)};
        self.pipelines = unsafe{functions::pipeline::create_pipelines(&self.device, self.pipeline_cache, self.pipeline_layout, self.render_pass, &self.shaders, self.swapchain_info.extent)};
        unsafe{self.recreate_command_buffers(self.swapchain_info.extent)};
        self.camera.correct_perspective(self.swapchain_info.extent);
    }
    ///Create a new buffer to hold the matrices and apply them
    pub fn set_matrixes(&mut self, models : Vec<ModelMatrix>){
        unsafe{self.device.device_wait_idle()}.expect("Failed to wait for device");
        unsafe{self.vertex_buffers[INSTANCE_BUFFERS[0]].1.destroy(&mut self.allocator)};
        self.vertex_buffers[INSTANCE_BUFFERS[0]].0 = models.len() as u32;
        self.vertex_buffers[INSTANCE_BUFFERS[0]].1 = unsafe{functions::vertex::create_object_buffer(&self.device, &mut self.allocator, models, self.graphics_command_pool, self.graphics_queue)};
        unsafe{self.recreate_command_buffers(self.swapchain_info.extent)};
    }
    ///Update the projection and view matices
    unsafe fn update_uniform_buffer(&self, current_frame : u32, object : UniformBuffer){
        self.uniform_buffer.update_uniform_buffer(object, current_frame, &self.device);
    }
    ///Rerecord the render commands
    unsafe fn recreate_command_buffers(&mut self, extent : Extent2D){
        self.drawing_command_buffers = functions::command::create_drawing_command_buffers(&self.device, self.graphics_command_pool, self.pipeline_layout, &self.pipelines, self.render_pass, &self.framebuffers, &self.descriptor_sets, &self.vertex_buffers, extent);
    }
    unsafe fn destroy_swapchain(&mut self){
        self.device.device_wait_idle().expect("Failed to wait for device");
        self.device.free_command_buffers(self.graphics_command_pool, &self.drawing_command_buffers);
        for &pipeline in self.pipelines.iter(){
            self.device.destroy_pipeline(pipeline, None);
        }
        for &framebuffer in self.framebuffers.iter(){
            self.device.destroy_framebuffer(framebuffer, None);
        }
        self.depth_image.destroy(&mut self.allocator);
        for &image_view in self.swapchain_image_views.iter(){
            self.device.destroy_image_view(image_view, None);
        }
        self.swapchain_loader.destroy_swapchain(self.swapchain, None);
    }
}
///Ensure all elements of the renderer are destroyed in the right order
impl Drop for RenderOnThread{
    fn drop(&mut self) {
        unsafe{
            self.device.device_wait_idle().expect("Failed to wait for device handle to finish");

            self.destroy_swapchain();
            for &shader in self.shaders.iter(){
                self.device.destroy_shader_module(shader, None);
            }
            functions::pipeline::save_pipeline_cache(&self.device, self.pipeline_cache);
            for buffer in self.vertex_buffers.iter(){
                buffer.1.destroy(&mut self.allocator);
            }
            self.synchronizer.destroy(&self.device);
            self.device.destroy_command_pool(self.graphics_command_pool, None);
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_pipeline_cache(self.pipeline_cache, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.uniform_buffer.buffer.destroy(&mut self.allocator);
            self.device.destroy_render_pass(self.render_pass, None);
            self.allocator.destroy();
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}
///Convert the grid to the corresponding transformation matrices
pub fn grid_to_matrices(grid : Vec<Vec<Vec<([f32;3],[f32;3])>>>) -> Vec<ModelMatrix>{
    let mut matrices = vec!();

    for x in 0..grid.len(){
        for y in 0..grid[x].len(){
            for z in 0..grid[x][y].len(){
                let point = Vector3::new((x as f32+0.5)/grid.len() as f32 - 0.5,(y as f32+0.5)/grid[x].len() as f32 - 0.5,(z as f32+0.5)/grid[x][y].len() as f32 - 0.5);
                let vector = Vector3::new(grid[x][y][z].0[0], grid[x][y][z].0[1], grid[x][y][z].0[2]).normalize();
                //let vector = Vector3::new(0.0, 1.0, 0.0);
                //println!("{:?}",vector);
                //println!("{},{},{}",grid[x][y][z][0], grid[x][y][z][1], grid[x][y][z][2]);
                let mut translation = Matrix4::from_translation(point);
                translation*=vector.magnitude2();
                //TODO: Rotate arrow to point to vector
                let cross = NEUTRAL_ARROW_VECTOR.cross(vector.normalize());
                let quat = Quaternion::from_sv(
                    (NEUTRAL_ARROW_VECTOR.magnitude2().powf(2.0)) * (vector.normalize().magnitude2().powf(2.0)).sqrt(), 
                    cross
                ).normalize();
                translation = translation*Matrix4::from(quat);
                translation.swap_columns(1, 2);
                translation*=vector.magnitude2();
                matrices.push(ModelMatrix{matrix:translation,color:grid[x][y][z].1});
            }
        }
    }
    return matrices; //WARNING, this function crashes if matrices is empty
    //return vec!(ModelMatrix{matrix:Matrix4::identity()});
}