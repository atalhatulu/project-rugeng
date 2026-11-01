# Project Rugeng — Motor Geliştirme & Dogfooding İlerleme Raporu

**Tarih:** 31 Ekim 2026  
**Kapsam:** Motor Çekirdeği (`rugeng_core`, `rugeng_physics`) Geliştirmeleri ve Platformer Test Doğrulaması

---

## 1. Mimari Karar ve Yaklaşım
"Sadece oyun koduna hardcoded yama yapmak" yerine **Motor Geliştirme (Engine-First Dogfooding)** yaklaşımı benimsendi. Geliştirilen özellikler önce motor crate'lerinde genel amaçlı olarak kodlandı, birim testleri yazıldı, ardından `platformer` oyunu üzerinden doğrulandı.

---

## 2. Motora Kazandırılan Yeni Yetenekler

### A. Prosedürel Ses Sistemi ve Gerçek Donanım Çıkışı (`rugeng_core::audio`)
- **Yeni SFX Önayarları (`SfxPreset`):**
  - `SfxPreset::Jump`: 140Hz -> 520Hz yukarı frekans süpürmesi (Retro Mario zıplama sesi).
  - `SfxPreset::Coin`: B5 -> E6 iki tonlu parlak çınlama efekti.
  - `SfxPreset::Stomp`: 220Hz -> 50Hz tok düşman ezme darbesi.
  - `SfxPreset::Hit`: Distorsiyonlu gürültü + düşük kare dalga hasar darbesi.
- **Asenkron Sıfır-Gecikmeli Donanım Çıkışı (`AudioEngine::play_preset`):**
  - Motor ana döngüsünü veya render hattını bloklamayan bağımsız arka plan akışı (`std::thread::spawn`).
  - Sistemde PipeWire (`pw-play`) veya ALSA (`aplay`) üzerinden doğrudan 16-bit Mono PCM akışı tetiklenir. Harici ağır C ses kütüphanelerine ihtiyaç duyulmaz.

### B. Tek Yönlü Platform Fiziği (`rugeng_physics::collision_solver`)
- **`resolve_one_way_platform`:**
  - Karakter alttan zıplarken veya blok içindeyken platformdan serbestçe geçer.
  - Sadece düşüş aşamasında (`vel.y > 0.0`) ve taban platformun üst yüzeyini geçtiğinde (`margin` tolerans penceresiyle) karakteri platformun üzerine oturtur ve dikey hızı sıfırlar.
  - `test_one_way_platform_upward_and_downward` birim testi ile matematiksel doğruluğu tescillendi.

### C. Gelişmiş Kare Harita Desteği (`rugeng_core::tilemap`)
- `TileData` yapısına serileştirilebilir `one_way: bool` desteği eklendi.
- `Tilemap2DComponent::set_tile_ex` ve `one_way_aabb_boxes()` fonksiyonları motora kazandırıldı.

---

## 3. Platformer Oyununa Entegrasyon (`games/platformer`)
- **Ses Olayları:** Karakter zıpladığında `Jump`, para topladığında `Coin`, düşmanın kafasına bastığında `Stomp`, hasar aldığında `Hit`, çukura düştüğünde veya can bittiğinde `Explosion`, bayrağa ulaştığında `Powerup` sesleri çalınır.
- **Tek Yönlü Platformlar:** İkinci tuğla yükseltisi ve çukur köprüsü tek yönlü (`one_way=true`) platforma dönüştürüldü. Artık alttan zıplanarak köprünün üstüne çıkılabiliyor.
- **Karakter Hissi (Juice & Procedural Animation):**
  - Koşma esnasında adım frekansına göre ayak salınımı ve dikey gövde sallanması (bobbing).
  - Zıplama esnasında fiziksel esneme (Squash & Stretch: en daralma, boy uzaması).

---

## 4. Test ve Derleme Durumu
- `cargo test --workspace`: **29/29 test başarıyla geçti.**
- `cargo clippy --workspace --all-targets`: **0 hata, 0 uyarı.**
- `cargo build -p platformer --release`: **14 saniyede hatasız tamamlandı.**
