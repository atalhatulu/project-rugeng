# Editör Menü Sistemi ve Açılır Menü (Dropdown) Mimarisi Raporu

**Proje:** Project Ember / Rugeng Engine  
**Modül:** `rugeng_editor`  
**Tarih:** 30 Ekim 2026  

---

## 1. Tespit Edilen Problemler ve Yapılan Değişiklikler

Eski menü yapısında ciddi ergonomik ve görsel tasarım sorunları bulunmaktaydı:
1. **Görsel Çakışma (Layout Collision):** Üst çubukta yer alan 8 adet büyük aksiyon butonu (`[YENI]`, `[KAYDET]`, `[FARKLI]`, `[YUKLE]`, `[GERI]`, `[YINELE]`, `[SARSINTI]`, `[DISA AKTAR]`) toplamda 600 pikselden fazla genişlik kaplayarak ekranın ortasında bulunan çalışma alanı sekmeleriyle (`2D`, `3D`, `Script`, `Game`, `Asset Store`) doğrudan çakışıyor ve üst üste biniyordu.
2. **Kukla (İşlevsiz) Menü Başlıkları:** Üst çubukta yer alan `Scene`, `Edit`, `Project`, `Debug`, `Help` başlıkları yalnızca pasif metin olarak çizilmekteydi; tıklandığında hiçbir tepki vermiyordu.
3. **Eksik Kısayol Geri Bildirimi:** Kullanıcı klavye kısayollarını menü üzerinden göremiyordu.

### Uygulanan Çözüm

- **Gerçek Açılır Menü (Dropdown Menu) Mimarisi:**
  - `OpenMenu` numaralandırması (`Scene`, `Edit`, `Project`, `Debug`, `Help`) eklendi.
  - Menü başlığına tıklandığında başlık aktif mavi çerçeve ile vurgulanır ve hemen altında gölgeli, koyu temalı Godot 4 tarzı bir açılır menü paneli açılır.
  - Açılır menü tüm tuval ve dock katmanlarının üzerinde (en üst Z katmanında) çizilir.
  - Her menü maddesinin yanında kısayol tuşu (örneğin `Ctrl+T`, `Ctrl+K`, `Ctrl+Z`, `Ctrl+Y`, `Ctrl+C`, `Del`, `X`, `Esc`) sağa hizalı ve sönük renkte gösterilir.
  - Fare menü öğelerinin üzerine geldiğinde arka plan yumuşak maviyle (`[44, 62, 90, 255]`) aydınlanır.
  - Menü maddesine tıklandığında ilgili motor fonksiyonu çalıştırılır, `sfx_blip` sesi çalınır ve menü kapanır.
  - Menü dışındaki herhangi bir yere tıklandığında açılır menü güvenle kapatılır ve alt katmanlara yanlış tıklama iletilmesi engellenir.

- **Üst Çubuk Düzenlemesi:**
  - Sol tarafta: Godot `[G]` ikonu ve açılır menü başlıkları (`Scene`, `Edit`, `Project`, `Debug`, `Help`).
  - Ortada: Çakışmasız, ferah çalışma alanı sekmeleri (`2D`, `3D`, `Script`, `Game`, `Asset Store`).
  - Sağ tarafta: Sık kullanılan hızlı aksiyonlar (`[KAYDET]`, `[SARSINTI]`) ve oyun simülasyon kontrolleri (`[ > ]`, `[ || ]`, `[ # ]`, `[ @ ]`).

---

## 2. Açılır Menü İçerikleri

### Scene Menüsü
- `[+] Yeni Sahne` (Ctrl+T)
- `[S] Sahneyi Kaydet` (Ctrl+K)
- `[F] Farklı Kaydet...`
- `[O] Sahne Aç...` (Ctrl+L)
- *Ayırıcı Çizgi*
- `[X] Çıkış` (Esc)

### Edit Menüsü
- `[<] Geri Al` (Ctrl+Z)
- `[>] Yinele` (Ctrl+Y)
- *Ayırıcı Çizgi*
- `[D] Varlığı Çoğalt` (Ctrl+C)
- `[X] Varlığı Sil` (Del)
- `[A] Tümünü Seç`

### Project Menüsü
- `[E] Bağımsız Oyunu Dışa Aktar`
- `[R] Dosya Ağacını Yenile`
- `[S] Proje Ayarları...`

### Debug Menüsü
- `[!] Sarsıntı & Patlama FX` (X)
- `[#] Izgara Yapışması (Snap)` (S)
- `[C] Konsol Kayıtlarını Temizle`

### Help Menüsü
- `[?] Klavye Kısayolları`
- `[D] Dokümantasyon (docs/README.md)`
- `[i] RugEng Motoru Hakkında`

---

## 3. Eleştiri Protokolü (Riskler ve Alternatifler)

- **Menü Dışı Tıklama ve Olay Yutma (Event Swallowing) Riski:**
  - *Risk:* Bir açılır menü açıkken kullanıcının tuvaldeki bir varlığı seçmek veya taşımak için tıklaması durumunda, hem menü kapanıp hem de arkadaki nesne yanlışlıkla sürüklenebilirdi.
  - *Çözüm:* Menü açıkken yapılan dış tıklama, menüyü kapatır ve tıklama olayını tüketerek arkadaki tuval ve araç panellerine sızmasını (event leaking) engeller.
- **Alternatif Yaklaşım (İkon Tabanlı Yatay Araç Çubuğu):**
  - *Alternatif:* Açılır menü yapmayıp sadece 8 aksiyon butonunu küçük piktogram/ikonlara dönüştürerek yan yana dizmek.
  - *Eleştiri:* İkonlar tek başına ne yaptıklarını açıklamakta yetersiz kalır, menü başlıkları ("Scene", "Project" vb.) yine işlevsiz kalır ve küçük ekran çözünürlüklerinde çakışma riski sürer. Açılır menü hiyerarşik, temiz ve genişletilebilir tek doğru masaüstü editör standardıdır.
