# Rugeng Engine — Gelişmiş Modüller ve Parça Mimarisi Raporu

Bu rapor, motorun temel omurgasını oluşturan 6 kritik parçanın planlanması, mimari eleştirisi ve toplu olarak uygulanmasının ardından hazırlanmıştır.

---

## 1. Mimari Eleştiri ve Risk Analizi

### 1.1 Mevcut Kararların Zayıf Noktaları
1. **Düz Liste Tabanlı Hiyerarşi:**
   - *Mevcut Durum:* `Parent` ve `Children` bileşenleri sparse-set ECS üzerinde ilişkilendirildi ve yerel ofsetler dünya koordinatlarına `compute_world_transform` ile dönüştürüldü.
   - *Risk / Zayıf Nokta:* 3 veya daha fazla derinlikteki hiyerarşilerde (büyükbaba -> baba -> çocuk) sıra bağımlılığı (topolojik sıralama - topological sort) olmaksızın çalıştığında 1 karelik gecikme (frame latency) oluşabilir. Çok derin ağaçlar için önceden sıralanmış derinlik öncelikli (DFS) matris çarpım önbelleği eklenmelidir.
2. **Hafızadaki Undo Yığını Sınırı:**
   - *Mevcut Durum:* `undo_stack` dinamik `Vec<UndoAction>` olarak tutuluyor.
   - *Risk:* 10.000 işlem yapıldığında yığın şişebilir. Prodüksiyon aşamasında sabit boyutlu (örneğin son 100 işlem) döngüsel tampon (circular ring buffer) kullanılmalıdır.

### 1.2 Alternatif Yaklaşımlar
- **Çoklu Seçim İçin Alternatif:** Bounding Volume Hierarchy (BVH) veya QuadTree ile kutu kesişim sorgusu. Şu anki düz sparse-set taranması 1.000 nesneye kadar sıfır gecikmeyle çalışır; 50.000+ nesnede ise Spatial Hash Grid ile kutu sorgusu yapılmalıdır.
- **Bileşen Ekleme İçin Alternatif:** Dinamik DLL/SO tabanlı yansıma (reflection). Rust statik yapısı gereği tip güvenli monomorfik kayıt tercih edilmiştir.

---

## 2. Uygulanan 6 Kritik Parça

### Parça 1: Transform Hiyerarşisi (Parent - Child & Koordinat Bileşimi)
- `rugeng_core::hierarchy`: `Parent` ve `Children` bileşenleri çekirdeğe eklendi.
- `compute_world_transform(local_pos, local_rot, parent_pos, parent_rot) -> (world_pos, world_rot)` fonksiyonu ile 2D dönüş matrisi kompozisyonu sağlandı.
- Sahnede bir ebeveyn taşındığında veya döndüğünde (`turret` vb.) çocuk nesneler otomatik olarak ebeveynin dünya koordinatlarını takip eder.

### Parça 2: Scene Graph Ağaç Görünümü (Scene Tree Indentation)
- Scene Tree dock'unda çocuk düğümler hiyerarşik olarak ` > ` öneki ve 14 piksellik girinti (indent) ile gösterilir.
- Varlığın türüne göre rozetler güncellendi: Görsel düğümler için `[2D]`, metin düğümleri için mor `[TXT]`.

### Parça 3: İşlem Geçmişi (Geri Al / Yinele — Undo / Redo)
- `UndoAction` komut deseni entegre edildi:
  - `CreateEntity`: Yeni oluşturulan varlığı geri alır.
  - `DeleteEntity`: Silinen varlığın tüm bileşenlerini (Transform, Renk, Doku, Çarpıştırıcı, Hız, Betik, Metin, Ebeveyn) eksiksiz geri yükler.
  - `MoveEntity`: Nesne konum değişikliklerini eski haline döndürür.
- Üst barda `[GERI]` ve `[YINELE]` butonları; klavyede `KeyZ` (Ctrl+Z) ve `KeyY` (Ctrl+Y) kısayolları bağlandı.

### Parça 4: Kutu ile Çoklu Seçim (Marquee / Box Selection)
- Tuvalde boş bir alana tıklayıp sürüklendiğinde yarı saydam mavi seçim çerçevesi (`BoxSelection`) dinamik olarak çizilir.
- Fare bırakıldığında kutu sınırları (`min_x, max_x, min_y, max_y`) içindeki tüm varlıklar tespit edilir ve `selected_entities` listesine alınır.

### Parça 5: Inspector Dinamik Bileşen Yönetimi (Add Component Popup)
- Inspector panelindeki `+ Add Component` butonuna tıklandığında açılır menü devreye girer:
  - `+ Collider2D Ekle`: Çarpışma kutusu ekler.
  - `+ Velocity2D Ekle`: Hız vektörü ekler.
  - `+ Script (Rotator) Ekle`: Dönüş betiği bağlar.
  - `+ Text2D (Metin) Ekle`: Sahne metin bileşeni ekler.

### Parça 6: Sahne İçi Metin Çizim Bileşeni (Text2DComponent)
- `Text2DComponent { text, scale, color }` bileşeni eklendi.
- Sahnede dünya koordinatlarında konumlanan ve kamera zoom/pan hareketlerine göre ekrana yansıtılan metin çizim hattı devreye alındı.
- Sahneye doğrudan metin ekleme şablonu (`preset: "text"`) ve taret şablonu (`preset: "turret"`) bağlandı.

---

## 3. Doğrulama ve Test Sonuçları

- **Workspace Testleri (`cargo test --workspace`):**
  - 15 testin tamamı (Hiyerarşi matris testleri, ECS kural denetimi, ses sentezi, sahne serileştirme) sıfır hata ile geçti.
- **Release Derlemesi (`cargo build --workspace --release`):**
  - Tüm kütüphane ve çalıştırılabilir paketler 27 saniyede sıfır hata ve sıfır uyarı ile derlendi.

---

## 4. Kullanım ve Kısayol Tablosu

| Kısayol | İşlem |
| :--- | :--- |
| `KeyZ` / `[GERI]` | Son İşlemi Geri Al (`Undo`) |
| `KeyY` / `[YINELE]` | Son İşlemi Yinele (`Redo`) |
| Tuvalde Sürükleme | Kutu ile Çoklu Seçim (`Box Selection`) |
| `KeyT` / `[YENI]` | Yeni Sahne Aç (`New Scene`) |
| `KeyC` / `[Kopyala]` | Seçili Nesneyi Çoğalt (`Duplicate`) |
| `KeyK` / `[KAYDET]` | Sahneyi Kaydet (`Save`) |
| `KeyL` / `[YUKLE]` | Sahneyi Diskten Yükle (`Load`) |
| `KeyN` | Hızlı Gemi Ekle |
| `Delete` / `Backspace` | Seçili Nesneyi Sil (`Delete`) |
