//! Merkezi Varlık Yöneticisi ve Hot Reload (Asset Manager).
//!
//! Dosya sistemini izler (mtime kontrolü), değiştirilen doku ve sahneleri
//! otomatik olarak bellek ve GPU atlası için yeniden yükler.

use crate::handle::{Handle, LoadingState};
use crate::scene::VersionedScene;
use crate::texture::TextureAsset;
use crate::vfs::Vfs;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

#[allow(dead_code)]
struct TrackedAsset<T> {
    handle: Handle<T>,
    virtual_path: String,
    real_path: PathBuf,
    last_modified: Option<SystemTime>,
    state: LoadingState,
    asset: Option<Arc<T>>,
}

/// Merkezi Varlık Yöneticisi
pub struct AssetServer {
    vfs: Arc<Vfs>,
    textures: RwLock<HashMap<String, TrackedAsset<TextureAsset>>>,
    scenes: RwLock<HashMap<String, TrackedAsset<VersionedScene>>>,
    reloaded_textures: RwLock<Vec<String>>,
    reloaded_scenes: RwLock<Vec<String>>,
}

impl AssetServer {
    pub fn new(vfs: Arc<Vfs>) -> Self {
        Self {
            vfs,
            textures: RwLock::new(HashMap::new()),
            scenes: RwLock::new(HashMap::new()),
            reloaded_textures: RwLock::new(Vec::new()),
            reloaded_scenes: RwLock::new(Vec::new()),
        }
    }

    pub fn vfs(&self) -> &Vfs {
        &self.vfs
    }

    /// Bir dokuyu VFS üzerinden yükler. Eksik veya bozuksa placeholder döner, asla çökmez.
    pub fn load_texture(&self, virtual_path: &str) -> (Handle<TextureAsset>, Arc<TextureAsset>) {
        let mut textures = self.textures.write();
        if let Some(entry) = textures.get(virtual_path) {
            if let Some(asset) = &entry.asset {
                return (entry.handle.clone(), Arc::clone(asset));
            }
        }

        let real_path = self.vfs.resolve(virtual_path);
        let mtime = real_path.metadata().and_then(|m| m.modified()).ok();

        let (state, asset) = match self.vfs.read_mmap(virtual_path) {
            Ok(mmap) => match TextureAsset::from_png_bytes(mmap.as_slice()) {
                Ok(tex) => (LoadingState::Loaded, Arc::new(tex)),
                Err(err) => {
                    eprintln!(
                        "[ASSET HATA] Doku cozumlenemedi ({}): {}. Placeholder yuklendi.",
                        virtual_path, err
                    );
                    (LoadingState::Failed, Arc::new(TextureAsset::placeholder()))
                }
            },
            Err(err) => {
                eprintln!(
                    "[ASSET HATA] Doku bulunamadi ({}): {}. Placeholder yuklendi.",
                    virtual_path, err
                );
                (LoadingState::Failed, Arc::new(TextureAsset::placeholder()))
            }
        };

        let handle = Handle::new();
        textures.insert(
            virtual_path.to_string(),
            TrackedAsset {
                handle: handle.clone(),
                virtual_path: virtual_path.to_string(),
                real_path,
                last_modified: mtime,
                state,
                asset: Some(Arc::clone(&asset)),
            },
        );

        (handle, asset)
    }

    /// Bir sahneyi VFS üzerinden yükler. Hata durumunda boş güvenli sahne döner.
    pub fn load_scene(&self, virtual_path: &str) -> (Handle<VersionedScene>, Arc<VersionedScene>) {
        let mut scenes = self.scenes.write();
        if let Some(entry) = scenes.get(virtual_path) {
            if let Some(asset) = &entry.asset {
                return (entry.handle.clone(), Arc::clone(asset));
            }
        }

        let real_path = self.vfs.resolve(virtual_path);
        let mtime = real_path.metadata().and_then(|m| m.modified()).ok();

        let (state, asset) = match self.vfs.read_to_string(virtual_path) {
            Ok(json_str) => match VersionedScene::from_json(&json_str) {
                Ok(scene) => (LoadingState::Loaded, Arc::new(scene)),
                Err(err) => {
                    eprintln!(
                        "[ASSET HATA] Sahne cozulemedi ({}): {}. Bos sahne olusturuldu.",
                        virtual_path, err
                    );
                    (
                        LoadingState::Failed,
                        Arc::new(VersionedScene::new("FallbackScene")),
                    )
                }
            },
            Err(err) => {
                eprintln!(
                    "[ASSET HATA] Sahne dosyasi okunamadi ({}): {}. Bos sahne olusturuldu.",
                    virtual_path, err
                );
                (
                    LoadingState::Failed,
                    Arc::new(VersionedScene::new("FallbackScene")),
                )
            }
        };

        let handle = Handle::new();
        scenes.insert(
            virtual_path.to_string(),
            TrackedAsset {
                handle: handle.clone(),
                virtual_path: virtual_path.to_string(),
                real_path,
                last_modified: mtime,
                state,
                asset: Some(Arc::clone(&asset)),
            },
        );

        (handle, asset)
    }

    /// Disk üzerindeki dosya değişikliklerini tarar (Hot Reloading).
    /// Değişen dosyaları yeniden yükler ve adlarını listeye ekler.
    pub fn poll_hot_reload(&self) {
        // 1. Dokuları kontrol et
        {
            let mut textures = self.textures.write();
            let mut reloaded = self.reloaded_textures.write();

            for (path, tracked) in textures.iter_mut() {
                if let Ok(m) = tracked.real_path.metadata() {
                    if let Ok(new_mtime) = m.modified() {
                        let changed = match tracked.last_modified {
                            Some(old_mtime) => new_mtime > old_mtime,
                            None => true,
                        };

                        if changed {
                            tracked.last_modified = Some(new_mtime);
                            match self.vfs.read_mmap(path) {
                                Ok(mmap) => match TextureAsset::from_png_bytes(mmap.as_slice()) {
                                    Ok(tex) => {
                                        tracked.asset = Some(Arc::new(tex));
                                        tracked.state = LoadingState::Loaded;
                                        reloaded.push(path.clone());
                                        println!("[HOT RELOAD] Doku guncellendi: {}", path);
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "[HOT RELOAD HATA] Guncellenen doku bozuk ({}): {}",
                                            path, e
                                        );
                                    }
                                },
                                Err(e) => {
                                    eprintln!("[HOT RELOAD HATA] Doku okunamadi ({}): {}", path, e);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Sahneleri kontrol et
        {
            let mut scenes = self.scenes.write();
            let mut reloaded = self.reloaded_scenes.write();

            for (path, tracked) in scenes.iter_mut() {
                if let Ok(m) = tracked.real_path.metadata() {
                    if let Ok(new_mtime) = m.modified() {
                        let changed = match tracked.last_modified {
                            Some(old_mtime) => new_mtime > old_mtime,
                            None => true,
                        };

                        if changed {
                            tracked.last_modified = Some(new_mtime);
                            match self.vfs.read_to_string(path) {
                                Ok(json_str) => match VersionedScene::from_json(&json_str) {
                                    Ok(scene) => {
                                        tracked.asset = Some(Arc::new(scene));
                                        tracked.state = LoadingState::Loaded;
                                        reloaded.push(path.clone());
                                        println!("[HOT RELOAD] Sahne guncellendi: {}", path);
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "[HOT RELOAD HATA] Guncellenen sahne bozuk ({}): {}",
                                            path, e
                                        );
                                    }
                                },
                                Err(e) => {
                                    eprintln!(
                                        "[HOT RELOAD HATA] Sahne okunamadi ({}): {}",
                                        path, e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Hot reload ile değişmiş dokuların listesini alıp sıfırlar.
    pub fn drain_reloaded_textures(&self) -> Vec<String> {
        let mut reloaded = self.reloaded_textures.write();
        std::mem::take(&mut *reloaded)
    }

    /// Hot reload ile değişmiş sahnelerin listesini alıp sıfırlar.
    pub fn drain_reloaded_scenes(&self) -> Vec<String> {
        let mut reloaded = self.reloaded_scenes.write();
        std::mem::take(&mut *reloaded)
    }

    /// Belirtilen dokunun güncel verisini döner.
    pub fn get_texture(&self, virtual_path: &str) -> Option<Arc<TextureAsset>> {
        self.textures
            .read()
            .get(virtual_path)
            .and_then(|t| t.asset.clone())
    }

    /// Handle üzerinden dokunun güncel verisini ve yükleme durumunu döner.
    pub fn get_texture_by_handle(
        &self,
        handle: &Handle<TextureAsset>,
    ) -> Option<(Arc<TextureAsset>, LoadingState)> {
        let textures = self.textures.read();
        textures
            .values()
            .find(|t| &t.handle == handle)
            .and_then(|t| t.asset.as_ref().map(|a| (Arc::clone(a), t.state)))
    }

    /// Belirtilen sahnenin güncel verisini döner.
    pub fn get_scene(&self, virtual_path: &str) -> Option<Arc<VersionedScene>> {
        self.scenes
            .read()
            .get(virtual_path)
            .and_then(|s| s.asset.clone())
    }

    /// Handle üzerinden sahnenin güncel verisini ve yükleme durumunu döner.
    pub fn get_scene_by_handle(
        &self,
        handle: &Handle<VersionedScene>,
    ) -> Option<(Arc<VersionedScene>, LoadingState)> {
        let scenes = self.scenes.read();
        scenes
            .values()
            .find(|s| &s.handle == handle)
            .and_then(|s| s.asset.as_ref().map(|a| (Arc::clone(a), s.state)))
    }

    /// Takip edilen toplam doku sayısı
    pub fn texture_count(&self) -> usize {
        self.textures.read().len()
    }

    /// Takip edilen toplam sahne sayısı
    pub fn scene_count(&self) -> usize {
        self.scenes.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_missing_texture_returns_placeholder_not_panic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let vfs = Arc::new(Vfs::new(temp_dir.path()));
        let server = AssetServer::new(vfs);

        let (_handle, tex) = server.load_texture("non_existent.png");
        assert!(tex.is_placeholder);
        assert_eq!(tex.width, 16);
        assert_eq!(tex.height, 16);
    }

    #[test]
    fn test_corrupt_texture_returns_placeholder_not_panic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let bad_path = temp_dir.path().join("bad.png");
        {
            let mut f = File::create(&bad_path).unwrap();
            f.write_all(b"NOT A REAL PNG FILE DATA").unwrap();
        }

        let vfs = Arc::new(Vfs::new(temp_dir.path()));
        let server = AssetServer::new(vfs);

        let (_handle, tex) = server.load_texture("bad.png");
        assert!(tex.is_placeholder);
    }

    #[test]
    fn test_hot_reload_scene() {
        let temp_dir = tempfile::tempdir().unwrap();
        let scene_path = temp_dir.path().join("test.scene");
        {
            let mut f = File::create(&scene_path).unwrap();
            f.write_all(br#"{"name":"V1Scene","version":1,"entities":[]}"#)
                .unwrap();
        }

        let vfs = Arc::new(Vfs::new(temp_dir.path()));
        let server = AssetServer::new(vfs);

        let (_h, scene) = server.load_scene("test.scene");
        assert_eq!(scene.name, "V1Scene");

        // Zaman farkı oluşturmak için bekle ve dosyayı değiştir
        std::thread::sleep(std::time::Duration::from_millis(50));
        {
            let mut f = File::create(&scene_path).unwrap();
            f.write_all(br#"{"name":"V2Modified","version":1,"entities":[]}"#)
                .unwrap();
        }

        server.poll_hot_reload();
        let changed = server.drain_reloaded_scenes();
        assert_eq!(changed, vec!["test.scene".to_string()]);

        let updated_scene = server.get_scene("test.scene").unwrap();
        assert_eq!(updated_scene.name, "V2Modified");
    }

    #[test]
    fn test_get_by_handle_and_counts() {
        let temp_dir = tempfile::tempdir().unwrap();
        let vfs = Arc::new(Vfs::new(temp_dir.path()));
        let server = AssetServer::new(vfs);

        let (h_tex, _tex) = server.load_texture("missing.png");
        assert_eq!(server.texture_count(), 1);
        let (found_tex, state) = server.get_texture_by_handle(&h_tex).unwrap();
        assert!(found_tex.is_placeholder);
        assert_eq!(state, LoadingState::Failed);

        let (h_scene, _scene) = server.load_scene("missing.scene");
        assert_eq!(server.scene_count(), 1);
        let (found_scene, s_state) = server.get_scene_by_handle(&h_scene).unwrap();
        assert_eq!(found_scene.name, "FallbackScene");
        assert_eq!(s_state, LoadingState::Failed);
    }
}
