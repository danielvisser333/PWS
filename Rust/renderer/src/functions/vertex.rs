use ash::{Device, vk::{CommandPool, Queue}};

use crate::{allocator::{BufferAndAllocation, Allocator}, math::{Vertex, ModelMatrix, InstanceVertex}};

pub const INDEX_BUFFERS : [usize;2] = [4,5];
pub const VERTEX_BUFFERS : [usize;2] = [0,1];
pub const GRID_BUFFERS : [usize;1] = [2];
pub const INSTANCE_BUFFERS : [usize;1] = [3];

pub unsafe fn create_vertex_buffers(device : &Device, allocator : &mut Allocator, command_pool : CommandPool, queue : Queue, instance_positions : Vec<ModelMatrix>) -> Vec<(u32,BufferAndAllocation)>{
    let vertex_data = InstanceVertex::get_initial_vertex_data();
    let grid_data = Vertex::get_grid();
    let mut buffers = vec!();
    buffers.push((vertex_data[0].0.len() as u32,super::buffer::create_vertex_buffer(device, allocator, command_pool, queue, vertex_data[0].0.to_vec())));
    buffers.push((vertex_data[1].0.len() as u32,super::buffer::create_vertex_buffer(device, allocator, command_pool, queue, vertex_data[1].0.to_vec())));
    buffers.push((grid_data.len() as u32, super::buffer::create_vertex_buffer(device, allocator, command_pool, queue, grid_data.to_vec())));
    buffers.push((instance_positions.len()as u32, super::buffer::create_vertex_buffer(device, allocator, command_pool, queue, instance_positions)));
    buffers.push((vertex_data[0].1.len() as u32, super::buffer::create_index_buffer(device, allocator, command_pool, queue, vertex_data[0].1.clone())));
    buffers.push((vertex_data[1].1.len() as u32, super::buffer::create_index_buffer(device, allocator, command_pool, queue, vertex_data[1].1.clone())));
    return buffers;
}
pub unsafe fn create_object_buffer(device : &Device, allocator : &mut Allocator, objects : Vec<ModelMatrix>, command_pool : CommandPool, queue : Queue,) -> BufferAndAllocation{
    return super::buffer::create_vertex_buffer(device, allocator, command_pool, queue, objects);
}