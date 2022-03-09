pub mod camera;

use ash::vk::{DescriptorSetLayoutBinding, DescriptorType, ShaderStageFlags, VertexInputBindingDescription, VertexInputRate, VertexInputAttributeDescription, Format};
use cgmath::{Matrix4, Vector4, SquareMatrix};
use memoffset::offset_of;
use tobj::LoadOptions;

pub struct ModelMatrix{
    pub matrix : Matrix4<f32>,
}
impl ModelMatrix{
    pub fn get_default() -> Vec<Self>{
        return vec!(
            Self{matrix:Matrix4::identity()},
        );
    }
}
#[derive(Clone, Copy, Debug)]
pub struct InstanceVertex{
    pub pos : [f32;3],
    pub color : [f32;3],
}
impl InstanceVertex{
    pub fn get_bindings() -> Vec<VertexInputBindingDescription>{
        return vec!(
            VertexInputBindingDescription{
                binding : 0,
                input_rate : VertexInputRate::VERTEX,
                stride : std::mem::size_of::<f32>() as u32 * 6,
            },
            VertexInputBindingDescription{
                binding : 1,
                input_rate : VertexInputRate::INSTANCE,
                stride : std::mem::size_of::<UniformBuffer>() as u32,
            }
        );
    }
    pub fn get_attributes() -> Vec<VertexInputAttributeDescription>{
        return vec!(
            VertexInputAttributeDescription{
                binding : 0,
                format : Format::R32G32B32_SFLOAT,
                location : 0,
                offset : offset_of!(Self,pos) as u32,
            },
            VertexInputAttributeDescription{
                binding : 0,
                format : Format::R32G32B32_SFLOAT,
                location : 1,
                offset : offset_of!(Self,color) as u32,
            },
            VertexInputAttributeDescription{
                binding : 1,
                format : Format::R32G32B32A32_SFLOAT,
                location : 2,
                offset : 0,
            },
            VertexInputAttributeDescription{
                binding : 1,
                format : Format::R32G32B32A32_SFLOAT,
                location : 3,
                offset : std::mem::size_of::<Vector4<f32>>() as u32,
            },
            VertexInputAttributeDescription{
                binding : 1,
                format : Format::R32G32B32A32_SFLOAT,
                location : 4,
                offset : 2 * std::mem::size_of::<Vector4<f32>>() as u32,
            },
            VertexInputAttributeDescription{
                binding : 1,
                format : Format::R32G32B32A32_SFLOAT,
                location : 5,
                offset : 3 * std::mem::size_of::<Vector4<f32>>() as u32,
            },
        );
    }
    pub fn get_initial_vertex_data() -> Vec<(Vec<Self>,Vec<u32>)>{
        let arrow_file = format!("{}/arrow.obj",std::env::current_exe().unwrap().parent().unwrap().to_string_lossy());
        println!("{}",arrow_file);
        let obj = tobj::load_obj(arrow_file, &LoadOptions{triangulate:true,..Default::default()}).unwrap().0;
        let mut data = vec!();
        for model in obj{
            let mut vertices = vec!();
            let vertex_count = model.mesh.positions.len()/3;
            for i in 0..vertex_count{
                let vertex = Self{
                    pos : [
                        model.mesh.positions[i*3]/50.0,
                        model.mesh.positions[i*3+1]/50.0,
                        model.mesh.positions[i*3+2]/50.0],
                    color : [
                        0.0,
                        0.0,
                        0.0
                        ],
                };
                vertices.push(vertex);
            }
            data.push((vertices,model.mesh.indices));
        }
        return data;
    }
    
}
#[derive(Clone, Copy)]
pub struct Vertex{
    pub pos : [f32;3],
    pub color : [f32;3],
}
impl Vertex{
    pub fn get_grid() -> [Self;6]{
        return [
                Vertex{pos:[1.0,0.0,0.0],color:[0.0,0.0,0.0]},
                Vertex{pos:[-1.0,0.0,0.0],color:[0.0,0.0,0.0]},
                Vertex{pos:[0.0,1.0,0.0],color:[0.0,0.0,0.0]},
                Vertex{pos:[0.0,-1.0,0.0],color:[0.0,0.0,0.0]},
                Vertex{pos:[0.0,0.0,1.0],color:[0.0,0.0,0.0]},
                Vertex{pos:[0.0,0.0,-1.0],color:[0.0,0.0,0.0]},
        ];
    }
    pub fn get_bindings() -> Vec<VertexInputBindingDescription>{
        return vec!(
            VertexInputBindingDescription{
                binding : 0,
                input_rate : VertexInputRate::VERTEX,
                stride : std::mem::size_of::<f32>() as u32 * 6,
            },
        );
    }
    pub fn get_attributes() -> Vec<VertexInputAttributeDescription>{
        return vec!(
            VertexInputAttributeDescription{
                binding : 0,
                format : Format::R32G32B32_SFLOAT,
                location : 0,
                offset : offset_of!(Self,pos) as u32,
            },
            VertexInputAttributeDescription{
                binding : 0,
                format : Format::R32G32B32_SFLOAT,
                location : 1,
                offset : offset_of!(Self,color) as u32,
            },
        );
    }
}
#[derive(Clone, Copy)]
pub struct UniformBuffer{
    pub matrix : Matrix4<f32>,
}
impl UniformBuffer{
    pub fn get_bindings() -> Vec<DescriptorSetLayoutBinding>{
        return vec!(
            DescriptorSetLayoutBinding{
                binding : 0,
                descriptor_count : 1,
                descriptor_type : DescriptorType::UNIFORM_BUFFER,
                p_immutable_samplers : std::ptr::null(),
                stage_flags : ShaderStageFlags::VERTEX,
            }
        );
    }
}