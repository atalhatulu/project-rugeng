//! Project Ember — Dahili Ses Sentezleyicisi ve Ses Yöneticisi (Audio Synthesizer).
//!
//! Harici ağır ses kütüphanelerine ve C kütüphanelerine bağımlı olmadan,
//! 8-bit retro ses efektleri (Laser, Patlama, Blip, Güç Artışı) sentezler,
//! PCM WAV bayt dizileri üretir ve ses kanallarını yönetir.

pub struct SoundClip {
    pub name: String,
    pub sample_rate: u32,
    pub samples: Vec<i16>,
}

impl SoundClip {
    /// Sentezlenen PCM verisini standart 16-bit Mono WAV formatına kodlar
    pub fn to_wav_bytes(&self) -> Vec<u8> {
        let mut wav = Vec::with_capacity(44 + self.samples.len() * 2);
        let subchunk2_size = (self.samples.len() * 2) as u32;
        let chunk_size = 36 + subchunk2_size;

        // RIFF header
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&chunk_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");

        // fmt subchunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size (16 for PCM)
        wav.extend_from_slice(&1u16.to_le_bytes()); // AudioFormat (1 for PCM)
        wav.extend_from_slice(&1u16.to_le_bytes()); // NumChannels (1 = Mono)
        wav.extend_from_slice(&self.sample_rate.to_le_bytes());
        let byte_rate = self.sample_rate * 2;
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes()); // BlockAlign (2 bytes)
        wav.extend_from_slice(&16u16.to_le_bytes()); // BitsPerSample (16 bits)

        // data subchunk
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&subchunk2_size.to_le_bytes());
        for sample in &self.samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }

        wav
    }

    /// Diskten veya bayt dizisinden standart PCM WAV dosyasını ayrıştırır.
    pub fn from_wav_bytes(name: impl Into<String>, bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 12 {
            return Err("WAV verisi gecersiz: Boyut cok kucuk (en az 12 bayt olmali)".to_string());
        }
        if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err("Gecersiz RIFF/WAVE basligi".to_string());
        }

        let mut offset = 12;
        let mut fmt_found = false;
        let mut sample_rate = 44100u32;
        let mut num_channels = 1u16;
        let mut bits_per_sample = 16u16;
        let mut audio_format = 1u16;
        let mut pcm_samples = Vec::new();
        let mut data_found = false;

        while offset + 8 <= bytes.len() {
            let chunk_id = &bytes[offset..offset + 4];
            let chunk_size = u32::from_le_bytes(
                bytes[offset + 4..offset + 8]
                    .try_into()
                    .map_err(|e| format!("Chunk boyutu okunamadi: {e}"))?,
            ) as usize;

            let chunk_start = offset + 8;
            let chunk_end = (chunk_start + chunk_size).min(bytes.len());

            if chunk_id == b"fmt " {
                if chunk_size < 16 || chunk_start + 16 > bytes.len() {
                    return Err("Gecersiz 'fmt ' blogu".to_string());
                }
                audio_format =
                    u16::from_le_bytes(bytes[chunk_start..chunk_start + 2].try_into().unwrap());
                num_channels =
                    u16::from_le_bytes(bytes[chunk_start + 2..chunk_start + 4].try_into().unwrap());
                sample_rate =
                    u32::from_le_bytes(bytes[chunk_start + 4..chunk_start + 8].try_into().unwrap());
                bits_per_sample = u16::from_le_bytes(
                    bytes[chunk_start + 14..chunk_start + 16]
                        .try_into()
                        .unwrap(),
                );

                if sample_rate == 0 {
                    return Err("WAV ornekleme frekansi (sample rate) 0 olamaz".to_string());
                }
                if num_channels == 0 {
                    return Err("WAV kanal sayisi 0 olamaz".to_string());
                }
                fmt_found = true;
            } else if chunk_id == b"data" {
                if !fmt_found {
                    return Err("WAV 'fmt ' blogu 'data' blogundan once bulunamadi".to_string());
                }
                if audio_format != 1 && audio_format != 3 {
                    return Err(format!(
                        "Desteklenmeyen WAV formati (yalnizca PCM=1 veya Float=3 desteklenir, format={audio_format})"
                    ));
                }

                let data = &bytes[chunk_start..chunk_end];
                match (bits_per_sample, num_channels) {
                    (16, 1) => {
                        pcm_samples.reserve(data.len() / 2);
                        for chunk in data.chunks_exact(2) {
                            pcm_samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
                        }
                    }
                    (16, 2) => {
                        pcm_samples.reserve(data.len() / 4);
                        for chunk in data.chunks_exact(4) {
                            let left = i16::from_le_bytes([chunk[0], chunk[1]]) as i32;
                            let right = i16::from_le_bytes([chunk[2], chunk[3]]) as i32;
                            pcm_samples.push(((left + right) / 2) as i16);
                        }
                    }
                    (8, 1) => {
                        pcm_samples.reserve(data.len());
                        for &byte in data {
                            pcm_samples.push(((byte as i32 - 128) * 256) as i16);
                        }
                    }
                    (8, 2) => {
                        pcm_samples.reserve(data.len() / 2);
                        for chunk in data.chunks_exact(2) {
                            let left = (chunk[0] as i32 - 128) * 256;
                            let right = (chunk[1] as i32 - 128) * 256;
                            pcm_samples.push(((left + right) / 2) as i16);
                        }
                    }
                    (32, 1) if audio_format == 3 => {
                        pcm_samples.reserve(data.len() / 4);
                        for chunk in data.chunks_exact(4) {
                            let val = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                            pcm_samples.push((val.clamp(-1.0, 1.0) * 32767.0) as i16);
                        }
                    }
                    (32, 2) if audio_format == 3 => {
                        pcm_samples.reserve(data.len() / 8);
                        for chunk in data.chunks_exact(8) {
                            let left = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                            let right =
                                f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
                            let mixed = ((left + right) * 0.5).clamp(-1.0, 1.0);
                            pcm_samples.push((mixed * 32767.0) as i16);
                        }
                    }
                    _ => {
                        return Err(format!(
                            "Desteklenmeyen bit derinligi/kanal: {bits_per_sample} bit, {num_channels} kanal"
                        ));
                    }
                }
                data_found = true;
                break;
            }

            // RIFF chunk'ları 2 bayta hizalanır
            let padded_size = chunk_size + (chunk_size % 2);
            offset = chunk_start + padded_size;
        }

        if !data_found {
            return Err("WAV dosyasinda 'data' blogu bulunamadi".to_string());
        }

        Ok(Self {
            name: name.into(),
            sample_rate,
            samples: pcm_samples,
        })
    }

    /// Dosya yolundan bir WAV dosyasını okur.
    pub fn from_wav_file(
        name: impl Into<String>,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, String> {
        let bytes = std::fs::read(path.as_ref())
            .map_err(|e| format!("WAV dosyasi okunamadi ({}): {e}", path.as_ref().display()))?;
        Self::from_wav_bytes(name, &bytes)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AudioBus {
    Master,
    Sfx,
    Music,
}

/// Aktif çalan bir ses örneği (Ses kanalı / Voice)
#[derive(Clone, Debug)]
pub struct AudioVoice {
    pub id: u64,
    pub clip_name: String,
    pub bus: AudioBus,
    pub volume: f32, // Ses örneğine özel lokal genlik (0.0..=1.0)
    pub cursor_f64: f64,
    pub is_looping: bool,
    pub is_finished: bool,
}

/// Dahili Yazılımsal Ses Mikseri (Software Audio Mixer)
///
/// Harici süreç (fork/exec pw-play/aplay) çağrıları yerine:
/// - Çok kanallı eşzamanlı ses çalma (polyphony / voices)
/// - Bus bazlı ses seviyesi ölçekleme (Master, SFX, Music)
/// - Stereo miksleme, format dönüşümü ve yumuşak kırpma (soft limiting)
pub struct AudioMixer {
    pub voices: Vec<AudioVoice>,
    next_voice_id: u64,
    pub master_volume: f32,
    pub sfx_volume: f32,
    pub music_volume: f32,
    pub output_sample_rate: u32,
    pub max_voices: usize,
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self::new(44100)
    }
}

impl AudioMixer {
    pub const DEFAULT_MAX_VOICES: usize = 64;

    pub fn new(output_sample_rate: u32) -> Self {
        Self {
            voices: Vec::with_capacity(Self::DEFAULT_MAX_VOICES),
            next_voice_id: 1,
            master_volume: 1.0,
            sfx_volume: 0.8,
            music_volume: 0.7,
            output_sample_rate: output_sample_rate.max(8000),
            max_voices: Self::DEFAULT_MAX_VOICES,
        }
    }

    #[inline]
    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| !v.is_finished).count()
    }

    pub fn play(
        &mut self,
        clip_name: impl Into<String>,
        bus: AudioBus,
        volume: f32,
        looping: bool,
    ) -> u64 {
        // Voice-stealing: Kapasite dolduğunda bitmiş veya en eski SFX sesini çal
        if self.active_voice_count() >= self.max_voices {
            // Önce bitmiş bir ses ara
            if let Some(finished_idx) = self.voices.iter().position(|v| v.is_finished) {
                self.voices.swap_remove(finished_idx);
            } else if let Some(oldest_idx) = self.voices.iter().position(|v| v.bus == AudioBus::Sfx)
            {
                // SFX sesini feda et
                self.voices.swap_remove(oldest_idx);
            } else if !self.voices.is_empty() {
                // Herhangi bir en eski sesi feda et
                self.voices.remove(0);
            }
        }

        let id = self.next_voice_id;
        self.next_voice_id = self.next_voice_id.wrapping_add(1).max(1);

        self.voices.push(AudioVoice {
            id,
            clip_name: clip_name.into(),
            bus,
            volume: volume.clamp(0.0, 1.0),
            cursor_f64: 0.0,
            is_looping: looping,
            is_finished: false,
        });

        id
    }

    pub fn stop_voice(&mut self, voice_id: u64) {
        if let Some(voice) = self.voices.iter_mut().find(|v| v.id == voice_id) {
            voice.is_finished = true;
        }
    }

    pub fn stop_all(&mut self) {
        self.voices.clear();
    }

    pub fn stop_bus(&mut self, bus: AudioBus) {
        for voice in &mut self.voices {
            if voice.bus == bus {
                voice.is_finished = true;
            }
        }
    }

    /// Verilen ses kütüphanesindeki klipleri miksleyerek çıkış stereo tamponunu (Interleaved L, R f32) doldurur.
    /// f32 değerler -1.0 .. 1.0 aralığındadır.
    pub fn mix_f32(
        &mut self,
        clips: &std::collections::HashMap<String, SoundClip>,
        out_stereo_frames: &mut [f32],
    ) {
        // Çıkış tamponunu sıfırla
        out_stereo_frames.fill(0.0);
        let num_frames = out_stereo_frames.len() / 2;

        let master = self.master_volume.clamp(0.0, 1.0);
        let sfx_eff = (master * self.sfx_volume).clamp(0.0, 1.0);
        let music_eff = (master * self.music_volume).clamp(0.0, 1.0);

        for voice in &mut self.voices {
            if voice.is_finished {
                continue;
            }

            let Some(clip) = clips.get(&voice.clip_name) else {
                voice.is_finished = true;
                continue;
            };

            if clip.samples.is_empty() {
                voice.is_finished = true;
                continue;
            }

            let bus_vol = match voice.bus {
                AudioBus::Master => master,
                AudioBus::Sfx => sfx_eff,
                AudioBus::Music => music_eff,
            };
            let gain = (bus_vol * voice.volume).clamp(0.0, 1.0);
            let step = clip.sample_rate as f64 / self.output_sample_rate as f64;
            let clip_len = clip.samples.len();

            if gain <= 0.0001 {
                let advanced = num_frames as f64 * step;
                voice.cursor_f64 += advanced;
                if voice.cursor_f64 >= clip_len as f64 {
                    if voice.is_looping {
                        voice.cursor_f64 %= clip_len as f64;
                    } else {
                        voice.is_finished = true;
                    }
                }
                continue;
            }

            let gain_norm = gain / 32768.0;

            for frame_idx in 0..num_frames {
                let current_pos = voice.cursor_f64 + (frame_idx as f64 * step);

                if current_pos >= clip_len as f64 {
                    if voice.is_looping {
                        let looped_pos = current_pos % (clip_len as f64);
                        let idx0 = looped_pos as usize;
                        let idx1 = if idx0 + 1 < clip_len { idx0 + 1 } else { 0 };
                        let frac = (looped_pos - idx0 as f64) as f32;
                        let s0 = clip.samples[idx0] as f32;
                        let s1 = clip.samples[idx1] as f32;
                        let s_interp = (s0 + frac * (s1 - s0)) * gain_norm;

                        let out_idx = frame_idx * 2;
                        out_stereo_frames[out_idx] += s_interp;
                        out_stereo_frames[out_idx + 1] += s_interp;
                    } else {
                        voice.is_finished = true;
                        break;
                    }
                } else {
                    let idx0 = current_pos as usize;
                    let frac = (current_pos - idx0 as f64) as f32;
                    let s0 = clip.samples[idx0] as f32;
                    let s1 = if idx0 + 1 < clip_len {
                        clip.samples[idx0 + 1] as f32
                    } else if voice.is_looping {
                        clip.samples[0] as f32
                    } else {
                        s0
                    };
                    let s_interp = (s0 + frac * (s1 - s0)) * gain_norm;

                    let out_idx = frame_idx * 2;
                    out_stereo_frames[out_idx] += s_interp;
                    out_stereo_frames[out_idx + 1] += s_interp;
                }
            }

            if !voice.is_finished {
                let advanced = num_frames as f64 * step;
                voice.cursor_f64 += advanced;
                if voice.cursor_f64 >= clip_len as f64 {
                    if voice.is_looping {
                        voice.cursor_f64 %= clip_len as f64;
                    } else {
                        voice.is_finished = true;
                    }
                }
            }
        }

        // Biten sesleri temizle
        self.voices.retain(|v| !v.is_finished);

        // Soft-limiter / Clamp to -1.0 .. 1.0 to avoid harsh clipping
        for s in out_stereo_frames.iter_mut() {
            *s = s.clamp(-1.0, 1.0);
        }
    }

    /// Çıkışı 16-bit stereo PCM baytlarına dönüştürür (Interleaved L16, R16 little-endian)
    pub fn mix_i16_pcm(
        &mut self,
        clips: &std::collections::HashMap<String, SoundClip>,
        num_frames: usize,
        out_pcm_bytes: &mut Vec<u8>,
    ) {
        let mut float_buf = vec![0.0f32; num_frames * 2];
        self.mix_f32(clips, &mut float_buf);

        out_pcm_bytes.clear();
        out_pcm_bytes.reserve(num_frames * 4); // 2 channels * 2 bytes

        for &val in &float_buf {
            let i_val = (val * 32767.0).clamp(-32768.0, 32767.0) as i16;
            out_pcm_bytes.extend_from_slice(&i_val.to_le_bytes());
        }
    }
}

pub struct AudioEngine {
    pub clips: std::collections::HashMap<String, SoundClip>,
    pub mixer: AudioMixer,
    pub stream_driver: Option<AudioStreamDriver>,
    pub active_voices: usize,
    pub volume: f32, // Geriye dönük uyumluluk için (Master)
    pub master_volume: f32,
    pub sfx_volume: f32,
    pub music_volume: f32,
    pub last_played_sound: String,
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            clips: std::collections::HashMap::new(),
            mixer: AudioMixer::new(44100),
            stream_driver: Some(AudioStreamDriver::new(44100, 2)),
            active_voices: 0,
            volume: 0.8,
            master_volume: 1.0,
            sfx_volume: 0.8,
            music_volume: 0.7,
            last_played_sound: String::new(),
        };
        engine.generate_builtin_sfx();
        engine
    }

    /// Belirtilen ses veriyolunun (Bus) ses seviyesini ayarlar (0.0..=1.0).
    pub fn set_bus_volume(&mut self, bus: AudioBus, volume: f32) {
        let clamped = volume.clamp(0.0, 1.0);
        match bus {
            AudioBus::Master => {
                self.master_volume = clamped;
                self.volume = clamped;
                self.mixer.master_volume = clamped;
            }
            AudioBus::Sfx => {
                self.sfx_volume = clamped;
                self.mixer.sfx_volume = clamped;
            }
            AudioBus::Music => {
                self.music_volume = clamped;
                self.mixer.music_volume = clamped;
            }
        }
    }

    /// Belirtilen ses veriyolunun (Bus) etkin ses seviyesini döndürür.
    pub fn get_bus_volume(&self, bus: AudioBus) -> f32 {
        match bus {
            AudioBus::Master => self.master_volume,
            AudioBus::Sfx => self.sfx_volume,
            AudioBus::Music => self.music_volume,
        }
    }

    /// Master ve belirtilen veriyolunun çarpımı olan nihai ses seviyesini hesaplar.
    pub fn effective_volume(&self, bus: AudioBus) -> f32 {
        match bus {
            AudioBus::Master => self.master_volume,
            AudioBus::Sfx => (self.master_volume * self.sfx_volume).clamp(0.0, 1.0),
            AudioBus::Music => (self.master_volume * self.music_volume).clamp(0.0, 1.0),
        }
    }

    /// Dahili 8-bit retro ses efektlerini prosedürel olarak sentezler
    pub fn generate_builtin_sfx(&mut self) {
        let sample_rate = 22050;

        // 1. Laser Shoot (Frekans rampası 900Hz -> 120Hz)
        let laser_len = (sample_rate as f32 * 0.15) as usize;
        let mut laser_samples = Vec::with_capacity(laser_len);
        let mut phase = 0.0f32;
        for i in 0..laser_len {
            let t = i as f32 / laser_len as f32;
            let freq = 900.0 * (1.0 - t).powi(2) + 120.0;
            phase += std::f32::consts::TAU * freq / sample_rate as f32;
            let amp = (1.0 - t) * 0.7;
            let val = (phase.sin() * amp * 32767.0) as i16;
            laser_samples.push(val);
        }
        self.clips.insert(
            "sfx_laser".to_string(),
            SoundClip {
                name: "sfx_laser".to_string(),
                sample_rate,
                samples: laser_samples,
            },
        );

        // 2. Explosion / Boom (Beyaz gürültü + Üstel sönümleme)
        let boom_len = (sample_rate as f32 * 0.35) as usize;
        let mut boom_samples = Vec::with_capacity(boom_len);
        let mut lcg = 0x12345678u64;
        for i in 0..boom_len {
            let t = i as f32 / boom_len as f32;
            lcg = lcg.wrapping_mul(6364136223846793005).wrapping_add(1);
            let noise = ((lcg >> 32) as i32 % 65536 - 32768) as f32 / 32768.0;
            let env = (-(t * 8.0)).exp();
            let val = (noise * env * 0.8 * 32767.0) as i16;
            boom_samples.push(val);
        }
        self.clips.insert(
            "sfx_explosion".to_string(),
            SoundClip {
                name: "sfx_explosion".to_string(),
                sample_rate,
                samples: boom_samples,
            },
        );

        // 3. UI Blip / Click (Kısa yüksek frekans 1200Hz 40ms)
        let blip_len = (sample_rate as f32 * 0.04) as usize;
        let mut blip_samples = Vec::with_capacity(blip_len);
        let mut phase = 0.0f32;
        for i in 0..blip_len {
            let t = i as f32 / blip_len as f32;
            phase += std::f32::consts::TAU * 1200.0 / sample_rate as f32;
            let env = 1.0 - t;
            let val = (phase.sin() * env * 0.5 * 32767.0) as i16;
            blip_samples.push(val);
        }
        self.clips.insert(
            "sfx_blip".to_string(),
            SoundClip {
                name: "sfx_blip".to_string(),
                sample_rate,
                samples: blip_samples,
            },
        );

        // 4. Powerup Chime (Arpej: C5 -> E5 -> G5 -> C6)
        let chime_len = (sample_rate as f32 * 0.28) as usize;
        let mut chime_samples = Vec::with_capacity(chime_len);
        let notes = [523.25, 659.25, 783.99, 1046.50];
        let mut phase = 0.0f32;
        for i in 0..chime_len {
            let t = i as f32 / chime_len as f32;
            let note_idx = ((t * 4.0) as usize).min(3);
            let freq = notes[note_idx];
            phase += std::f32::consts::TAU * freq / sample_rate as f32;
            let env = 1.0 - (t * 0.8);
            let val = (phase.sin() * env * 0.6 * 32767.0) as i16;
            chime_samples.push(val);
        }
        self.clips.insert(
            "sfx_powerup".to_string(),
            SoundClip {
                name: "sfx_powerup".to_string(),
                sample_rate,
                samples: chime_samples,
            },
        );

        // 5. Jump (Karakter Zıplaması: 150Hz -> 480Hz yukarı frekans sweep)
        let jump_len = (sample_rate as f32 * 0.14) as usize;
        let mut jump_samples = Vec::with_capacity(jump_len);
        let mut phase = 0.0f32;
        for i in 0..jump_len {
            let t = i as f32 / jump_len as f32;
            let freq = 140.0 + 380.0 * t.powf(0.8);
            phase += std::f32::consts::TAU * freq / sample_rate as f32;
            let env = (1.0 - t * 0.4).max(0.0);
            let val = (phase.sin() * env * 0.5 * 32767.0) as i16;
            jump_samples.push(val);
        }
        self.clips.insert(
            "sfx_jump".to_string(),
            SoundClip {
                name: "sfx_jump".to_string(),
                sample_rate,
                samples: jump_samples,
            },
        );

        // 6. Coin / Altın Alma (İki tonlu parlak çınlama: B5 -> E6)
        let coin_len = (sample_rate as f32 * 0.22) as usize;
        let mut coin_samples = Vec::with_capacity(coin_len);
        let mut phase = 0.0f32;
        let split = coin_len / 3;
        for i in 0..coin_len {
            let freq = if i < split { 987.77 } else { 1318.51 };
            phase += std::f32::consts::TAU * freq / sample_rate as f32;
            let env = 1.0 - (i as f32 / coin_len as f32);
            let val = (phase.sin() * env * 0.55 * 32767.0) as i16;
            coin_samples.push(val);
        }
        self.clips.insert(
            "sfx_coin".to_string(),
            SoundClip {
                name: "sfx_coin".to_string(),
                sample_rate,
                samples: coin_samples,
            },
        );

        // 7. Stomp / Düşman Ezme (Tok küt vuruş: 220Hz -> 60Hz hızlı düşüş)
        let stomp_len = (sample_rate as f32 * 0.12) as usize;
        let mut stomp_samples = Vec::with_capacity(stomp_len);
        let mut phase = 0.0f32;
        for i in 0..stomp_len {
            let t = i as f32 / stomp_len as f32;
            let freq = 220.0 * (1.0 - t).powi(2) + 50.0;
            phase += std::f32::consts::TAU * freq / sample_rate as f32;
            let env = 1.0 - t;
            let val = (phase.sin() * env * 0.7 * 32767.0) as i16;
            stomp_samples.push(val);
        }
        self.clips.insert(
            "sfx_stomp".to_string(),
            SoundClip {
                name: "sfx_stomp".to_string(),
                sample_rate,
                samples: stomp_samples,
            },
        );

        // 8. Hit / Hasar Alma (Kısa distorsiyonlu vuruş)
        let hit_len = (sample_rate as f32 * 0.15) as usize;
        let mut hit_samples = Vec::with_capacity(hit_len);
        let mut lcg = 0x87654321u64;
        let mut phase = 0.0f32;
        for i in 0..hit_len {
            let t = i as f32 / hit_len as f32;
            lcg = lcg.wrapping_mul(6364136223846793005).wrapping_add(1);
            let noise = ((lcg >> 32) as i32 % 65536 - 32768) as f32 / 32768.0;
            phase += std::f32::consts::TAU * 180.0 / sample_rate as f32;
            let env = 1.0 - t;
            let mixed = (phase.sin() * 0.5 + noise * 0.5) * env;
            hit_samples.push((mixed * 0.6 * 32767.0) as i16);
        }
        self.clips.insert(
            "sfx_hit".to_string(),
            SoundClip {
                name: "sfx_hit".to_string(),
                sample_rate,
                samples: hit_samples,
            },
        );
    }

    pub fn play(&mut self, sound_name: &str) {
        self.play_on_bus(sound_name, AudioBus::Sfx);
    }

    pub fn play_with_volume(&mut self, sound_name: &str, volume: f32) {
        self.play_on_bus_with_volume(sound_name, AudioBus::Sfx, volume);
    }

    pub fn play_on_bus(&mut self, sound_name: &str, bus: AudioBus) {
        self.play_on_bus_with_volume(sound_name, bus, 1.0);
    }

    pub fn play_on_bus_with_volume(&mut self, sound_name: &str, bus: AudioBus, volume: f32) {
        if let Some(clip) = self.clips.get(sound_name) {
            self.last_played_sound = sound_name.to_string();
            let vol_clamped = volume.clamp(0.0, 1.0);
            self.mixer.play(sound_name, bus, vol_clamped, false);
            self.active_voices = self.mixer.active_voice_count();

            let vol = (self.effective_volume(bus) * vol_clamped).clamp(0.0, 1.0);

            // 1. Eğer kesintisiz akış sürücüsü aktifse, tek seferlik PCM parçacığını ring tampona bas
            if let Some(driver) = &self.stream_driver {
                let mut stereo_samples = Vec::with_capacity(clip.samples.len() * 2);
                for &s in &clip.samples {
                    let scaled = (s as f32 * vol).clamp(-32768.0, 32767.0) as i16;
                    stereo_samples.push(scaled);
                    stereo_samples.push(scaled);
                }
                driver.push_samples(&stereo_samples);
            } else {
                // Akış sürücüsü yoksa yedek harici süreç oynatımı
                Self::play_clip_backend(clip, vol);
            }
        }
    }

    /// Çerçeve başına ses mikserini güncelleyip kesintisiz akış sürücüsüne PCM basar.
    pub fn update_stream(&mut self, num_frames: usize) {
        if self.mixer.active_voice_count() == 0 {
            return;
        }
        if let Some(driver) = &self.stream_driver {
            let mut pcm_bytes = Vec::new();
            self.mixer
                .mix_i16_pcm(&self.clips, num_frames, &mut pcm_bytes);
            let mut i16_samples = Vec::with_capacity(pcm_bytes.len() / 2);
            for chunk in pcm_bytes.chunks_exact(2) {
                i16_samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
            }
            driver.push_samples(&i16_samples);
            self.active_voices = self.mixer.active_voice_count();
        }
    }

    pub fn play_preset(&mut self, preset: SfxPreset) {
        self.play_on_bus(preset.as_str(), AudioBus::Sfx);
    }

    pub fn play_preset_on_bus(&mut self, preset: SfxPreset, bus: AudioBus) {
        self.play_on_bus(preset.as_str(), bus);
    }

    /// Linux/Unix platformunda pw-play veya aplay ile sıfır gecikmeli, asenkron ses çalma
    fn play_clip_backend(clip: &SoundClip, volume: f32) {
        let sample_rate = clip.sample_rate;
        let samples: Vec<u8> = clip
            .samples
            .iter()
            .flat_map(|&s| {
                let scaled = (s as f32 * volume).clamp(-32768.0, 32767.0) as i16;
                scaled.to_le_bytes()
            })
            .collect();

        std::thread::spawn(move || {
            use std::io::Write;
            use std::process::{Command, Stdio};

            // Önce PipeWire (pw-play) dene
            let pw = Command::new("pw-play")
                .args([
                    "--raw",
                    &format!("--rate={sample_rate}"),
                    "--channels=1",
                    "--format=s16",
                    "-",
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            if let Ok(mut child) = pw {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(&samples);
                }
                let _ = child.wait();
                return;
            }

            // ALSA (aplay) yedeği
            let alsa = Command::new("aplay")
                .args([
                    "-q",
                    "-f",
                    "S16_LE",
                    "-r",
                    &format!("{sample_rate}"),
                    "-c",
                    "1",
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            if let Ok(mut child) = alsa {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(&samples);
                }
                let _ = child.wait();
            }
        });
    }

    /// Bayt dizisinden WAV yükleyip ses motoruna kaydeder.
    pub fn load_clip_from_wav_bytes(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let clip = SoundClip::from_wav_bytes(name, bytes)?;
        self.clips.insert(name.to_string(), clip);
        Ok(())
    }

    /// Dosyadan WAV yükleyip ses motoruna kaydeder.
    pub fn load_clip_from_wav_file(
        &mut self,
        name: &str,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), String> {
        let clip = SoundClip::from_wav_file(name, path)?;
        self.clips.insert(name.to_string(), clip);
        Ok(())
    }
}

/// Sabit boyutlu, eşzamanlı kilit korumalı ses ring tamponu (PCM Ring Buffer).
/// Üretici (AudioEngine / AudioMixer) ile tüketici (Streaming thread) arasında
/// sıfır ayrıştırma (allocation) ve kesintisiz PCM akışı sağlar.
pub struct AudioRingBuffer {
    buffer: Vec<i16>,
    capacity: usize,
    read_pos: usize,
    write_pos: usize,
    count: usize,
}

impl AudioRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0i16; capacity],
            capacity,
            read_pos: 0,
            write_pos: 0,
            count: 0,
        }
    }

    pub fn available_read(&self) -> usize {
        self.count
    }

    pub fn available_write(&self) -> usize {
        self.capacity - self.count
    }

    /// Tampona örnekler yazar. Kapasite aşılırsa en eski örneklerin üzerine yazar (drop oldest)
    /// böylece ses sürücüsü asla kilitlenmez (overflow safe).
    pub fn write(&mut self, samples: &[i16]) -> usize {
        let mut written = 0;
        for &s in samples {
            if self.count == self.capacity {
                // Tampon dolu: En eskiyi atla
                self.read_pos = (self.read_pos + 1) % self.capacity;
            } else {
                self.count += 1;
            }
            self.buffer[self.write_pos] = s;
            self.write_pos = (self.write_pos + 1) % self.capacity;
            written += 1;
        }
        written
    }

    /// Tampondan çıkış dilimine örnekleri okur. Okunan gerçek örnek sayısını döndürür.
    pub fn read(&mut self, out: &mut [i16]) -> usize {
        let to_read = out.len().min(self.count);
        for item in out.iter_mut().take(to_read) {
            *item = self.buffer[self.read_pos];
            self.read_pos = (self.read_pos + 1) % self.capacity;
        }
        self.count -= to_read;
        to_read
    }

    pub fn clear(&mut self) {
        self.read_pos = 0;
        self.write_pos = 0;
        self.count = 0;
    }
}

/// Dahili Kesintisiz Ses Akış Sürücüsü (Continuous Streaming Audio Driver).
/// Arka planda çalışan tek bir iş parçacığı üzerinden donanım veya ses sunucusu
/// (PipeWire / ALSA) ile açık bir boru (pipe) hattı kurar ve ring-buffer'dan
/// kesintisiz PCM akışı besler. Her ses için süreç (process fork/exec) maliyetini sıfırlar.
pub struct AudioStreamDriver {
    ring_buffer: std::sync::Arc<std::sync::Mutex<AudioRingBuffer>>,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl AudioStreamDriver {
    pub const DEFAULT_BUFFER_SIZE: usize = 32768; // ~370ms buffer @ 44.1kHz stereo
    pub const CHUNK_FRAMES: usize = 512; // ~11.6ms chunks for ultra low latency

    pub fn new(sample_rate: u32, channels: u16) -> Self {
        let ring_buffer = std::sync::Arc::new(std::sync::Mutex::new(AudioRingBuffer::new(
            Self::DEFAULT_BUFFER_SIZE,
        )));
        let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));

        let rb_clone = std::sync::Arc::clone(&ring_buffer);
        let run_clone = std::sync::Arc::clone(&running);

        let thread_handle = std::thread::spawn(move || {
            use std::io::Write;
            use std::process::{Command, Stdio};

            // Önce PipeWire raw stdin akışı başlatmayı dene
            let pw_proc = Command::new("pw-play")
                .args([
                    "--raw",
                    &format!("--rate={sample_rate}"),
                    &format!("--channels={channels}"),
                    "--format=s16",
                    "-",
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            let mut active_pipe: Option<std::process::ChildStdin> = None;
            let mut active_child: Option<std::process::Child> = None;

            if let Ok(mut child) = pw_proc {
                if let Some(stdin) = child.stdin.take() {
                    active_pipe = Some(stdin);
                    active_child = Some(child);
                }
            }

            // PW yoksa ALSA aplay akışı dene
            if active_pipe.is_none() {
                let alsa_proc = Command::new("aplay")
                    .args([
                        "-q",
                        "-f",
                        "S16_LE",
                        "-r",
                        &format!("{sample_rate}"),
                        "-c",
                        &format!("{channels}"),
                        "-",
                    ])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();

                if let Ok(mut child) = alsa_proc {
                    if let Some(stdin) = child.stdin.take() {
                        active_pipe = Some(stdin);
                        active_child = Some(child);
                    }
                }
            }

            let mut chunk = vec![0i16; Self::CHUNK_FRAMES * (channels as usize)];
            let mut byte_chunk = vec![0u8; chunk.len() * 2];

            while run_clone.load(std::sync::atomic::Ordering::Relaxed) {
                let samples_read = {
                    let mut lock = rb_clone.lock().unwrap();
                    lock.read(&mut chunk)
                };

                if samples_read > 0 {
                    // Donanım akış borusu varsa baytları bas
                    if let Some(pipe) = active_pipe.as_mut() {
                        byte_chunk.clear();
                        for &s in &chunk[..samples_read] {
                            byte_chunk.extend_from_slice(&s.to_le_bytes());
                        }
                        if pipe.write_all(&byte_chunk).is_err() {
                            // Boru kapandıysa boruyu bırak, sessiz ring buffer olarak çalışmaya devam et
                            active_pipe = None;
                        }
                    }
                } else {
                    // Tamponda ses yokken CPU yakmamak için uyu (~5ms)
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }

            // Kapanış: açık boruları kapat
            drop(active_pipe);
            if let Some(mut child) = active_child {
                let _ = child.kill();
                let _ = child.wait();
            }
        });

        Self {
            ring_buffer,
            running,
            thread_handle: Some(thread_handle),
        }
    }

    /// Tampona PCM örnekleri (interleaved stereo i16) ekler.
    pub fn push_samples(&self, samples: &[i16]) {
        if let Ok(mut lock) = self.ring_buffer.lock() {
            lock.write(samples);
        }
    }

    /// Tampondaki mevcut okunabilir örnek sayısını döndürür.
    pub fn buffered_samples(&self) -> usize {
        if let Ok(lock) = self.ring_buffer.lock() {
            lock.available_read()
        } else {
            0
        }
    }

    /// Akış sürücüsünü durdurur ve iş parçacığını temizler.
    pub fn stop(&mut self) {
        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for AudioStreamDriver {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Standart Oyun SFX Önayarları (Prosedürel Sentez Tanımları)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SfxPreset {
    Laser,
    Explosion,
    Blip,
    Powerup,
    Jump,
    Coin,
    Stomp,
    Hit,
}

impl SfxPreset {
    pub fn as_str(&self) -> &'static str {
        match self {
            SfxPreset::Laser => "sfx_laser",
            SfxPreset::Explosion => "sfx_explosion",
            SfxPreset::Blip => "sfx_blip",
            SfxPreset::Powerup => "sfx_powerup",
            SfxPreset::Jump => "sfx_jump",
            SfxPreset::Coin => "sfx_coin",
            SfxPreset::Stomp => "sfx_stomp",
            SfxPreset::Hit => "sfx_hit",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_clip_wav_roundtrip() {
        let original = SoundClip {
            name: "test_sweep".to_string(),
            sample_rate: 44100,
            samples: vec![-32768, -16000, 0, 16000, 32767, -100, 100],
        };

        let wav_bytes = original.to_wav_bytes();
        let decoded =
            SoundClip::from_wav_bytes("decoded_sweep", &wav_bytes).expect("WAV decode basarisiz");

        assert_eq!(decoded.name, "decoded_sweep");
        assert_eq!(decoded.sample_rate, original.sample_rate);
        assert_eq!(decoded.samples, original.samples);
    }

    #[test]
    fn test_sound_clip_from_wav_stereo_downmix() {
        // 2 kanallı 16-bit 22050Hz yapay WAV oluştur
        let sample_rate = 22050u32;
        let left_right_pairs: &[(i16, i16)] = &[(1000, 3000), (-2000, 4000), (0, 0)];
        let num_samples = left_right_pairs.len();
        let subchunk2_size = (num_samples * 4) as u32;

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + subchunk2_size).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");

        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&2u16.to_le_bytes()); // Stereo
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&(sample_rate * 4).to_le_bytes()); // byte rate
        bytes.extend_from_slice(&4u16.to_le_bytes()); // block align
        bytes.extend_from_slice(&16u16.to_le_bytes()); // 16 bits

        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&subchunk2_size.to_le_bytes());
        for &(l, r) in left_right_pairs {
            bytes.extend_from_slice(&l.to_le_bytes());
            bytes.extend_from_slice(&r.to_le_bytes());
        }

        let clip =
            SoundClip::from_wav_bytes("stereo_test", &bytes).expect("Stereo decode basarisiz");
        assert_eq!(clip.sample_rate, 22050);
        assert_eq!(clip.samples.len(), 3);
        assert_eq!(clip.samples[0], 2000); // (1000 + 3000) / 2
        assert_eq!(clip.samples[1], 1000); // (-2000 + 4000) / 2
        assert_eq!(clip.samples[2], 0);
    }

    #[test]
    fn test_sound_clip_from_wav_invalid_header() {
        let corrupt = b"NOT_A_WAV_FILE_AT_ALL";
        assert!(SoundClip::from_wav_bytes("corrupt", corrupt).is_err());
    }

    #[test]
    fn test_audio_engine_load_clip_from_wav_bytes() {
        let mut engine = AudioEngine::new();
        let clip = SoundClip {
            name: "temp".to_string(),
            sample_rate: 22050,
            samples: vec![100, 200, 300],
        };
        let wav = clip.to_wav_bytes();
        let res = engine.load_clip_from_wav_bytes("custom_effect", &wav);
        assert!(res.is_ok());
        assert!(engine.clips.contains_key("custom_effect"));
        assert_eq!(engine.clips["custom_effect"].samples.len(), 3);
    }

    #[test]
    fn test_sound_clip_from_wav_float_stereo() {
        // IEEE Float (format 3), 32-bit stereo (44100Hz)
        let sample_rate = 44100u32;
        let left_right: &[(f32, f32)] = &[(0.5, 0.5), (-1.0, 1.0), (0.0, -0.5)];
        let subchunk2_size = (left_right.len() * 8) as u32;

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + subchunk2_size).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");

        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&3u16.to_le_bytes()); // IEEE Float format = 3
        bytes.extend_from_slice(&2u16.to_le_bytes()); // Stereo = 2
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&(sample_rate * 8).to_le_bytes());
        bytes.extend_from_slice(&8u16.to_le_bytes());
        bytes.extend_from_slice(&32u16.to_le_bytes()); // 32 bits

        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&subchunk2_size.to_le_bytes());
        for &(l, r) in left_right {
            bytes.extend_from_slice(&l.to_le_bytes());
            bytes.extend_from_slice(&r.to_le_bytes());
        }

        let clip =
            SoundClip::from_wav_bytes("float_test", &bytes).expect("Float stereo parse failed");
        assert_eq!(clip.sample_rate, 44100);
        assert_eq!(clip.samples.len(), 3);
        // (0.5 + 0.5) / 2 = 0.5 -> 0.5 * 32767 = 16383
        assert!((clip.samples[0] - 16383).abs() <= 1);
        // (-1.0 + 1.0) / 2 = 0.0 -> 0
        assert_eq!(clip.samples[1], 0);
        // (0.0 + -0.5) / 2 = -0.25 -> -0.25 * 32767 = -8191
        assert!((clip.samples[2] - (-8191)).abs() <= 1);
    }

    #[test]
    fn test_audio_mixer_polyphony_and_bus_mixing() {
        let mut clips = std::collections::HashMap::new();
        clips.insert(
            "tone_a".to_string(),
            SoundClip {
                name: "tone_a".to_string(),
                sample_rate: 44100,
                samples: vec![16384, 16384, 16384, 16384],
            },
        );
        clips.insert(
            "tone_b".to_string(),
            SoundClip {
                name: "tone_b".to_string(),
                sample_rate: 44100,
                samples: vec![16384, 16384, 16384, 16384],
            },
        );

        let mut mixer = AudioMixer::new(44100);
        mixer.master_volume = 1.0;
        mixer.sfx_volume = 1.0;
        mixer.music_volume = 0.5;

        // 2 ses başlat: biri SFX, biri Music
        mixer.play("tone_a", AudioBus::Sfx, 1.0, false);
        mixer.play("tone_b", AudioBus::Music, 1.0, false);
        assert_eq!(mixer.active_voice_count(), 2);

        // 4 çerçeve miksle (stereo => 8 float)
        let mut out_stereo = vec![0.0f32; 8];
        mixer.mix_f32(&clips, &mut out_stereo);

        // tone_a katkısı: 16384 / 32768 = 0.5 * 1.0 = 0.5
        // tone_b katkısı: 16384 / 32768 = 0.5 * 0.5 = 0.25
        // Toplam = 0.75
        for &sample in &out_stereo {
            assert!((sample - 0.75).abs() < 1e-3);
        }

        // Sesler 4 örnek uzunluğundaydı, miks sonrası tamamlanmış olmalı
        assert_eq!(mixer.active_voice_count(), 0);

        // PCM testi
        let mut pcm = Vec::new();
        mixer.play("tone_a", AudioBus::Sfx, 1.0, false);
        mixer.mix_i16_pcm(&clips, 2, &mut pcm);
        // 2 çerçeve stereo = 4 i16 = 8 bayt
        assert_eq!(pcm.len(), 8);
    }

    #[test]
    fn test_audio_mixer_linear_interpolation_resampling() {
        let mut clips = std::collections::HashMap::new();
        // 22050 Hz klip, 44100 Hz mikser çıkışı (2x upsampling)
        clips.insert(
            "ramp".to_string(),
            SoundClip {
                name: "ramp".to_string(),
                sample_rate: 22050,
                samples: vec![0, 16384, 32767],
            },
        );

        let mut mixer = AudioMixer::new(44100);
        mixer.master_volume = 1.0;
        mixer.sfx_volume = 1.0;
        mixer.play("ramp", AudioBus::Sfx, 1.0, false);

        // 4 çerçeve miksle (step = 0.5)
        // frame 0: pos = 0.0 -> clip[0] = 0.0
        // frame 1: pos = 0.5 -> interp(0, 16384, 0.5) = 8192 / 32768 = 0.25
        // frame 2: pos = 1.0 -> clip[1] = 16384 / 32768 = 0.5
        // frame 3: pos = 1.5 -> interp(16384, 32767, 0.5) approx 0.75
        let mut out = vec![0.0f32; 8];
        mixer.mix_f32(&clips, &mut out);

        assert!((out[0] - 0.0).abs() < 1e-3);
        assert!((out[2] - 0.25).abs() < 1e-3);
        assert!((out[4] - 0.5).abs() < 1e-3);
        assert!((out[6] - 0.75).abs() < 1e-3);
    }

    #[test]
    fn test_audio_mixer_voice_stealing() {
        let mut mixer = AudioMixer::new(44100);
        mixer.max_voices = 2;

        let v1 = mixer.play("sfx1", AudioBus::Sfx, 1.0, false);
        let v2 = mixer.play("music1", AudioBus::Music, 1.0, false);
        assert_eq!(mixer.active_voice_count(), 2);

        // max_voices = 2 iken 3. ses eklendiğinde SFX (v1) steal edilmeli
        let v3 = mixer.play("sfx2", AudioBus::Sfx, 1.0, false);
        assert_eq!(mixer.active_voice_count(), 2);
        assert!(!mixer.voices.iter().any(|v| v.id == v1));
        assert!(mixer.voices.iter().any(|v| v.id == v2));
        assert!(mixer.voices.iter().any(|v| v.id == v3));
    }

    #[test]
    fn test_audio_ring_buffer_fifo_and_overflow_protection() {
        let mut rb = AudioRingBuffer::new(4);
        assert_eq!(rb.available_write(), 4);
        assert_eq!(rb.available_read(), 0);

        // 3 örnek yaz
        rb.write(&[10, 20, 30]);
        assert_eq!(rb.available_read(), 3);
        assert_eq!(rb.available_write(), 1);

        // 2 örnek oku
        let mut out = [0i16; 2];
        let read_count = rb.read(&mut out);
        assert_eq!(read_count, 2);
        assert_eq!(out, [10, 20]);
        assert_eq!(rb.available_read(), 1);

        // Taşırma (overflow) testi: 4 eleman daha yaz (toplam kapasiteyi aşar)
        rb.write(&[40, 50, 60, 70]);
        assert_eq!(rb.available_read(), 4);

        // En son 4 eleman sırasıyla gelmeli (en eski olan 30 atılmış oldu)
        let mut full_out = [0i16; 4];
        let n = rb.read(&mut full_out);
        assert_eq!(n, 4);
        assert_eq!(full_out, [40, 50, 60, 70]);
        assert_eq!(rb.available_read(), 0);
    }

    #[test]
    fn test_audio_stream_driver_push_and_lifecycle() {
        let mut driver = AudioStreamDriver::new(44100, 2);
        let samples = vec![1000i16, -1000i16, 2000i16, -2000i16];
        driver.push_samples(&samples);
        // Tamponda örnekler bulunmalı (veya arka plan iş parçacığı tarafından okunana dek işlenebilir)
        assert!(driver.buffered_samples() <= 4);
        driver.stop();
    }
}
