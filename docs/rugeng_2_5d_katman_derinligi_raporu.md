# Project Rugeng — 2.5D Katman Derinliği ve Z-Index Sıralama Raporu

**Tarih:** 31 Ekim 2026  
**Konu:** 2D Sahnenin 3D Görünümde Katman Derinliği (2.5D Hibrit Modu) ve Z-Index Sıralama Mimarisi

---

## 1. Uygulanan Mimari ve Değişiklikler

Kullanıcının seçimi doğrultusunda (Seçenek B: Katman Derinliği & Z-Index Destekli Hibrit 2.5D), 2D çizimlerin hem 2D tuvalde doğru sırada çizilmesi hem de 3D görünümde derinlik (Z ekseni mesafesi) kazanarak katman katman incelenebilmesi sağlandı.

### A. ECS Çekirdeği (`crates/rugeng_core`)
1. **`ZIndex2D` Bileşeni:**
   - `crates/rugeng_core/src/components.rs` dosyasına eklendi:
     ```rust
     #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
     pub struct ZIndex2D {
         pub z: i32,
     }
     ```
   - `crates/rugeng_core/src/lib.rs` üzerinden dışa aktarıldı.
2. **Sahne Serileştirme (`SceneEntity`):**
   - `crates/rugeng_core/src/scene.rs` içerisindeki `SceneEntity` yapısına `#[serde(default)] pub z_index: i32` alanı eklendi. Geriye dönük uyumluluk bozulmadan mevcut sahnelerin varsayılan 0 değeriyle yüklenmesi korundu.

### B. 2D Çizim Sıralaması (`crates/rugeng_editor`)
- 2D görünümde sahne varlıkları çizilirken `ZIndex2D` değerine göre küçükten büyüğe sıralama (`sort_by_key`) uygulandı:
  - Düşük Z-Index değerine sahip varlıklar (ör. arka plan `z = -5`) önce çizilir.
  - Yüksek Z-Index değerine sahip varlıklar (ör. UI ve ön plan `z = 5`) üstte çizilir.

### C. 3D Çalışma Alanında 2.5D Katman Önizlemesi (`crates/rugeng_editor`)
- 3D çalışma alanı döngüsüne 2.5D izdüşüm entegre edildi:
  - 2D piksel koordinatları 3D dünya koordinatına dönüştürülür: `scale_factor = 1.0 / 64.0` (64 piksel = 1.0 metre).
  - Varlıkların Z eksenindeki fiziksel konumu `z_index * 0.25` metre olarak hesaplanır.
  - Varlıklar çift taraflı düzlemsel mesh (`gpu_sprite_3d`) kullanılarak PBR shader ile çizilir.
  - Seçili olan varlık belirgin metalik ve pürüzsüzlük vurgusu kazanır.
  - Kullanıcı 3D serbest uçuş kamerası (Sağ Tık + WASD veya Orta Tuş Orbit) ile sahneyi döndürdüğünde tüm 2D katmanların derinliğini uzayda açıkça görebilir.

### D. Müfettiş (Inspector) Arayüzü ve Geri Al / Yinele (Undo/Redo)
- `Transform 2D` kartı genişletildi ve `Z-Index: <z>` satırı eklendi.
- `[Z-]` ve `[Z+]` butonları ile fare tıklamasıyla varlığın Z-Index'i anında artırılıp azaltılabilir.
- `EntitySnapshot` yapısına `z_index: Option<ZIndex2D>` eklenerek varlık silme/geri yükleme işlemlerinde Z-Index bütünlüğü garanti altına alındı.

---

## 2. Doğrulama ve Test Sonuçları

- **Test Paketi:**
  - `crates/rugeng_editor/src/main.rs` içine `test_z_index_sorting_and_2_5d_depth` birim testi eklendi:
    - 2D varlıkların Z-Index sıralaması doğrulandı.
    - Sahneye `.scene` dosyası olarak kaydedilip geri yüklendiğinde Z-Index değerlerinin eksiksiz korunduğu kanıtlandı.
- **Tüm Çalışma Alanı Doğrulaması:**
  - `cargo test --workspace` -> 95/95 test BAŞARILI.
  - `cargo clippy --workspace --all-targets -- -D warnings` -> Sıfır hata / sıfır uyarı.
  - `cargo fmt --check` -> Tam uyumlu formatlama.
