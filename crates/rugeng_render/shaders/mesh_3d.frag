#version 450

layout(set = 0, binding = 0) uniform sampler2D u_texture;
layout(set = 0, binding = 1) uniform sampler2D u_metallic_roughness;
layout(set = 0, binding = 2) uniform sampler2D u_shadow_map;

layout(push_constant) uniform PushConstants {
    mat4 view_proj;
    mat4 model;
    mat4 light_space;         // Işık uzayı View-Projection matrisi (64 bayt)
    vec4 light_dir_ambient;   // xyz: light_dir, w: ambient_intensity
    vec4 light_color_pbr;     // xyz: light_color, w: specular_shininess / pbr_flag
    vec4 camera_pos_flags;    // xyz: camera_pos, w: has_texture (1.0 or 0.0)
    vec4 pbr_params;          // x: roughness (0.04..1.0), y: metallic (0.0..1.0), z: has_mr_texture (1.0 or 0.0), w: shadow_bias
} pc;

layout(location = 0) in vec3 in_frag_pos;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec2 in_uv;
layout(location = 3) in vec4 in_color;
layout(location = 4) in vec4 in_light_space_pos;

layout(location = 0) out vec4 out_color;

const float PI = 3.14159265359;

// Trowbridge-Reitz GGX Normal Dağılım Fonksiyonu (NDF)
float distribution_ggx(vec3 N, vec3 H, float roughness) {
    float a = roughness * roughness;
    float a2 = a * a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;

    float num = a2;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;

    return num / max(denom, 0.000001);
}

// Schlick-GGX Geometri Fonksiyonu
float geometry_schlick_ggx(float NdotV, float roughness) {
    float r = (roughness + 1.0);
    float k = (r * r) / 8.0;

    float num = NdotV;
    float denom = NdotV * (1.0 - k) + k;

    return num / max(denom, 0.000001);
}

// Smith Geometri Fonksiyonu
float geometry_smith(vec3 N, vec3 V, vec3 L, float roughness) {
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    float ggx2 = geometry_schlick_ggx(NdotV, roughness);
    float ggx1 = geometry_schlick_ggx(NdotL, roughness);

    return ggx1 * ggx2;
}

// Fresnel-Schlick Yaklaşımı
vec3 fresnel_schlick(float cos_theta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

// 3x3 PCF (Percentage-Closer Filtering) ile Gölge Katsayısı Hesaplama
// Çıktı: 0.0 (tam gölgede), 1.0 (tam ışıkta)
float calculate_shadow(vec4 light_space_pos, vec3 normal, vec3 light_dir) {
    // Perspektif bölme
    vec3 proj_coords = light_space_pos.xyz / light_space_pos.w;

    // Vulkan NDC: x [-1, 1] -> [0, 1], y [-1, 1] -> [0, 1], z [0, 1]
    proj_coords.xy = proj_coords.xy * 0.5 + 0.5;

    // Işık frustum'ı dışındaysa gölge üretme
    if (proj_coords.z > 1.0 || proj_coords.z < 0.0 ||
        proj_coords.x < 0.0 || proj_coords.x > 1.0 ||
        proj_coords.y < 0.0 || proj_coords.y > 1.0) {
        return 1.0;
    }

    // Eğim duyarlı dinamik bias (Slope-scaled bias)
    float base_bias = pc.pbr_params.w > 0.0 ? pc.pbr_params.w : 0.0025;
    float cos_theta = max(dot(normal, light_dir), 0.0);
    float bias = max(base_bias * (1.0 - cos_theta), base_bias * 0.2);

    float current_depth = proj_coords.z;
    float shadow = 0.0;
    vec2 texel_size = 1.0 / textureSize(u_shadow_map, 0);

    // 3x3 PCF filtreleme
    for (int x = -1; x <= 1; ++x) {
        for (int y = -1; y <= 1; ++y) {
            float pcf_depth = texture(u_shadow_map, proj_coords.xy + vec2(x, y) * texel_size).r;
            shadow += (current_depth - bias > pcf_depth) ? 0.0 : 1.0;
        }
    }
    shadow /= 9.0;

    return shadow;
}

void main() {
    vec3 N = normalize(in_normal);
    vec3 L = normalize(pc.light_dir_ambient.xyz);
    vec3 V = normalize(pc.camera_pos_flags.xyz - in_frag_pos);
    vec3 H = normalize(L + V);

    vec4 albedo_sample = in_color;
    if (pc.camera_pos_flags.w > 0.5) {
        vec4 tex_val = texture(u_texture, in_uv);
        albedo_sample *= tex_val;
    }

    vec3 albedo = albedo_sample.rgb;
    float roughness = clamp(pc.pbr_params.x, 0.04, 1.0);
    float metallic = clamp(pc.pbr_params.y, 0.0, 1.0);

    // glTF 2.0 PBR standardı: G kanalı = Roughness, B kanalı = Metallic
    if (pc.pbr_params.z > 0.5) {
        vec4 mr_sample = texture(u_metallic_roughness, in_uv);
        roughness = clamp(roughness * mr_sample.g, 0.04, 1.0);
        metallic = clamp(metallic * mr_sample.b, 0.0, 1.0);
    }

    // F0 diyalektrikler için varsayılan 0.04, metaller için albedo değeri
    vec3 F0 = vec3(0.04);
    F0 = mix(F0, albedo, metallic);

    // Cook-Torrance BRDF
    float NDF = distribution_ggx(N, H, roughness);
    float G = geometry_smith(N, V, L, roughness);
    vec3 F = fresnel_schlick(max(dot(H, V), 0.0), F0);

    vec3 kS = F;
    vec3 kD = vec3(1.0) - kS;
    kD *= 1.0 - metallic;

    vec3 numerator = NDF * G * F;
    float denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.0001;
    vec3 specular = numerator / denominator;

    float NdotL = max(dot(N, L), 0.0);
    vec3 light_radiance = pc.light_color_pbr.xyz;

    // Gölge çarpanı (Shadow factor 0.0..1.0)
    float shadow = calculate_shadow(in_light_space_pos, N, L);

    // Gelen doğrudan ışık katkısı (Gölge katsayısı ile modüle edilir)
    vec3 Lo = (kD * albedo / PI + specular) * light_radiance * NdotL * shadow;

    // Ortam (Ambient) aydınlatması (Gölgeden etkilenmez)
    float ambient_str = pc.light_dir_ambient.w;
    vec3 ambient = ambient_str * light_radiance * albedo;

    vec3 color = ambient + Lo;

    // Tonemapping ve gama düzeltmesi
    color = color / (color + vec3(1.0));
    color = pow(color, vec3(1.0 / 2.2));

    out_color = vec4(color, albedo_sample.a);
}
