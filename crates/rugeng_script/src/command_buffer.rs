//! Komut Tamponu (ScriptCommandBuffer): Lua/Rust Sınırında Toplu Değişiklik Kuyruğu.
//!
//! Lua sanal makinesinden doğrudan her varlık (entity) için Rust FFI çağrısı yapmak (per-entity FFI)
//! FFI geçiş maliyetini katlayarak performansı düşürür.
//! `ScriptCommandBuffer`, Lua ortamından üretilen spawn, destroy ve bileşen atama işlemlerini
//! düz ve ardışık bir tamponda toplar. Tek bir Rust geçişinde ECS'ye sıfır heap tahsisi ile uygular.

use mlua::{UserData, UserDataMethods};
use rugeng_core::{CommandBuffer, Entity};

/// Mermi oluşturma parametreleri (batch spawn için)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpawnBulletCmd {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub is_enemy: bool,
    pub radius: f32,
}

/// Script tarafından üretilen yapısal komutlar
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScriptCommand {
    SpawnBullet(SpawnBulletCmd),
    Destroy(Entity),
}

/// Lua ortamına doğrudan sunulan toplu komut tamponu
#[derive(Default)]
pub struct ScriptCommandBuffer {
    commands: Vec<ScriptCommand>,
}

impl ScriptCommandBuffer {
    pub fn new() -> Self {
        Self {
            commands: Vec::with_capacity(1024),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            commands: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn push_spawn_bullet(&mut self, cmd: SpawnBulletCmd) {
        self.commands.push(ScriptCommand::SpawnBullet(cmd));
    }

    #[inline]
    pub fn push_destroy(&mut self, entity: Entity) {
        self.commands.push(ScriptCommand::Destroy(entity));
    }

    #[inline]
    pub fn commands(&self) -> &[ScriptCommand] {
        &self.commands
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// Tampondaki tüm komutları `rugeng_core::CommandBuffer` üzerine veya
    /// özel spawn handler fonksiyonuna aktarır.
    pub fn apply_to_core(
        &mut self,
        core_cmd: &mut CommandBuffer,
        mut spawn_bullet_fn: impl FnMut(SpawnBulletCmd),
    ) {
        for cmd in self.commands.drain(..) {
            match cmd {
                ScriptCommand::SpawnBullet(b) => {
                    spawn_bullet_fn(b);
                }
                ScriptCommand::Destroy(e) => {
                    core_cmd.destroy(e);
                }
            }
        }
    }
}

impl UserData for ScriptCommandBuffer {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut(
            "spawn_bullet",
            |_lua, this, (x, y, vx, vy, is_enemy, radius): (f32, f32, f32, f32, bool, f32)| {
                this.push_spawn_bullet(SpawnBulletCmd {
                    x,
                    y,
                    vx,
                    vy,
                    is_enemy,
                    radius,
                });
                Ok(())
            },
        );

        methods.add_method_mut("destroy", |_lua, this, entity_bits: u32| {
            this.push_destroy(Entity::from_bits(entity_bits));
            Ok(())
        });

        methods.add_method("len", |_lua, this, ()| Ok(this.len()));
        methods.add_method_mut("clear", |_lua, this, ()| {
            this.clear();
            Ok(())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_command_buffer_basic() {
        let mut buffer = ScriptCommandBuffer::new();
        buffer.push_spawn_bullet(SpawnBulletCmd {
            x: 10.0,
            y: 20.0,
            vx: 1.0,
            vy: 2.0,
            is_enemy: true,
            radius: 4.0,
        });
        buffer.push_destroy(Entity::new(5, 1));

        assert_eq!(buffer.len(), 2);

        let mut spawned = Vec::new();
        let mut core_cmd = CommandBuffer::new();
        buffer.apply_to_core(&mut core_cmd, |b| {
            spawned.push(b);
        });

        assert_eq!(buffer.len(), 0);
        assert_eq!(spawned.len(), 1);
        assert_eq!(spawned[0].x, 10.0);
    }
}
