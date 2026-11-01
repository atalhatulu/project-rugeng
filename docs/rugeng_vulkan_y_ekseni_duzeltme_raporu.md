# Vulkan Y Ekseni Hizalaması ve Oryantasyon Düzeltme Raporu

## 1. Tespit Edilen Kök Neden (Root Cause)

Vulkan grafik API'sinde Normalized Device Coordinates (NDC) standardında:
- OpenGL ve klasik sağ el kartezyen sisteminde: $Y = -1$ ekranın altı, $Y = +1$ ekranın tepesidir.
- **Vulkan standardında**: $Y = -1$ ekranın tepesi, $Y = +1$ ekranın altıdır.

`crates/rugeng_render/src/pipeline_3d.rs` içerisinde `vk::Viewport` tanımı daha önce şu şekilde pozitif yükseklik ile tanımlanmıştı:
```rust
let viewport = vk::Viewport {
    x: vp_x,
    y: vp_y,
    width: vp_w,
    height: vp_h,
    min_depth: 0.0,
    max_depth: 1.0,
};
```
Bu konfigürasyonda, kameranın sağ el kuralına uygun perspektif projeksiyon matrisi (`Mat4::perspective_rh`) dünyadaki $+Y$ (yukarı) eksenini Vulkan NDC'nin $+Y$ (aşağı) koordinatına düşürmekteydi. Sonuç olarak:
1. Sahnedeki tüm 3D varlıklar (küpler, küreler, zemin ızgarası) dikey eksende baş aşağı (ters) çiziliyordu.
2. Editörün 2D UI katmanında çizilen seçim çerçeveleri, 3D yönlendirme küpü (orientation gizmo) ve merkez tutamaçları ile Vulkan'da çizilen 3D mesh'ler birbirine ters yönlü oluyordu.

---

## 2. Uygulanan Mühendislik Çözümü

Vulkan 1.1+ spesifikasyonunun önerdiği standart ve donanım düzeyinde en performanslı yöntem uygulandı:

1. **Negatif Viewport Yüksekliği (Flipped Viewport):**
   - [`crates/rugeng_render/src/pipeline_3d.rs`](file:///home/teha/Documents/GitHub/project-rugeng/crates/rugeng_render/src/pipeline_3d.rs) içerisindeki `draw_mesh_pbr_viewport` fonksiyonunda:
     - `y: vp_y + vp_h`
     - `height: -vp_h`
     olarak ayarlandı.
   - Bu sayede projeksiyon matrisinin standart yapısı bozulmadan Vulkan donanım rasterizer'ı Y eksenini dikeyde çevirerek $+Y$ yönünü doğrudan ekranın yukarısına hizaladı.

2. **Winding Order ve Back-face Culling Uyumu:**
   - Viewport yüksekliğinin eksi işaretli yapılması ekran uzayında üçgen dönüş yönünü ters çevirir.
   - Orijinal dış yüzeylerin (CCW) doğru şekilde görünmeye devam etmesi ve arkada kalan yüzeylerin cullenmesi için pipeline rasterization ayarındaki `front_face` değeri:
     - `vk::FrontFace::COUNTER_CLOCKWISE` -> `vk::FrontFace::CLOCKWISE`
     olarak güncellendi.
   - Böylece küp, küre, silindir, koni, piramit gibi tüm 3D gövdelerin dış yüzeyleri doğru oryantasyonla çizilirken, culling davranışı da kusursuz korundu.

3. **2D Ekran Projeksiyonu ve Gizmo Uyumu:**
   - [`crates/rugeng_editor/src/main.rs`](file:///home/teha/Documents/GitHub/project-rugeng/crates/rugeng_editor/src/main.rs) içerisindeki `project_pt` ekran hesaplaması:
     ```rust
     let sx = vp_x + (ndc_x * 0.5 + 0.5) * vp_w;
     let sy = vp_canvas_y + (1.0 - (ndc_y * 0.5 + 0.5)) * vp_canvas_h;
     ```
     formülüyle Vulkan'ın yeni oryantasyonu ile birebir senkronize hale getirildi. 3D nesnelerin ekran üzerindeki merkezleri, fare tıklama koordinatları ve gizmo okları tam olarak üst üste oturdu.

---

## 3. Doğrulama ve Test Sonuçları

- **Birim & Entegrasyon Testleri:** `cargo test --workspace` çalıştırıldı, 94 testin 94'ü de başarıyla geçti.
- **Linter & Uyarı Denetimi:** `cargo clippy --workspace --all-targets -- -D warnings` sıfır uyarı ile geçti.
- **Kod Formatı:** `cargo fmt --check` tamamen temiz.
