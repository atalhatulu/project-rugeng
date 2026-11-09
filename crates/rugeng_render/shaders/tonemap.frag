#version 450

layout(set = 0, binding = 0) uniform sampler2D u_hdr_texture;

layout(push_constant) uniform TonemapPushConstants {
    float exposure;
    float gamma;
    float pad0;
    float pad1;
} pc;

layout(location = 0) in vec2 in_uv;
layout(location = 0) out vec4 out_color;

// ACES Filmik Ton Eşleme (Filmic Tonemapping Curve)
vec3 aces_filmic(vec3 x) {
    float a = 2.51;
    float b = 0.03;
    float c = 2.43;
    float d = 0.59;
    float e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);
}

void main() {
    vec4 hdr_color = texture(u_hdr_texture, in_uv);
    float exposure = pc.exposure > 0.0 ? pc.exposure : 1.0;
    float gamma = pc.gamma > 0.0 ? pc.gamma : 2.2;

    // Pozlama (Exposure) uygulama
    vec3 mapped = hdr_color.rgb * exposure;

    // ACES Ton Haritalama
    mapped = aces_filmic(mapped);

    // Gama düzeltmesi (Gamma Correction)
    mapped = pow(mapped, vec3(1.0 / gamma));

    out_color = vec4(mapped, hdr_color.a);
}
