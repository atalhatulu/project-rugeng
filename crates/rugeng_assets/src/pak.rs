//! Project Ember — Varlık Arşiv Paketi (.pak Paketi) Modülü.
//!
//! Bu modül, oyunun tüm sahnelerini, dokularını ve ses varlıklarını
//! tek bir `.pak` ikili (binary) arşiv dosyasında paketler ve mmap veya bellekten
//! anında doğrudan indeksli okuma sağlar.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub const PAK_MAGIC: &[u8; 8] = b"RUGENGPK";

/// Paket İçi Dosya Girişi
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PakEntry {
    pub name: String,
    pub offset: u64,
    pub size: u64,
}

/// .pak Paketleyici (PakWriter)
pub struct PakWriter {
    files: Vec<(String, Vec<u8>)>,
}

impl Default for PakWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl PakWriter {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn add_file(&mut self, name: impl Into<String>, data: Vec<u8>) {
        self.files.push((name.into(), data));
    }

    /// Verilen dosyaya tüm içeriği .pak formatında yazar
    pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<(), String> {
        let mut file =
            File::create(path).map_err(|e| format!("PAK dosyasi olusturulamadi: {e}"))?;

        // 1. Magic
        file.write_all(PAK_MAGIC)
            .map_err(|e| format!("Magic yazilamadi: {e}"))?;

        // 2. Dosya Sayısı
        let count = self.files.len() as u32;
        file.write_all(&count.to_le_bytes())
            .map_err(|e| format!("Dosya sayisi yazilamadi: {e}"))?;

        // 3. Header Girişlerinin Yer Tutucuları ve Veri Başlangıç Ofsetinin Hesaplanması
        // Header her dosya için: [name_len (2 bayt), name (name_len), offset (8 bayt), size (8 bayt)]
        let mut header_size = 8 + 4; // magic + count
        for (name, _) in &self.files {
            header_size += 2 + name.len() + 8 + 8;
        }

        let mut current_data_offset = header_size as u64;
        let mut entries = Vec::new();

        for (name, data) in &self.files {
            let size = data.len() as u64;
            entries.push(PakEntry {
                name: name.clone(),
                offset: current_data_offset,
                size,
            });
            current_data_offset += size;
        }

        // 4. Header'ı yaz
        for entry in &entries {
            let name_bytes = entry.name.as_bytes();
            file.write_all(&(name_bytes.len() as u16).to_le_bytes())
                .map_err(|e| format!("Ad uzunlugu yazilamadi: {e}"))?;
            file.write_all(name_bytes)
                .map_err(|e| format!("Ad yazilamadi: {e}"))?;
            file.write_all(&entry.offset.to_le_bytes())
                .map_err(|e| format!("Ofset yazilamadi: {e}"))?;
            file.write_all(&entry.size.to_le_bytes())
                .map_err(|e| format!("Boyut yazilamadi: {e}"))?;
        }

        // 5. Veri Bloklarını yaz
        for (_, data) in &self.files {
            file.write_all(data)
                .map_err(|e| format!("Veri blogu yazilamadi: {e}"))?;
        }

        file.flush().map_err(|e| format!("Flush hatasi: {e}"))?;
        Ok(())
    }
}

/// .pak Paket Okuyucusu (PakReader)
pub struct PakReader {
    entries: HashMap<String, PakEntry>,
    raw_data: Vec<u8>,
}

impl PakReader {
    /// Bayt dizisinden paketi yükler
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        if bytes.len() < 12 || &bytes[..8] != PAK_MAGIC {
            return Err("Gecersiz PAK dosyasi veya sihirli bayt uyusmuyor".to_string());
        }

        let count = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let mut cursor = 12;
        let mut entries = HashMap::new();

        for _ in 0..count {
            if cursor + 2 > bytes.len() {
                return Err("PAK headeri eksik (ad uzunlugu)".to_string());
            }
            let name_len = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]) as usize;
            cursor += 2;

            if cursor + name_len > bytes.len() {
                return Err("PAK headeri eksik (ad verisi)".to_string());
            }
            let name = std::str::from_utf8(&bytes[cursor..cursor + name_len])
                .map_err(|e| format!("Gecersiz UTF-8 dosya adi: {e}"))?
                .to_string();
            cursor += name_len;

            if cursor + 16 > bytes.len() {
                return Err("PAK headeri eksik (ofset/boyut)".to_string());
            }
            let offset = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().unwrap());
            cursor += 8;
            let size = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().unwrap());
            cursor += 8;

            entries.insert(name.clone(), PakEntry { name, offset, size });
        }

        Ok(Self {
            entries,
            raw_data: bytes,
        })
    }

    /// Dosyadan .pak yükler
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let mut file = File::open(path).map_err(|e| format!("PAK dosyasi acilamadi: {e}"))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| format!("PAK dosyasi okunamadi: {e}"))?;
        Self::from_bytes(bytes)
    }

    /// Arşivdeki dosya adlarını listeler
    pub fn list_files(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }

    /// Arşivden belirli bir dosyanın bayt dilimini döndürür
    pub fn get_file(&self, name: &str) -> Option<&[u8]> {
        let entry = self.entries.get(name)?;
        let start = entry.offset as usize;
        let end = start + entry.size as usize;
        if end <= self.raw_data.len() {
            Some(&self.raw_data[start..end])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pak_archive_write_and_read_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let pak_path = dir.path().join("game_assets.pak");

        let mut writer = PakWriter::new();
        writer.add_file("scenes/main.scene", b"{\"name\": \"TestLevel\"}".to_vec());
        writer.add_file("sounds/laser.wav", vec![1, 2, 3, 4, 5, 6, 7, 8]);
        writer
            .write_to_file(&pak_path)
            .expect("Pak yazimi basarili olmali");

        let reader = PakReader::from_file(&pak_path).expect("Pak okumasi basarili olmali");
        assert_eq!(reader.list_files().len(), 2);

        let scene_bytes = reader
            .get_file("scenes/main.scene")
            .expect("Scene dosyasi bulunmali");
        assert_eq!(scene_bytes, b"{\"name\": \"TestLevel\"}");

        let sound_bytes = reader
            .get_file("sounds/laser.wav")
            .expect("Sound dosyasi bulunmali");
        assert_eq!(sound_bytes, &[1, 2, 3, 4, 5, 6, 7, 8]);

        assert!(reader.get_file("non_existent.txt").is_none());
    }
}
