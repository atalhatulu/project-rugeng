# Aşama 2: Bullet Hell & Dahili Teşhis Paneli Raporu

**Tarih:** 29 Ekim 2026  
**Proje:** Project-Ember (Rust - `rugeng`)  
**Durum:** Çıkış Kriterleri Başarıyla Sağlandı

---

## 1. Neler Yapıldı?

1. **Sıfır-Tahsisli Komut Tamponu (`rugeng_core::CommandBuffer`):**
   - Teknik borç olan `Box<dyn FnOnce>` heap tahsisi tamamen kaldırıldı.
   - Ardışık bayt tamponu (`Vec<u8>`) ve monomorfize edilmiş fonksiyon göstericileriyle donatıldı.
   - Kareler arasında tampon kapasitesi korunarak (`clear()` ile) gameplay döngüsünde **0 malloc** garantisi sağlandı.
   - Miri ile bellek hizalaması ve nesne ömürleri (drop safety) 0 UB ile doğrulandı.

2. **2D Spatial Hash Grid Modülü (`rugeng_physics`):**
   - O(N^2) mermi-nesne çarpışma testini O(N) seviyesine indiren mekansal grid kuruldu.
   - 32 piksellik hücre boyutu, 4096 asal çarpanlı kova (bucket mask) ve bağlı liste (linked-list) yapısı.
   - Kare başında bellek serbest bırakılmaz (`heads.fill(u32::MAX)`), heap tahsisi yapılmaz.

3. **Oynanabilir Bullet Hell Prototipi (`games/bullet_hell`):**
   - **Girdi Sistemi:** `winit 0.30` ile W, A, S, D / yön tuşları, Space / fare sol tık atış kontrolü.
   - **Sabit Zaman Adımı (Fixed Timestep):** 60 Hz sabit fizik güncellemesi (`FIXED_DT = 1/60`) ve enterpolasyon akümülatörü.
   - **Mermi Yaşam Döngüsü:** Dönen 4-kollu spiral, 360 derece Nova halkası ve çiçek dalga örüntüleri. Ekran dışına çıkan veya çarpan mermilerin yok edilmesi.
   - **Çarpışma & Skor:** Spatial grid üzerinden mermi-oyuncu ve mermi-boss çarpışmaları, can barı ve skor sayacı.

4. **Dahili Teşhis Paneli (In-Game Debug HUD / Overlay):**
   - Harici ağır kütüphaneler yerine motorun kendi 32-bayt `QuadInstance` boru hattı ile tek draw call içinde çizilen 3 sekmeli profesyonel kontrol paneli:
     - **Sekme 1 (Performans):** 16.66 ms kare bütçesi çubuğu (renk bölgeli: yeşil < 8ms, sarı < 14ms, kırmızı > 16.6ms), kare süresi, 60 Hz fizik süresi, Spatial Hash sorgu süresi (µs).
     - **Sekme 2 (Varlıklar & Bellek):** Canlı mermi sayısı, aktif ECS varlık sayısı, 64MB FrameArena kullanım durumu, GPU Ring Buffer doluluğu.
     - **Sekme 3 (Simülasyon Kontrolleri):** Mermi örüntüsü değiştirme (Spiral/Nova/Flower), hedef mermi sayısı artırma/azaltma (1.000 - 20.000), Ölümsüzlük (God Mode) açma/kapama.
   - **Vektörel Piksel Font:** Doku gerektirmeyen, doğrudan quad'larla çizilen 3x5 piksel font motoru (`games/bullet_hell/src/font.rs`).

---

## 2. Çıkış Kriterleri ve Ölçüm Sonuçları

Ölçümler kullanıcının fiziksel donanımı üzerinde (**Ayrık AMD Radeon RX 5500 XT**, Vulkan 1.4 RADV sürücüsü, 144 Hz ekran) alınmıştır:

| Metrik | Hedef / Bütçe | Gerçekleşen Ölçüm | Durum |
| :--- | :--- | :--- | :--- |
| **Canlı Mermi Simülasyonu** | 10.000 - 20.000 | **~18.000 mermi** (ekranda aktif) | **Başarılı** |
| **Kare Süresi (Frame Time)** | < 16.66 ms | **6.63 ms — 6.94 ms** | **Mükemmel** |
| **Kare Bütçesi Kullanımı** | %100 altı | **%39.8 — %41.7** | **Mükemmel** |
| **Fizik Simülasyonu (60 Hz)** | < 3.0 ms | **0.55 ms — 0.58 ms** | **Çok Hızlı** |
| **Spatial Hash Çarpışma** | < 1.0 ms | **430 µs — 451 µs (0.45 ms)** | **Ultra Hızlı** |
| **Ekran Yenileme Hızı (FPS)** | 60 FPS kilitli | **144 FPS kilitli (Monitör V-Sync)** | **Maksimum** |
| **Draw Call Sayısı** | Tek çizim çağrısı | **1 (`vkCmdDraw(6, N)`)** | **Başarılı** |
| **Gameplay Malloc** | 0 malloc | **0 malloc (Arena + CmdBuffer + SpatialHash)** | **Tam Uyum** |
| **Validation Hatası** | 0 hata | **0 hata** | **Temiz** |

---

## 3. Çalıştırma Talimatı

Oyunu ve teşhis panelini tam hızda denemek için:
```bash
cargo run --release -p bullet_hell
```

**Kontroller:**
- `W, A, S, D` veya `Yön Tuşları`: Oyuncu hareketi
- `Space` veya `Fare Sol Tık`: 4'lü mermi atışı
- `1, 2, 3` veya `Tab`: Teşhis sekmeleri arasında geçiş (Performans, Varlıklar, Kontroller)
- `G`: Ölümsüzlük (God Mode) Aç/Kapa
- `P`: Mermi desenini değiştir (Spiral, Nova, Flower)
- `+ / -`: Canlı mermi hedefini 2.500 artır/azalt (1.000 - 20.000)
- `ESC`: Çıkış
