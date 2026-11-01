# Project Ember — Motor Çalışma Alanı Gelişmiş Özellikler Raporu

**Tarih:** 29 Ekim 2026  
**Motor Çekirdeği:** `rugeng` (Rust, Vulkan 1.3, Flat Sparse ECS)

---

## 1. Yeni Eklenen ve Derinleştirilen Editör Sistemleri

### A. Gerçek Dosya Sistemi Gezgini (FileSystem Navigator)
- **Dizin İçi Gezinme:** Mock liste yerine diskin gerçek hiyerarşisi (`self.editor.files_in_dir`) entegre edildi.
- **`<` ve `[..]` Butonları:** Klasörlerin içine girildiğinde bir üst dizine çıkma düğmesi aktifleşir.
- **Tip Rozetleri:**
  - Klasörler: `[DIR]` (Sarı)
  - Rust Kaynakları: `[RS]` (Turuncu)
  - Yapılandırma Dosyaları: `[CFG]` (Yeşil)
  - Belgeler: `[DOC]` (Mavi)
  - Gölgelendiriciler / SPIR-V: `[SPV]` (Mor)
  - Diğer: `[FILE]` (Gri)

### B. Canlı Renk Paleti ve Modulate Swatches (Sağ Denetçi)
- `Sprite 2D` kartı içerisine 6 adet tıklanabilir renk paleti eklendi:
  - Godot Mavisi (`#478cbf`)
  - Kızıl Kırmızı (`#e45b5b`)
  - Zümrüt Yeşili (`#42b983`)
  - Amber Sarısı (`#e0a942`)
  - Siber Mor (`#a855f7`)
  - Kar Beyazı (`#f0f0f0`)
- Renk paletine tıklandığında seçili varlığın `SpriteColor` bileşeni anında güncellenir ve tuvalde canlı olarak değişir.

### C. Kamera Odaklama (Focus Key: `F`)
- Tuvalde veya sahne ağacında herhangi bir düğüm seçiliyken `F` tuşuna basıldığında, editör kamerası doğrudan seçili varlığın konumuna (`Transform2D.pos`) yumuşakça odaklanır ve merkezler.

### D. Izgara Yapışması (Grid Snapping - `[SNAP]`)
- Viewport araç çubuğundaki `[SNAP]` butonu veya klavyeden `S` tuşu ile ızgaraya yapışma modu açılıp kapatılabilir.
- Aktifken nesne taşıma 16 piksel katlarına, boyutlandırma 8 piksel katlarına otomatik olarak kilitlenir.

### E. Canlı Simülasyon / Oynat Modu (`[>] Play` & `[#] Stop`)
- **Anlık Görüntü (Snapshot):** `[>]` Play butonuna basıldığı anda sahnedeki tüm varlıkların konumları kaydedilir.
- **Canlı Güncelleme:** `Velocity2D` bileşenine sahip tüm varlıklar 60 Hz fizik mantığıyla hareket eder, sahnede seker ve döner.
- **İlk Haline Dönüş (Reset):** `[#]` Stop butonuna basıldığında sahne ilk anlık görüntüsüne kusursuz olarak geri döndürülür.
