# RugEng Motor Mimarisi ve 3D Geçiş Yol Haritası

Bu doküman, RugEng oyun motorunun mevcut mimarisini, bileşenlerini ve 2D'den 3D'ye geçiş için gereken teknik değişiklikleri, zorluk seviyelerini ve gereksinimleri yapay zeka modellerine ve geliştiricilere aktarmak üzere hazırlanmıştır.

---

## 1. Mevcut Motor Mimarisi (Mevcut Durum: v0.1.0-beta)

RugEng, sıfır harici oyun motoru bağımlılığıyla (Unity/Godot kullanılmadan), saf Rust üzerinde geliştirilmiş, veri odaklı (Data-Oriented) bir oyun motorudur.

### Kütüphane Yapısı (Workspace Crates):
1. **`rugeng_core` (Motor Çekirdeği & ECS):**
   - **Sparse Set ECS:** Varlıklar (`Entity`) 32-bit ID + 32-bit Generation ile yönetilir. Bileşenler (`Component`) sıkı bellek dizilerinde (packed array) önbellek dostu (cache-friendly) tutulur.
   - **FrameArena:** Her kare için sıfır heap tahsisi (zero-alloc) sağlayan bump allocator.
   - **CommandBuffer:** Sistemler çalışırken güvenli varlık oluşturma, silme ve bileşen ekleme/çıkarma kuyruğu.
   - **Ses Motoru:** Saf yazılımsal 44.1kHz WAV/PCM sentezleyici ve mikser.

2. **`rugeng_render` (Grafik Hattı):**
   - **Vulkan 1.3 + Dynamic Rendering:** Klasik ve hantal Vulkan RenderPass nesneleri kaldırılmış, yerine modern `VK_KHR_dynamic_rendering` ve `VK_KHR_synchronization2` entegre edilmiştir.
   - **Instanced Quad Pipeline:** 32 baytlık sıkıştırılmış `QuadInstance` yapısı ile tek bir `vkCmdDraw(6, N)` çağrısıyla yüzbinlerce 2D quad/sprite çizimi.
   - **Fontdue & TrueType Motoru:** Vektörel font rasterizasyonu ve dinamik glif atlası.
   - **UNORM Swapchain:** Piksel renklerinin 1:1 doğrulukla ekrana aktarımı.

3. **`rugeng_physics` (Fizik & Çarpışma):**
   - **Spatial Hash Grid:** Geniş dünyalarda $O(1)$ mekânsal sorgulama.
   - **AABB & Raycasting:** Karo tabanlı (Tilemap) DDA ızgara taraması ve sürekli ışın çarpışmaları.
   - **Slope (Eğim) Çözücü:** 45 derece ve ters açılı zemin rampaları için penetration ve velocity vektör çözümü.

4. **`rugeng_editor` (Görsel Sahne ve Harita Editörü):**
   - Sahne Ağacı (Scene Tree), Gerçek Dosya Gezgini (FileSystem), Denetçi (Inspector), Canlı Çıktı ve Debugger Konsolu.
   - Viewport: Çoklu seçim, taşıma/döndürme/ölçekleme gizmoleri, canlı Tilemap fırçası ve simülasyon kontrolü (Play/Pause/Stop/Step).

---

## 2. 3D'ye Geçiş Analizi: Ne Kadar Kolay / Zor?

Mevcut mimari **3D'ye genişletilmeye son derece uygundur**. Çünkü motor OpenGL 2/3 gibi eski bir mimariye değil, doğrudan modern **Vulkan 1.3** ve **boyuttan bağımsız ECS** yapısına dayanmaktadır.

### Kolay Olan Kısımlar (Hazır Altyapı):
1. **Matematik Kütüphanesi (`glam`):** Zaten `glam::Mat4`, `Vec3`, `Quat` türlerini desteklemektedir. Shader push constant'ları halihazırda 64 baytlık $4 \times 4$ matris taşımaktadır.
2. **Dynamic Rendering:** Vulkan 1.3 dynamic rendering kullanıldığı için geleneksel Vulkan'daki gibi karmaşık subpass ve frame-buffer yeniden oluşturma zahmeti yoktur.
3. **ECS Veri Katmanı:** `Transform2D` yanına `Transform3D { pos: Vec3, rot: Quat, scale: Vec3 }` eklenmesi çekirdek ECS'i hiç etkilemez.

---

## 3. 3D İçin Nelerin Değişmesi / Eklenmesi Gerekiyor?

| Alan | Yapılması Gereken Değişiklik | Zorluk Seviyesi | Tahmini Efor |
| :--- | :--- | :--- | :--- |
| **1. Derinlik Tamponu (Depth Buffer)** | Swapchain ile birlikte `vk::Format::D32_SFLOAT` formatında bir derinlik görüntüsü (Depth Image) oluşturulmalı. Dynamic rendering `depth_attachment`'a bağlanmalı. | **Kolay** | 1 Gün |
| **2. 3D Mesh Render Pipeline** | Quad yerine `Vertex3D { pos: [f32; 3], normal: [f32; 3], uv: [f32; 2] }` ve index buffer kullanan yeni bir `Pipeline3D` yazılmalı. | **Orta** | 2-3 Gün |
| **3. 3D Kamera Matrisi** | `Mat4::orthographic_rh` yerine `Mat4::perspective_rh` entegre edilmeli. Editör viewport'una 3D serbest uçuş (FPS/Orbit) kamera kontrolü eklenmeli. | **Kolay** | 1 Gün |
| **4. Işıklandırma (Lighting)** | Fragment shader'a temel yönlü ışık (Directional Light / Sun), ortam ışığı (Ambient) ve Blinn-Phong ya da basit PBR modeli eklenmeli. | **Orta** | 2-3 Gün |
| **5. 3D Model Yükleyici** | OBJ veya glTF 2.0 formatındaki modelleri sahneye aktaracak bir varlık yükleyicisi eklenmeli. | **Orta** | 2-3 Gün |
| **6. 3D Çarpışma ve Fizik** | 2D AABB yerine 3D Raycasting, Bounding Box (AABB3D) ve Sphere-Sphere çarpışma çözücüsü yazılmalı. | **Orta-İleri** | 3-5 Gün |

---

## 4. Model / Geliştirici Talimatı (Prompt Taslağı)

Yapay zeka modellerine veya yeni geliştiricilere şu özet talimat iletilmelidir:

> "RugEng, Rust ve Vulkan 1.3 (Dynamic Rendering) ile yazılmış, Sparse-Set ECS tabanlı modüler bir oyun motorudur. 3D genişletmesi için motor mimarisini baştan yazmaya gerek yoktur. Render katmanına (`rugeng_render`) bir `DepthBuffer` ve `Pipeline3D` (Vertex3D + Perspective Projeksiyon) eklenmesi, ECS tarafında ise `Transform3D` ile `MeshRenderer` bileşenlerinin tanımlanması 3D desteği için yeterlidir."
