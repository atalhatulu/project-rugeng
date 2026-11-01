# Project Ember — Proje Aşamaları ve Yol Haritası Durum Raporu

**Tarih:** 29 Ekim 2026  
**Motor Çekirdeği:** `rugeng` (Rust, Vulkan 1.3, Flat Sparse ECS, Lock-Free FrameArena)

---

## 1. Tamamlanan Aşamalar

### Aşama 0: Çekirdek Sağlamlaştırma (Core Hardening) — [TAMAMLANDI]
- **ECS Mimarisi:** 1.000.000 varlık üzerinde Flat Sparse-Set ve Paged Sparse-Set kıyaslandı; 200 KB RAM ile %11 daha hızlı olan Flat Sparse varsayılan seçildi.
- **Sıfır-Malloc Komut Tamponu (`CommandBuffer`):** Sistemlerin paralel koşumu sırasında yapısal değişikliklerin (`spawn`, `destroy`, `emplace`) tamponlanması ve senkronizasyon anında tek seferde 0 bellek tahsisiyle uygulanması sağlandı.
- **`AccessSet` ve Güvenlik:** Aynı sistemde çakışan okuma/yazma talepleri engellendi.
- **Lock-Free FrameArena:** Worker thread'ler için `AtomicUsize` ile lock-free tahsis kuruldu.
- **Miri Doğrulaması:** 0 tanımsız davranış (UB), 0 bellek sızıntısı.

### Aşama 1: Vulkan 2D Render Pipeline — [TAMAMLANDI]
- **Vulkan 1.3 Altyapısı:** Dynamic Rendering ve Synchronization2 ile RenderPass/Framebuffer külfetsiz modern çizim boru hattı kuruldu.
- **Ayrık GPU Önceliği:** AMD Radeon RX 5500 XT GPU'su aktif seçilerek doğrulandı.
- **32-Bayt `QuadInstance`:** `pos[2]`, `size[2]`, `uv[2]`, `color(u32)`, `tex_id(u32)` ile bellek optimizasyonu.
- **Persistent Ring Upload Buffer:** 6.4 MB (200.000 sprite) kapasiteli frames-in-flight ring buffer.
- **Tek Draw Call:** Tüm sahne `vkCmdDraw(6, N)` ile tek seferde çizildi (144 FPS kilitli).

### Aşama 2: Bullet Hell & Dahili Teşhis Paneli (In-Game Overlay) — [TAMAMLANDI]
- **2D Spatial Hash Grid:** O(N^2) çarpışma karmaşıklığı O(N) seviyesine indirildi (0.45 ms).
- **Simülasyon:** 18.000 mermili canlı simülasyon, 60 Hz sabit fizik adımı (`FIXED_DT`).
- **Dahili Teşhis HUD'ı:** Kare bütçesi barı (16.66 ms), bellek kullanımı ve simülasyon kontrolleri.

---

## 2. Şu An Üzerinde Çalıştığımız Aşama

### Aşama 3: Motor Çalışma Alanı ve Editör (Godot 4 Tasarımlı Workbench) — [ŞU AN BURADAYIZ]
Kullanıcının "Önce motor görsel olarak Godot/Unity gibi gözüksün, sonra sekme sekme ilerleyelim" talebi doğrultusunda başlatıldı:
- **Tasarım:** Godot 4 arayüzü referans alınarak 6 bölmeli yerleşim uygulandı:
  1. Üst Menü Çubuğu (Top Bar - Mod Seçicileri & Oynatma Butonları)
  2. Sol Üst Sahne Ağacı (Scene Tree & Kök Düğüm Ekleme)
  3. Sol Alt Dosya Sistemi (FileSystem & res:// Tarayıcısı)
  4. Orta Viewport Tuvali (Sonsuz Izgara, Piksel Cetvelleri, Zum Rozeti)
  5. Alt Konsol Rafı (Output, Log Akışı, Hata Sayaçları)
  6. Sağ Tam Boy Denetçi (Inspector & Transform/Sprite/Collider Bileşenleri)
- **Doğrudan Viewport Manipülasyonu:**
  - Taşıma (Translate / W tuşu)
  - Boyutlandırma (Scale / R tuşu - 4 köşe kutusu)
  - Döndürme (Rotate / E tuşu - yön çizgisi ve sarı uç)
  - Seçim (Select / Q tuşu)
- **Sırada Ne Var:** Motorun 6 panelini tek tek, detaylı widget'ları ve işlevleriyle tamamlama adımı.

---

## 3. Gelecek Aşamalar (Yol Haritası)

- **Aşama 4: Varlık Yükleyici ve Doku Atlası (Asset Pipeline & Texture Atlasing)**
  - Gerçek PNG/SVG dokularının Vulkan belleğine yüklenmesi.
  - Çalışma zamanı otomatik atlas paketleme (bin-packing) ve sprite animasyonları.
- **Aşama 5: Betik Motoru (Scripting - Luau Entegrasyonu)**
  - Oyun mantığını C++/Rust derlemesi beklemeden Luau betikleriyle yazma.
  - Hot-reload ile oyunu kapatmadan kod güncelleme.
- **Aşama 6: Ses ve Gelişmiş Girdi Sistemi (Audio & Input Action Map)**
  - Ogg/Wav çalma, mekansal ses efektleri.
  - Gamepad, klavye, fare haritalama (action mapping).
- **Aşama 7: Sahne Serileştirme ve 3D Modu**
  - Sahneleri diske kaydetme ve geri yükleme (`.scene` / `.ron`).
  - 3D temel mesh ve kamera sistemi.
- **Aşama 8: Nihai Dağıtım ve Dışa Aktarım (Export Pipeline)**
  - Windows / Linux bağımsız çalıştırılabilir oyun paketleme.
