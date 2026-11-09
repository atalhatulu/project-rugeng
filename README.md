# Project Rugeng (Ember Engine) — Kapsamlı Durum ve Mimari Raporu

**Tarih:** 9 Kasım 2026  
**Sürüm:** 0.1.0-alpha (Stage 18+)  
**Hedef:** Veri odaklı (data-oriented), Rust + Vulkan 1.3/1.4 hibrit 2D/3D oyun motoru ve editörü.  

---

## 1. Genel Durum ve Kalite Metrikleri

- **Doğrulanmış Test Sayısı:** **183 Test (%100 Yeşil)**
- **Test Baseline Referansı:** [`docs/TEST_BASELINE.md`](docs/TEST_BASELINE.md)
- **Clippy Denetimi:** **0 Uyarı** (`cargo clippy --workspace --all-targets -- -D warnings`)
- **Format Uyumu:** Temiz (`cargo fmt --check`)
- **Güvenlik ve Bağımlılık Denetimi (Audit):** RUSTSEC-2026-0192 (ttf-parser) arındırıldı, `swash` katmanına taşındı.
- **Eşzamanlılık ve Bellek Güvenliği:** Loom ve Miri ile veri yarışı, dead-lock ve tanımsız davranış (UB) onaylı.

---

## 2. Doğrulanmış Performans Metrikleri

| Alan | Ölçülen Değer | Hedef Eşik | Durum |
| :--- | :--- | :--- | :--- |
| **50.000 Sprite 2D Çizimi** | Ortalama **1.01 ms** (p99: **3.32 ms**) | < 5.0 ms | Başarılı (4x marj) |
| **200.000 Sprite 2D Stres** | Toplam **1.67 ms** | < 10.0 ms | Başarılı |
| **500.000 Sprite 2D Stres** | Toplam **4.34 ms** | < 16.6 ms | Başarılı |
| **20.000 Mermi Simülasyonu** | Ortalama **0.22 ms** (p99: **3.88 ms**) | < 8.33 ms (60 FPS bütçesinin %50'si) | Başarılı (2.1x marj) |
| **Steady-State Bellek Tahsisi** | **0 malloc** | Sabit 0 malloc | Başarılı |

---

## 3. Crate ve Modül Mimarisi

Project Rugeng, birbirine gevşek bağlı (loosely coupled), yüksek performanslı alt kütüphanelerden oluşur:

```
project-rugeng/
├── crates/
│   ├── rugeng_core/       # Flat Sparse-Set ECS, FrameArena, CommandBuffer, AccessSet, Animasyon Durum Makinesi
│   ├── rugeng_render/     # Vulkan 1.3/1.4 Dynamic Rendering + Sync2, Instanced 2D, D32 Depth, PBR 3D, Gölge (PCF 3x3), Swash Font
│   ├── rugeng_editor/     # Godot 4 tarzı modüler dock layout (ui/), 3D flycam, çoklu sahne sekmeleri, gizmolar, dosya sistemi
│   ├── rugeng_physics/    # 2D Spatial Hash, AABB & Slope çözücü, 3D AABB/Sphere/Capsule, Kinematik Karakter Kontrolcüsü
│   ├── rugeng_scheduler/  # Panic-safe work-stealing thread pool, DAG zamanlayıcı, Loom onaylı
│   ├── rugeng_assets/     # Bellek eşlemeli (mmap) VFS, glTF 2.0/GLB yükleyici, İkili/JSON VersionedScene, .pak arşivi
│   └── rugeng_script/     # Luau sandbox, SoA scratch view, durum korumalı sıcak yeniden yükleme (hot reload)
├── games/
│   ├── bullet_hell/       # 20.000 mermi performans dogfood oyunu
│   ├── platformer/        # 2D eğim ve fizik platform oyunu
│   └── demo_3d/           # 3D PBR, gölge haritası, kinematik karakter ve kamera dogfood demosu
└── docs/                  # Her aşama ve doğrulamanın detaylı mühendislik raporları
```

---

## 4. Uygulanan Özellikler (Mevcut Durum)

### A. Çekirdek ve Mantık (`rugeng_core`, `rugeng_scheduler`, `rugeng_script`)
- **Flat Sparse-Set ECS:** Cache-dostu dizi yerleşimi, minimum bellek ayak izi.
- **Sıfır-Alloc CommandBuffer:** Çerçeve içi yapısal değişikliklerin (spawn/despawn) gecikmeli ve toplu uygulanması.
- **AccessSet Tabanlı Paralel Scheduler:** Sistemlerin bileşen okuma/yazma çakışmalarını compile/runtime anında DAG'a dizerek güvenli paralel çalıştırma.
- **Kinematik Animasyon Durum Makinesi (3D):** Durumlar arası doğrusal harmanlama (cross-fade / lerp) ve hız duyarlı geçişler.
- **Luau Script Sandbox:** SoA scratch bellek alanı üzerinden güvenli bellek ve döngü limitli betik çalıştırma.

### B. Grafik ve Render Motoru (`rugeng_render`)
- **Vulkan 1.3/1.4 Dynamic Rendering:** Render pass nesneleri olmadan modern ve hafif render hattı.
- **Instanced 2D Render:** Tek bir `vkCmdDraw(6, N)` çağrısıyla yüz binlerce sprite çizimi.
- **Cook-Torrance PBR 3D Render:** GGX/Smith mikro yüzey dağılımı, Fresnel-Schlick yansıması, Reinhard ton haritalama ve gama 2.2 düzeltmesi.
- **Directional Shadow Mapping:** 2048x2048 D32_SFLOAT derinlik tamponu ve 3x3 PCF (Percentage-Closer Filtering) yumuşak gölgelendirme.
- **Skeletal Skinning Altyapısı:** Matris paleti ve GPU vertex skinning veri yapıları.
- **Tipografi ve Font:** `swash` 0.1 motoruyla Türkçe karakter (`ş`, `ç`, `ğ`, `ı`, `ö`, `ü`) ve vektörel UI ikonları.

### C. Fizik ve Mekânsal Hızlandırma (`rugeng_physics`)
- **2D Spatial Hash Grid & Slope Solver:** 45 derece eğim, tek yönlü platformlar ve dinamik AABB çarpışmaları.
- **3D Çarpışma ve Işın Kesişimi:** Ray-AABB, Ray-Sphere, 3D Spatial Hash ve Ray Picking.
- **3D Kinematik Karakter Kontrolcüsü:** Yerçekimi, ivmelenme, zemin normali kayması (slope sliding), tavan çarpışma iptali ve basamak yapışması (step down snapping).
- **SpringArm / Orbit Camera:** Kamera engellerle kesiştiğinde sönümlü yakınlaşan 3. şahıs serbest kamera.

### D. Editör ve Kullanıcı Deneyimi (`rugeng_editor`)
- **Godot 4 Tarzı Modüler Dock Düzeni:** Yeniden boyutlandırılabilir bölücüler (splitters).
- **Çoklu Sahne Sekmeleri:** Birden fazla sahneyi aynı anda açma, sekmeler arası geçiş, kirli (`*`) bayrağı ve kapatma.
- **Boş Sahne Şablonları:** Tek tıkla boş 2D (`Node2D_Root`) veya boş 3D (`World3D_Root` + `Ground_Plane` + `Center_Cube`) sahnesi oluşturma.
- **Düğüm Kütüphanesi (Node Library Modal):** `[+]` butonu ile filtrelenebilir kategorik 2D/3D varlık ekleme penceresi.
- **Bağlamsal Sağ Tık Menüsü:** Viewport üzerinde sağ tıklanan dünya koordinatına doğrudan nesne spawn etme.
- **Dosya Sistemi (FileSystem):** Res:// tabanlı gezinme, tek tıkla `+ Klasör` ve `+ Sahne` oluşturma, çift tıkla sahne açma.
- **Canlı Simülasyon ve Rollback:** Play/Stop ile editör içinde canlı fizik simülasyonu ve durdurulduğunda ilk konumlara tam geri dönüş.
- **Tek Tıkla Export:** Varlıkları `.pak` ikili arşivinde toplayıp dağıtıma hazır hale getirme.

---

## 5. Eksiklikler, Sınırlar ve Açık Alanlar (Teknik Analiz)

Motor mimari olarak sağlam temellere oturmuş olsa da ticari seviyede bir oyun geliştirme ortamı için aşağıdaki kritik eksiklikler mevcuttur:

### A. Render ve Grafik Eksiklikleri
1. **Noktasal ve Spot Işıklar (Point/Spot Lights):** Şu an yalnızca tek bir yönlü ışık (Sun / Directional Light) desteklenmektedir. Çoklu ışıklandırma (Forward+ veya Clustered Shading) henüz eklenmemiştir.
2. **Kaskadlı Gölge Haritaları (Cascaded Shadow Maps - CSM):** Mevcut gölge haritası tek bir ortografik kutu kullanır; geniş açık dünya sahnelerinde uzak mesafelerde gölge çözünürlüğü düşer.
3. **glTF İskelet Doku/Ağırlık Yükleyicisi:** `rugeng_render` içindeki skinning iskeleti hazırdır ancak `rugeng_assets::gltf` tam animasyonlu glTF node hiyerarşisini henüz otomatik bağlamamaktadır.
4. **Post-Processing Hattı:** Bloom, SSAO (Screen Space Ambient Occlusion), FXAA/TAA kenar yumuşatma filtreleri eksiktir.

### B. Editör ve Araç Zinciri Eksiklikleri
1. **Görsel Sahne Ağacı Sürükle-Bırak (Drag & Drop Reparenting):** Hiyerarşi kod seviyesinde (`set_parent`) desteklenirken, fareyle sürükleyip başka bir düğümün altına bırakma etkileşimi henüz görselleştirilmemiştir.
2. **Metin Kutusu Girişleri (Inspector Input Fields):** Denetçi panelindeki koordinat ve boyut değerleri şu an `[-]` ve `[+]` butonlarıyla değiştirilmektedir; klavyeden doğrudan sayı yazma alanı eklenmelidir.
3. **Görsel Eğri ve Animasyon Editörü:** Durum makinesi parametreleri kod tabanlıdır; Unity/Godot tarzı görsel durum geçiş grafiği (Animation Graph) yoktur.

### C. Ses ve Fizik Eksiklikleri
1. **3D Konumsal Ses (Spatial Audio):** PipeWire sürücüsü stereo PCM çalmaktadır; dinleyici ve ses kaynağı mesafesine göre zayıflama (distance attenuation) ve pan hesabı eklenmelidir.
2. **Karmaşık 3D Konveks/Mesh Çözücü:** 3D fizik şu an AABB, Küre ve Kapsül ile sınırlıdır; rastgele meşler için Convex Hull veya GJK/EPA çarpışma çözücüsü bulunmamaktadır.

---

## 6. Hızlı Başlangıç

Projeyi derlemek ve test etmek için `start.sh` betiği kullanılır:

```bash
# Editörü release modunda başlat
./start.sh editor

# Geliştirici (hızlı derleme) modunda başlat
./start.sh editor-dev

# Bullet Hell prototipini çalıştır (20k mermi dogfood testi)
./start.sh bullet-hell

# 3D Demo sahnesini çalıştır
./start.sh demo-3d

# Tüm statik analiz, Miri, Loom ve birim testlerini doğrula
./start.sh check
```
