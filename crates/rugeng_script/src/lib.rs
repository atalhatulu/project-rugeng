//! Luau Script Motoru ve Korumalı Çalışma Zamanı (rugeng_script).
//!
//! Bu modül Luau sanal makinesini oyun motoru için sarmalar:
//! - Bellek sınırları (Memory Limit sandbox)
//! - Sonsuz döngü ve aşırı CPU koruması (Interrupt-based instruction limit)
//! - Detaylı hata mesajları (dosya adı, satır ve traceback)
//! - Durum korumalı hot reload desteği (state migration)
//! - Toplu veri aktarımı (SoA Scratch View + ScriptCommandBuffer)

pub mod command_buffer;
pub mod engine;
pub mod scratch_view;

pub use command_buffer::{ScriptCommand, ScriptCommandBuffer, SpawnBulletCmd};
pub use engine::{ScriptEngine, ScriptEngineConfig, ScriptError};
pub use scratch_view::ScratchSoAView;
