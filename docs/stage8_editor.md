# Aşama 8: Editör 3D/2D Viewport ve Gelişmiş Sahne Yönetimi Raporu

**Tarih:** 31 Ekim 2026  
**Proje:** Project-Ember (Rust - `rugeng`)  
**Kapsam:** `crates/rugeng_editor` Pipeline3D & 3D/2D Viewport entegrasyonu, serbest uçuş kamerası (`EditorCamera3D`), 3D zemin ızgarası ve X/Y/Z eksenleri, Sahne Ağacı (Scene Tree) 3D desteği, Denetçi (Inspector) Transform3D ve Mesh3D/PBR kartları, Versiyonlu Sahne kaydetme/yükleme (`rugeng_assets::VersionedScene`).  
**Durum:** Aşama 8 isterleri ve kalite kapıları başarıyla tamamlandı.

---

## 1. Mimari Genel Bakış ve Gerçekleştirilen Hedefler

### A. 3D / 2D Viewport ve Serbest Uçuş Kamerası
- `crates/rugeng_editor/src/types.rs`:
  - `EditorCamera3D`: Serbest uçuş ve odaklanma yeteneğine sahip 3D editör kamerası.
    - Pozisyon (`eye`), bakış hedefi (`target`), `yaw`, `pitch`, `fov_degrees`, `move_speed`.
    - Yön vektörleri (`forward`, `right`, `up`).
    - Fare tekeri ile yaklaşma/uzaklaşma (`dolly`).
    - Vulkan NDC uyumlu perspektif kamera projeksiyon matrisine dönüştürme (`to_render_camera`).
- `crates/rugeng_editor/src/main.rs`:
  - Sağ fare tuşu basılı tutularak fare hareketi ile 3D kamera yörünge dönüşü (orbit: yaw & pitch).
  - Orta fare tuşu basılı tutularak fare hareketi ile yatay/dikey kaydırma (pan: right & up).
  - Fare tekerleği (Mouse Wheel) ile ileri/geri yumuşak yaklaşma/uzaklaşma (dolly zoom).
  - W, A, S, D, Space tuşları ile simülasyon/düzenleme sırasında serbest uçuş hareketi.
  - `KeyF` kısayolu ile seçili 3D nesneye odaklanma ve kamerayı nesnenin yakınına konumlandırma.

### B. 3D Zemin Izgarası ve Koordinat Eksenleri
- `crates/rugeng_render/src/mesh.rs`:
  - `create_grid_and_axes(size: f32, step: f32) -> Mesh3D`:
    - XZ düzleminde zemin ızgarası oluşturan prosedürel geometri üreticisi.
    - X ekseni (Kırmızı), Y ekseni (Yeşil) ve Z ekseni (Mavi) renk bantları ile 3D yönelim görselleştirmesi.
- Editör başlatılırken GPU belleğine (`gpu_grid`) statik olarak yüklenir ve 3D modunda zemin referansı olarak çizilir.

### C. Sahne Hiyerarşisi (Scene Tree) 3D Desteği
- `crates/rugeng_editor/src/main.rs`:
  - Sahne ağacında hem 2D hem 3D varlıklar taranır ve hiyerarşik sırada listelenir.
  - `Transform3D` veya `Mesh3DComponent` barındıran varlıklar için sarı `[3D]` rozeti ve `Node3D_#index:gen` etiketi gösterilir.
  - Tıklama ile hem 2D hem de 3D varlıklar anında seçilebilir ve denetçide etkinleştirilir.

### D. Denetçi (Inspector) 3D Kartları ve Etkileşimli Düzenleme
- `crates/rugeng_editor/src/main.rs`:
  - **Transform 3D Kartı:**
    - Pozisyon (X, Y, Z): Mevcut koordinatların gösterimi ve `[-]` / `[+]` butonlarıyla 0.5 birimlik artırma/azaltma.
    - Rotasyon (Euler Pitch, Yaw, Roll): Açıların derece cinsinden gösterimi ve `P-`, `P+`, `Y-`, `Y+` butonlarıyla açısal kontrol.
    - Ölçek (Scale): `S-` ve `S+` butonlarıyla boyutlandırma.
  - **Mesh 3D & PBR Kartı:**
    - `Mesh3DComponent` parametreleri: Geometri tipi (`Cube`, `Sphere`, `Plane`), `roughness` ve `metallic` değerleri.
    - Tıklanabilir butonlarla nesne geometrisini anında küp, küre veya düzlem olarak değiştirme olanağı.

### E. Versiyonlu Sahne Kaydetme ve Yükleme (`rugeng_assets`)
- `crates/rugeng_assets::VersionedScene` entegrasyonu:
  - Editörün `save_current_scene` fonksiyonu, sahnedeki tüm 2D ve 3D varlıkları `SceneEntity` yapısına paketler (`transform_3d` ve `mesh_3d` alanları dahil).
  - Dosyaya `VersionedScene` (v2) JSON formatında kaydedilir.
  - `load_scene_from_file` fonksiyonu, JSON dosyasını okuyarak v1 formatındaysa otomatik v2'ye yükseltir; ardından `Transform3D` ve `Mesh3DComponent` bileşenlerini ECS kayıt defterine yerleştirir.

### F. 3D Gizmo, Ekran İzdüşüm Seçimi (Ray/Screen Picking) ve Hazır Şablonlar
- `crates/rugeng_editor/src/main.rs`:
  - **3D Gizmo Görselleştirmesi**: 3D modunda seçili varlığın dünya koordinatları kamera matrisi ile ekrana izdüşürülerek altın sarısı merkez göstergesi ve Kırmızı (X), Yeşil (Y), Mavi (Z) koordinat okları çizilir.
  - **3D Viewport Seçimi (Screen Picking)**: 3D modda viewport alanına sol tıklandığında kamera görüş açısındaki 3D nesnelerin ekran mesafesi hesaplanarak en yakın nesne seçilir.
  - **3D Hazır Varlık Oluşturma (Presets)**: Sahne Ağacı üst paneline `+ 3D Kup` ve `+ 3D Kure` butonları eklendi. `spawn_preset_entity("cube_3d")` ve `"sphere_3d"` işlevleriyle hazır PBR parametreleriyle anında nesne üretilebilir.
  - **Dinamik Denetçi Düzeni**: Denetçi kartları varlığın türüne (2D / 3D) göre dinamik dikey ofset (`cur_insp_y`) ile yerleştirilir; Transform 3D ve Transform 2D kartlarının çakışması ve hayalet tıklama etkileşimleri engellenmiştir.
