//! Vektörel / Prosedürel İkon Çizim Motoru (QuadInstance tabanlı).
//!
//! Harici atlas veya texture yüklemesi gerektirmeden, tek draw call instancing
//! boru hattında yüksek performanslı ve çözünürlükten bağımsız vektörel ikonlar çizer.

use crate::pipeline_2d::QuadInstance;

/// Oynat ikonu (▶ - sağa bakan üçgen).
pub fn draw_icon_play(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    let w = (size * 0.85).round().max(4.0);
    let h = size;
    let steps = (w as usize).max(4);
    for i in 0..steps {
        let t = i as f32 / steps as f32;
        let col_h = (h * (1.0 - t)).max(1.0);
        let col_x = x + i as f32;
        let col_y = y + (h - col_h) * 0.5;
        instances.push(QuadInstance::new([col_x, col_y], [1.0, col_h], color, 0));
    }
}

/// Duraklat ikonu (⏸ - iki dikey çubuk).
pub fn draw_icon_pause(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    let bar_w = (size * 0.28).round().max(2.0);
    let gap = (size - 2.0 * bar_w).max(2.0);
    instances.push(QuadInstance::new([x, y], [bar_w, size], color, 0));
    instances.push(QuadInstance::new(
        [x + bar_w + gap, y],
        [bar_w, size],
        color,
        0,
    ));
}

/// Durdur ikonu (⏹ - kare).
pub fn draw_icon_stop(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    instances.push(QuadInstance::new([x, y], [size, size], color, 0));
}

/// Adım adım ilerlet ikonu (⏭ - üçgen ve dikey çubuk).
pub fn draw_icon_step(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    let bar_w = (size * 0.22).round().max(2.0);
    let tri_w = (size - bar_w - 2.0).max(4.0);
    let steps = (tri_w as usize).max(3);
    for i in 0..steps {
        let t = i as f32 / steps as f32;
        let col_h = (size * (1.0 - t)).max(1.0);
        let col_x = x + i as f32;
        let col_y = y + (size - col_h) * 0.5;
        instances.push(QuadInstance::new([col_x, col_y], [1.0, col_h], color, 0));
    }
    instances.push(QuadInstance::new(
        [x + size - bar_w, y],
        [bar_w, size],
        color,
        0,
    ));
}

/// Seçim ikonu (↖ - imleç oku).
pub fn draw_icon_select(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    let h = size;
    let steps = (h as usize).max(4);
    for i in 0..steps {
        let t = i as f32 / steps as f32;
        let w = if t < 0.65 {
            (size * t * 0.85).max(1.5)
        } else {
            (size * (1.0 - t) * 0.5).max(1.5)
        };
        let row_y = y + i as f32;
        instances.push(QuadInstance::new([x, row_y], [w, 1.0], color, 0));
    }
    let stem_w = (size * 0.25).max(1.5);
    let stem_h = (size * 0.42).max(2.0);
    instances.push(QuadInstance::new(
        [x + size * 0.25, y + size * 0.5],
        [stem_w, stem_h],
        color,
        0,
    ));
}

/// Taşıma ikonu (✥ - 4 yönlü eksen oku).
pub fn draw_icon_move(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    let mid_x = x + size * 0.5;
    let mid_y = y + size * 0.5;
    let thick = 2.0;
    // Ana eksen çizgileri
    instances.push(QuadInstance::new(
        [x + 2.0, mid_y - thick * 0.5],
        [size - 4.0, thick],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [mid_x - thick * 0.5, y + 2.0],
        [thick, size - 4.0],
        color,
        0,
    ));
    // Ok uçları
    instances.push(QuadInstance::new([x, mid_y - 2.5], [1.5, 5.0], color, 0));
    instances.push(QuadInstance::new(
        [x + 1.5, mid_y - 1.5],
        [1.0, 3.0],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [x + size - 1.5, mid_y - 2.5],
        [1.5, 5.0],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [x + size - 2.5, mid_y - 1.5],
        [1.0, 3.0],
        color,
        0,
    ));
    instances.push(QuadInstance::new([mid_x - 2.5, y], [5.0, 1.5], color, 0));
    instances.push(QuadInstance::new(
        [mid_x - 1.5, y + 1.5],
        [3.0, 1.0],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [mid_x - 2.5, y + size - 1.5],
        [5.0, 1.5],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [mid_x - 1.5, y + size - 2.5],
        [3.0, 1.0],
        color,
        0,
    ));
}

/// Döndürme ikonu (↻ - dairesel yay ve ok).
pub fn draw_icon_rotate(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    let pad = (size * 0.15).round();
    let d = size - pad * 2.0;
    let t = 1.5;
    instances.push(QuadInstance::new(
        [x + pad + 2.0, y + pad],
        [d - 4.0, t],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [x + size - pad - t, y + pad + 2.0],
        [t, d - 4.0],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [x + pad + 2.0, y + size - pad - t],
        [d - 4.0, t],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [x + pad, y + pad + 2.0],
        [t, d - 4.0],
        color,
        0,
    ));
    // Ok ucu
    instances.push(QuadInstance::new(
        [x + size - pad - 4.0, y + pad - 2.0],
        [4.0, 1.5],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [x + size - pad + 1.0, y + pad - 2.0],
        [1.5, 4.0],
        color,
        0,
    ));
}

/// Boyutlandırma / Ölçekleme ikonu (iç içe iki kare kutu).
pub fn draw_icon_scale(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    let sm_sz = (size * 0.4).round();
    let lg_sz = (size * 0.55).round();
    let t = 1.5;
    // Küçük sol alt kutu
    instances.push(QuadInstance::new(
        [x, y + size - sm_sz],
        [sm_sz, t],
        color,
        0,
    ));
    instances.push(QuadInstance::new([x, y + size - t], [sm_sz, t], color, 0));
    instances.push(QuadInstance::new(
        [x, y + size - sm_sz],
        [t, sm_sz],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [x + sm_sz - t, y + size - sm_sz],
        [t, sm_sz],
        color,
        0,
    ));
    // Büyük sağ üst kutu
    instances.push(QuadInstance::new(
        [x + size - lg_sz, y],
        [lg_sz, t],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [x + size - lg_sz, y + lg_sz - t],
        [lg_sz, t],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [x + size - lg_sz, y],
        [t, lg_sz],
        color,
        0,
    ));
    instances.push(QuadInstance::new([x + size - t, y], [t, lg_sz], color, 0));
}

/// Fırça ikonu (🖌 - eğik boya fırçası).
pub fn draw_icon_brush(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    let steps = (size as usize).max(6);
    for i in 0..steps {
        let t = i as f32 / steps as f32;
        let px = x + size * (1.0 - t);
        let py = y + size * t;
        let w = if t > 0.65 {
            (size * 0.35).max(3.0)
        } else {
            (size * 0.2).max(2.0)
        };
        instances.push(QuadInstance::new([px - w * 0.5, py], [w, 1.2], color, 0));
    }
    // Uç damlası
    instances.push(QuadInstance::new(
        [x + 1.0, y + size - 2.5],
        [2.5, 2.5],
        color,
        0,
    ));
}

/// Silgi ikonu (eğik çift renkli silgi bloğu).
pub fn draw_icon_eraser(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    let ew = (size * 0.85).round();
    let eh = (size * 0.6).round();
    let ey = y + (size - eh) * 0.5;
    let ex = x + (size - ew) * 0.5;
    // Silgi gövdesi
    instances.push(QuadInstance::new(
        [ex, ey],
        [ew, eh],
        [
            color[0],
            color[1],
            color[2],
            (color[3] as u16 * 180 / 255) as u8,
        ],
        0,
    ));
    // Uç kauçuk kısmı
    instances.push(QuadInstance::new([ex, ey], [ew * 0.45, eh], color, 0));
    // Ayırıcı bant
    instances.push(QuadInstance::new(
        [ex + ew * 0.45, ey - 1.0],
        [1.5, eh + 2.0],
        [255, 255, 255, 220],
        0,
    ));
}

/// Izgara / Snap ikonu (3x3 # çizgileri).
pub fn draw_icon_grid(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    let t = 1.2;
    let p1 = size * 0.33;
    let p2 = size * 0.67;
    instances.push(QuadInstance::new([x, y + p1], [size, t], color, 0));
    instances.push(QuadInstance::new([x, y + p2], [size, t], color, 0));
    instances.push(QuadInstance::new([x + p1, y], [t, size], color, 0));
    instances.push(QuadInstance::new([x + p2, y], [t, size], color, 0));
}

/// Kilit ikonu (asma kilit).
pub fn draw_icon_lock(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    let body_w = (size * 0.75).round();
    let body_h = (size * 0.5).round();
    let body_x = x + (size - body_w) * 0.5;
    let body_y = y + size - body_h;
    let sh_w = (body_w * 0.6).round();
    let sh_x = x + (size - sh_w) * 0.5;
    let sh_h = (size * 0.45).round();
    let t = 1.5;
    instances.push(QuadInstance::new([sh_x, y + 1.0], [sh_w, t], color, 0));
    instances.push(QuadInstance::new([sh_x, y + 1.0], [t, sh_h], color, 0));
    instances.push(QuadInstance::new(
        [sh_x + sh_w - t, y + 1.0],
        [t, sh_h],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [body_x, body_y],
        [body_w, body_h],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [body_x + body_w * 0.5 - 1.0, body_y + body_h * 0.3],
        [2.0, body_h * 0.4],
        [20, 22, 27, 255],
        0,
    ));
}

/// Göz / Görünürlük ikonu.
pub fn draw_icon_eye(instances: &mut Vec<QuadInstance>, x: f32, y: f32, size: f32, color: [u8; 4]) {
    let mid_y = y + size * 0.5;
    let t = 1.2;
    instances.push(QuadInstance::new(
        [x + size * 0.2, y + size * 0.25],
        [size * 0.6, t],
        color,
        0,
    ));
    instances.push(QuadInstance::new([x, mid_y], [size * 0.25, t], color, 0));
    instances.push(QuadInstance::new(
        [x + size * 0.75, mid_y],
        [size * 0.25, t],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [x + size * 0.2, y + size * 0.75],
        [size * 0.6, t],
        color,
        0,
    ));
    let ps = (size * 0.3).round();
    instances.push(QuadInstance::new(
        [x + (size - ps) * 0.5, y + (size - ps) * 0.5],
        [ps, ps],
        color,
        0,
    ));
}

/// Cetvel ikonu.
pub fn draw_icon_ruler(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    let rh = (size * 0.5).round();
    let ry = y + (size - rh) * 0.5;
    instances.push(QuadInstance::new([x, ry], [size, rh], color, 0));
    let tick_col = [20, 22, 27, 255];
    let ticks = 5;
    for i in 0..ticks {
        let tx = x + (i as f32 + 0.5) * (size / ticks as f32);
        let th = if i % 2 == 0 { rh * 0.6 } else { rh * 0.35 };
        instances.push(QuadInstance::new([tx, ry], [1.0, th], tick_col, 0));
    }
}

/// Kaydet ikonu (disket).
pub fn draw_icon_save(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    instances.push(QuadInstance::new([x, y], [size, size], color, 0));
    let sh_w = (size * 0.55).round();
    let sh_h = (size * 0.35).round();
    instances.push(QuadInstance::new(
        [x + (size - sh_w) * 0.5, y],
        [sh_w, sh_h],
        [24, 26, 32, 255],
        0,
    ));
    instances.push(QuadInstance::new(
        [x + (size - sh_w) * 0.5 + 2.0, y + 1.0],
        [2.0, sh_h - 2.0],
        color,
        0,
    ));
    let lb_w = (size * 0.7).round();
    let lb_h = (size * 0.38).round();
    instances.push(QuadInstance::new(
        [x + (size - lb_w) * 0.5, y + size - lb_h - 1.0],
        [lb_w, lb_h],
        [24, 26, 32, 255],
        0,
    ));
}

/// Şimşek / Sarsıntı FX ikonu.
pub fn draw_icon_bolt(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    instances.push(QuadInstance::new(
        [x + size * 0.45, y],
        [size * 0.25, size * 0.5],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [x + size * 0.2, y + size * 0.4],
        [size * 0.6, 2.0],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [x + size * 0.25, y + size * 0.45],
        [size * 0.25, size * 0.55],
        color,
        0,
    ));
}

/// Artı ikonu (+).
pub fn draw_icon_plus(
    instances: &mut Vec<QuadInstance>,
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    let t = 2.0;
    instances.push(QuadInstance::new(
        [x, y + size * 0.5 - t * 0.5],
        [size, t],
        color,
        0,
    ));
    instances.push(QuadInstance::new(
        [x + size * 0.5 - t * 0.5, y],
        [t, size],
        color,
        0,
    ));
}
