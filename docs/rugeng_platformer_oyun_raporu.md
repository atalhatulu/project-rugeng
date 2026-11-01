# Project Rugeng: 2D Platformer (Mario Tarzı Prototip) Raporu

**Tarih:** 30 Ekim 2026  
**Hedef Paket:** `games/platformer` (`/home/teha/Documents/GitHub/project-rugeng/games/platformer`)  
**Durum:** Başarıyla Tamamlandı ve Optimize Derlendi

---

## 1. Mimari Tasarım ve Gerçekleştirilen Sistemler

Projede Mario 1-1 tasarımından esinlenen eksiksiz bir 2D platform oyunu mekaniği kurulmuştur:

### A. Kinematik Karakter Kontrolcüsü (Kinematic Character Controller)
- **Yerçekimi ve Düşüş İvmesi:** $980.0\text{ px/s}^2$ aşağı yönlü yerçekimi ivmesi.
- **Yatay Koşu ve Zemin Sürtünmesi:** $1400.0\text{ px/s}^2$ yatay ivmelenme, $280.0\text{ px/s}$ hız tavanı ve tuş bırakıldığında $1200.0\text{ px/s}^2$ zemin sürtünmesi ile kayarak durma.
- **Değişken Zıplama Gücü (Variable Jump Height):** Oyuncu zıplama tuşuna (`Space` / `W` / `Yukarı Ok`) basılı tuttuğu sürece tam yükselir, tuşu erken bırakırsa dikey hız kesilerek kısa zıplama (short hop) yapılır.
- **Zemin Kontrolü (`is_on_floor`):** Zemin teması fizik motorunun `resolve_aabb_collision_with_velocity` fonksiyonundan dönen normal vektörleri ile doğrulanır; havadayken zıplama engellenir.

### B. Prosedürel Mario Tarzı Seviye ve Çevre Düzeni
- **Tilemap2D Entegrasyonu:**
  - $32\text{ px}$ ızgara boyutunda zemin katmanı (`TileType::Grass` ve `TileType::Stone`).
  - Çukurlar (Pits): $gx = 25..28$ ve $gx = 65..68$ aralıklarında zemin boşlukları.
  - Havada asılı tuğla ve metal platform blokları (`TileType::Metal`).
  - Merdiven blokları ve boru (pipe) sütunları.
  - Bölüm sonu bayrak kaidesi ve bayrak direği ($X \ge 3340\text{ px}$).

### C. Düşman ve Toplanabilir Eşya Mekanikleri
- **Goomba Benzeri Devriye Düşmanları:** Sabit hızla yatay yürüyen, taş veya duvar bloklarına çarptığında yön tersleyen düşmanlar.
- **Kafaya Basarak Ezme (Stomp Attack):** Oyuncu düşmana yukarıdan düşerek bastığında düşman yok edilir, oyuncu yukarı doğru geri zıplatılır ($360.0\text{ px/s}$) ve $+200$ skor eklenir.
- **Gövde Teması ve Geri Tepme (Knockback):** Yandan temas durumunda oyuncu can kaybeder (maksimum 3 can) ve geriye doğru fırlatılır.
- **Altın Toplama (Coins):** Platformların üzerine dizilmiş altın paralar oyuncu temas ettiğinde $+100$ skor verir ve yok edilir.

### D. Kamera ve Ekran Sınırları
- **Yumuşak Kamera Takibi (Lerp):** Kamera oyuncunun yatay konumunu $dt \times 6.0$ katsayısıyla pürüzsüz takip eder; ekranın sol sınırının ($0.0$) gerisine geçiş engellenir.
- **Ölüm Çukuru (Pit Death):** $Y > 720.0\text{ px}$ altına düşüldüğünde oyuncu doğrudan can kaybeder ve oyun biter.
- **Game Over & Stage Clear:** Can bittiğinde veya bayrak direğine ulaşıldığında oyun ekranı durur; `[R]` tuşuna basılarak seviye anında yeniden başlatılır.

---

## 2. Doğrulama ve Derleme

- **`cargo check -p platformer`:** 0 hata, 0 uyarı.
- **`cargo clippy -p platformer --all-targets`:** 0 hata, 0 uyarı.
- **`cargo build -p platformer --release`:** Sıfır hata ile optimize derlendi.
