# RugEng Editör Kullanılabilirlik, Sağ Tık Bağlam Menüsü ve Ekleme Arayüzü Raporu

**Tarih:** 31 Ekim 2026  
**Motor:** Project RugEng / Ember 2D & 3D  
**Konum:** `crates/rugeng_editor`  
**Test Durumu:** 18/18 Editör Testi, 96/96 Workspace Testi Başarılı  
**Kod Kalitesi:** `cargo clippy -D warnings` sıfır hata/uyarı, `cargo fmt` temiz  

---

## 1. Giriş ve Kullanıcı Talebi

Kullanıcının ilettiği gereksinim:
> "Boşluğa Sağ tık + add kısmı açılabilir oradan eklenecek şeyleri ekleyebiliriz. Gerisinide o şekilde ayarla"

Editörün daha önce sadece sol paneldeki butonlar üzerinden varlık eklemeye izin vermesi iş akışını yavaşlatıyordu. Modern oyun motorlarında (Godot 4, Unreal, Blender) olduğu gibi doğrudan sahne tuvalinde boşluğa sağ tıklandığında nesne ekleme (`Add`), seçili nesneye sağ tıklandığında ise nesne manipülasyonu (`Duplicate`, `Delete`, `Z-Index`) yapabilme özelliği entegre edilmiştir. Ayrıca üst menü çubuğuna `Add` kategorisi eklenmiş ve araç çubuğuna kısayol rozetleri getirilmiştir.

---

## 2. Mimari Tasarım ve Karar Analizi

### Karşılaştırmalı Yaklaşım Değerlendirmesi:
1. **Alternatif A (Yalnızca Pencere Tabanlı Modal Popup):**  
   - *Risk:* Ekranın ortasını kapatır, kullanıcının tıkladığı dünya koordinatını unutturur ve akışı keser.
2. **Alternatif B (Seçilen - Dinamik Ekran Koordinatlı Bağlam Menüsü):**  
   - Fare sağ tık bırakıldığı anda (< 6 piksel delta ile, kamera pan/fly hareketinden ayırt edilerek) tam imleç konumunda hafif, gölgeli ve şık bir `ContextMenu` açılır.
   - Boşlukta tıklandıysa: 2D/3D varlık ekleme seçenekleri (`[+] 2D Sprite`, `[+] 2D Dusman`, `[+] 2D Kalp`, `[+] 2D Enerji Kuresi`, `[+] 2D Karo Haritasi`, `[+] 2D Metin`, `[3] Kup 3D`, `[3] Kure 3D`, `[3] Duzlem`, `[3] Silindir`, vb.).
   - Varlık üzerinde tıklandıysa: Varlık odaklı işlemler (`[D] Varligi Cogalt`, `[X] Varligi Sil`, `[Z+] Z-Index Artir (+1)`, `[Z-] Z-Index Azalt (-1)`).

---

## 3. Yapılan Değişiklikler ve Entegrasyon

### 3.1. Veri Yapıları (`crates/rugeng_editor/src/types.rs`)
- `ContextMenuItem` ve `ContextMenu` yapıları tanımlandı:
  - `label: String`: Görüntülenen menü başlığı ve rozet.
  - `shortcut: String`: Kısayol tuşu ipucu (`Ctrl+C`, `Del`, vb.).
  - `action_id: String`: Tetiklenecek aksiyon belirteci (`duplicate`, `delete`, `z_up`, `add_ship`, vb.).
  - `target_entity: Option<Entity>`: Tıklanan varlığın referansı.
- `OpenMenu` enum'ına `Add` eklendi (`Scene`, `Edit`, `Add`, `Project`, `Debug`, `Help`).
- `VIEWPORT_TOOLS` araç butonları kısayol tuş rozetleriyle donatıldı (`Sec [Q]`, `Tasi [W]`, `Dondur [E]`, `Boyut [R]`, `Firca`, `Silgi`) ve taşma yapmayacak şekilde piksel genişlikleri optimize edildi.

### 3.2. Editör Durumu (`EditorState` in `crates/rugeng_editor/src/main.rs`)
- `context_menu: Option<ContextMenu>` eklendi.
- `right_click_start: Option<Vec2>` ve `right_click_down: bool` alanları ile sağ tıklama basılı tutma (kamera kaydırma/uçuşu) ile tek tık (bağlam menüsü açma) ayrıştırıldı.

### 3.3. Fare Girişi ve Seçim (`WindowEvent::MouseInput`)
- Sağ tık basıldığında kamera kaydırma ve başlangıç noktası kaydedilir.
- Sağ tık bırakıldığında, hareket mesafesi 6 pikselin altındaysa (`delta < 6.0`):
  - 2D modda: Tuval üzerindeki dünya koordinatında varlık taranır (AABB hit test).
  - 3D modda: Kamera projeksiyon matrisi ile en yakın 3D nesne taranır.
  - Varlık varsa nesne işlem menüsü, yoksa hızlı nesne ekleme (`Add`) menüsü oluşturulup `self.editor.context_menu` alanına atanır.

### 3.4. Bağlam Menüsü Çizimi ve Tıklama Yönetimi (`handle_mouse_click`)
- `handle_mouse_click` başında `self.editor.context_menu.take()` kontrol edilir:
  - Tıklama menü öğelerinden birinin üzerine denk geliyorsa aksiyon çalıştırılır (`duplicate`, `delete`, `z_up`, `z_down`, `add_*`).
  - Menü dışına tıklandığında menü yumuşak şekilde kapanır.
- Üst menü çubuğundaki `Add` açılır menüsü de aynı şekilde `handle_mouse_click` içerisine entegre edilmiştir.

---

## 4. Test ve Doğrulama Kanıtları

1. **Birim Testleri (`cargo test -p rugeng_editor`):**
   - `test_context_menu_and_add_menu`:
     - Üst menüden `Add -> add_ship` tıklaması ve varlık artışı doğrulandı.
     - Boşlukta açılan Context Menu'den `add_enemy` tıklaması ve varlık oluşturma doğrulandı.
     - Varlık üzerinde açılan Context Menu'den `z_up` ile Z-Index artışı (0 -> 1) doğrulandı.
   - `test_toolbar_and_palette_layout_no_overflow`:
     - 700 piksel genişliğindeki viewport sınırları içinde butonların metin ve sağ kenar taşması yapmadığı doğrulandı.
   - Toplam 18 editör testi eksiksiz geçti.

2. **Workspace Testleri (`cargo test --workspace`):**
   - Toplam 96 birim/entegrasyon testi hatasız geçti.

3. **Kod Kalitesi:**
   - `cargo clippy --workspace --all-targets -- -D warnings`: 0 hata, 0 uyarı.
   - `cargo fmt --check`: Tam uyumlu.
   - Başlatma scripti `./start.sh` ile motor ve editör doğrudan çalıştırılabilir.
