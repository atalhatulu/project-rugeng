#version 450

layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec2 in_uv;
layout(location = 3) in vec4 in_color;

layout(push_constant) uniform ShadowPushConstants {
    mat4 light_space_mvp; // LightSpaceMatrix * ModelMatrix
} pc;

void main() {
    gl_Position = pc.light_space_mvp * vec4(in_position, 1.0);
}
