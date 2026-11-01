//! Rugeng Mmap VFS (Virtual File System)
//!
//! Linux/Unix `libc::mmap` ve Windows `CreateFileMapping`/`MapViewOfFile` ile
//! sıfır kopyalı dosya haritalama sağlar.
//! Ayrıca dosya sistemi üzerinde sanal mount noktaları (alias) destekler.

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::Arc;

/// Belleğe haritalanmış bir dosya dilimi (Memory Mapped File).
pub struct MmapFile {
    ptr: NonNull<u8>,
    len: usize,
    #[cfg(windows)]
    _handle: *mut libc::c_void,
}

// SAFETY: MmapFile salt okunur (read-only) paylaşım sağlar, thread-safe'tir.
unsafe impl Send for MmapFile {}
unsafe impl Sync for MmapFile {}

impl MmapFile {
    /// Bir dosyayı salt okunur (PROT_READ) mmap ile belleğe haritalar.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let p = path.as_ref();
        let file = File::open(p).map_err(|e| format!("Dosya acilamadi ({}): {e}", p.display()))?;
        let metadata = file
            .metadata()
            .map_err(|e| format!("Dosya metadatasina erisilemedi: {e}"))?;
        let len = metadata.len() as usize;

        if len == 0 {
            // 0 baytlık dosyalar için geçerli boş dilim (dangling non-null pointer)
            return Ok(Self {
                ptr: NonNull::dangling(),
                len: 0,
                #[cfg(windows)]
                _handle: std::ptr::null_mut(),
            });
        }

        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = file.as_raw_fd();
            // SAFETY: mmap salt-okunur (PROT_READ) ve MAP_SHARED/PRIVATE olarak geçerli açık fd üzerinden çağrılır.
            let raw = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    len,
                    libc::PROT_READ,
                    libc::MAP_PRIVATE,
                    fd,
                    0,
                )
            };

            if raw == libc::MAP_FAILED {
                return Err(format!(
                    "mmap cagrisi basarisiz oldu: {}",
                    std::io::Error::last_os_error()
                ));
            }

            let non_null = NonNull::new(raw as *mut u8).ok_or("mmap null pointer dondurdu")?;
            Ok(Self { ptr: non_null, len })
        }

        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            extern "system" {
                fn CreateFileMappingA(
                    hFile: *mut libc::c_void,
                    lpFileMappingAttributes: *mut libc::c_void,
                    flProtect: u32,
                    dwMaximumSizeHigh: u32,
                    dwMaximumSizeLow: u32,
                    lpName: *const libc::c_char,
                ) -> *mut libc::c_void;

                fn MapViewOfFile(
                    hFileMappingObject: *mut libc::c_void,
                    dwDesiredAccess: u32,
                    dwFileOffsetHigh: u32,
                    dwFileOffsetLow: u32,
                    dwNumberOfBytesToMap: usize,
                ) -> *mut libc::c_void;

                fn CloseHandle(hObject: *mut libc::c_void) -> libc::c_int;
            }

            const PAGE_READONLY: u32 = 0x02;
            const FILE_MAP_READ: u32 = 0x0004;

            let handle = file.as_raw_handle() as *mut libc::c_void;
            // SAFETY: Win32 API dosya haritalama çağrısı
            unsafe {
                let mapping = CreateFileMappingA(
                    handle,
                    std::ptr::null_mut(),
                    PAGE_READONLY,
                    0,
                    0,
                    std::ptr::null(),
                );
                if mapping.is_null() {
                    return Err(format!(
                        "CreateFileMappingA basarisiz: {}",
                        std::io::Error::last_os_error()
                    ));
                }

                let view = MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, len);
                if view.is_null() {
                    CloseHandle(mapping);
                    return Err(format!(
                        "MapViewOfFile basarisiz: {}",
                        std::io::Error::last_os_error()
                    ));
                }

                let non_null =
                    NonNull::new(view as *mut u8).ok_or("MapViewOfFile null pointer dondurdu")?;
                Ok(Self {
                    ptr: non_null,
                    len,
                    _handle: mapping,
                })
            }
        }
    }

    /// Haritalanmış bellek dilimini (slice) döndürür.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        if self.len == 0 {
            &[]
        } else {
            // SAFETY: Bellek geçerli mmap aralığındadır ve len bayttır.
            unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Drop for MmapFile {
    fn drop(&mut self) {
        if self.len > 0 {
            #[cfg(unix)]
            // SAFETY: munmap ile daha önce mmap ile ayrılmış bellek kapatılır.
            unsafe {
                libc::munmap(self.ptr.as_ptr() as *mut libc::c_void, self.len);
            }

            #[cfg(windows)]
            // SAFETY: UnmapViewOfFile ve CloseHandle çağrıları
            unsafe {
                extern "system" {
                    fn UnmapViewOfFile(lpBaseAddress: *mut libc::c_void) -> libc::c_int;
                    fn CloseHandle(hObject: *mut libc::c_void) -> libc::c_int;
                }
                UnmapViewOfFile(self.ptr.as_ptr() as *mut libc::c_void);
                if !self._handle.is_null() {
                    CloseHandle(self._handle);
                }
            }
        }
    }
}

/// Sanal Dosya Sistemi (Virtual File System).
///
/// Mantıksal sanal yolları ("textures/player.png" veya "assets://textures/player.png")
/// gerçek dosya yollarına çözümler ve dosyaları mmap ile sıfır kopyalı açar.
pub struct Vfs {
    mounts: HashMap<String, PathBuf>,
    base_dir: PathBuf,
}

impl Vfs {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            mounts: HashMap::new(),
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    /// Bir sanal ön ek mount eder (Örn: "textures" -> "assets/textures").
    pub fn mount<P: AsRef<Path>>(&mut self, prefix: &str, real_path: P) {
        let clean_prefix = prefix.trim_matches('/').to_string();
        self.mounts
            .insert(clean_prefix, real_path.as_ref().to_path_buf());
    }

    /// Sanal bir yolu gerçek disk yoluna çözümler.
    pub fn resolve(&self, virtual_path: &str) -> PathBuf {
        let cleaned = virtual_path
            .trim_start_matches("assets://")
            .trim_start_matches('/');

        // Mount kontrolü
        for (prefix, target) in &self.mounts {
            if cleaned == prefix || cleaned.starts_with(&format!("{prefix}/")) {
                let remainder = &cleaned[prefix.len()..].trim_start_matches('/');
                return target.join(remainder);
            }
        }

        // Mount yoksa base_dir altına göre çöz
        self.base_dir.join(cleaned)
    }

    /// Dosyanın var olup olmadığını kontrol eder.
    pub fn exists(&self, virtual_path: &str) -> bool {
        self.resolve(virtual_path).exists()
    }

    /// Dosyayı mmap ile okur.
    pub fn read_mmap(&self, virtual_path: &str) -> Result<Arc<MmapFile>, String> {
        let real_path = self.resolve(virtual_path);
        let mmap = MmapFile::open(&real_path)?;
        Ok(Arc::new(mmap))
    }

    /// Dosyayı UTF-8 metin olarak okur.
    pub fn read_to_string(&self, virtual_path: &str) -> Result<String, String> {
        let mmap = self.read_mmap(virtual_path)?;
        std::str::from_utf8(mmap.as_slice())
            .map(|s| s.to_string())
            .map_err(|e| format!("Dosya gecerli UTF-8 icermiyor: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_mmap_file_read() {
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        temp.write_all(b"Hello Mmap VFS World!").unwrap();
        temp.flush().unwrap();

        let mmap = MmapFile::open(temp.path()).expect("Mmap basarisiz");
        assert_eq!(mmap.as_slice(), b"Hello Mmap VFS World!");
        assert_eq!(mmap.len(), 21);
        assert!(!mmap.is_empty());
    }

    #[test]
    fn test_mmap_empty_file() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let mmap = MmapFile::open(temp.path()).expect("Bos mmap acilamadi");
        assert_eq!(mmap.len(), 0);
        assert!(mmap.is_empty());
        assert_eq!(mmap.as_slice(), b"");
    }

    #[test]
    fn test_vfs_resolve_and_mount() {
        let mut vfs = Vfs::new("/game/assets");
        assert_eq!(
            vfs.resolve("scenes/level1.scene"),
            PathBuf::from("/game/assets/scenes/level1.scene")
        );
        assert_eq!(
            vfs.resolve("assets://textures/player.png"),
            PathBuf::from("/game/assets/textures/player.png")
        );

        vfs.mount("custom", "/opt/mod/custom_assets");
        assert_eq!(
            vfs.resolve("custom/hero.png"),
            PathBuf::from("/opt/mod/custom_assets/hero.png")
        );
    }
}
