# Project Rugeng — Denetim ve Test Raporu

**Tarih:** 1 Kasım 2026
**Kapsam:** Tüm workspace (8 crate + 3 oyun, ~30.400 satır, 69 dosya)
**Ortam:** CachyOS, nightly Miri (2026-08-08), RX 5500 XT

---

## 1. Doğrulama Sonuçları (çalıştırılmış, kanıtlı)

| Kontrol | Komut | Sonuç |
|---|---|---|
| Clippy | `cargo clippy --workspace --all-targets` | ✅ 0 uyarı |
| Testler | `cargo test --workspace` | ✅ 116/116 geçti |
| Format | `cargo fmt --check` | ✅ temiz |
| Bağımlılık güvenliği | `cargo audit` (277 crate) | ✅ 0 zafiyet, 1 uyarı |
| Release build + GUI smoke | `cargo build --release` + editör 10 sn çalıştırma | ✅ Vulkan RX 5500XT seçildi, kare 0.33–1.62 ms |
| Miri — çekirdek | `MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-strict-provenance" cargo +nightly miri test -p rugeng_core` | ✅ 31/31 temiz (CAS'li arena eşzamanlı test + UnsafeCell ECS dahil). 1 test hariç: `test_audio_synthesizer_wav_encoding` (aplay spawn eder, Miri desteklemez — UB değil) |
| Miri — scheduler | `MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-tree-borrows" cargo +nightly miri test -p rugeng_scheduler` | ✅ 8/8 UB yok (pool, work-stealing, Scope transmute emüle edildi) |
| Loom | `RUSTFLAGS="--cfg loom" cargo test -p rugeng_scheduler --test loom_scheduler` | ✅ 2/2 geçti |

Önemli notlar:
- `rugeng_core` Miri koşusu strict provenance + Stacked Borrows ile **tam temiz** — docs'taki "Miri: 0 UB" iddiası **doğrulandı**.
- `rugeng_scheduler` Stacked Borrows altında `crossbeam-epoch` içinde SB ihlali veriyor; bu crossbeam-epoch'ın bilinen kısıtlamasıdır, Tree Borrows altında temiz. Scheduler'ı denetlerken `-Zmiri-tree-borrows` kullanılmalı.
- Miri'nin sonda raporladığı 13 "memory leaked" tamamen crossbeam-epoch iç yapısı (SealedBag/Local); process-exit'te bırakılan epoch tasarımı, proje kodu değil.

## 2. Güvenlik Yüzeyi

- **Script sandbox (rugeng_script):** mlua-luau; 32 MB bellek limiti + interrupt tabanlı sonsuz-döngü koruması + hot reload. Luau'da os/io yok. Testli. Makul.
- **Vulkan (ash):** validation layer çalışma zamanında tespit edilip koşuluyor; FFI unsafe'leri standart desen.
- **cargo-audit uyarısı:** `ttf-parser 0.25.1` bakımsız (RUSTSEC-2026-0192). Font çözümleme zincirinde; saldırı yüzeyi düşük ama orta vadede alternative (örn. `read-fonts`) planlanmalı.

## 3. Bulunan Riskler

### R1 (YÜKSEK) — panik = kalıcı worker kaybı + deadlock
`pool.rs` worker_loop'ta görev çağrısı `j()` **catch_unwind ile sarmalanmamış**. Bir görev paniklerse:
1. worker thread'i unwind ile kalıcı ölür (havucu sessizce küçülür),
2. `active_jobs` / `active_tasks` sayacı düşmez → `wait_idle()` ve `Scope::wait()` **sonsuza dek bloklar**.

Oyun script'leri (Lua API köprüleri) görev içinde panikletebilir; gerçekçi senaryo.
**Çözüm:** görevi `catch_unwind(AssertUnwindSafe(...))` ile çağır; sayaç düşümünü Drop-guard'a al.

### R2 (YÜKSEK) — Scope: closure paniklerse use-after-free
`ThreadPool::scope` içinde `f(&scope)` paniklerse `scope.wait()` atlanır; `Scope`'un `Drop`'ı yok → kuyrukta `'_env` verisine referans taşıyan görevler kalır, worker sonra çalıştırırsa **UB**. Standart leakpunch deseni Drop'ta wait gerektirir.
**Çözüm:** `impl Drop for Scope { fn drop(&mut self) { self.wait(); } }` + R1'deki catch_unwind ile ikisi birlikte anlamlı.

### R3 (ORTA) — FrameArena seslik yüzeyi
- `unsafe impl Sync` + `alloc(&self) -> &mut T`: ayrık bölgeler verdiği sürece seslik; ancak `reset_to_marker(&self)` (unsafe) canlı `&mut T` varken çağrılırsa UB. Şu anki kullanım (reset yalnız `&mut self` üzerinden) güvenli.
- `alloc<T>` Drop çağırmaz → arenaya Drop-tipi (String/Vec) koyulursa **sessiz bellek sızıntısı**. Mevcut kullanım Copy-tipli (`alloc_slice_copy`) — iyi. `T: Copy` veya debug-assert'li `!needs_drop::<T>()` ekleme değerlendirilebilir.

### R4 (ORTA) — Registry soundness'i scheduler'a devredilmiş
`unsafe impl Send/Sync for Registry` + `storage()/storage_mut()` güvenli tip sistemiyle değil, scheduler'ın AccessSet çakışma denetimiyle korunuyor. `simulation.rs` bu kaçış kapısını doğrudan kullanıyor (seri koşumda ayrık tipler — şu an güvenli). Kural netleştirilmeli: **scheduler dışından asla `storage()` çağrılma**; API doc'una ve AGENTS/CONTRIBUTING'e yazılmalı.

### R5 (DÜŞÜK) — mühendislik hijyeni
- **Git deposu yok** (`git status` → fatal). `target/` klasörle birlikte duruyor. İlk iş: `git init` + `.gitignore` (`/target`).
- 141 `unwrap()`; en risklisi `texture.rs` `mapped_ptr().unwrap()` → GPU OOM'da render thread panikler. Editör main.rs'deki 31 unwrap prototype için tolere edilebilir.
- `ui.rs` 4588 satır — modularizasyon raporu var ama editör hâlâ tek dosyaya yığılmış; UI'ı pane'lere bölmek sıradan refactor.
- `docs/` 30+ durum raporu birikmiş; geçerli mimariyi tek SOURCE-OF-TRUTH belgede toparlamak taze katkıda bulunan için tek gerçek kaynak olur.

## 4. Öneri Sırası

1. **R1+R2 birlikte düzelt** (tek PR, ~30 satır): `catch_unwind` + Drop-guard sayaç + `Drop for Scope`. Regresyon testi: panikleyen görev içeren `pool.scope(...)` testi.
2. `git init` + `.gitignore`.
3. Miri komutlarını `start.sh check`e opsiyonel adım olarak ekle (core strict-provenance, scheduler tree-borrows, `--skip test_audio_synthesizer_wav_encoding`) — rapordaki komutlar aynen kopyalanabilir.
4. `reset_to_marker` kullanımını tamamıyla kaldırma veya `&mut self`'e çevirme seçeneğini değerlendir (mevcut tek çağrısı `ScopedScratch::drop`).
5. ttf-parser değişim planı.

**Genel hüküm:** Motor, unsafe yoğunluğuna rağmen ölçülmüş olarak temiz: 0 clippy uyarısı, 116 yeşil test, 0 CVE, çekirdek Miri-temiz (strict provenance dahil), GUI gerçek GPU'da çalışıyor. Asıl açık havuzu thread-pool panik dayanıklılığı (R1/R2) — tek küçük PR ile kapanır.

---

## 5. Düzeltme Notu (1 Kasım — uygulama sonrası)

Uygulanan düzeltmeler ve doğrulama kanıtları:

- **R1+R2 kapatıldı** (`rugeng_scheduler/src/pool.rs`): worker_loop'ta görev `catch_unwind` ile koşar (worker ölmez, sayaç düşer); `Scope::spawn` sarmalayıcısı panikte de `active_tasks` düşürür; `impl Drop for Scope` kapanış paniklerinde `wait()` garanti eder; panikleyen job'ın `join()`'i bloklamaz, net panik verir. **4 yeni regresyon testi**; Miri (Tree Borrows) 12/12 UB/data-race yok.
- **Sahne sadakati:** `ParticleEmitterComponent` serde ile serialize; canlı parçacıklar ve `emission_timer` `#[serde(skip)]` (runtime durum diske sızmaz — kanıt testi var). `SceneEntity.particle` (`#[serde(default)]` → eski dosyalarla uyumlu). Save/roundtrip testleri genişletildi.
- **EntitySnapshot sızıntısı kapatıldı:** sil-geri-al artık parçacık yayıcısını koruyor (konfig olarak).
- **Undo/redo kapsamı:** `Transform2DChanged`/`Transform3DChanged` aksiyonları eklendi; denetçi butonlarının tamamı (2D pos/rot/scale ×8, 3D pos/rot/scale ×12) artık geri alınabilir, redo destekli ve dirty-flag'li.
- **Denetçi bileşen değişlikleri de undo'da (2. tur):** `SpriteTextureChanged`, `SpriteColorChanged`, `ZIndexChanged`, `Collider2DChanged`, `ScriptChanged` — doku/renk paleti, Z-Index, sensor anahtarı ve script atama/bırakma butonları `Option<T>` semantiğiyle (None = bileşen yoktu → undo kaldırır/geri koyar) geri alınabilir. Ortak `apply_option_comp` uygulayıcısı ile. Editördeki geri-alınmayan tek bilinen yüzey: klavye metin girişi henüz olmadığı için inline isim/doku yolu düzenleme (özellik olarak yok).

Durum: 116→121→**131 test yeşil** (+4 panik regresyonu, +1 particle serileştirme kanıtı, +2 NodeName serde, +5 InlineTextEdit birim, +3 rename E2E akışı), clippy `-D warnings` temiz, fmt temiz, GUI smoke ✅ (RX 5500XT, kare döngüsü canlı, panic yok; spectacle ekran görüntüsünde sahne ağacı etiketleri düzgün çiziliyor).

**Üçüncü tur — NodeName + klavye metin girişi (F2 yeniden adlandırma):**
- `NodeName` bileşeni (core) + serde; save/load: özel ad diskte `name` alanına yazılır, üretik kalıba uymayan ad yüklemede `NodeName` olarak geri döner.
- `InlineTextEdit` (types.rs, yan etkisiz): karakter ekleme, kontrol karakteri reddi, MAX_LEN char-bazlı (Türkçe/ÇJK güvenli), backspace char-boundary güvenli. 5 birim test.
- Klavye girişi: F2 düzenleme modunu açar; Enter onay / Esc vazgeç / Backspace; yazılabilir karakterler `KeyEvent.text` + `Ime::Commit` (IBus Türkçe) ile akışa girer; mod açıkken editör kısayolları bloklanır.
- `UndoAction::NodeNameChanged` + `apply_node_name` (yaşayan-entity guard'lı).
- **Bulunan gerçek üretim hataları (test sayesinde):** (1) `NodeNameChanged` undo kolu `apply_option_comp::<String>` ile yanlış tip hedefliyordu — NodeName asla kalkmıyordu; ayrı applier yazıldı. (2) Sahne yeniden yüklendikten sonra undo, ölü entity'de `emplace` assert'iyle panic atıyordu — `apply_option_comp`/`apply_node_name` yaşayan-entity guard'ı eklendi; regresyon testi var.
- R3 kapandı: `FrameArena::alloc` debug_assert ile Drop-tipi tahsisini debug'da yakalar + doc uyarısı. R4 kapandı: `StorageCell` Send/Sync üstüne scheduler-dışı kullanım yasağı dokümante edildi.

Kalan açık kalemler: ttf-parser değişimi (RUSTSEC-2026-0192), git deposu, inline düzenlemede imleç/caret hareketi (şimdilik sona ekleme + tampon temizleme), ui.rs 4588 satır bölme borcu.

**Dördüncü tur — caret + çift tıkla adlandırma + git:**
- `InlineTextEdit` v2: karakter-tabanlı `caret`; `insert` imleç konumuna yazar, `backspace` sol-dan siler, `delete_forward` (Delete), `move_caret` (Sol/Sağ, kenetli), `Home`/`End`. Bayt kayması imkânsız (çok baytlık testler var). Ölümsüz-girdi savunması: buffer dışı caret normalize edilir (panic yok).
- Ağaç satırına **çift tıklama** ile rename: bu winit derlemesinde `MouseEvent`/`double_click` kanalı yok (0.30.13'te `WindowEvent::MouseInput` alan taşımıyor) → manuel algılama `App::is_double_click` (350 ms + 6 px penceresi). Çift tıkta IME otomatik açılır.
- Editör kısayolları rename modunda tamamen bloklu (Delete artık entity SİLMEZ, metinden karakter siler).
- **git deposu kuruldu** (`main` dalı; iç içe boş `rugeng_script/.git` iskeleti temizlendi) — ilk commit 68095e6.
- Durum: **135 test yeşil**, clippy `-D warnings` 0, release smoke ✅ (4000+ FPS).

Kalan: ttf-parser, imleç fare ile konumlandırma, ui.rs bölme, push (remote yok).
