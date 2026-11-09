//! 3D Mesh Veri Yapısı, Prosedürel Geometriler (Küp, Küre, Düzlem) ve OBJ Yükleyici.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use rugeng_core::{BoundingSphere, AABB3D};

/// 3D Köşe Verisi (Vertex3D: 72 bayt)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Vertex3D {
    pub position: [f32; 3],      // 12 bayt
    pub normal: [f32; 3],        // 12 bayt
    pub uv: [f32; 2],            // 8 bayt
    pub color: [f32; 4],         // 16 bayt (RGBA f32)
    pub joint_indices: [u16; 4], // 8 bayt
    pub joint_weights: [f32; 4], // 16 bayt
}

impl Vertex3D {
    pub const fn new(position: [f32; 3], normal: [f32; 3], uv: [f32; 2], color: [f32; 4]) -> Self {
        Self {
            position,
            normal,
            uv,
            color,
            joint_indices: [0; 4],
            joint_weights: [1.0, 0.0, 0.0, 0.0],
        }
    }

    pub const fn with_skinning(
        position: [f32; 3],
        normal: [f32; 3],
        uv: [f32; 2],
        color: [f32; 4],
        joint_indices: [u16; 4],
        joint_weights: [f32; 4],
    ) -> Self {
        Self {
            position,
            normal,
            uv,
            color,
            joint_indices,
            joint_weights,
        }
    }
}

/// CPU tarafı 3D Mesh Verisi
#[derive(Clone, Debug, Default)]
pub struct Mesh3D {
    pub vertices: Vec<Vertex3D>,
    pub indices: Vec<u32>,
    pub aabb: AABB3D,
    pub bounding_sphere: BoundingSphere,
    pub material_index: Option<usize>,
    pub pbr_factors: Option<[f32; 4]>, // [roughness, metallic, _, _]
}

impl Mesh3D {
    pub fn new(vertices: Vec<Vertex3D>, indices: Vec<u32>) -> Self {
        let mut mesh = Self {
            vertices,
            indices,
            aabb: AABB3D::default(),
            bounding_sphere: BoundingSphere::default(),
            material_index: None,
            pbr_factors: None,
        };
        mesh.recompute_bounds();
        mesh
    }

    pub fn with_material(
        mut self,
        material_index: Option<usize>,
        roughness: f32,
        metallic: f32,
    ) -> Self {
        self.material_index = material_index;
        self.pbr_factors = Some([roughness, metallic, 0.0, 0.0]);
        self
    }

    /// Sınırlayıcı kutu (AABB) ve sınırlayıcı küreyi yeniden hesaplar.
    pub fn recompute_bounds(&mut self) {
        if self.vertices.is_empty() {
            self.aabb = AABB3D::default();
            self.bounding_sphere = BoundingSphere::default();
            return;
        }

        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        for v in &self.vertices {
            let p = Vec3::from(v.position);
            min = min.min(p);
            max = max.max(p);
        }

        let aabb = AABB3D::new(min, max);
        let sphere = BoundingSphere::from_aabb(&aabb);
        self.aabb = aabb;
        self.bounding_sphere = sphere;
    }

    /// 1x1x1 birim küp üretir (merkez 0,0,0).
    pub fn create_cube(size: f32, color: [f32; 4]) -> Self {
        let h = size * 0.5;
        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);

        // 6 yüz: Ön, Arka, Üst, Alt, Sağ, Sol
        let faces = [
            // +Z (Ön)
            (
                [0.0, 0.0, 1.0],
                [
                    ([-h, -h, h], [0.0, 1.0]),
                    ([h, -h, h], [1.0, 1.0]),
                    ([h, h, h], [1.0, 0.0]),
                    ([-h, h, h], [0.0, 0.0]),
                ],
            ),
            // -Z (Arka)
            (
                [0.0, 0.0, -1.0],
                [
                    ([h, -h, -h], [0.0, 1.0]),
                    ([-h, -h, -h], [1.0, 1.0]),
                    ([-h, h, -h], [1.0, 0.0]),
                    ([h, h, -h], [0.0, 0.0]),
                ],
            ),
            // +Y (Üst)
            (
                [0.0, 1.0, 0.0],
                [
                    ([-h, h, h], [0.0, 1.0]),
                    ([h, h, h], [1.0, 1.0]),
                    ([h, h, -h], [1.0, 0.0]),
                    ([-h, h, -h], [0.0, 0.0]),
                ],
            ),
            // -Y (Alt)
            (
                [0.0, -1.0, 0.0],
                [
                    ([-h, -h, -h], [0.0, 1.0]),
                    ([h, -h, -h], [1.0, 1.0]),
                    ([h, -h, h], [1.0, 0.0]),
                    ([-h, -h, h], [0.0, 0.0]),
                ],
            ),
            // +X (Sağ)
            (
                [1.0, 0.0, 0.0],
                [
                    ([h, -h, h], [0.0, 1.0]),
                    ([h, -h, -h], [1.0, 1.0]),
                    ([h, h, -h], [1.0, 0.0]),
                    ([h, h, h], [0.0, 0.0]),
                ],
            ),
            // -X (Sol)
            (
                [-1.0, 0.0, 0.0],
                [
                    ([-h, -h, -h], [0.0, 1.0]),
                    ([-h, -h, h], [1.0, 1.0]),
                    ([-h, h, h], [1.0, 0.0]),
                    ([-h, h, -h], [0.0, 0.0]),
                ],
            ),
        ];

        for (normal, quad) in faces {
            let base_idx = vertices.len() as u32;
            for (pos, uv) in quad {
                vertices.push(Vertex3D::new(pos, normal, uv, color));
            }
            // CCW sargı düzeni (Dışarıdan bakıldığında ön yüzey): (0, 2, 1) ve (0, 3, 2)
            indices.extend_from_slice(&[
                base_idx,
                base_idx + 2,
                base_idx + 1,
                base_idx,
                base_idx + 3,
                base_idx + 2,
            ]);
        }

        Self::new(vertices, indices)
    }

    /// UV Küre (Sphere) üretir.
    pub fn create_sphere(radius: f32, sectors: u32, stacks: u32, color: [f32; 4]) -> Self {
        let sectors = sectors.max(3);
        let stacks = stacks.max(2);
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let pi = std::f32::consts::PI;
        let tau = std::f32::consts::TAU;

        for i in 0..=stacks {
            let stack_angle = pi / 2.0 - (i as f32) * pi / (stacks as f32);
            let xy = radius * stack_angle.cos();
            let z = radius * stack_angle.sin();

            for j in 0..=sectors {
                let sector_angle = (j as f32) * tau / (sectors as f32);
                let x = xy * sector_angle.cos();
                let y = xy * sector_angle.sin();

                let pos = [x, z, y]; // Y yukarı olacak şekilde z ile y yer değiştirildi
                let normal = Vec3::new(x, z, y).normalize().to_array();
                let uv = [(j as f32) / (sectors as f32), (i as f32) / (stacks as f32)];

                vertices.push(Vertex3D::new(pos, normal, uv, color));
            }
        }

        for i in 0..stacks {
            let base1 = i * (sectors + 1);
            let base2 = base1 + sectors + 1;

            for j in 0..sectors {
                let k1 = base1 + j;
                let k2 = base2 + j;

                if i != 0 {
                    indices.push(k1);
                    indices.push(k2);
                    indices.push(k1 + 1);
                }

                if i != (stacks - 1) {
                    indices.push(k1 + 1);
                    indices.push(k2);
                    indices.push(k2 + 1);
                }
            }
        }

        Self::new(vertices, indices)
    }

    /// XZ Düzlemi (Plane / Grid) üretir (Normal +Y).
    pub fn create_plane(width: f32, depth: f32, color: [f32; 4]) -> Self {
        let hw = width * 0.5;
        let hd = depth * 0.5;
        let normal = [0.0, 1.0, 0.0];

        let vertices = vec![
            Vertex3D::new([-hw, 0.0, -hd], normal, [0.0, 0.0], color),
            Vertex3D::new([hw, 0.0, -hd], normal, [1.0, 0.0], color),
            Vertex3D::new([hw, 0.0, hd], normal, [1.0, 1.0], color),
            Vertex3D::new([-hw, 0.0, hd], normal, [0.0, 1.0], color),
        ];

        // Çift taraflı (Hem üstten hem alttan görünür)
        let indices = vec![
            0, 1, 2, 0, 2, 3, // Üstten
            0, 2, 1, 0, 3, 2, // Alttan
        ];
        Self::new(vertices, indices)
    }

    /// 3D Sprite / Billboard Quad üretir (XY Düzleminde dikey, Normal +Z).
    pub fn create_sprite_3d(width: f32, height: f32, color: [f32; 4]) -> Self {
        let hw = width * 0.5;
        let hh = height * 0.5;
        let normal = [0.0, 0.0, 1.0];

        let vertices = vec![
            Vertex3D::new([-hw, -hh, 0.0], normal, [0.0, 1.0], color),
            Vertex3D::new([hw, -hh, 0.0], normal, [1.0, 1.0], color),
            Vertex3D::new([hw, hh, 0.0], normal, [1.0, 0.0], color),
            Vertex3D::new([-hw, hh, 0.0], normal, [0.0, 0.0], color),
        ];

        // Çift taraflı (Ön ve arkadan görünür)
        let indices = vec![
            0, 1, 2, 0, 2, 3, // Ön
            0, 2, 1, 0, 3, 2, // Arka
        ];
        Self::new(vertices, indices)
    }

    /// Silindir (Cylinder) üretir (Y ekseni boyunca).
    pub fn create_cylinder(radius: f32, height: f32, segments: u32, color: [f32; 4]) -> Self {
        let segments = segments.max(3);
        let hh = height * 0.5;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let tau = std::f32::consts::TAU;

        // Yan yüzeyler
        for i in 0..=segments {
            let angle = (i as f32) * tau / (segments as f32);
            let cos = angle.cos();
            let sin = angle.sin();
            let x = radius * cos;
            let z = radius * sin;
            let u = (i as f32) / (segments as f32);

            let normal = [cos, 0.0, sin];
            // Alt köşe
            vertices.push(Vertex3D::new([x, -hh, z], normal, [u, 1.0], color));
            // Üst köşe
            vertices.push(Vertex3D::new([x, hh, z], normal, [u, 0.0], color));
        }

        for i in 0..segments {
            let base = i * 2;
            indices.extend_from_slice(&[base, base + 2, base + 1, base + 1, base + 2, base + 3]);
        }

        // Üst kapak
        let top_center_idx = vertices.len() as u32;
        vertices.push(Vertex3D::new(
            [0.0, hh, 0.0],
            [0.0, 1.0, 0.0],
            [0.5, 0.5],
            color,
        ));
        for i in 0..=segments {
            let angle = (i as f32) * tau / (segments as f32);
            let x = radius * angle.cos();
            let z = radius * angle.sin();
            vertices.push(Vertex3D::new(
                [x, hh, z],
                [0.0, 1.0, 0.0],
                [0.5 + 0.5 * angle.cos(), 0.5 + 0.5 * angle.sin()],
                color,
            ));
        }
        for i in 0..segments {
            let curr = top_center_idx + 1 + i;
            indices.extend_from_slice(&[top_center_idx, curr, curr + 1]);
        }

        // Alt kapak
        let bot_center_idx = vertices.len() as u32;
        vertices.push(Vertex3D::new(
            [0.0, -hh, 0.0],
            [0.0, -1.0, 0.0],
            [0.5, 0.5],
            color,
        ));
        for i in 0..=segments {
            let angle = (i as f32) * tau / (segments as f32);
            let x = radius * angle.cos();
            let z = radius * angle.sin();
            vertices.push(Vertex3D::new(
                [x, -hh, z],
                [0.0, -1.0, 0.0],
                [0.5 + 0.5 * angle.cos(), 0.5 + 0.5 * angle.sin()],
                color,
            ));
        }
        for i in 0..segments {
            let curr = bot_center_idx + 1 + i;
            indices.extend_from_slice(&[bot_center_idx, curr + 1, curr]);
        }

        Self::new(vertices, indices)
    }

    /// Koni (Cone) üretir (Y ekseni boyunca, tepe +Y).
    pub fn create_cone(radius: f32, height: f32, segments: u32, color: [f32; 4]) -> Self {
        let segments = segments.max(3);
        let hh = height * 0.5;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let tau = std::f32::consts::TAU;

        // Tepe noktası
        let apex_idx = 0u32;
        vertices.push(Vertex3D::new(
            [0.0, hh, 0.0],
            [0.0, 1.0, 0.0],
            [0.5, 0.0],
            color,
        ));

        // Taban çemberi
        for i in 0..=segments {
            let angle = (i as f32) * tau / (segments as f32);
            let cos = angle.cos();
            let sin = angle.sin();
            let x = radius * cos;
            let z = radius * sin;
            let normal = Vec3::new(cos, radius / height, sin).normalize().to_array();
            vertices.push(Vertex3D::new(
                [x, -hh, z],
                normal,
                [(i as f32) / (segments as f32), 1.0],
                color,
            ));
        }

        for i in 0..segments {
            let curr = 1 + i;
            indices.extend_from_slice(&[apex_idx, curr, curr + 1]);
        }

        // Taban kapağı
        let bot_center_idx = vertices.len() as u32;
        vertices.push(Vertex3D::new(
            [0.0, -hh, 0.0],
            [0.0, -1.0, 0.0],
            [0.5, 0.5],
            color,
        ));
        for i in 0..=segments {
            let angle = (i as f32) * tau / (segments as f32);
            let x = radius * angle.cos();
            let z = radius * angle.sin();
            vertices.push(Vertex3D::new(
                [x, -hh, z],
                [0.0, -1.0, 0.0],
                [0.5 + 0.5 * angle.cos(), 0.5 + 0.5 * angle.sin()],
                color,
            ));
        }
        for i in 0..segments {
            let curr = bot_center_idx + 1 + i;
            indices.extend_from_slice(&[bot_center_idx, curr + 1, curr]);
        }

        Self::new(vertices, indices)
    }

    /// Dörtgen Tabanlı Piramit / Üçgen Prizma (Pyramid) üretir.
    pub fn create_pyramid(base_size: f32, height: f32, color: [f32; 4]) -> Self {
        let h = base_size * 0.5;
        let hh = height * 0.5;
        let apex = [0.0, hh, 0.0];

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let add_tri = |v: &mut Vec<Vertex3D>,
                       ind: &mut Vec<u32>,
                       p1: [f32; 3],
                       p2: [f32; 3],
                       p3: [f32; 3]| {
            let edge1 = Vec3::from(p2) - Vec3::from(p1);
            let edge2 = Vec3::from(p3) - Vec3::from(p1);
            let n = edge1.cross(edge2).normalize().to_array();
            let base = v.len() as u32;
            v.push(Vertex3D::new(p1, n, [0.5, 0.0], color));
            v.push(Vertex3D::new(p2, n, [0.0, 1.0], color));
            v.push(Vertex3D::new(p3, n, [1.0, 1.0], color));
            ind.extend_from_slice(&[base, base + 1, base + 2]);
        };

        // 4 Yan yüzey
        add_tri(&mut vertices, &mut indices, apex, [-h, -hh, h], [h, -hh, h]); // Ön
        add_tri(&mut vertices, &mut indices, apex, [h, -hh, h], [h, -hh, -h]); // Sağ
        add_tri(
            &mut vertices,
            &mut indices,
            apex,
            [h, -hh, -h],
            [-h, -hh, -h],
        ); // Arka
        add_tri(
            &mut vertices,
            &mut indices,
            apex,
            [-h, -hh, -h],
            [-h, -hh, h],
        ); // Sol

        // Taban kapağı
        let bot_normal = [0.0, -1.0, 0.0];
        let base_idx = vertices.len() as u32;
        vertices.push(Vertex3D::new([-h, -hh, -h], bot_normal, [0.0, 0.0], color));
        vertices.push(Vertex3D::new([h, -hh, -h], bot_normal, [1.0, 0.0], color));
        vertices.push(Vertex3D::new([h, -hh, h], bot_normal, [1.0, 1.0], color));
        vertices.push(Vertex3D::new([-h, -hh, h], bot_normal, [0.0, 1.0], color));
        indices.extend_from_slice(&[
            base_idx,
            base_idx + 2,
            base_idx + 1,
            base_idx,
            base_idx + 3,
            base_idx + 2,
        ]);

        Self::new(vertices, indices)
    }

    /// Kapsül (Capsule) üretir (Y ekseni boyunca silindir + iki yarım küre).
    pub fn create_capsule(
        radius: f32,
        cylinder_height: f32,
        sectors: u32,
        stacks: u32,
        color: [f32; 4],
    ) -> Self {
        let sectors = sectors.max(4);
        let stacks = stacks.max(2);
        let hh = cylinder_height * 0.5;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let pi = std::f32::consts::PI;
        let tau = std::f32::consts::TAU;

        // Üst yarım küre + Silindir gövde + Alt yarım küre
        for i in 0..=stacks {
            let lat = (i as f32 / stacks as f32) * pi;
            let y_offset = if lat < pi * 0.5 { hh } else { -hh };
            let y = radius * lat.cos() + y_offset;
            let r_ring = radius * lat.sin();

            for j in 0..=sectors {
                let lng = (j as f32 / sectors as f32) * tau;
                let x = r_ring * lng.cos();
                let z = r_ring * lng.sin();
                let normal = Vec3::new(x, radius * lat.cos(), z).normalize().to_array();
                let uv = [j as f32 / sectors as f32, i as f32 / stacks as f32];
                vertices.push(Vertex3D::new([x, y, z], normal, uv, color));
            }
        }

        for i in 0..stacks {
            let base1 = i * (sectors + 1);
            let base2 = (i + 1) * (sectors + 1);
            for j in 0..sectors {
                let k1 = base1 + j;
                let k2 = base2 + j;
                indices.extend_from_slice(&[k1, k2, k1 + 1, k1 + 1, k2, k2 + 1]);
            }
        }

        Self::new(vertices, indices)
    }

    /// 3D Viewport için ızgara çizgileri ve X/Y/Z eksen çizgilerini (küçük şeritler halinde) üretir.
    pub fn create_grid_and_axes(size: f32, step: f32) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let half = size * 0.5;
        let line_w = 0.02;
        let normal = [0.0, 1.0, 0.0];
        let grid_col = [0.25, 0.28, 0.35, 0.8];

        let mut add_quad =
            |p1: [f32; 3], p2: [f32; 3], p3: [f32; 3], p4: [f32; 3], col: [f32; 4]| {
                let base = vertices.len() as u32;
                vertices.push(Vertex3D::new(p1, normal, [0.0, 0.0], col));
                vertices.push(Vertex3D::new(p2, normal, [1.0, 0.0], col));
                vertices.push(Vertex3D::new(p3, normal, [1.0, 1.0], col));
                vertices.push(Vertex3D::new(p4, normal, [0.0, 1.0], col));
                // Çift taraflı görünüm: Hem üstten (CW) hem alttan (CCW) culling'e takılmadan görünür
                indices.extend_from_slice(&[
                    base,
                    base + 1,
                    base + 2,
                    base,
                    base + 2,
                    base + 3, // Üstten görünüm
                    base,
                    base + 2,
                    base + 1,
                    base,
                    base + 3,
                    base + 2, // Alttan görünüm
                ]);
            };

        // Izgara çizgileri
        let mut x = -half;
        while x <= half {
            if x.abs() > 0.001 {
                add_quad(
                    [x - line_w, 0.0, -half],
                    [x + line_w, 0.0, -half],
                    [x + line_w, 0.0, half],
                    [x - line_w, 0.0, half],
                    grid_col,
                );
            }
            x += step;
        }

        let mut z = -half;
        while z <= half {
            if z.abs() > 0.001 {
                add_quad(
                    [-half, 0.0, z - line_w],
                    [half, 0.0, z - line_w],
                    [half, 0.0, z + line_w],
                    [-half, 0.0, z + line_w],
                    grid_col,
                );
            }
            z += step;
        }

        // X Ekseni (Kırmızı)
        let axis_w = 0.05;
        let axis_len = half * 0.8;
        add_quad(
            [0.0, 0.002, -axis_w],
            [axis_len, 0.002, -axis_w],
            [axis_len, 0.002, axis_w],
            [0.0, 0.002, axis_w],
            [0.95, 0.2, 0.2, 1.0],
        );

        // Z Ekseni (Mavi)
        add_quad(
            [-axis_w, 0.002, 0.0],
            [axis_w, 0.002, 0.0],
            [axis_w, 0.002, axis_len],
            [-axis_w, 0.002, axis_len],
            [0.2, 0.4, 0.95, 1.0],
        );

        // Y Ekseni (Yeşil - Dikey şerit, her iki yönden de görünür)
        let base = vertices.len() as u32;
        let y_normal = [1.0, 0.0, 0.0];
        let y_col = [0.2, 0.9, 0.2, 1.0];
        vertices.push(Vertex3D::new(
            [-axis_w, 0.0, 0.0],
            y_normal,
            [0.0, 0.0],
            y_col,
        ));
        vertices.push(Vertex3D::new(
            [axis_w, 0.0, 0.0],
            y_normal,
            [1.0, 0.0],
            y_col,
        ));
        vertices.push(Vertex3D::new(
            [axis_w, axis_len, 0.0],
            y_normal,
            [1.0, 1.0],
            y_col,
        ));
        vertices.push(Vertex3D::new(
            [-axis_w, axis_len, 0.0],
            y_normal,
            [0.0, 1.0],
            y_col,
        ));
        indices.extend_from_slice(&[
            base,
            base + 1,
            base + 2,
            base,
            base + 2,
            base + 3,
            base,
            base + 2,
            base + 1,
            base,
            base + 3,
            base + 2,
        ]);

        Self::new(vertices, indices)
    }

    /// Basit Wavefront OBJ metin verisini ayrıştırır.
    pub fn parse_obj(obj_content: &str, default_color: [f32; 4]) -> Result<Self, String> {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for line in obj_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut parts = line.split_whitespace();
            let prefix = match parts.next() {
                Some(p) => p,
                None => continue,
            };

            match prefix {
                "v" => {
                    let x = parts
                        .next()
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(0.0);
                    let y = parts
                        .next()
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(0.0);
                    let z = parts
                        .next()
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(0.0);
                    positions.push([x, y, z]);
                }
                "vn" => {
                    let x = parts
                        .next()
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(0.0);
                    let y = parts
                        .next()
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(0.0);
                    let z = parts
                        .next()
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(0.0);
                    normals.push([x, y, z]);
                }
                "vt" => {
                    let u = parts
                        .next()
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(0.0);
                    let v = parts
                        .next()
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(0.0);
                    uvs.push([u, v]);
                }
                "f" => {
                    let face_tokens: Vec<&str> = parts.collect();
                    if face_tokens.len() < 3 {
                        continue;
                    }

                    // Fan triangulation
                    for i in 1..(face_tokens.len() - 1) {
                        let triangle = [face_tokens[0], face_tokens[i], face_tokens[i + 1]];
                        for token in triangle {
                            let mut sub = token.split('/');
                            let v_idx = sub
                                .next()
                                .and_then(|s| s.parse::<usize>().ok())
                                .unwrap_or(1)
                                .saturating_sub(1);
                            let vt_idx = sub
                                .next()
                                .and_then(|s| s.parse::<usize>().ok())
                                .map(|idx| idx.saturating_sub(1));
                            let vn_idx = sub
                                .next()
                                .and_then(|s| s.parse::<usize>().ok())
                                .map(|idx| idx.saturating_sub(1));

                            let pos = positions.get(v_idx).cloned().unwrap_or([0.0, 0.0, 0.0]);
                            let uv = vt_idx
                                .and_then(|idx| uvs.get(idx).cloned())
                                .unwrap_or([0.0, 0.0]);
                            let norm = vn_idx
                                .and_then(|idx| normals.get(idx).cloned())
                                .unwrap_or([0.0, 1.0, 0.0]);

                            let vertex = Vertex3D::new(pos, norm, uv, default_color);
                            indices.push(vertices.len() as u32);
                            vertices.push(vertex);
                        }
                    }
                }
                _ => {}
            }
        }

        if vertices.is_empty() {
            return Err("OBJ dosyasinda gecerli tepe noktasi bulunamadi".to_string());
        }

        Ok(Self::new(vertices, indices))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube_generation() {
        let cube = Mesh3D::create_cube(2.0, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(cube.vertices.len(), 24);
        assert_eq!(cube.indices.len(), 36);
        assert!((cube.aabb.min.x - (-1.0)).abs() < 1e-4);
        assert!((cube.aabb.max.x - 1.0).abs() < 1e-4);
        assert!(cube.bounding_sphere.radius > 1.0);
    }

    #[test]
    fn test_sphere_generation() {
        let sphere = Mesh3D::create_sphere(1.5, 8, 6, [0.5, 0.5, 0.5, 1.0]);
        assert!(!sphere.vertices.is_empty());
        assert!(!sphere.indices.is_empty());
        assert!((sphere.aabb.extents().x - 3.0).abs() < 0.2);
    }

    #[test]
    fn test_obj_parser() {
        let obj_data = r#"
        # Basit Ucgen
        v 0.0 0.0 0.0
        v 1.0 0.0 0.0
        v 0.0 1.0 0.0
        vn 0.0 0.0 1.0
        vt 0.0 0.0
        vt 1.0 0.0
        vt 0.0 1.0
        f 1/1/1 2/2/1 3/3/1
        "#;
        let mesh = Mesh3D::parse_obj(obj_data, [1.0, 0.0, 0.0, 1.0]).expect("OBJ parse hatasi");
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices.len(), 3);
        assert_eq!(mesh.vertices[0].normal, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_cylinder_cone_pyramid_capsule_sprite_generation() {
        let cyl = Mesh3D::create_cylinder(1.0, 2.0, 12, [1.0, 1.0, 1.0, 1.0]);
        assert!(!cyl.vertices.is_empty());
        assert!(!cyl.indices.is_empty());
        assert!((cyl.aabb.extents().y - 2.0).abs() < 1e-4);

        let cone = Mesh3D::create_cone(1.0, 2.0, 12, [1.0, 0.0, 0.0, 1.0]);
        assert!(!cone.vertices.is_empty());
        assert!(!cone.indices.is_empty());
        assert!((cone.aabb.extents().y - 2.0).abs() < 1e-4);

        let pyr = Mesh3D::create_pyramid(2.0, 2.0, [0.0, 1.0, 0.0, 1.0]);
        assert!(!pyr.vertices.is_empty());
        assert_eq!(pyr.indices.len(), 18); // 4 sides * 3 + base 6 = 18

        let cap = Mesh3D::create_capsule(1.0, 2.0, 8, 4, [0.0, 0.0, 1.0, 1.0]);
        assert!(!cap.vertices.is_empty());
        assert!(!cap.indices.is_empty());

        let spr = Mesh3D::create_sprite_3d(2.0, 3.0, [1.0, 1.0, 0.0, 1.0]);
        assert_eq!(spr.vertices.len(), 4);
        assert_eq!(spr.indices.len(), 12); // Çift taraflı (Ön 6 + Arka 6 = 12)
        assert!((spr.aabb.extents().x - 2.0).abs() < 1e-4);
        assert!((spr.aabb.extents().y - 3.0).abs() < 1e-4);
    }
}
