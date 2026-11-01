# Project Ember — Motor Panel Mimarisi ve Viewport Manipülasyon Raporu

## 1. Viewport Nesne Manipülasyon Sistemi (Tamamlandı)
Tuval üzerindeki varlıkların doğrudan fare ve kısayollarla dönüştürülmesi (Translation, Scaling, Rotation) sıfır bellek tahsisli ECS döngüsüyle entegre edildi.

### A. Etkileşim Mekanizması
- **Taşıma (Translate):** Seçili nesnenin gövdesine tıklandığında veya aktif araç `[TASI]` (W tuşu) iken nesne farenin dünya koordinat farkına göre taşınır.
- **Boyutlandırma (Scale):** Seçili nesnenin 4 köşesindeki mavi tutamaçlara tıklandığında veya aktif araç `[BOYUT]` (R tuşu) iken nesnenin genişlik/yükseklik boyutu fare sürüklemesine bağlı olarak dinamik genişletilir/daraltılır (asgari sınır: 8x8 px).
- **Döndürme (Rotate):** Nesnenin merkezinden dışarı uzanan yön vektörü çizgisi ve ucundaki sarı yön tutamacı (`Heading Tip`) tutulup sürüklendiğinde veya aktif araç `[DONDUR]` (E tuşu) iken nesne merkez açı farkına göre kesintisiz döndürülür.
- **Seçim & Kısayollar:** Tuvaldeki herhangi bir nesneye tıklandığında doğrudan seçilir. Boş tuval alanına tıklandığında seçim kaldırılır. `Q` (Seç), `W` (Taşı), `E` (Döndür), `R` (Boyutlandır), `N` (Kameraya Yeni Düğüm Ekle), `Delete` / `Backspace` (Seçili Düğümü Sil).

---

## 2. Motor Panel Mimarisi (Mevcut Durum ve İnceleme Sırası)

Arayüz Godot 4 mimarisi temel alınarak 5 ana panele ve 1 üst denetim çubuğuna ayrılmıştır. Sırayla tek tek ele alacağımız paneller şunlardır:

### Panel 1: Üst Menü ve Çalışma Alanı Çubuğu (Top Bar)
- **Konum:** Ekranın en üstü (30 px yükseklik).
- **İçerik:**
  - Sol: Motor logosu `[G]`, `Scene`, `Project`, `Debug`, `Editor`, `Help` açılır menü başlıkları.
  - Orta: Çalışma Alanı Seçicileri (`2D`, `3D`, `Script`, `Game`, `Asset Store`). Aktif sekme Godot mavisi hap şeklinde vurgulanır.
  - Sağ: Simülasyon Kontrolleri (`[>]` Play/Oynat, `[||]` Pause/Duraklat, `[#]` Stop/Durdur, `[@]` Movie/Film Modu).

### Panel 2: Sol Üst Panel — Sahne Hiyerarşisi (Scene Tree Dock)
- **Konum:** Sol sütun (260 px genişlik), üst yarı.
- **İçerik:**
  - Sekmeler: `[Scene]` ve `[Import]`.
  - Hızlı Düğmeler: `+` (Düğüm Ekle), `@` (Örnekle / Instantiate), `Filter Nodes` arama kutusu.
  - Kök Düğüm Oluşturucular: `2D Scene` (Node2D), `3D Scene` (Node3D), `User Interface` (Control), `Other Node`.
  - ECS Sahne Ağacı: Sahnedeki tüm varlıkların kimlikleri, adları ve tür rozetleri (`[2D]`).

### Panel 3: Sol Alt Panel — Dosya Sistemi (FileSystem Dock)
- **Konum:** Sol sütun (260 px genişlik), alt yarı.
- **İçerik:**
  - Sekmeler: `[FileSystem]` ve `[History]`.
  - Yol Çubuğu: `< > res://` dizin göstergesi ve dosya arama filtresi.
  - Ağaç Listesi: Proje içi GDScript (`.gd`), gölgelendirici (`.gdshader`), vektör (`.svg`) ve sahne (`.tscn`) kaynakları.

### Panel 4: Orta Panel — 2D/3D Çalışma Alanı (Viewport Canvas)
- **Konum:** Sol ve sağ panellerin arasında, üst menü ile alt panel arasındaki ana alan.
- **İçerik:**
  - Sahne Sekmeleri: `player`, `world`, `[empty]`, `+` (Çoklu sahne düzenleme).
  - Alt Araç Çubuğu: `[SEC]` (Q), `[TASI]` (W), `[DONDUR]` (E), `[BOYUT]` (R), `[CETVEL]`, `[SNAP]`, `[KILIT]`, `[GORUNUM]`.
  - Piksel Cetvelleri: Üst ve sol eksenlerde zum oranına duyarlı dinamik piksel işaretleri.
  - Eksenler ve Izgara: Kırmızı X ve yeşil Y koordinat merkezleri, sonsuz ızgara çizgileri.
  - Etkileşimli Gizmo: Çerçeve, 4 köşe boyutlandırma tutamacı, yön çizgisi ve döndürme tutamacı.
  - Yüzen Zum Rozeti: `[-] %82.6 [+]` (tıklanabilir ve tekerlek duyarlı).

### Panel 5: Alt Panel — Konsol ve Çıktı Rafı (Output & Bottom Shelf)
- **Konum:** Sadece Viewport'un hemen altı (160 px yükseklik), sol ve sağ panellerin arasını doldurur.
- **İçerik:**
  - Sekmeler: `[Output]`, `[Debugger]`, `[Audio]`, `[Animation]`, `[Shader Editor]`.
  - Günlük Çıktısı: Godot 4 sürüm başlığı, uyarılar, hata bildirimleri ve çalışma alanı log akışı.
  - Durum Rozetleri: `! 1` (Sarı Uyarı Sayacı), `X 0` (Kırmızı Hata Sayacı), `4.7.2.stable` motor sürüm rozeti.

### Panel 6: Sağ Sütun — Tam Boy Denetçi (Inspector & Signals)
- **Konum:** Sağ sütun (300 px genişlik), ekranın tepesinden tabanına kesintisiz uzanır.
- **İçerik:**
  - Sekmeler: `[Inspector]` ve `[Signals]`.
  - Filtreleme: `Filter Properties` arama kutusu.
  - Bileşen Kartı 1 (`Transform 2D`): Position X, Position Y, Rotation (açı derece ve R-/R+), Scale (genişlik/yükseklik ve W-/W+).
  - Bileşen Kartı 2 (`Sprite 2D`): Doku yolu (`res://icon.svg`) ve Modulate renk önizleme bloğu.
  - Bileşen Kartı 3 (`CollisionShape 2D`): Şekil türü ve aktif simülasyon durumu.
  - Bileşen Ekleme: `+ Add Component` butonu.

---

## 3. Eleştiri Protokolü ve Yol Haritası
- **Zayıf Nokta / Risk:** Panelleri aynı anda toptan değiştirmek görsel ve mimari tutarsızlığa yol açar; her panelin veri akışı (örneğin FileSystem tıklandığında Inspector'da dosya detayı açılması, Scene Tree tıklandığında Viewport'ta gizmo belirmesi) birbiriyle kenetlidir.
- **Alternatif Yaklaşım:** Panelleri tek tek ele alırken, önce kullanıcının odaklanmak istediği paneli seçip (örneğin Panel 2: Sahne Ağacı veya Panel 6: Denetçi) o panelin detaylı widget'larını, çift tıklama davranışlarını ve stilini son haline getirmek en temiz ve hatasız ilerlemedir.
