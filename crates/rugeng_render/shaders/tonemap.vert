#version 450

layout(location = 0) out vec2 out_uv;

void main() {
    // 3 köşe ile ekranı tamamen kaplayan tam ekran üçgeni (Full-screen triangle)
    // Vertex 0: (-1, -1), UV: (0, 0)
    // Vertex 1: ( 3, -1), UV: (2, 0)
    // Vertex 2: (-1,  3), UV: (0, 2)
    vec2 pos = vec2(float((gl_VertexIndex << 1) & 2), float(gl_VertexIndex & 2));
    out_uv = pos;
    gl_Position = vec4(pos * 2.0 - 1.0, 0.0, 1.0);
}
