# Project Rugeng — Doğrulanmış Test Referans Tabanı (Test Baseline)

**Tarih:** 5 Kasım 2026  
**Platform:** Linux (CachyOS x86_64) | AMD Radeon RX 5500 XT (RADV Vulkan 1.4)  
**Rust Versiyonu:** rustc 1.97.1  
**Komut:** `cargo test --workspace`

---

## 1. Crate Bazlı Test Dağılımı

| Crate / Hedef | Test Türü | Başarılı Test Sayısı | Durum |
| :--- | :--- | :--- | :--- |
| **`rugeng_core`** | Birim Testleri (`src/lib.rs`) | **46** | Başarılı (Yeşil) |
| **`rugeng_core`** | Entegrasyon: Counting Allocator (`tests/counting_allocator_churn.rs`) | **1** | Başarılı (Yeşil) |
| **`rugeng_core`** | Entegrasyon: Loom Arena (`tests/loom_arena.rs`) | **1** | Başarılı (Yeşil) |
| **`rugeng_editor`** | Birim & UI/Dock Testleri (`src/main.rs`) | **44** | Başarılı (Yeşil) |
| **`rugeng_physics`** | Birim Testleri (`src/lib.rs`) | **28** | Başarılı (Yeşil) |
| **`rugeng_render`** | Birim, Font, Mesh, PBR & Gölge Testleri (`src/lib.rs`) | **22** | Başarılı (Yeşil) |
| **`rugeng_scheduler`** | Birim & ThreadPool Testleri (`src/lib.rs`) | **12** | Başarılı (Yeşil) |
| **`rugeng_scheduler`** | Eşzamanlılık Doğrulaması (`tests/loom_scheduler.rs`) | **2** | Başarılı (Yeşil) |
| **`rugeng_assets`** | VFS, Mmap, Sürümleme, glTF & Pak Testleri (`src/lib.rs`) | **18** | Başarılı (Yeşil) |
| **`rugeng_script`** | Luau Sandbox & SoA Testleri (`src/lib.rs`) | **9** | Başarılı (Yeşil) |
| **TOPLAM DOĞRULANMIŞ TEST** | Workspace Genel | **183** | **TÜMÜ GEÇTİ (0 HATA)** |

---

## 2. Eşzamanlılık ve Bellek Güvenliği Doğrulaması (Loom & Miri)

1. **Loom Eşzamanlılık Testleri:**
   - `RUSTFLAGS="--cfg loom" cargo test --test loom_scheduler`: 2/2 test başarılı (Veri yarışı ve dead-lock yok).
2. **Miri Bellek Güvenliği (Strict Provenance & UB):**
   - `cargo +nightly miri test -p rugeng_physics --lib`: 20/20 test 0 UB ile başarılı.
   - `cargo +nightly miri test -p rugeng_core --lib arena`: 0 UB, strict provenance uyumlu.

---

## 3. Performans Eşik Kriterleri (Baseline Thresholds)

- **2D Sprite Çizim Performansı (`render_bench`):**
  - **Hedef:** 50.000 Sprite Toplam Süre < 5.0 ms.
  - **Ölçülen:** 50.000 Sprite CPU: 0.89 ms (p99: 3.20 ms) | Toplam: **1.01 ms** (p99: 3.32 ms) — **HEDEF SAĞLANDI**.
  - **Stres:** 200.000 Sprite Toplam: **1.67 ms**, 500.000 Sprite Toplam: **4.34 ms**.
- **Bullet Hell Simülasyonu (`games/bullet_hell`):**
  - **Hedef:** 20.000 mermi simülasyonunda p99 kare süresi < 8.33 ms (60 FPS bütçesinin %50'si).
  - **Ölçülen:** Ortalama 0.22 ms | p99: **3.88 ms** — **HEDEF SAĞLANDI**.
  - **Steady-State Heap Tahsisi:** 0 malloc.
