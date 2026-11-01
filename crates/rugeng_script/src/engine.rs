//! Korumalı Luau Script Motoru (ScriptEngine).

use crate::command_buffer::ScriptCommandBuffer;
use crate::scratch_view::ScratchSoAView;
use mlua::{Lua, LuaOptions, StdLib, Table, VmState};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

/// Script çalıştırma veya derleme hataları
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptError {
    pub file: String,
    pub message: String,
    pub traceback: String,
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[SCRIPT HATA] Dosya: {}\nMesaj: {}\nİzleme:\n{}",
            self.file, self.message, self.traceback
        )
    }
}

impl std::error::Error for ScriptError {}

/// Yüklenen script dosyasının canlı durumu
#[allow(dead_code)]
struct LoadedScript {
    path: PathBuf,
    name: String,
    last_modified: SystemTime,
    code: String,
}

/// Script yapılandırma seçenekleri
#[derive(Clone, Debug)]
pub struct ScriptEngineConfig {
    /// Maksimum heap bellek sınırı (Bayt cinsinden, örn. 16MB)
    pub memory_limit_bytes: usize,
    /// Sonsuz döngü ve kilitlenme kesme eşiği
    pub max_instructions_per_call: usize,
}

impl Default for ScriptEngineConfig {
    fn default() -> Self {
        Self {
            memory_limit_bytes: 32 * 1024 * 1024, // 32 MB
            max_instructions_per_call: 20_000,    // Kesme adımı
        }
    }
}

pub struct ScriptEngine {
    lua: Lua,
    config: ScriptEngineConfig,
    loaded_scripts: HashMap<String, LoadedScript>,
    instruction_counter: Arc<AtomicUsize>,
    last_error: Arc<Mutex<Option<ScriptError>>>,
}

impl ScriptEngine {
    #[inline]
    pub fn config(&self) -> &ScriptEngineConfig {
        &self.config
    }
    /// Yeni ve korumalı bir Luau sanal makinesi oluşturur
    pub fn new(config: ScriptEngineConfig) -> Result<Self, ScriptError> {
        // Güvenli kütüphaneler: Luau'da ALL_SAFE veya TABLE | STRING | MATH | UTF8 | BIT | BUFFER | VECTOR
        let stdlib = StdLib::TABLE
            | StdLib::STRING
            | StdLib::MATH
            | StdLib::UTF8
            | StdLib::BIT
            | StdLib::BUFFER
            | StdLib::VECTOR;

        let lua = Lua::new_with(stdlib, LuaOptions::default()).map_err(|e| ScriptError {
            file: "engine_init".into(),
            message: e.to_string(),
            traceback: String::new(),
        })?;

        // 1. Bellek Sınırı (G6: Memory limit sandbox)
        if config.memory_limit_bytes > 0 {
            let _ = lua.set_memory_limit(config.memory_limit_bytes);
        }

        // 2. Sonsuz Döngü Koruması (G6: Instruction limit / interrupt hook)
        let instruction_counter = Arc::new(AtomicUsize::new(0));
        let max_inst = config.max_instructions_per_call;
        let counter_clone = instruction_counter.clone();

        lua.set_interrupt(move |_| {
            let count = counter_clone.fetch_add(1, Ordering::Relaxed);
            if count > max_inst {
                Err(mlua::Error::RuntimeError(
                    "Script zaman/talimat aşımı (sonsuz döngü koruması tetiklendi)".into(),
                ))
            } else {
                Ok(VmState::Continue)
            }
        });

        Ok(Self {
            lua,
            config,
            loaded_scripts: HashMap::new(),
            instruction_counter,
            last_error: Arc::new(Mutex::new(None)),
        })
    }

    /// Son oluşan hatayı alır
    pub fn last_error(&self) -> Option<ScriptError> {
        self.last_error.lock().clone()
    }

    /// Hata kaydını temizler
    pub fn clear_error(&self) {
        *self.last_error.lock() = None;
    }

    /// Kesme sayacını sıfırlar (her çağrı öncesi)
    #[inline]
    fn reset_instruction_counter(&self) {
        self.instruction_counter.store(0, Ordering::Relaxed);
    }

    /// Bir script metnini derler ve modül olarak kaydeder
    pub fn load_script(
        &mut self,
        name: &str,
        source: &str,
        path: Option<PathBuf>,
    ) -> Result<(), ScriptError> {
        self.reset_instruction_counter();

        let chunk = self.lua.load(source).set_name(name);
        let res = chunk.exec();

        match res {
            Ok(_) => {
                let mtime = if let Some(ref p) = path {
                    std::fs::metadata(p)
                        .and_then(|m| m.modified())
                        .unwrap_or_else(|_| SystemTime::now())
                } else {
                    SystemTime::now()
                };

                self.loaded_scripts.insert(
                    name.to_string(),
                    LoadedScript {
                        path: path.unwrap_or_else(|| PathBuf::from(name)),
                        name: name.to_string(),
                        last_modified: mtime,
                        code: source.to_string(),
                    },
                );
                Ok(())
            }
            Err(e) => {
                let err = Self::parse_mlua_error(name, e);
                *self.last_error.lock() = Some(err.clone());
                Err(err)
            }
        }
    }

    /// Bir script dosyasını diskten yükler
    pub fn load_file(&mut self, name: &str, path: impl AsRef<Path>) -> Result<(), ScriptError> {
        let p = path.as_ref();
        let content = std::fs::read_to_string(p).map_err(|e| ScriptError {
            file: p.to_string_lossy().to_string(),
            message: format!("Dosya okunamadı: {}", e),
            traceback: String::new(),
        })?;

        self.load_script(name, &content, Some(p.to_path_buf()))
    }

    /// Disk üzerindeki script dosyalarını kontrol ederek değişenleri durumu koruyarak yeniden yükler (G3: Hot Reload)
    pub fn poll_hot_reload(&mut self) -> Vec<String> {
        let mut reloaded = Vec::new();

        // Değişen scriptleri tespit et
        let mut to_reload = Vec::new();
        for (name, item) in &self.loaded_scripts {
            if let Ok(metadata) = std::fs::metadata(&item.path) {
                if let Ok(modified) = metadata.modified() {
                    if modified > item.last_modified {
                        if let Ok(new_code) = std::fs::read_to_string(&item.path) {
                            to_reload.push((name.clone(), item.path.clone(), modified, new_code));
                        }
                    }
                }
            }
        }

        for (name, _path, modified, new_code) in to_reload {
            // Durum koruma (State preservation):
            // Eğer script tablosu `export_state` fonksiyonu içeriyorsa state alınır,
            // yeniden yüklemeden sonra `import_state` ile geri verilir.
            let saved_state = self.export_module_state(&name);

            self.reset_instruction_counter();
            let chunk = self.lua.load(&new_code).set_name(&name);

            match chunk.exec() {
                Ok(_) => {
                    if let Some(state) = saved_state {
                        let _ = self.import_module_state(&name, state);
                    }

                    if let Some(item) = self.loaded_scripts.get_mut(&name) {
                        item.last_modified = modified;
                        item.code = new_code;
                    }
                    reloaded.push(name);
                }
                Err(e) => {
                    let err = Self::parse_mlua_error(&name, e);
                    *self.last_error.lock() = Some(err);
                    // Hatalı dosyayı her karede tekrar tekrar derleyip CPU boğulmasını önlemek için last_modified güncellenir
                    if let Some(item) = self.loaded_scripts.get_mut(&name) {
                        item.last_modified = modified;
                    }
                }
            }
        }

        reloaded
    }

    /// Modülün durumunu (state) kaydeder
    fn export_module_state(&self, module_name: &str) -> Option<Table> {
        let globals = self.lua.globals();
        if let Ok(module_tbl) = globals.get::<Table>(module_name) {
            if let Ok(export_fn) = module_tbl.get::<mlua::Function>("export_state") {
                return export_fn.call::<Table>(()).ok();
            }
        }
        None
    }

    /// Modülün durumunu (state) geri yükler
    fn import_module_state(&self, module_name: &str, state: Table) -> Result<(), ScriptError> {
        let globals = self.lua.globals();
        if let Ok(module_tbl) = globals.get::<Table>(module_name) {
            if let Ok(import_fn) = module_tbl.get::<mlua::Function>("import_state") {
                import_fn.call::<()>(state).map_err(|e| {
                    let err = Self::parse_mlua_error(module_name, e);
                    *self.last_error.lock() = Some(err.clone());
                    err
                })?;
            }
        }
        Ok(())
    }

    /// Bir Lua global fonksiyonunu veya modül fonksiyonunu çağırır
    pub fn call_function<A, R>(&self, func_name: &str, args: A) -> Result<R, ScriptError>
    where
        A: mlua::IntoLuaMulti,
        R: mlua::FromLuaMulti,
    {
        self.reset_instruction_counter();
        let globals = self.lua.globals();

        let func: mlua::Function = if func_name.contains('.') {
            let parts: Vec<&str> = func_name.split('.').collect();
            let mut current = globals.get::<Table>(parts[0]).map_err(|e| {
                Self::parse_mlua_error(parts[0], mlua::Error::RuntimeError(e.to_string()))
            })?;
            for part in parts.iter().take(parts.len() - 1).skip(1) {
                current = current.get::<Table>(*part).map_err(|e| {
                    Self::parse_mlua_error(part, mlua::Error::RuntimeError(e.to_string()))
                })?;
            }
            current
                .get::<mlua::Function>(parts[parts.len() - 1])
                .map_err(|e| {
                    Self::parse_mlua_error(func_name, mlua::Error::RuntimeError(e.to_string()))
                })?
        } else {
            globals.get::<mlua::Function>(func_name).map_err(|e| {
                Self::parse_mlua_error(func_name, mlua::Error::RuntimeError(e.to_string()))
            })?
        };

        func.call::<R>(args).map_err(|e| {
            let err = Self::parse_mlua_error(func_name, e);
            *self.last_error.lock() = Some(err.clone());
            err
        })
    }

    /// Boss/Düşman mermi örüntüsünü çağırır
    /// ScriptCommandBuffer parametresi ile FFI çağrısı tek nesne başına değil, toplu kuyrukla yapılır.
    #[allow(clippy::too_many_arguments)]
    pub fn update_boss_pattern(
        &self,
        module_name: &str,
        pattern_mode: usize,
        pattern_angle: f32,
        boss_x: f32,
        boss_y: f32,
        spawn_count: usize,
        cmd_buffer: &mut ScriptCommandBuffer,
    ) -> Result<f32, ScriptError> {
        self.reset_instruction_counter();
        let globals = self.lua.globals();

        let module = globals.get::<Table>(module_name).map_err(|e| {
            Self::parse_mlua_error(module_name, mlua::Error::RuntimeError(e.to_string()))
        })?;

        let update_fn = module.get::<mlua::Function>("update_boss").map_err(|e| {
            Self::parse_mlua_error(module_name, mlua::Error::RuntimeError(e.to_string()))
        })?;

        // ScriptCommandBuffer UserData olarak aktarılır
        let cmd_ud = self
            .lua
            .create_userdata(std::mem::take(cmd_buffer))
            .map_err(|e| {
                Self::parse_mlua_error(module_name, mlua::Error::RuntimeError(e.to_string()))
            })?;

        let new_angle: f32 = update_fn
            .call((
                pattern_mode,
                pattern_angle,
                boss_x,
                boss_y,
                spawn_count,
                cmd_ud.clone(),
            ))
            .map_err(|e| {
                let err = Self::parse_mlua_error(module_name, e);
                *self.last_error.lock() = Some(err.clone());
                err
            })?;

        // Tampondaki komutları geri al
        if let Ok(mut ud_mut) = cmd_ud.borrow_mut::<ScriptCommandBuffer>() {
            *cmd_buffer = std::mem::take(&mut *ud_mut);
        }

        Ok(new_angle)
    }

    /// Flat SoA scratch tamponu üzerinde mermi fizik simülasyonunu çağırır (G2 & G4)
    pub fn update_bullets_batch(
        &self,
        module_name: &str,
        view: &mut ScratchSoAView,
        dt: f32,
        screen_w: f32,
        screen_h: f32,
        cmd_buffer: &mut ScriptCommandBuffer,
    ) -> Result<(), ScriptError> {
        self.reset_instruction_counter();
        let globals = self.lua.globals();

        let module = globals.get::<Table>(module_name).map_err(|e| {
            Self::parse_mlua_error(module_name, mlua::Error::RuntimeError(e.to_string()))
        })?;

        let update_fn = module
            .get::<mlua::Function>("update_bullets")
            .map_err(|e| {
                Self::parse_mlua_error(module_name, mlua::Error::RuntimeError(e.to_string()))
            })?;

        let view_ud = self
            .lua
            .create_userdata(std::mem::take(view))
            .map_err(|e| {
                Self::parse_mlua_error(module_name, mlua::Error::RuntimeError(e.to_string()))
            })?;

        let cmd_ud = self
            .lua
            .create_userdata(std::mem::take(cmd_buffer))
            .map_err(|e| {
                Self::parse_mlua_error(module_name, mlua::Error::RuntimeError(e.to_string()))
            })?;

        let res: mlua::Result<()> =
            update_fn.call((view_ud.clone(), dt, screen_w, screen_h, cmd_ud.clone()));

        if let Ok(mut v_mut) = view_ud.borrow_mut::<ScratchSoAView>() {
            *view = std::mem::take(&mut *v_mut);
        }
        if let Ok(mut c_mut) = cmd_ud.borrow_mut::<ScriptCommandBuffer>() {
            *cmd_buffer = std::mem::take(&mut *c_mut);
        }

        res.map_err(|e| {
            let err = Self::parse_mlua_error(module_name, e);
            *self.last_error.lock() = Some(err.clone());
            err
        })
    }

    /// mlua hatasını detaylı dosya, satır ve traceback içeren `ScriptError` tipine dönüştürür (G1)
    fn parse_mlua_error(file_context: &str, err: mlua::Error) -> ScriptError {
        let full_str = err.to_string();
        let mut lines = full_str.lines();
        let first_line = lines.next().unwrap_or("Bilinmeyen script hatası");

        // Format: [string "file.lua"]:line: message
        let traceback = lines.collect::<Vec<&str>>().join("\n");

        ScriptError {
            file: file_context.to_string(),
            message: first_line.to_string(),
            traceback,
        }
    }

    pub fn raw_lua(&self) -> &Lua {
        &self.lua
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_error_reporting_with_line_and_file() {
        let mut engine = ScriptEngine::new(ScriptEngineConfig::default()).unwrap();
        let script = r#"
            local x = 10
            error("Kritik test hatasi!")
        "#;
        let res = engine.load_script("test_err.lua", script, None);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert_eq!(err.file, "test_err.lua");
        assert!(
            err.message.contains("test_err.lua") || err.message.contains("Kritik test hatasi!")
        );
        assert!(engine.last_error().is_some());
    }

    #[test]
    fn test_infinite_loop_protection() {
        let config = ScriptEngineConfig {
            max_instructions_per_call: 500,
            ..Default::default()
        };
        let mut engine = ScriptEngine::new(config).unwrap();

        let script = r#"
            while true do
                -- sonsuz döngü
            end
        "#;
        let res = engine.load_script("infinite.lua", script, None);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.message.contains("sonsuz döngü") || err.message.contains("zaman/talimat"));
    }

    #[test]
    fn test_memory_limit_sandbox() {
        let config = ScriptEngineConfig {
            memory_limit_bytes: 256 * 1024, // 256 KB sıkı limit
            ..Default::default()
        };
        let mut engine = ScriptEngine::new(config).unwrap();

        let script = r#"
            local arr = {}
            for i = 1, 100000 do
                arr[i] = "buyuk string bellek tasmasi denemesi " .. i
            end
        "#;
        let res = engine.load_script("oom.lua", script, None);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.message.to_lowercase().contains("memory"));
    }

    #[test]
    fn test_state_preservation_hot_reload() {
        let mut engine = ScriptEngine::new(ScriptEngineConfig::default()).unwrap();
        let script_v1 = r#"
            BossLogic = {
                phase = 1,
                hp = 5000
            }
            function BossLogic.export_state()
                return { phase = BossLogic.phase, hp = BossLogic.hp }
            end
            function BossLogic.import_state(s)
                BossLogic.phase = s.phase
                BossLogic.hp = s.hp
            end
        "#;
        engine.load_script("BossLogic", script_v1, None).unwrap();

        // Lua içinde phase ve hp'yi güncelle
        engine
            .lua
            .load("BossLogic.phase = 2; BossLogic.hp = 3200")
            .exec()
            .unwrap();

        // V2 script yükle (aynı export/import ile)
        let script_v2 = r#"
            BossLogic = {
                phase = 1,
                hp = 10000,
                new_feature = true
            }
            function BossLogic.export_state()
                return { phase = BossLogic.phase, hp = BossLogic.hp }
            end
            function BossLogic.import_state(s)
                BossLogic.phase = s.phase
                BossLogic.hp = s.hp
            end
        "#;

        // export state
        let state = engine.export_module_state("BossLogic").unwrap();
        // reload v2
        engine
            .lua
            .load(script_v2)
            .set_name("BossLogic")
            .exec()
            .unwrap();
        // import state
        engine.import_module_state("BossLogic", state).unwrap();

        let phase: i32 = engine.lua.load("return BossLogic.phase").eval().unwrap();
        let hp: i32 = engine.lua.load("return BossLogic.hp").eval().unwrap();
        let new_feature: bool = engine
            .lua
            .load("return BossLogic.new_feature")
            .eval()
            .unwrap();

        assert_eq!(phase, 2);
        assert_eq!(hp, 3200);
        assert!(new_feature);
    }

    #[test]
    fn test_poll_hot_reload_file_and_error_handling() {
        let temp_dir =
            std::env::temp_dir().join(format!("rugeng_hotreload_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let test_file = temp_dir.join("test_mod.lua");

        let mut engine = ScriptEngine::new(ScriptEngineConfig::default()).unwrap();
        std::fs::write(&test_file, "TestMod = { val = 42 }\n").unwrap();

        engine.load_file("TestMod", &test_file).unwrap();
        let val: i32 = engine.lua.load("return TestMod.val").eval().unwrap();
        assert_eq!(val, 42);

        // Değişiklik olmaksızın poll
        let reloaded = engine.poll_hot_reload();
        assert!(reloaded.is_empty());

        // Hatalı içerik yaz
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&test_file, "syntax error here !!!").unwrap();

        let reloaded = engine.poll_hot_reload();
        assert!(reloaded.is_empty());
        assert!(engine.last_error().is_some());

        // Tekrar poll edildiğinde mtime güncellendiği için hata fırtınası oluşmamalı
        engine.clear_error();
        let reloaded2 = engine.poll_hot_reload();
        assert!(reloaded2.is_empty());
        assert!(engine.last_error().is_none());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
