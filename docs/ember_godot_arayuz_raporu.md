# Project Ember — Godot 4 Panel Mimarisi ve Arayüz Raporu

İletilen Godot Engine (Godot 4) ekran görüntüsü piksel ve yerleşim bazında analiz edilerek `rugeng_editor` arayüzü birebir bu şablona uyarlanmıştır.

---

## 1. Mimari Karşılaştırma ve Eleştiri

### 1.1 Önceki Hatalı Varsayımlar
- Önceki düzende Dosya Sistemi (FileSystem) ekranın en altına tam genişlikte yerleştirilmiş, Denetçi (Inspector) ise sadece tuval yüksekliğinde sınırlandırılmıştı.
- Bu yaklaşım, karmaşık düğüm bileşenleri açıldığında dikey çalışma alanını daraltıyordu.

### 1.2 Godot 4 Düzeninin Üstünlüğü ve Uygulaması
1. **Sol Sütun (İkiye Bölünmüş Dock - 260px):**
   - **Üst Kısım:** `[Scene]` ve `[Import]` sekmeleri, `[+]` düğüm ekleme, `Filter Nodes` arama kutusu ve Godot'un `Create Root Node` seçenekleri (`2D Scene`, `3D Scene`, `User Interface`, `Other Node`). Yüklü sahnede düğüm hiyerarşisi ağaç yapısında listelenir.
   - **Alt Kısım:** `[FileSystem]` ve `[History]` sekmeleri, `< > res://` yol gezgini, dosya filtreleme ve `res://` ağacı (`camera_pivot.gd`, `fish_eye.gdshader`, `icon.svg`, `player.gd`, `player.tscn`, `weapon_rig.gd`, `world.tscn`).
2. **Orta Alan: 2D Tuval (Viewport):**
   - Üstte sahne sekmeleri: `[player]`, `[world]`, `[empty] x`, `[+]`.
   - Altında araç çubuğu: `[SEC]`, `[TASI]`, `[DONDUR]`, `[BOYUT]`, `[CETVEL]`, `[SNAP]`, `[KILIT]`, `[GORUNUM]`.
   - Tuval: Üst ve sol kenarda piksel koordinat cetvelleri (`-256`, `-128`, `0`, `128`, `256`, `384`...), merkezde yeşil dikey ve kırmızı yatay eksen.
   - Yüzen Yakınlaştırma Rozeti: Ekran görüntüsündeki gibi sol üstte `[-] %82.6 [+]` kutucuğu (tıklanabilir).
3. **Alt Panel (Sadece Tuvalin Altında!):**
   - Sol ve sağ panelleri kesmez, yalnızca orta tuvalin tabanında konumlanır.
   - Çıktı metin alanı: Godot 4 açılış ve hata günlükleri formatında.
   - Alt sekmeler: `[Output]`, `[Debugger]`, `[Audio]`, `[Animation]`, `[Shader Editor]`.
   - Sağ alt rozetler: `! 1` (sarı uyarı), `X 0` (kırmızı hata) ve `4.7.2.stable` versiyon etiketi.
4. **Sağ Sütun: Tam Boy Denetçi (300px):**
   - Yukarıdan aşağıya tam boy uzanır.
   - `[Inspector]` ve `[Signals]` sekmeleri, `Filter Properties` arama çubuğu.
   - Modüler kartlar: `Transform 2D` (konum X/Y, boyut W/H değiştirme butonları), `Sprite 2D` (`res://icon.svg` dokusu ve renk modülasyonu), `CollisionShape 2D` (`RectangleShape2D`) ve `[+ Add Component]` butonu.
5. **Üst Çubuk (30px):**
   - Solda Godot mavi logosu ve `Scene`, `Project`, `Debug`, `Editor`, `Help` menüleri.
   - Ortada çalışma alanı seçicileri: `[2D]` (aktif mavi vurgulu), `[3D]`, `[Script]`, `[Game]`, `[Asset Store]`.
   - Sağda simülasyon taşıma butonları: `[>]` (Oynat), `[||]` (Duraklat), `[#]` (Durdur), `[@]` (Kamera/Film).

---

## 2. Doğrulama ve Performans
- **Vulkan Pipeline:** Tüm paneller, yazılar, cetveller, butonlar ve varlıklar tek bir `vkCmdDraw(6, N)` çağrısıyla çizilmeye devam etmektedir.
- **Derleme:** `cargo clippy -p rugeng_editor -- -D warnings` sıfır uyarı.
- **Otomatik Test:** `AMD Radeon RX 5500 XT (RADV NAVI14)` üzerinde 120 karelik test hatasız tamamlandı.
