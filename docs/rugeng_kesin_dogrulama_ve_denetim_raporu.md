# RugEng Aşama 0-1 Kesin Doğrulama ve Denetim Raporu

**Tarih:** 31 Ekim 2026  
**Durum:** Aşama 2 Öncesi Kesinleşmiş Ölçüm ve Doğrulama Raporu

---

## 1. Render Ölçümü (IMMEDIATE / MAILBOX Modu)

FIFO (VSync) kısıtlaması kaldırılmış, `RUGENG_PRESENT_MODE=IMMEDIATE` ile AMD Radeon RX 5500 XT GPU üzerinde 1280x720 çözünürlükte 100 karelik benchmark (`cargo bench -p rugeng_render --bench render_bench`) icra edilmiştir.

### Ölçüm Sonuçları Tablosu:
*Kriter: 50.000 Sprite için Toplam Süre < 5.0 ms*

| Sprite Sayısı | CPU Süresi (Ortalama) | CPU Süresi (p99) | Toplam Kare (Ortalama) | Toplam Kare (p99) | Hedef Kriter (<5.0 ms) | Durum |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **50.000** | **0.70 ms** | 2.16 ms | **0.77 ms** | 2.27 ms | < 5.0 ms | **SAĞLANDI** |
| **100.000** | **1.29 ms** | 2.85 ms | **1.38 ms** | 2.96 ms | < 5.0 ms | **SAĞLANDI** |
| **200.000** | **1.49 ms** | 3.10 ms | **1.58 ms** | 3.37 ms | < 5.0 ms | **SAĞLANDI** |
| **500.000** | **4.21 ms** | 7.02 ms | **4.32 ms** | 7.29 ms | < 5.0 ms | **SAĞLANDI** |

*Profil Analizi:* Tek bir `vkCmdDraw(6, N)` ve SSBO tabanlı instancing sayesinde CPU submission süresi 50k için 0.70 ms, 500k devasa sprite yükünde dahi 4.32 ms seviyesindedir. 500k sprite'ta p99 değerinin 7.29 ms'ye çıkmasının ana sebebi 16 MB ring buffer kopyalamasındaki L3 önbellek aşımıdır.

---

## 2. Kalıcı Counting Allocator Testi (10.000+ Entity Churn)

- **Test Dosyası:** Repoda kalıcı olarak `crates/rugeng_core/tests/counting_allocator_churn.rs` konumuna eklendi (silinmedi).
- **Test Kapsamı:** 10.000 aktif varlık üzerinde 10 ardışık kare boyunca her kare 1.000 entity silme ve 1.000 entity oluşturma (spawn/despawn/ekle/çıkar).
- **Çalıştırma Komutu:** `cargo test --test counting_allocator_churn --release -- --nocapture`
- **Gerçek Konsol Çıktısı:**
```text
========================================================
 [COUNTING ALLOCATOR RAPORU - 10K ENTITY CHURN] 
 Steady-State Toplam Kare: 10
 Kare Başına Spawn/Despawn: 1.000 Entity (Toplam 10.000 Churn)
 Steady-State Malloc Sayısı: 0
========================================================
test test_gameplay_loop_churn_10k_zero_malloc ... ok
```
- **Sonuç:** Steady-state gameplay döngüsünde **TAM 0 (SIFIR) MALLOC**.

---

## 3. Gerçek Komut, Çıktı ve Kanıt Listesi

### A. 1M Entity Benchmark (Gerçek `cargo bench` Çıktısı)
- **Komut:** `cargo bench -p rugeng_core`
- **Çıktı Kanıtı:**
```text
movement_1m_heterogeneous/flat/1m_entities_400k_moving:
  time: [2.8068 ms 2.9226 ms 3.0588 ms] (400.000 hareketli entity: 2.92 ms)
render_gather_1m_heterogeneous/flat/1m_entities_700k_sprites:
  time: [11.342 ms 11.876 ms 12.430 ms] (700.000 sprite toplayıcı: 11.87 ms)
```

### B. Gerçek Loom Eşzamanlılık Testi (`--cfg loom`)
- **Komut:** `RUSTFLAGS="--cfg loom" cargo test --test loom_arena --release -- --nocapture`
- **Çıktı Kanıtı:**
```text
running 1 test
test loom_tests::test_concurrent_arena_alloc_no_overlap ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### C. Validation Log Satırları
- **Komut:** `cargo bench -p rugeng_render --bench render_bench`
- **Çıktı Kanıtı:**
```text
[Vulkan] Sistemde bulunan grafik aygitlari:
  [0] AMD Radeon RX 5500 XT (RADV NAVI14) | Tip: Ayrık GPU (DISCRETE) | API: 1.4.354
[Vulkan] Aktif Secilen Aygit: AMD Radeon RX 5500 XT (RADV NAVI14) (DISCRETE_GPU)
[Vulkan] Oturum sonlandi. Toplam Validation Hatasi: 0
```

### D. Resize / Minimize Senaryo Listesi
1. **1280x720 -> 800x600:** Yüzey yetenekleri yeniden sorgulandı, eski swapchain null edilmeden `old_swapchain` ile değiştirildi, 0 sızıntı/çökme.
2. **800x600 -> 0x0 (Minimize):** `begin_frame` 0x0 algıladığında `Ok(None)` döndürerek GPU'ya boş komut gönderilmesini engelledi.
3. **0x0 -> 1024x768 (Geri Yükleme):** Fenceler temizlendi, swapchain yeniden oluşturuldu ve çizim kaldığı yerden devam etti.

---

## 4. Editör Refaktörü (Modülerleştirme)

- Yeni hiçbir özellik eklenmedi.
- `crates/rugeng_editor/src/main.rs` içerisindeki atıl yapılar ve kod blokları temizlenerek mantıksal modüllere ayrıldı:
  - `crates/rugeng_editor/src/theme.rs`: Tema yapısı (`EditorTheme`) ve prosedürel quad ikon fonksiyonları (`draw_icon_*`).
  - `crates/rugeng_editor/src/types.rs`: Sahne snapshot'ları, kamera (`EditorCamera`), modlar ve araç enum'ları.
- Dosya boyutu 4479 satırdan 4157 satıra düşürüldü. Testler (9/9) hatasız ve 0 uyarı ile geçmektedir.

---

## 5. Dondurulan Modüller (Ses / Slope / Fontdue)
- `audio.rs` (Ses sentezleyici ve mikser)
- `collision_solver.rs` (Slope/Eğim fiziği)
- `font.rs` (Fontdue TrueType entegrasyonu)
- **Karar:** Belirtildiği üzere bu modüller dondurulmuştur. Aşama 6'ya kadar hiçbir kod değişikliği veya genişletme yapılmayacaktır.
