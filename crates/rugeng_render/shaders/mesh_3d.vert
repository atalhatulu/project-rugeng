#version 450

layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec2 in_uv;
layout(location = 3) in vec4 in_color;

layout(push_constant) uniform PushConstants {
    mat4 view_proj;
    mat4 model;
    mat4 light_space;       // Işık uzayı View-Projection matrisi (64 bayt)
    vec4 light_dir_ambient; // xyz: light_dir (normalized pointing toward light), w: ambient_intensity
    vec4 light_color;       // xyz: light_color, w: specular_shininess
    vec4 camera_pos;        // xyz: camera_pos, w: unused
    vec4 pbr_params;        // x: roughness, y: metallic, z: has_mr_texture, w: shadow_bias
} pc;

layout(location = 0) out vec3 out_frag_pos;
layout(location = 1) out vec3 out_normal;
layout(location = 2) out vec2 out_uv;
layout(location = 3) out vec4 out_color;
layout(location = 4) out vec4 out_light_space_pos;

void main() {
    vec4 world_pos = pc.model * vec4(in_position, 1.0);
    out_frag_pos = world_pos.xyz;
    
    // Normal transform
    out_normal = normalize(mat3(pc.model) * in_normal);
    out_uv = in_uv;
    out_color = in_color;
    out_light_space_pos = pc.light_space * world_pos;

    gl_Position = pc.view_proj * world_pos;
}
