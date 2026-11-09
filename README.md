# Project Rugeng (Ember Engine) — Kapsamlı Durum ve Mimari Raporu

**Tarih:** 9 Kasım 2026  
**Sürüm:** 0.1.0-alpha (Stage 18+)  
**Hedef:** Veri odaklı (data-oriented), Rust + Vulkan 1.3/1.4 hibrit 2D/3D oyun motoru ve editörü.  

---

## 1. Genel Durum ve Kalite Metrikleri

- **Doğrulanmış Test Sayısı:** **149+ Test (%100 Yeşil)**
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
│   ├── rugeng_core/       # Flat Sparse-Set ECS, FrameArena, CommandBuffer, AccessSet, Animasyon Durum Makinesi, emplace_checked
│   ├── rugeng_render/     # Vulkan 1.3/1.4 Dynamic Rendering + Sync2, Instanced 2D, D32 Depth, PBR 3D, Noktasal Işıklar SSBO, 4-Kaskad CSM, HDR Tonemapping, Swash Font
│   ├── rugeng_editor/     # Godot 4 tarzı modüler dock layout (ui/), Inspector sayısal veri girişi & geri alma, sahne ağacı sürükle-bırak reparenting, 3D flycam, çoklu sahne sekmeleri, gizmolar, dosya sistemi
│   ├── rugeng_physics/    # 2D Spatial Hash, AABB & Slope çözücü, 3D AABB/Sphere/Capsule, Kinematik Karakter Kontrolcüsü
│   ├── rugeng_scheduler/  # Panic-safe work-stealing thread pool, DAG zamanlayıcı, SystemAccess çakışma tespiti, Loom onaylı
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
- **Flat Sparse-Set ECS:** Cache-dostu dizi yerleşimi, minimum bellek ayak izi, `emplace_checked` ile güvenli varlık doğrulama.
- **Sıfır-Alloc CommandBuffer:** Çerçeve içi yapısal değişikliklerin (spawn/despawn) gecikmeli ve toplu uygulanması.
- **AccessSet Tabanlı Paralel Scheduler (SystemDAG):** Sistemlerin bileşen okuma/yazma çakışmalarını analiz ederek çakışmayan sistemleri aynı paralel aşamada (stage) iş parçacığı havuzu üzerinde eşzamanlı çalıştırır.
- **3D İskelet Ciltleme ve Animasyon:** `Animation3DComponent`, `AnimationClip3D`, `Skeleton`, `Joint` veri yapıları ve doğrusal harmanlama.
- **Luau Script Sandbox:** SoA scratch bellek alanı üzerinden güvenli bellek ve döngü limitli betik çalıştırma.

### B. Grafik ve Render Motoru (`rugeng_render`)
- **Vulkan 1.3/1.4 Dynamic Rendering:** Render pass nesneleri olmadan modern ve hafif render hattı.
- **Instanced 2D Render:** Tek bir `vkCmdDraw(6, N)` çağrısıyla yüz binlerce sprite çizimi.
- **Cook-Torrance PBR 3D Render:** GGX/Smith mikro yüzey dağılımı, Fresnel-Schlick yansıması, 72 bayt `Vertex3D` (skinning indeksleri ve ağırlıkları dahil).
- **GpuPointLight Çoklu Noktasal Işık (SSBO Binding 4):** Push constants ve SSBO üzerinden 64 adede kadar dinamik noktasal ışıklandırma.
- **Kaskadlı Gölge Haritaları (Cascaded Shadow Maps - CSM):** 4 katmanlı 2D Array dokusu (`ImageViewType::TYPE_2D_ARRAY`), SSBO Binding 5 matrisleri ve `sampler2DArrayShadow` PCF yumuşak gölgelendirme.
- **HDR & Tonemapping Hattı:** 16-bit kayan noktalı (`R16G16B16A16_SFLOAT`) HDR hedef tamponu, Reinhard ton haritalama ve gama 2.2 düzeltmesi.
- **Tipografi ve Vektörel İkonlar:** `swash` motoruyla UTF-8 Türkçe desteği, tekilleştirilmiş paylaşımlı prosedürel UI ikonları.

### C. Fizik ve Mekânsal Hızlandırma (`rugeng_physics`)
- **2D Spatial Hash Grid & Slope Solver:** 45 derece eğim, tek yönlü platformlar ve dinamik AABB çarpışmaları.
- **3D Çarpışma ve Işın Kesişimi:** Ray-AABB, Ray-Sphere, 3D Spatial Hash ve Ray Picking.
- **3D Kinematik Karakter Kontrolcüsü:** Yerçekimi, ivmelenme, zemin normali kayması (slope sliding), tavan çarpışma iptali ve basamak yapışması (step down snapping).
- **SpringArm / Orbit Camera:** Kamera engellerle kesiştiğinde sönümlü yakınlaşan 3. şahıs serbest kamera.

### D. Editör ve Kullanıcı Deneyimi (`rugeng_editor`)
- **Modüler Panel Mimarisi (`trait Panel`):** ~2650 satırlık monolitik girdi işleyici yerine bağımsız panellere ayrılmış `handle_mouse_click`.
- **Denetçi (Inspector) Sayısal Metin Girişi:** Klavye ile doğrudan `f32` değer girişi, imleç gezinimi, geçersiz girişte kırmızı kenarlık geri bildirimi ve tam geri alma (Undo/Redo).
- **Sahne Ağacı Sürükle & Bırak (Drag & Drop Reparenting):** `Before`/`Into`/`After` bırakma alanları, ata-döngü tespiti (`is_ancestor_of`) ile döngüsel hiyerarşi engelleme ve geri alma desteği.
- **Çoklu Sahne Sekmeleri & Şablonlar:** 2D ve 3D boş şablonlar, sekme yönetimi, kirli (`*`) bayrağı.
- **Düğüm Kütüphanesi & Bağlamsal Menü:** Tek tıkla veya sağ tık koordinatında hızlı varlık yerleştirme.
- **Paralel Simülasyon & Rollback:** Simülasyon aşamalarının `SystemDAG` üzerinden eşzamanlı işletimi ve durdurulduğunda snapshot anına geri dönüş.

---

## 5. Hızlı Başlangıç

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

