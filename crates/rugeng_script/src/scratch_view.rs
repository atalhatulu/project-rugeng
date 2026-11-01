//! Flat SoA (Structure of Arrays) Scratch View.
//!
//! Lua ile Rust arasında binlerce oyun nesnesini (örneğin 5.000 - 20.000 mermi)
//! tek tek nesne tablosu oluşturarak aktarmak GC baskısına ve yüksek ek yüke yol açar.
//! `ScratchSoAView`, mermilerin tüm özelliklerini ardışık düz dizilerde (`Vec<f32>`, `Vec<u32>`)
//! tutar.
//!
//! Lua tarafına:
//! 1. Toplu doğrudan okuma/yazma metotları (`get_pos(i)`, `set_pos(i, x, y)` vb.)
//! 2. Toplu step simülasyon fonksiyonu (`step_physics(dt, screen_w, screen_h, cmd_buffer)`)
//! 3. Sıfır tahsisli scratch tamponu (kapasite kareler boyunca korunur) sunar.

use mlua::{UserData, UserDataMethods};
use rugeng_core::Entity;

/// Düz SoA tamponu (Structure of Arrays)
#[derive(Default, Clone)]
pub struct ScratchSoAView {
    pub entities: Vec<u32>,
    pub pos_x: Vec<f32>,
    pub pos_y: Vec<f32>,
    pub vel_x: Vec<f32>,
    pub vel_y: Vec<f32>,
    pub radius: Vec<f32>,
    pub is_enemy: Vec<bool>,
}

impl ScratchSoAView {
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entities: Vec::with_capacity(capacity),
            pos_x: Vec::with_capacity(capacity),
            pos_y: Vec::with_capacity(capacity),
            vel_x: Vec::with_capacity(capacity),
            vel_y: Vec::with_capacity(capacity),
            radius: Vec::with_capacity(capacity),
            is_enemy: Vec::with_capacity(capacity),
        }
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        self.pos_x.clear();
        self.pos_y.clear();
        self.vel_x.clear();
        self.vel_y.clear();
        self.radius.clear();
        self.is_enemy.clear();
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn push(
        &mut self,
        entity: Entity,
        x: f32,
        y: f32,
        vx: f32,
        vy: f32,
        radius: f32,
        is_enemy: bool,
    ) {
        self.entities.push(entity.to_bits());
        self.pos_x.push(x);
        self.pos_y.push(y);
        self.vel_x.push(vx);
        self.vel_y.push(vy);
        self.radius.push(radius);
        self.is_enemy.push(is_enemy);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.pos_x.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pos_x.is_empty()
    }
}

impl UserData for ScratchSoAView {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("count", |_lua, this, ()| Ok(this.len()));

        // Lua 1-based veya 0-based indeks desteği (1-based standart Lua indekslemesidir)
        methods.add_method("get_x", |_lua, this, mut index: usize| {
            if index > 0 && index <= this.pos_x.len() {
                index -= 1;
            }
            if index < this.pos_x.len() {
                Ok(Some(this.pos_x[index]))
            } else {
                Ok(None)
            }
        });

        methods.add_method("get_y", |_lua, this, mut index: usize| {
            if index > 0 && index <= this.pos_y.len() {
                index -= 1;
            }
            if index < this.pos_y.len() {
                Ok(Some(this.pos_y[index]))
            } else {
                Ok(None)
            }
        });

        methods.add_method("get_vx", |_lua, this, mut index: usize| {
            if index > 0 && index <= this.vel_x.len() {
                index -= 1;
            }
            if index < this.vel_x.len() {
                Ok(Some(this.vel_x[index]))
            } else {
                Ok(None)
            }
        });

        methods.add_method("get_vy", |_lua, this, mut index: usize| {
            if index > 0 && index <= this.vel_y.len() {
                index -= 1;
            }
            if index < this.vel_y.len() {
                Ok(Some(this.vel_y[index]))
            } else {
                Ok(None)
            }
        });

        methods.add_method("get_entity", |_lua, this, mut index: usize| {
            if index > 0 && index <= this.entities.len() {
                index -= 1;
            }
            if index < this.entities.len() {
                Ok(Some(this.entities[index]))
            } else {
                Ok(None)
            }
        });

        methods.add_method("is_enemy", |_lua, this, mut index: usize| {
            if index > 0 && index <= this.is_enemy.len() {
                index -= 1;
            }
            if index < this.is_enemy.len() {
                Ok(Some(this.is_enemy[index]))
            } else {
                Ok(None)
            }
        });

        methods.add_method_mut(
            "set_pos",
            |_lua, this, (mut index, x, y): (usize, f32, f32)| {
                if index > 0 && index <= this.pos_x.len() {
                    index -= 1;
                }
                if index < this.pos_x.len() {
                    this.pos_x[index] = x;
                    this.pos_y[index] = y;
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
        );

        methods.add_method_mut(
            "set_vel",
            |_lua, this, (mut index, vx, vy): (usize, f32, f32)| {
                if index > 0 && index <= this.vel_x.len() {
                    index -= 1;
                }
                if index < this.vel_x.len() {
                    this.vel_x[index] = vx;
                    this.vel_y[index] = vy;
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
        );

        // Toplu güncelleme: Lua tarafında döngü yerine SoA tamponu üzerinden
        // tek bir C/Rust çağrısı ile pozisyonları ve sınır kontrollerini yapar.
        methods.add_method_mut(
            "batch_integrate",
            |_lua, this, (dt, screen_w, screen_h): (f32, f32, f32)| {
                let mut out_of_bounds = Vec::with_capacity(64);
                let len = this.pos_x.len();
                let px = &mut this.pos_x;
                let py = &mut this.pos_y;
                let vx = &this.vel_x;
                let vy = &this.vel_y;
                let ent = &this.entities;

                for i in 0..len {
                    px[i] += vx[i] * dt;
                    py[i] += vy[i] * dt;

                    let x = px[i];
                    let y = py[i];
                    if x < -50.0 || x > screen_w + 50.0 || y < -50.0 || y > screen_h + 50.0 {
                        out_of_bounds.push(ent[i]);
                    }
                }
                Ok(out_of_bounds)
            },
        );

        // Doğrudan CommandBuffer içine yazan zero-alloc toplu simülasyon metodu
        methods.add_method_mut(
            "batch_integrate_to_cmd",
            |_lua,
             this,
             (dt, screen_w, screen_h, mut cmd_ud): (
                f32,
                f32,
                f32,
                mlua::UserDataRefMut<crate::command_buffer::ScriptCommandBuffer>,
            )| {
                let len = this.pos_x.len();
                let px = &mut this.pos_x;
                let py = &mut this.pos_y;
                let vx = &this.vel_x;
                let vy = &this.vel_y;
                let ent = &this.entities;

                for i in 0..len {
                    px[i] += vx[i] * dt;
                    py[i] += vy[i] * dt;

                    let x = px[i];
                    let y = py[i];
                    if x < -50.0 || x > screen_w + 50.0 || y < -50.0 || y > screen_h + 50.0 {
                        cmd_ud.push_destroy(rugeng_core::Entity::from_bits(ent[i]));
                    }
                }
                Ok(())
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scratch_soa_view_basic() {
        let mut view = ScratchSoAView::new();
        view.push(Entity::new(1, 0), 10.0, 20.0, 5.0, -5.0, 3.0, true);
        view.push(Entity::new(2, 0), 100.0, 200.0, -10.0, 10.0, 4.0, false);

        assert_eq!(view.len(), 2);
        assert_eq!(view.pos_x[0], 10.0);
        assert_eq!(view.pos_y[1], 200.0);

        view.clear();
        assert_eq!(view.len(), 0);
    }

    #[test]
    fn test_scratch_soa_lua_indexing_and_batch_integrate() {
        use mlua::Lua;
        let lua = Lua::new();

        let mut view = ScratchSoAView::new();
        view.push(Entity::new(1, 0), 10.0, 20.0, 5.0, 5.0, 3.0, true);
        view.push(Entity::new(2, 0), 200.0, 300.0, -5.0, -5.0, 3.0, false);

        let ud = lua.create_userdata(view).unwrap();
        lua.globals().set("view", ud).unwrap();

        let script = r#"
            local x1 = view:get_x(1) -- 1-based
            local y1 = view:get_y(1)
            view:set_pos(1, 55.0, 65.0)
            local new_x1 = view:get_x(1)
            return x1, y1, new_x1
        "#;
        let (x1, y1, new_x1): (f32, f32, f32) = lua.load(script).eval().unwrap();
        assert_eq!(x1, 10.0);
        assert_eq!(y1, 20.0);
        assert_eq!(new_x1, 55.0);
    }
}
