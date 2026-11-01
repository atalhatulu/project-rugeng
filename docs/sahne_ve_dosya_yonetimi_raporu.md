# Rugeng Engine — Sahne ve Dosya Yönetimi Geliştirme Raporu

Bu rapor, motorun sahne yönetimi, nesne ekleme, kopyalama, silme, dosya kaydetme ve sahne açma yeteneklerinin tamamlanmasının ardından hazırlanmıştır.

---

## 1. Mimari Eleştiri ve Gelecek Risk Değerlendirmesi

### 1.1 Mevcut Mimari Eksiklikler ve Riskler
1. **Geri Al / Yinele (Undo/Redo) Komut Kuyruğunun Yokluğu:**
   - *Risk:* Kullanıcı nesne sildiğinde veya çoğalttığında bunu `Ctrl+Z` ile geri alamaz. Yanlışlıkla silinen varlıklar kaybolur.
   - *Zayıf Nokta:* Command Buffer deseni şu an tek yönlü çalışıyor; `UndoCommand` geçmişi tutulmuyor.
2. **Düz (Flat) Varlık Yapısı (Hiyerarşik Ağaç Eksikliği):**
   - *Risk:* Godot veya Unity'deki gibi ebeveyn-çocuk (parent-child transform) ilişkisi bulunmuyor. Bir gemiyi hareket ettirdiğinizde üzerindeki silah veya can barı otomatik olarak gemiyi takip etmez; her varlık bağımsız koordinatlara sahiptir.
   - *Çözüm:* Sonraki aşamada `Parent(Entity)` ve `Children(Vec<Entity>)` bileşeni ile matris hiyerarşisi eklenmelidir.

### 1.2 Alternatif Yaklaşımlar
- **Hiyerarşik Sahne Grafı (Scene Graph):** Düz Sparse-Set ECS yerine Scene Graph Transform sistemi. Çocuk nesnelerin global pozisyonu `parent.world_matrix * local_matrix` olarak hesaplanır. 2D oyunlar için bu sistem kaçınılmazdır.
- **İşlem Komut Kuyruğu (Command Pattern Undo/Redo):** Editördeki her eylemin (`AddEntity`, `RemoveEntity`, `MoveEntity`) bir komut nesnesi olarak kuyruğa yazılması ve 50 adımlık bir geri alma yığını tutulması.

---

## 2. Eklenen Yeni Motor Yetenekleri

### 2.1 Sahneye Doğrudan Nesne Ekleme (Presets)
Scene Tree panelinin üst kısmına ve klavye kısayollarına hazır varlık şablonları entegre edildi:
- **`[+ Gemi]` (`ship`):** `player_ship` dokusu, 48x48 boyut, `Transform2D`, `Collider2D` ve `RotatorScript` bileşenleriyle doğrudan kontrol edilebilir/dönen uzay aracı.
- **`[+ Bocek]` (`enemy`):** `enemy_bug` dokusu, 40x40 boyut, `Transform2D`, `Collider2D`, `Velocity2D` ve yatay devriye gezen `PatrolScript`.
- **`[+ Kure]` (`orb`):** `energy_orb` dokusu, 32x32 boyut, nabız gibi büyüyüp küçülen `PulseScript` ve sensör çarpıştırıcı.
- **`[+ Kalp]` (`heart`):** `heart_icon` dokusu, 32x32 boyut, statik arayüz/can simgesi.
- **`[+ Bos Dugum]` (`empty`):** 36x36 sarı renkli temel `Node2D`.

### 2.2 Seçili Düğüm Eylemleri
- **`[Kopyala]` (`Duplicate` / `KeyC`):** Seçili nesnenin tüm bileşenlerini (Transform, Renk, Doku, Çarpıştırıcı, Hız, Betik) kopyalayarak 24 piksel sağ-aşağı ofsetle yeni bir varlık üretir ve anında seçer.
- **`[Sil (Del)]` (`Delete` / `Delete` & `Backspace` tuşları):** Seçili nesneyi sahneden tamamen kaldırır ve patlama ses efekti çalar.
- **`[Yeni Sahne]` (`New Scene` / `KeyT`):** Sahnedeki tüm varlıkları temizler, yeni bir `untitled_X.scene` yolu atar ve merkezde varsayılan bir kök düğüm oluşturur.

### 2.3 Sahne Kayıt ve Dinamik Dosya Yönetimi
- **Üst Menü Butonları:**
  - `[YENI]`: Sahneyi sıfırlar ve yeni sahne açar.
  - `[KAYDET]`: Aktif sahne dosyasını (`current_scene_path`) üzerine yazar ve kirli bayrağını (`*`) temizler.
  - `[FARKLI]`: `scenes/level_X.scene` formatında yeni bir dosyaya kaydeder ve geçerli sahne yolunu günceller.
  - `[YUKLE]`: Aktif sahneyi diskten yeniden yükler.
  - `[DISA AKTAR]`: Bağımsız çalıştırıcı (`rugeng_standalone`) için sahneyi diske kaydedip hazır hale getirir.
- **Dinamik Sahne Sekmesi:**
  - Orta Viewport'un üstündeki sekmede aktif dosya adı (örnek: `main.scene` veya `untitled_2.scene`) dinamik olarak görüntülenir. Sahne üzerinde değişiklik yapıldığında başlıkta `(*)` işareti belirir.
  - Sekme çubuğundaki `[+]` butonuna tıklandığında yeni sahne oluşturulur.
- **FileSystem Dock Tıklama Yükleyicisi:**
  - Dosya gezgininde herhangi bir `.scene` dosyasına tıklandığında motor o sahneyi anında ayrıştırıp sahnede inşa eder ve aktif sahne yolunu günceller.

---

## 3. Klavye Kısayolları

| Kısayol | İşlem |
| :--- | :--- |
| `KeyT` | Yeni Sahne Aç (`New Scene`) |
| `KeyC` | Seçili Nesneyi Çoğalt (`Duplicate`) |
| `KeyK` | Sahneyi Kaydet (`Save Scene`) |
| `KeyL` | Sahneyi Diskten Yükle (`Load Scene`) |
| `KeyN` | Sahneye Hızlı Gemi Ekle |
| `Delete` / `Backspace` | Seçili Düğümü Sil |
| `KeyQ` / `KeyW` / `KeyE` / `KeyR` | Seçim / Taşıma / Döndürme / Boyutlandırma Araçları |
| `KeyF` | Kamerayı Seçili Düğüme Odakla |
| `KeyS` | Grid Kenetlenmesini Aç/Kapat (16px Snap) |
| `ESC` | Editörden Çıkış |
