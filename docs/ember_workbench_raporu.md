# Project Ember — Motor Çalışma Alanı (Workbench / Editör) İyileştirme Raporu

## 1. Tespit Edilen Kök Sorunlar ve Çözümleri

### 1.1 Yazıların ve Ekranın Ters Olması (Y-Axis Inversion)
- **Kök Neden:** Vulkan klip uzayında (Normalized Device Coordinates - NDC) Y ekseni tepe noktasında `-1.0`, taban noktasında `+1.0` değerini alır. OpenGL'de ise tepe `+1.0`, taban `-1.0` değerindedir.
- Önceki kodda `Mat4::orthographic_rh(0.0, screen_w, screen_h, 0.0, -1.0, 1.0)` çağrısı yapılıyordu (`bottom: screen_h, top: 0.0`). Bu durum Y eksenini ters çevirerek tepeyi `+1.0` (Vulkan tabanı), tabanı `-1.0` (Vulkan tepesi) konumuna eşlemekteydi. Sonuç olarak bütün pencereler, butonlar ve karakterler baş aşağı ve ters yönde çiziliyordu.
- **Çözüm:** Projeksiyon matrisi `Mat4::orthographic_rh(0.0, screen_w, 0.0, screen_h, -1.0, 1.0)` (`bottom: 0.0, top: screen_h`) olarak düzeltildi. Artık piksel koordinatında `y = 0` doğrudan pencerenin tepesine (`-1.0`), `y = screen_h` doğrudan pencerenin tabanına (`+1.0`) eşlenmektedir.
- Bu dönüşüm matematiksel bir birim testi (`test_orthographic_rh_mapping`) ile doğrulanıp regresyona karşı garanti altına alındı. Aynı düzeltme `games/bullet_hell` ve `sprites_50k` örneklerine de uygulandı.

---

## 2. Godot ve Unity Tarzı Arayüz ve Panel Mimarisi

Önceki tasarımda paneller düz metin yığınlarından ibaretti ve görsel hiyerarşi yetersizdi. Godot 4 ve Unity çalışma ortamlarının görsel standartlarına uygun olarak arayüz şu şekilde yeniden yapılandırıldı:

### 2.1 Üst Menü ve Oyun Taşıma Çubuğu (Top Bar - 28px)
- **Sol:** `EMBER 2D` motor marka rozeti ve standart menüler (`[DOSYA]`, `[DÜZENLE]`, `[SAHNE]`, `[PROJE]`, `[GÖRÜNÜM]`, `[YARDIM]`).
- **Orta:** Godot tarzı merkez taşıma kontrolleri:
  - `[> OYNAT]` (Çalışırken yeşil vurgulu, tıklanabilir).
  - `[|| DUR]` (Sarı vurgulu duraklatma).
- **Sağ:** Donanım ve grafik durumu rozeti (`VULKAN 1.3 | RADV GPU`).

### 2.2 Sol Panel: Sahne Ağacı (Scene Hierarchy - 280px)
- Üst başlık çubuğu: Mavi renkli sol kenarlık vurgusu ile belirginleştirildi.
- Eylem butonları:
  - Yeşil zeminli `[+ DÜĞÜM / VARLIK]` butonu ile kameranın baktığı yere yeni varlık oluşturma.
  - Kırmızı zeminli `[- SİL]` butonu ile seçili varlığı komut tamponu üzerinden güvenle yok etme.
- Düğüm Hiyerarşisi:
  - Kök Sahne: `v [SAHNE] MainScene.scene` (Altın sarısı rozet).
  - Sistem Düğümleri: `|-- [CAM] 2D Kamera` ve `|-- [LGT] Ortam Işığı`.
  - ECS Varlıkları: `v [VARLIKLAR (ENTITIES)]` altında her varlık için 1px kenarlıklı kart yapısı. Seçili varlık parlak mavi çerçeve ve beyaz metinle ayrışır.
  - Bileşen Rozetleri: Varlık satırında hangi bileşenlerin bulunduğu renkli etiketlerle (`[TR]`, `[SP]`, `[COL]`) gösterilir.

### 2.3 Orta Alan: 2D Sahne Görünümü (Viewport Canvas)
- Üst Araç Çubuğu (24px): `[Q: SEÇ]`, `[W: TAŞI]`, `[E: DÖNDÜR]`, `[R: BOYUT]` kısayol butonları, ızgara aralığı, kamera koordinatı ve yakınlaştırma katsayısı bilgisi.
- Tuval: Koyu arduvaz (`#10131b`) zemin üzerinde sonsuz 2D ızgara çizgileri.
- Merkez Eksenler: Unity/Godot standardında Kırmızı X Ekseni (`+X`) ve Yeşil Y Ekseni (`+Y`).
- Seçim Gizmosu: Sahneden bir varlık seçildiğinde, etrafında 4 köşe tutamacı olan sarı seçim çerçevesi ve merkezde yön okları çizilir.

### 2.4 Sağ Panel: Denetçi (Inspector - 330px)
- Seçili Varlık Başlık Kartı: `SEÇİLİ NESNE: VARLIK #ID` ve `[AKTİF]` yeşil göstergesi.
- Modüler Bileşen Kartları (Godot / Unity Component Cards):
  - **Transform 2D:** Yeşil sol vurgu çubuğu, kırmızı `[X]` ve yeşil `[Y]` koordinat göstergeleri, anlık konum değiştirme (`[X-]`, `[X+]`, `[Y-]`, `[Y+]`), boyut değiştirme (`[W-]`, `[W+]`).
  - **Sprite 2D:** Mavi sol vurgu çubuğu, atlas doku adı ve gerçek zamanlı renk önizleme kutucuğu (Color Swatch).
  - **Box Collider 2D:** Sarı sol vurgu çubuğu, dinamik gövde tipi ve çarpışma kutusu durumu.
- Alt Eylem: Mavi renkli belirgin `[+ BİLEŞEN EKLE (ADD COMPONENT)]` butonu.

### 2.5 Alt Panel: Dosya Sistemi & Konsol & Teşhis (190px)
- Sekmeli Düzen: `[📁 DOSYA SİSTEMİ]` ve `[📝 KONSOL]` sekmeleri, aktif sekme altında mavi çizgi göstergesi.
- Dosya Sistemi Tarayıcısı: Disk üzerindeki gerçek dosyaları tarar (`res://project-rugeng/...`), klasörler için altın sarısı `[DIR]`, Rust dosyaları için turuncu `[RS]`, yapılandırma dosyaları için mavi `[CFG]`, belgeler için mor `[DOC]` rozetleri gösterir. `[^ ÜST DİZİN]` butonu ve klasörlere tıklama ile dizinler arası geçiş yapılır.
- Konsol: Renkli günlük seviyesi (`[INFO]`, `[WARN]`, `[ERROR]`) ile sistem olaylarını listeler.
- Sağ Teşhis Paneli: 16.66 ms kare bütçesi ilerleme çubuğu, anlık milisaniye, FPS, aktif varlık sayısı ve Vulkan tek çekim (`1 vkCmdDraw`) durumunu gösterir.

---

## 3. Doğrulama ve Test Sonuçları
1. **Derleme ve Kod Kalitesi:** `cargo clippy --all-targets -- -D warnings` sıfır uyarı ile geçti.
2. **Birim Testleri:** `cargo test --all` bünyesindeki 10 testin tamamı başarılı oldu.
3. **Donanım ve Ekran Doğrulaması:**
   - Seçilen Donanım: `AMD Radeon RX 5500 XT (RADV NAVI14)` (Ayrık GPU, Vulkan 1.4).
   - `AUTO_TEST=1` ile 120 kare boyunca kesintisiz render alındı, sıfır çökme, sıfır doğrulama hatası.
