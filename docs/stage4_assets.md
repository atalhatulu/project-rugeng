# Aşama 4: Varlık Yönetimi, VFS ve Sahne Sistemi Raporu (Asset, VFS & Scene)

**Tarih:** 31 Ekim 2026  
**Proje:** Project-Ember (Rust - `rugeng`)  
**Kapsam:** `crates/rugeng_assets`, `crates/rugeng_render::TextureAtlas`, `assets/`, `games/bullet_hell` Entegrasyonu  
**Durum:** Çıkış Kriterleri ve G1–G5 Başarıyla Doğrulandı  

---

## 1. Mimari Genel Bakış ve Gerçekleştirilen Hedefler (G1–G5)

### G1: Mmap VFS ve Tip Güvenli Asset Handle Sistemi (`crates/rugeng_assets`)
- **Mmap VFS (`rugeng_assets::Vfs`, `rugeng_assets::MmapFile`):**
  - Linux/Unix sistemlerde `libc::mmap` (`PROT_READ`, `MAP_PRIVATE`) ve Windows sistemlerde `CreateFileMappingA` / `MapViewOfFile` API'leri kullanılarak sıfır kopyalı dosya eşleme (zero-copy memory mapping) sağlandı.
  - Sanal dizin ve montajlama (mount point / prefix routing) desteği getirildi (`assets://` ve sanal takma adlar).
  - 0 baytlık boş dosyalar ve geçersiz dosya yolları güvenle ele alındı.
- **Asset Handle & Referans Sayımı (`rugeng_assets::Handle<T>`):**
  - Atomik `Arc<AtomicU32>` referans sayacı ve 64-bit benzersiz tanıtıcı (`id`) ile bellek sızıntısını önleyen tip güvenli handle sistemi kuruldu.
  - Varlık durumları (`LoadingState::Unloaded`, `Loading`, `Loaded`, `Failed`) ile asenkron ve senkron yükleme yaşam döngüsü modellendi.

### G2: Sahne Serileştirme ve Eski Sürüm Uyumluluğu (`rugeng_assets::VersionedScene`)
- Versiyonlu sahne formatı (Sürüm 1 - Legacy, Sürüm 2 - Modern) geliştirildi.
- Sürüm 2 ile birlikte sahneye `SceneEnvironment` (ortam rengi, yerçekimi, sahne yapımcısı metadata) alanları dahil edildi.
- Eski sürüm (V1) JSON dosyaları yüklendiğinde otomatik olarak tespit edilip V2 formatına ve varsayılan ortam ayarlarına dinamik olarak yükseltilir (`test_v1_to_v2_migration`).

### G3: Çalışma Zamanı Canlı Güncelleme (Hot Reload)
- `AssetServer::poll_hot_reload`: Disk üzerindeki dosya modifikasyon zaman damgalarını (`mtime`) takip eder.
- Doku dosyası diskte değiştiğinde (`drain_reloaded_textures`):
  - `TextureAtlas::update_existing_region` çağrılarak GPU üzerindeki `VkImage` bellek alanı, staging buffer üzerinden yerinde (in-place) ve pipeline bariyer senkronizasyonu (`TRANSFER_DST_OPTIMAL` -> `SHADER_READ_ONLY_OPTIMAL`) ile yeniden yazılır.
- Sahne dosyası diskte değiştiğinde (`drain_reloaded_scenes`):
  - ECS üzerindeki varlıklar ve oyuncu parametreleri oyun yeniden başlatılmadan güncellenir.

### G4: Hata Dayanıklılığı (Fault Tolerance & Safe Placeholders)
- Eksik veya bozuk doku dosyaları kesinlikle oyunu çökertmez (`test_missing_texture_returns_placeholder_not_panic`, `test_corrupt_texture_returns_placeholder_not_panic`).
- Bozuk veya eksik dokularda otomatik olarak 16x16 mor-siyah dama tahtası (`TextureAsset::placeholder`) devreye girer ve standart hata logu (`[ASSET HATA]`) basılır.
- Bozuk sahne dosyalarında çökme yerine güvenli `FallbackScene` oluşturulur.

### G5: Bullet Hell'in Dosyalardan Yüklenmesi (Hardcode İzolasyonu)
- `games/bullet_hell` içerisindeki tüm sabit kodlanmış piksel dizileri temizlendi.
- Oyun dünyası `assets/scenes/bullet_hell.scene` üzerinden yüklenir.
- Uzay gemisi ve boss animasyon kareleri (`assets/textures/player_ship.png`, `assets/textures/player_ship_f2.png`, `assets/textures/boss_core.png`) doğrudan VFS üzerinden mmap ile okunur.
- Dahili Teşhis Paneli'nin (HUD) Varlıklar sekmesine canlı `ASSET & VFS: MMAP + HOT RELOAD (AKTIF)` göstergesi eklendi.

---

## 2. Doğrulama ve Test Kayıtları

1. **Birim & Entegrasyon Testleri:**
   ```bash
   cargo test --workspace
   ```
   - Sonuç: `rugeng_assets` (11 test), `rugeng_core` (27 test), `rugeng_physics` (18 test), `rugeng_render` (12 test), `rugeng_scheduler` (8 test + 2 entegrasyon testi), `rugeng_editor` (9 test) hepsi başarılı (`test result: ok`). Toplam 86 test sıfır hatayla geçti.

2. **Biçimlendirme ve Linter Denetimi:**
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   ```
   - Sonuç: 0 uyarı, 0 hata.

3. **Güvenlik ve `unsafe` Gerekçelendirmeleri:**
   - `crates/rugeng_assets/src/vfs.rs` içerisindeki tüm `libc::mmap`, `libc::munmap` ve Win32 handle çağrılarında eksiksiz `// SAFETY:` açıklamaları yazıldı.
   - `crates/rugeng_render/src/texture.rs` içindeki `update_existing_region` GPU staging ve bellek kopyalama adımları Vulkan memory modeline uygun şekilde doğrulandı.

---

## 3. Yeni Eklenen ve Güncellenen Dosyalar

- **Yeni Crate:** `crates/rugeng_assets`
  - `Cargo.toml`: Crate yapılandırması.
  - `src/lib.rs`: Modül dışa aktarımları.
  - `src/vfs.rs`: `MmapFile` ve `Vfs` sıfır kopyalı dosya yöneticisi.
  - `src/handle.rs`: `Handle<T>` ve `LoadingState` referans sayımı.
  - `src/texture.rs`: `TextureAsset` ve hata placeholder dama tahtası.
  - `src/scene.rs`: `VersionedScene`, `SceneEnvironment` ve V1->V2 geçiş motoru.
  - `src/server.rs`: `AssetServer` ve `poll_hot_reload`.
- **Render Güncellemesi:**
  - `crates/rugeng_render/src/texture.rs`: `TextureAtlas::update_existing_region` (canlı doku güncelleme desteği).
- **Varlık Dosyaları:**
  - `assets/textures/player_ship.png`
  - `assets/textures/player_ship_f2.png`
  - `assets/textures/boss_core.png`
  - `assets/scenes/bullet_hell.scene`
- **Oyun Entegrasyonu:**
  - `games/bullet_hell/Cargo.toml`
  - `games/bullet_hell/src/main.rs`: VFS/AssetServer entegrasyonu, dosyalardan başlatma ve render döngüsünde hot reload taraması.
