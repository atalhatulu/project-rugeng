#version 450

// 6 Vertex ile procedural quad pozisyonları ve UV'leri
const vec2 POSITIONS[6] = vec2[](
    vec2(0.0, 0.0),
    vec2(1.0, 0.0),
    vec2(1.0, 1.0),
    vec2(1.0, 1.0),
    vec2(0.0, 1.0),
    vec2(0.0, 0.0)
);

const vec2 UVS[6] = vec2[](
    vec2(0.0, 0.0),
    vec2(1.0, 0.0),
    vec2(1.0, 1.0),
    vec2(1.0, 1.0),
    vec2(0.0, 1.0),
    vec2(0.0, 0.0)
);

// 32-Baytlık QuadInstance tanımı (Std430 hizalama ile tam uyumlu)
struct QuadInstance {
    vec2 pos;      // Offset: 0,  Size: 8
    vec2 size;     // Offset: 8,  Size: 8
    uvec2 uv_rect; // Offset: 16, Size: 8 (x: uv_min packed, y: uv_size packed)
    uint color;    // Offset: 24, Size: 4 (RGBA8 paketlenmiş)
    uint tex_id;   // Offset: 28, Size: 4 (0: untextured, 1+: atlas slot)
};

layout(std430, set = 0, binding = 0) readonly buffer InstanceBuffer {
    QuadInstance instances[];
};

layout(push_constant) uniform PushConstants {
    mat4 view_proj;
} pc;

layout(location = 0) out vec4 out_color;
layout(location = 1) out vec2 out_uv;
layout(location = 2) flat out uint out_tex_id;

void main() {
    uint vertex_idx = gl_VertexIndex % 6;
    uint instance_idx = gl_InstanceIndex;

    QuadInstance inst = instances[instance_idx];
    
    // 2D Rotasyon desteği (Quad merkezi etrafında dönüş)
    uint rot_raw = (inst.tex_id >> 16) & 0xFFFFu;
    vec2 centered = (POSITIONS[vertex_idx] - 0.5) * inst.size;
    vec2 rotated = centered;
    if (rot_raw > 0u) {
        float angle = float(rot_raw) * (6.283185307179586 / 65535.0);
        float c = cos(angle);
        float s = sin(angle);
        rotated = vec2(
            centered.x * c - centered.y * s,
            centered.x * s + centered.y * c
        );
    }
    vec2 world_pos = inst.pos + 0.5 * inst.size + rotated;

    gl_Position = pc.view_proj * vec4(world_pos, 0.0, 1.0);

    // Renk: Unpack RGBA8 UNORM
    out_color = unpackUnorm4x8(inst.color);

    // UV Koordinatları: 16-bit UNORM açılımı (u, v)
    vec2 uv_min = unpackUnorm2x16(inst.uv_rect.x);
    vec2 uv_size = unpackUnorm2x16(inst.uv_rect.y);
    out_uv = uv_min + UVS[vertex_idx] * uv_size;

    out_tex_id = inst.tex_id & 0xFFFFu;
}
