#version 450

layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec2 in_uv;
layout(location = 3) in vec4 in_color;
layout(location = 4) in uvec4 in_joint_indices;
layout(location = 5) in vec4 in_joint_weights;

layout(push_constant) uniform PushConstants {
    mat4 view_proj;
    mat4 model;
    mat4 light_space;       // Işık uzayı View-Projection matrisi (64 bayt)
    vec4 light_dir_ambient; // xyz: light_dir (normalized pointing toward light), w: ambient_intensity
    vec4 light_color;       // xyz: light_color, w: specular_shininess
    vec4 camera_pos;        // xyz: camera_pos, w: unused
    vec4 pbr_params;        // x: roughness, y: metallic, z: has_mr_texture, w: unused
    uint bone_count;        // ciltleme kemik sayısı
    uint light_count;       // noktasal ışık sayısı
    uint pad0;
    uint pad1;
} pc;

layout(std430, set = 0, binding = 3) readonly buffer JointMatrices {
    mat4 joints[];
};

layout(location = 0) out vec3 out_frag_pos;
layout(location = 1) out vec3 out_normal;
layout(location = 2) out vec2 out_uv;
layout(location = 3) out vec4 out_color;
layout(location = 4) out vec4 out_light_space_pos;

void main() {
    uint bone_count = pc.bone_count;
    vec4 local_pos = vec4(in_position, 1.0);
    vec3 local_normal = in_normal;

    if (bone_count > 0u) {
        mat4 skin_mat = in_joint_weights.x * joints[in_joint_indices.x]
                      + in_joint_weights.y * joints[in_joint_indices.y]
                      + in_joint_weights.z * joints[in_joint_indices.z]
                      + in_joint_weights.w * joints[in_joint_indices.w];
        local_pos = skin_mat * vec4(in_position, 1.0);
        local_normal = mat3(skin_mat) * in_normal;
    }

    vec4 world_pos = pc.model * local_pos;
    out_frag_pos = world_pos.xyz;
    
    // Normal transform
    out_normal = normalize(mat3(pc.model) * local_normal);
    out_uv = in_uv;
    out_color = in_color;
    out_light_space_pos = pc.light_space * world_pos;

    gl_Position = pc.view_proj * world_pos;
}
