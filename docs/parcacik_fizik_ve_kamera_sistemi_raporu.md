# 2D Parçacık, Fizik Çözücü ve Kamera Sarsıntı Sistemi Raporu

**Proje:** Project Ember / Rugeng Engine  
**Modüller:** `rugeng_core`, `rugeng_physics`, `rugeng_render`, `rugeng_editor`  
**Tarih:** 30 Ekim 2026  

---

## 1. Genel Bakış ve Gerçekleştirilen Parçalar

Motor mimarisinde oyunlaştırmadan ziyade saf motor kabiliyetlerine odaklanılarak aşağıdaki kritik alt sistemler sıfır hata ve sıfır uyarı ile çalışma alanına entegre edilmiştir:

1. **2D Dönüşüm Hiyerarşisi (Parent-Child Transform):**
   - Ebeveyn nesnenin konumu ve açısı değiştikçe bağlı çocuk nesnelerin dünya koordinatlarını matematiksel olarak dönüştüren `compute_world_transform` algoritması ([crates/rugeng_core/src/hierarchy.rs](file:///home/teha/Documents/GitHub/project-rugeng/crates/rugeng_core/src/hierarchy.rs)).
   - Editör Sahne Ağacı (Scene Tree) panelinde girintili (` > ` ve 14 piksel ofset) hiyerarşik görselleştirme.

2. **GPU Uyumlu 2D Parçacık Sistemi (Particle System):**
   - Bellek tahsisatsız (zero-alloc) çalışan parçacık havuzu (`ParticleEmitterComponent`, `Particle`).
   - Tek seferlik patlama (`burst`) ve sürekli arkalık izi (`trail`) emisyon modları ([crates/rugeng_core/src/particle.rs](file:///home/teha/Documents/GitHub/project-rugeng/crates/rugeng_core/src/particle.rs)).
   - Parçacık ömrüne bağlı lineer renk ve boyut geçişi (`interpolate_particle_color`).

3. **Katı AABB Çarpışma ve İtme Çözücüsü (Solid Collision Solver):**
   - Minimum Çeviri Vektörü (MTV - Minimum Translation Vector) tabanlı penetration ayrıştırması ([crates/rugeng_physics/src/collision_solver.rs](file:///home/teha/Documents/GitHub/project-rugeng/crates/rugeng_physics/src/collision_solver.rs)).
   - Statik ve dinamik gövdeleri ayırt ederek iç içe geçen varlıkların birbirini itmesini sağlayan katı fizik çözümü.

4. **Kamera Sarsıntısı ve Hedef Takibi (Trauma & Smooth Follow):**
   - Doğrusal olmayan `trauma^2` sönümlenme modeli ile ekran sarsıntısı (`add_trauma`, `trauma_decay`).
   - Kamera hedef takibinde pürüzsüz `lerp` yumuşatma.

5. **Editör Entegrasyonu ve Kısayollar:**
   - Üst araç çubuğuna `[SARSINTI]` butonu eklendi.
   - `X` tuşuna basıldığında hem kamera travması (0.7) hem de seçili düğüm konumunda patlama parçacık efekti ve ses tetiklenir.
   - Geri al (`Ctrl+Z`), Yinele (`Ctrl+Y`), Yeni Sahne (`T`), Kopyala (`C`), Kaydet (`K`), Yükle (`L`), Sil (`Delete`).

---

## 2. Eleştiri Protokolü (Riskler, Zayıflıklar ve Alternatifler)

### Parça 1: Dönüşüm Hiyerarşisi (Parent - Child)
- **Risk ve Zayıf Nokta:** Mevcut sistem tek derinlikli (1-level) ebeveyn-çocuk ilişkisini çözmektedir. Çocuk nesnenin de bir çocuğu olduğunda derinlemesine yinelemeli (recursive) bir matris ağacı yerine sıralı güncelleme gerekmektedir. Eğer DFS/topolojik sıralama yapılmazsa, derin zincirlerde 1 kare gecikme (frame lag) oluşabilir.
- **Alternatif Yaklaşım:** Düz matris dizisi (Flat Affine2 Transform Hierarchy). ECS içinde her varlığa `GlobalTransform(Affine2)` ve `LocalTransform(Affine2)` atanarak tek geçişli topolojik sırayla hesaplanması.
- **Neden Tercih Edildi:** 2D piksel tabanlı motorda matris yükü yerine pozisyon ve radyan açı matematiği (`w_pos = parent_pos + rotate(local_pos)`) CPU önbellek dostudur ve ek bağımlılık gerektirmez.

### Parça 2: Parçacık Sistemi
- **Risk ve Zayıf Nokta:** Parçacıklar CPU tarafında güncellenip ana `QuadInstance` dizisine kopyalanmaktadır. Parçacık sayısı 50.000 üzerine çıktığında CPU-GPU veri aktarımı darboğaz yaratabilir.
- **Alternatif Yaklaşım:** Compute Shader tabanlı parçacık simülasyonu. Vulkan 1.3 Compute Pipeline ile parçacıklar doğrudan VRAM'de saklanıp simüle edilir.
- **Mevcut Kararın Gerekçesi:** 2D retro/piksel tarzı projelerde 500-2.000 parçacık ekran için fazlasıyla yeterlidir. CPU tabanlı yaklaşım; çarpışma tespiti, sahne kaydetme/yükleme ve editör içi denetimle anında etkileşime olanak tanır.

### Parça 3: AABB Çarpışma Ayrıştırması (MTV)
- **Risk ve Zayıf Nokta:** Yüksek hızdaki nesnelerde tünelleme (tunneling) riski vardır. Sürekli Çarpışma Tespiti (CCD - Continuous Collision Detection) uygulanmadığı takdirde, çok ince duvarlardan yüksek hızlı nesneler geçebilir.
- **Alternatif Yaklaşım:** Ray-casting destekli Swept-AABB çözümü.
- **Mevcut Kararın Gerekçesi:** Üstten bakışlı (top-down) ve piksel platform oyunlarında sabit kare zamanı (`fixed delta time`) ile MTV ayrıştırması kararlı ve öngörülebilir sonuç verir.

### Parça 4: Kamera Travması ve Sarsıntı
- **Risk ve Zayıf Nokta:** Sinüzoidal harmonik fonksiyon (`sin(t)`, `cos(1.3t)`) aşırı yüksek travmalarda periyodik bir desene dönüşebilir.
- **Alternatif Yaklaşım:** Perlin/Simplex gürültüsü tabanlı sarsıntı (Perlin Noise Camera Shake).
- **Mevcut Kararın Gerekçesi:** Çoklu harmonik frekanslar (`sin(35t)` ve `cos(45.5t)`) sıfır bellek ve sıfır harici kütüphane ile organik, titreşimsiz ve tok bir patlama hissi üretir.

---

## 3. Doğrulama ve Test Sonuçları

- **Birim ve Entegrasyon Testleri:**
  - `rugeng_core`: 15 test başarılı.
  - `rugeng_physics`: 2 test başarılı.
  - `rugeng_render`: 1 test başarılı.
  - **Toplam:** 18 test passed, 0 failed.
- **Derleme:** Workspace genelinde 0 uyarı (warnings: 0) ve 0 hata (errors: 0).
