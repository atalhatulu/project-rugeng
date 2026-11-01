# Project Rugeng (Project Ember) — Mimari Tarama, Eksikler ve Gelistirme Yol Haritasi

`/home/teha/Documents/GitHub/project-rugeng` calisma alanindaki tum crate'ler (`rugeng_core`, `rugeng_render`, `rugeng_physics`, `rugeng_editor`, `rugeng_standalone`, `games/bullet_hell`), shader kaynaklari, sahne dosyalari ve dokumanlar satir satir incelenmistir.

---

## 1. Cekirdek Katman (`rugeng_core`) — Bellek Guvenligi (UB) ve Mimari Aciklar

### 1.1. `FrameArena` ve `ScopedScratch` Omur Sizintisi (Use-After-Free & Mutable Aliasing)
- **Risk (`crates/rugeng_core/src/arena.rs`):** `ScopedScratch::arena(&self) -> &'a FrameArena` metodu `&'_ self` omru yerine dis `'a` omrunu dondurur. `ScopedScratch` kapsam disina cikip `Drop` icinde `reset_to_marker` cagirdiginda, `scratch.arena().alloc(...)` ile alinan `&'a mut T` referanslari yasamaya devam eder. Sonraki `alloc` cagrisi ayni bellek bolgesini yeniden tahsis ederek guvenli Rust icinde iki adet `&mut T`'nin ayni adresi gostermesine (Undefined Behavior) yol acar.
- **Gereksiz Cift Kopyalama:** `rugeng_editor`, `bullet_hell` ve `sprites_50k` her karede once heap uzerinde `Vec::with_capacity(...)` olusturup doldurmakta, ardindan `arena.alloc_slice_copy(&instances)` ile arenaya, oradan da `pipe.upload_instances` ile GPU tamponuna kopyalamaktadir.
- **Alternatif Yaklasim:** `ScopedScratch` `&'a mut FrameArena` alip `arena(&mut self) -> &FrameArena` dondurmelidir. Render toplama asamasinda ise ara `Vec` ve `FrameArena` kopyasi tamamen kaldirilarak dogrudan `Pipeline2D`'nin persistent-mapped GPU dilimine (`&mut [MaybeUninit<QuadInstance>]`) yazilmalidir.

### 1.2. `CommandBuffer` Hizalama (Alignment) UB, Panic ve Unwind Sizintisi
- **Risk (`crates/rugeng_core/src/command_buffer.rs`):** `emplace` ve `defer` hizalama dolgusunu (`padding`) o anki `self.buffer.as_ptr() as usize + current_len` adresine gore hesaplar. Ancak `self.buffer` 1-bayt hizali bir `Vec<u8>`'dir. Ayni karede sonraki bir komut `reserve` ile yeniden tahsis (`realloc`) tetiklediginde yeni taban adresinin hizalamasi degisebilir ve `std::ptr::read` hizasiz isaretci okudugu icin UB uretir. Ayrica ayni karede bir varlik `destroy` edilirken baska bir sistem `emplace` kuyruklarsa `assert!(self.is_alive(entity))` yuzunden oyun coker.
- **Alternatif Yaklasim:** Arka plan depolamasi `Vec<u8>` yerine 16-bayt hizali bir tamponda (`#[repr(align(16))]` blok veya `std::alloc::alloc`) tutulmali ve `apply_emplace` icinde `if reg.is_alive(entity)` kontrolu yapilmalidir.

### 1.3. `SparseSet::insert` Jenerasyon Cakismasi ve `Registry` API Eksikligi
- **Risk (`crates/rugeng_core/src/sparse_set.rs`):** `SparseSet::insert` icinde `sparse.get(index)` doluysa, yalnizca `dense_entities[dense_idx] == entity` (ayni jenerasyon) oldugunda uzerine yazar. Farkli jenerasyonlu bir `Entity` dogrudan eklenirse eski elemani temizlemek yerine `dense` dizilerinin sonuna ikinci bir kayit ekler ve eski kayit silinemez bir zombi olarak kalir.
- **`Entity::NULL` Cakismasi (`crates/rugeng_core/src/types.rs`):** `Entity::new(ENTITY_INDEX_MASK, ENTITY_GEN_MASK)` degeri `0xFFFF_FFFF` (`u32::MAX`) uretir ve `Entity::NULL` ile cakisir.
- **`unsafe` Kirliligi (`crates/rugeng_core/src/registry.rs`):** `Registry` uzerinde guvenli `get<T>(&self, e)`, `get_mut<T>(&mut self, e)` ve `remove<T>(&mut self, e)` metotlari olmadigi icin editorde 60'tan fazla yerde `unsafe { self.registry.storage_mut::<T>() }` cagrilmistir.

---

## 2. Render Katmani (`rugeng_render`) — Vulkan 1.3 ve GPU Bellek Hatalari

### 2.1. `VulkanContext::drop` Icinde Yikici Use-After-Free (`VkDevice` Siralama Hatasi)
- **Risk (`crates/rugeng_render/src/context.rs:744-779`):** `VulkanContext` yapisinin `allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>` alani, `VulkanContext::drop` tamamlandiktan sonra Rust tarafindan otomatik dusurulur. Ancak `VulkanContext::drop` icinde satir 767'de `self.device.destroy_device(None)` cagrilmaktadir. Hemen ardindan `allocator` dusuruldugunde yok edilmis `VkDevice` uzerinde `vkFreeMemory` cagrilarak erisim ihlaline yol acar.
- **Alternatif Yaklasim:** `allocator` alani `ManuallyDrop` veya `Option` icinde tutulmali ve `destroy_device` cagrilmadan once acikca dusurulmelidir.

### 2.2. Shader'da Rotasyon Eksikligi ve Derleme Boru Hatti (`build.rs`) Olmamasi
- **Risk (`crates/rugeng_render/src/pipeline_2d.rs`, `shaders/quad.vert`):** `Transform2D.rotation` hem betiklerde (`RotatorScript`), hem editor gizmo'sunda hem de Inspector'da degistirilmesine ragmen `QuadInstance` ve `quad.vert` icinde rotasyon alani yoktur; hicbir sprite ekranda donmez. Ayrica `build.rs` olmadigi icin shader degisiklikleri otomatik derlenmez.
- **Alternatif Yaklasim:** 32-bayt hizalamasini bozmadan `QuadInstance.tex_id: u32` alaninin ust 16 bitine aciyi (`u16` kuantize radyan) paketleyip `quad.vert` icinde 2D donusum matrisi uygulamaktir.

### 2.3. `sprites_50k.rs` Render Pass Ihlali, Semafor Indeksleme ve `TextureAtlas` Darbogazi
- **Render Pass Ihlali (`examples/sprites_50k.rs:230-244`):** `ctx.draw_clear` goruntuyu `PRESENT_SRC_KHR` durumuna sokup render pass'i kapatir; ardindan cagrilan `pipe.draw` aktif bir Dynamic Rendering pass'i olmadan calisir.
- **Semafor Boyutu (`context.rs:649`):** `render_finished_semaphores` dizisi swapchain goruntu sayisi (`images.len()`) yerine `FRAMES_IN_FLIGHT = 2` boyutundadir.
- **`TextureAtlas` Senkron Bloke (`texture.rs:167-338`):** Her doku yuklemesinde yeni staging buffer acilip `queue_wait_idle` ile GPU tam bloke edilmektedir. Harici `.png`/`.jpg` yukleme destegi ve paneller icin donanimsal `vkCmdSetScissor` kirpmasi yoktur.

---

## 3. Fizik (`rugeng_physics`) ve Alt Sistem Kusurlari

### 3.1. `SpatialHashGrid` ve `collision_solver` Hatalari
- **Hucre Sinirinda Carpisma Kacirma (`spatial_hash.rs:58-108`):** `insert` nesneyi yaricapina bakmaksizin yalnizca merkez noktasinin dustugu tek hucreye ekler; `query_radius` ise komsu hucredeki genis yaricapli nesneleri kacirabilir. Ayrica hash cakismasinda ayni kova birden fazla kez taranarak mukerrer hasar callback'i tetikler.
- **`resolve_aabb_collision` (`collision_solver.rs:41-66`):** Iki statik nesne cakistiginda hicbirini hareket ettirmemesine ragmen `true` dondurur ve carpisma normali dogrultusundaki hiz bilesenini sifirlamaz.

### 3.2. `AudioEngine`, `ParticleEmitter` ve `.scene` Veri Kaybi
- **Ses Cikisi Olmayan `AudioEngine` (`audio.rs:162-167`):** `play()` yalnizca sayac artirir; uretilen WAV verisini isletim sistemi ses cikisina (`alsa`/`pipewire`/`rodio`) gonderen hicbir kod yoktur.
- **Her Zaman `(0, 0)` Noktasinda Patlayan Parcaciklar (`particle.rs:57`):** `new_explosion` icinde `burst(max_particles, Vec2::ZERO)` cagrildigi icin patlama efekti secili nesnenin konumunda degil, her zaman `(0, 0)` dunya merkezinde patlar.
- **`.scene` Serilestirme Veri Kaybi (`scene.rs` ve `rugeng_editor/src/main.rs`):** Sahne kaydedildiginde `Tilemap2DComponent`, `Animation2DComponent`, `Text2DComponent`, `Parent`/`Children` hiyerarsisi, `Collider2D.is_sensor` bayragi ve betik parametreleri dosyaya yazilmaz, yuklenirken kaybolur.

---

## 4. Editor (`rugeng_editor`), `rugeng_standalone` ve `bullet_hell` Hatalari

1. **Monolitik Dosya Yapisi (`rugeng_editor/src/main.rs`):** 3.657 satirlik tek bir dosyada hem ECS bilesenleri, hem sahne serilestirme, hem UI cizimi hem de pencere olaylari tutulmakta; bileşenler (`Transform2D`, `SpriteColor`, `Collider2D`, `Velocity2D`) ve `font.rs` crate'ler arasinda kopyalanmaktadir.
2. **Sahne Sekmesi (`[+]` Yeni Sahne) Tiklama Kaymasi (`rugeng_editor/src/main.rs:1117` vs `2592`):** Cizim sirasinda sekme genisligi dosya adindan (`main.scene`), tiklama sirasinda ise tam yoldan (`scenes/main.scene`) hesaplandigi icin `[+]` butonuna tiklandiginda yeni sahne acmak yerine eski sahne yeniden yuklenir.
3. **`Text2D` ve `Tilemap2D` Secim Gizmo'su Gorunmezligi (`rugeng_editor/src/main.rs:1295`):** Secim cercevesi ve gizmo yalnizca `SpriteColor` tasiyan varliklar icin cizildiginden, metin ve harita dugumleri secildiginde tuvalde gizmo gorunmez.
4. **Patlama Efektinin Hayalet Varlik Birakmasi (`rugeng_editor/src/main.rs:891-919`):** Her patlama tetiklendiginde olusturulan `p_ent` varligi efekt bittiginde silinmez; sahne agacinda birikir.
5. **Y-Ekseni Tersligi ve Sensor Yoksayilmasi:** Editor (`+Y` asagi) ile `rugeng_standalone` (`+Y` yukari) zit Y ekseni kullanir. Editordeki `update_simulation` `Collider2D.is_sensor` bayragini kontrol etmeden fiziksel itme uygular.
6. **Kirik Undo/Redo ve Duplicate:** `redo()` icinde `CreateEntity` ve `DeleteEntity` islevsizdir; fare ile tasima/boyutlandirma `undo_stack`'e kaydedilmez.
7. **`rugeng_standalone` Icinde Kalp ve Kurenin Mermiyle Vurulmasi (`rugeng_standalone/src/main.rs:336-357`):** `ScriptComponent` tasiyan tum varliklar dusman sayildigi icin sahnedeki `HealthHeart_Pickup` ve `EnergyOrb_Relic` mermiyle vurularak patlatilir.
8. **`bullet_hell` HUD ve Game Over Eksikligi (`games/bullet_hell/src/main.rs`):** HUD metni mermi sayisi icin yanlis tuslari (`YUKARI/ASAGI` yerine `+/-`) gosterir, oyuncu cani `0`'a dustugunde oyun bitmez ve kullanilmayan bir `TextureAtlas` GPU belleginde yer kaplar.
9. **Calisma Alani Hijyeni:** `docs/` altinda baska bir projeye ("Doodle Survivors" / GDScript) ait 2 dokuman (`temel_karakter_istatistikleri_sistemi.md`, `pasif_esya_sistemi_raporu.md`) bulunmakta ve `cargo clippy --workspace --all-targets` 23 uyari uretmektedir.

---

## 5. Onceliklendirilmis Eylem Plani

1. **Faz 1 — Acil Bellek Guvenligi ve Cokme Duzeltmeleri:**
   - `VulkanContext::drop` icinde `allocator` dusurulme sirasinin duzeltilmesi ve semaforlarin `images.len()` boyutuna cikarilmasi.
   - `CommandBuffer` 16-bayt hizali tampona gecisi ve `apply_emplace` icinde `is_alive` kontrolu.
   - `ScopedScratch` omur imzasinin ve `SparseSet::insert` jenerasyon cakismasinin duzeltilmesi.
2. **Faz 2 — Mimari Birlestirme ve Render/Fizik Dogrulugu:**
   - Ortak bilesenlerin (`Transform2D`, `Collider2D`, `Velocity2D`, `SpriteColor`, `SpriteTexture`) `rugeng_core` altina, `font.rs` modulunun `rugeng_render` altina tasinmasi.
   - `QuadInstance` ve `quad.vert` 2D rotasyon desteginin eklenmesi ve Y-ekseni yonunun standartlastirilmasi.
   - `SpatialHashGrid` yaricap ve kova tekillestirme duzeltmesi.
3. **Faz 3 — Editor Modulerlestirmesi ve Alt Sistemlerin Tamamlanmasi:**
   - `rugeng_editor/src/main.rs` dosyasinin modullere bolunmesi, `[+]` sekme tiklama hatasinin, gizmo gorunmezliginin, hayalet parcacik varliklarinin ve Undo/Redo sisteminin onarilmasi.
   - `.scene` serilestirme semasinin `Tilemap2D`, `Animation2D`, `Text2D` ve hiyerarsiyi kapsayacak sekilde genisletilmesi.
   - `AudioEngine` gercek ses cikisi baglantisi, `Doodle Survivors` dokumanlarinin temizlenmesi ve 23 Clippy uyarisinin sifirlanmasi.
