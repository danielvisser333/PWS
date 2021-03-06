#version 450

layout(location = 0) in vec3 inPosition;
layout(location = 5) in vec3 inColor;

layout(location = 0) out vec3 fragColor;

layout(location = 1) in mat4 transform; 

layout(binding = 0) uniform UniformBufferObject {
    mat4 transform;
} ubo;

void main() {
    gl_Position = ubo.transform * transform * vec4(inPosition, 1.0);
    fragColor = inColor;
}