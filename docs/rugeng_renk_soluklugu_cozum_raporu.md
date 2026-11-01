# RugEng Renk Solukluğu Teşhis ve Çözüm Raporu

## 1. Problem Tespiti
Kullanıcı tarafından iletilen ekran görüntüsünde, tema renk kodları koyu antrasit (`#232323`, `#282828`) olarak tanımlanmasına rağmen tüm arayüzün mat, griye çalan ve soluk (washed-out) bir tonda ekrana basıldığı görüldü.

## 2. Kök Neden Analizi (Root-Cause)
`crates/rugeng_render/src/context.rs` dosyası incelendiğinde Vulkan Swapchain yüzey formatı seçiminde şu mantık yer almaktaydı:

```rust
let format = formats
    .iter()
    .find(|f| {
        f.format == vk::Format::B8G8R8A8_SRGB
            && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
    })
    .unwrap_or(&formats[0]);
```

- Swapchain formatı `B8G8R8A8_SRGB` seçildiğinde, Vulkan donanım çıkışı (pipeline framebuffer write aşaması) shader'dan gelen değerleri lineer kabul eder ve ekrana sunmadan önce otomatik sRGB gama eğrisi ($x^{1/2.2}$) uygular.
- Arayüz renkleri (`[35, 35, 35, 255]`) zaten sRGB renk uzayında tanımlı değerlerdir. $35 / 255 \approx 0.137$ değeri lineer kabul edilip donanım tarafından $0.137^{1/2.2} \approx 0.40$ seviyesine yükseltilmiştir.
- Bu durum, `#232323` (35) olması gereken rengi monitörde `#666666` (102) açık gri tonuna fırlatarak arayüzün süt gibi soluk ve kontrasttan yoksun görünmesine neden olmuştur.

## 3. Yapılan Düzeltme
`crates/rugeng_render/src/context.rs` içinde swapchain yüzey formatı önceliği `B8G8R8A8_UNORM` ve `R8G8B8A8_UNORM` olarak güncellendi:

```rust
let format = formats
    .iter()
    .find(|f| {
        (f.format == vk::Format::B8G8R8A8_UNORM || f.format == vk::Format::R8G8B8A8_UNORM)
            && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
    })
    .or_else(|| {
        formats.iter().find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
    })
    .unwrap_or(&formats[0]);
```

Bu sayede:
1. Donanım fazladan gama yükseltmesi yapmaz.
2. Shader'dan ve arayüzden gönderilen `#232323` ve `#282828` renkleri monitöre 1:1 piksel doğruluğuyla doğrudan yazılır.
3. Kontrast tam oturur, siyahlar ve koyu tonlar derinleşir, solukluk kalkar.

## 4. Doğrulama ve Testler
- `cargo test --workspace`: 66/66 test hatasız geçti.
- `cargo clippy --workspace --all-targets`: 0 uyarı, temiz derleme.
