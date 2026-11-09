# Project Rugeng (Ember Engine)

Rust ve Vulkan 1.3/1.4 ile geliştirilen, yüksek performanslı, veri odaklı (data-oriented) 2D/3D oyun motoru ve editörü.

## Genel Durum ve Kalite Metrikleri

- **Doğrulanmış Test Sayısı:** **164 Test (Tümü Yeşil)**
- **Test Baseline Raporu:** [`docs/TEST_BASELINE.md`](docs/TEST_BASELINE.md)
- **Clippy:** 0 Uyarı (`cargo clippy --workspace --all-targets -- -D warnings`)
- **Format:** Temiz (`cargo fmt --check`)
- **Güvenlik / Audit:** RUSTSEC-2026-0192 (ttf-parser) motor içi `fontdue` bağımlılığından arındırıldı ve `swash` 0.1 katmanına geçirildi.
- **Bellek ve Eşzamanlılık:** Loom ve Miri ile veri yarışı ve tanımsız davranış (UB) denetimli.

## Performans Özetleri

- **2D Sprite Çizimi:** 50.000 Sprite ortalama **1.01 ms** (p99: 3.32 ms) — hedef < 5.0 ms.
- **20k Mermi Simülasyonu:** Ortalama **0.22 ms** (p99: **3.88 ms**) — hedef < 8.33 ms.
- **Bellek Tahsisi:** Oyun döngüsü sabit durumda (steady-state) **0 malloc**.

## Hızlı Başlangıç

Projeyi derlemek ve çalıştırmak için `start.sh` yardımcı betiğini kullanabilirsiniz:

```bash
# Editörü release modunda başlat
./start.sh editor

# Geliştirici (hızlı derleme) modunda başlat
./start.sh editor-dev

# Bullet Hell prototipini başlat
./start.sh bullet-hell

# 3D Demo sahnesini başlat
./start.sh demo-3d

# Tüm test ve kod kalitesi denetimlerini çalıştır
./start.sh check
```

## Mimari Yapı

- `rugeng_core`: Flat sparse-set ECS, lock-free FrameArena, sıfır-alloc CommandBuffer, AccessSet scheduler.
- `rugeng_render`: Vulkan 1.3/1.4 Dynamic Rendering + Sync2, tek instanced `vkCmdDraw(6, N)` 2D hattı, D32_SFLOAT depth, PBR 3D pipeline (GGX/Smith/Fresnel-Schlick, Reinhard tonemapping).
- `rugeng_editor`: Godot 4 tarzı modüler dock mimarisi (`ui/`), 3D flycam, gizmo etkileşimi, inline text düzenleme ve geri alma/yineleme (undo/redo).
- `rugeng_physics`: 2D spatial hash grid, AABB/eğim (slope) çözücü, 3D Bounding Box (`AABB3D`) & Raycasting.
- `rugeng_scheduler`: Panic-safe work-stealing thread pool, DAG zamanlayıcı, Loom onaylı.
- `rugeng_script`: Luau sanal makinesi, SoA scratch view, durum korumalı sıcak yeniden yükleme (stateful hot reload).
- `rugeng_assets`: Bellek eşlemeli VFS, Handle<T>, VersionedScene v1→v2.
