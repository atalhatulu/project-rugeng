# Project Ember: Motor Çalışma Alanı (Rugeng Workbench / Editor)

**Tarih:** 29 Ekim 2026  
**Durum:** Tam Donanımlı Olarak Hazır ve Doğrulandı (`crates/rugeng_editor`)

---

## 1. Neler Eklendi?

Önceki sade kabuk yerine, Godot ve Unity benzeri tam kapsamlı motor panelleri entegre edildi:

1. **Sahne Ağacı (Scene Hierarchy - Sol Panel):**
   - `▼ SAHNE: MainScene.scene` hiyerarşik ağacı.
   - Kamera `[KAMERA] 2D Viewport Cam` ve Işık `[ISIK] Ortam Işığı` düğümleri.
   - Canlı ECS varlıkları: `Varlık #0:0 [TR][SP]`, vb.
   - `[+ VARLIK]` ve `[- SIL]` butonları ile sahneye yeni nesne ekleme ve kaldırma.
   - Tıklayarak varlık seçimi (seçilen varlık mavi renkle vurgulanır).

2. **Gelişmiş Denetçi (Inspector - Sağ Panel):**
   - **Transform 2D:** Konum X, Y (canlı `[X-]`, `[X+]`, `[Y-]`, `[Y+]` butonları ile sahne üzerinde hareket ettirme), Boyut (W x H) ve Döndürme açısı.
   - **Sprite 2D:** Doku atlas adı (`default_atlas.png`), aktif renk önizleme kutusu, alfa kanalı.
   - **Fizik & Collider 2D:** Kutu AABB çarpışma durumu ve simülasyon tipi.
   - `[+ BILESEN EKLE (ADD COMPONENT)]`: Seçili varlığa anında yeni bileşen bağlama.

3. **Dosya Sistemi / Varlık Tarayıcısı (Asset Browser / Filesystem - Alt Panel):**
   - Projenin gerçek disk üzerindeki dosya ve dizin yapısını (`assets/`, `crates/`, `docs/`, `games/`, vb.) kutucuklar halinde listeler.
   - Klasörlere tıklayarak içine girme ve `[.. UST DIZIN]` butonu ile geri çıkma.
   - İstenildiğinde `[KONSOL]` sekmesine geçerek canlı motor loglarını inceleme.

4. **Viewport (Orta 2D Sahne Alanı):**
   - Sonsuz 2D ızgara çizgileri, kırmızı X ve yeşil Y koordinat eksenleri.
   - Sağ fare tuşu veya orta fare tuşu ile Pan (sonsuz kaydırma).
   - Fare tekerleği ile Zoom (0.2x - 5.0x yakınlaştırma/uzaklaştırma).
   - Seçilen varlığın etrafında canlı sarı seçim çerçevesi (Gizmo).

5. **Kare Bütçesi & Vulkan 1.3 Teşhisi:**
   - 16.66 ms kare bütçesi barı (Ayrık GPU üzerinde 7.1 ms, 144 FPS kilitli).
   - Tek çizim çağrısı `vkCmdDraw(6, N)`.
   - 0 gameplay malloc.

---

## 2. Çalıştırma

Terminalden doğrudan çalıştır:
```bash
cargo run --release -p rugeng_editor
```

**Kontroller:**
- **Sol Panel:** `[+ VARLIK]` ile yeni entity oluştur, listeden tıkla seç.
- **Sağ Panel:** `[X-]`, `[X+]`, `[Y-]`, `[Y+]` butonlarıyla seçili nesneyi hareket ettir.
- **Alt Panel:** `[DOSYA SISTEMI]` ile klasörleri gez, `[KONSOL]` ile loglara bak.
- **Orta Alan:** Sağ fare tuşuyla sahneyi kaydır, tekerlekle yakınlaş.
- **Kısayollar:** `N` (Yeni Varlık), `Delete` (Sil), `ESC` (Çıkış).
