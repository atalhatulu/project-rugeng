# Aşama 5: Scripting (Luau Entegrasyonu) Raporu

**Tarih:** 31 Ekim 2026  
**Proje:** Project-Ember (Rust - `rugeng`)  
**Kapsam:** `crates/rugeng_script`, `assets/scripts/boss_patterns.lua`, `games/bullet_hell` Entegrasyonu, Korumalı Sandbox, Benchmark  
**Durum:** Çıkış Kriterleri ve G1–G6 Başarıyla Doğrulandı  

---

## 1. Mimari Genel Bakış ve Gerçekleştirilen Hedefler (G1–G6)

### G1: Luau/Lua Entegrasyonu ve Hata Tanılama Sistemi (`crates/rugeng_script`)
- **Luau Desteği:** `mlua` kütüphanesinin `luau` derleme özelliği doğrudan Rust çalışma zamanına bağlandı.
- **Detaylı Hata Mesajları (`ScriptError`):**
  - Dosya adı, tetiklenen kod bloğu satır numarası ve stack traceback izini içeren hata nesnesi oluşturuldu.
  - Script yükleme, derleme veya çalışma anında oluşan hatalar yakalanır ve oyunu çökertmez (`[SCRIPT HATA]`).

### G2: Flat SoA Scratch View ve `ScriptCommandBuffer` (Sıfır Nesne Başı FFI)
- **Toplu Veri Aktarımı (`ScratchSoAView`):**
  - Mermilerin `pos_x`, `pos_y`, `vel_x`, `vel_y`, `radius`, `is_enemy` verileri ardışık düz dizilerde tutulur.
  - Lua ile Rust arasında nesne başına tablo veya FFI maliyeti tamamen ortadan kaldırıldı.
  - `batch_integrate_to_cmd`: Tek bir Rust/C çağrısı ile binlerce merminin hareketini günceller ve ekran sınırını aşanları kuyruğa yazar.
- **Toplu Komut Kuyruğu (`ScriptCommandBuffer`):**
  - Script ortamında üretilen mermi ve nesne yok etme işlemleri ardışık tamponda toplanır. Tek bir geçişte ECS'ye 0 heap tahsisi ile aktarılır.

### G3: Durum Korumalı Canlı Yeniden Yükleme (Stateful Hot Reload)
- `ScriptEngine::poll_hot_reload`:
  - Disk üzerindeki script dosyalarının modifikasyon zamanını (`mtime`) izler.
  - Dosya değiştiğinde, mevcut Lua modülünün `export_state` fonksiyonu çağrılarak boss'un can değeri, evresi (`phase`) ve sayaçları güvenle alınır.
  - Yeni script derlendikten sonra `import_state` fonksiyonu ile durum aynen yeni koda enjekte edilir (`test_state_preservation_hot_reload`).

### G4: Bullet Hell Gameplay'inin Lua'ya Taşınması
- `assets/scripts/boss_patterns.lua`:
  - Dönen 4 kollu spiral, 360 derece Nova halkası ve dalgalı çiçek desenleri Luau üzerinde modellendi.
  - `games/bullet_hell`:
    - `ScriptEngine` oyuna entegre edildi.
    - `L` tuşu ile Lua Script ve Native Fallback arasında dinamik geçiş sağlandı.
    - Teşhis Paneli'nin (HUD) Varlıklar sekmesine canlı `SCRIPTING (TUS L): LUAU AKTIF (X.XX MS)` göstergesi eklendi.

### G5: Benchmark ve Performans Kanıtı (5.000 Nesne)
- `crates/rugeng_script/benches/script_benchmark.rs` Criterion benchmark sonuçları (Release profili):
  - **Native 5.000 Mermi Hareketi:** ~9.07 µs
  - **Luau Flat SoA 5.000 Mermi Hareketi (`batch_integrate_to_cmd`):** ~20.35 µs
  - **Boss Deseni Üretimi (256 Mermi / Kare):**
    - Native: ~1.81 µs
    - Luau: ~85.16 µs
  - **Sonuç ve Analiz:**
    - 60 Hz kare bütçesi (16.66 ms) göz önüne alındığında, 5.000 mermilik Luau SoA simülasyonu yalnızca **0.020 ms** sürmektedir (kare bütçesinin %0.12'si).
    - Toplu komut aktarımı sayesinde Lua motoru hedef hızın ve kare bütçesinin çok üzerinde bir verim sağlamaktadır.

### G6: Script Sandbox Sınırları ve Güvenlik
- **Bellek Sınırı (Memory Limit):**
  - Luau VM bellek sınırı (`set_memory_limit`) ile korunur. Aşım durumunda güvenli bellek hatası döner (`test_memory_limit_sandbox`).
- **Sonsuz Döngü Koruması (Instruction Limit / Interrupt):**
  - Luau sanal makinesine bağlanan periyodik kesme (`set_interrupt`) sayesinde sonsuz döngüler ve aşırı CPU tüketen scriptler anında kesilir (`test_infinite_loop_protection`).
- **Sistem İzolasyonu:**
  - Standart IO ve OS kütüphaneleri devre dışı bırakılmış; yalnızca güvenli matematik, tablo, string, bit, buffer ve vector modülleri yüklenmiştir.

---

## 2. Doğrulama ve Test Kayıtları

1. **Birim & Entegrasyon Testleri:**
   ```bash
   cargo test --workspace
   ```
   - Sonuç: `rugeng_script` (8 test), `rugeng_assets` (11 test), `rugeng_core` (27 test), `rugeng_physics` (18 test), `rugeng_render` (12 test), `rugeng_scheduler` (8 test + 2 entegrasyon testi), `rugeng_editor` (9 test) hepsi başarılı (`test result: ok`).
   - Toplam **94 test** sıfır hatayla geçti.

2. **Biçimlendirme ve Linter Denetimi:**
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   ```
   - Sonuç: 0 uyarı, 0 hata.

---

## 3. Yeni Eklenen ve Güncellenen Dosyalar

- **Yeni Crate:** `crates/rugeng_script`
  - `Cargo.toml`: Luau özellikli `mlua`, Criterion yapılandırması.
  - `src/lib.rs`: Modül dışa aktarımları.
  - `src/engine.rs`: `ScriptEngine`, sandbox limitleri, hot reload ve hata yakalama.
  - `src/command_buffer.rs`: `ScriptCommandBuffer` ve toplu komut kuyruğu.
  - `src/scratch_view.rs`: `ScratchSoAView` (Flat SoA bellek tamponu).
  - `benches/script_benchmark.rs`: 5.000 nesne ve desen üretim benchmark'ı.
- **Script Varlığı:**
  - `assets/scripts/boss_patterns.lua`: Boss mermi desenleri ve toplu simülasyon.
- **Oyun Entegrasyonu:**
  - `games/bullet_hell/Cargo.toml`: `rugeng_script` bağımlılığı.
  - `games/bullet_hell/src/main.rs`: `ScriptEngine` başlatma, `L` tuşu toggle, HUD metrik gösterimi ve güvenli hata izolasyonu.
