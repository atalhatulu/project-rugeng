#version 450

layout(set = 0, binding = 1) uniform sampler2D u_atlas;

layout(location = 0) in vec4 in_color;
layout(location = 1) in vec2 in_uv;
layout(location = 2) flat in uint in_tex_id;

layout(location = 0) out vec4 out_frag_color;

void main() {
    if (in_tex_id > 0u) {
        vec4 tex_color = texture(u_atlas, in_uv);
        out_frag_color = in_color * tex_color;
    } else {
        out_frag_color = in_color;
    }
}
