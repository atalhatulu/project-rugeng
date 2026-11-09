use crate::types::*;
use crate::App;
use glam::Vec2;
use rugeng_core::{
    compute_world_transform, Animation2DComponent, Animation3DComponent, Collider2D, Entity,
    Parent, ParticleEmitterComponent, ScriptComponent, SpriteTexture, Tilemap2DComponent,
    Transform2D, Transform3D, Velocity2D,
};
use rugeng_physics::resolve_aabb_collision;
impl App {
    pub(crate) fn update_simulation(&mut self, dt: f32) {
        // 1. 3D Serbest Uçuş & Flycam Kamerası Hareketi (ThreeD modunda ve SADECE SAĞ TIK BASILI İKEN)
        if self.editor.workspace == WorkspaceMode::ThreeD && self.editor.camera_3d.is_flying {
            let speed = self.editor.camera_3d.effective_speed() * dt;
            let fwd = self.editor.camera_3d.forward();
            let right = self.editor.camera_3d.right();
            let up = glam::Vec3::Y; // Dünya dikey Y ekseni

            // W: İleri (Kameranın baktığı yöne doğru)
            if self.action_map.is_action_pressed("move_up") {
                self.editor.camera_3d.eye += fwd * speed;
            }
            // S: Geri (Kameranın baktığı yönün tersine)
            if self.action_map.is_action_pressed("move_down") {
                self.editor.camera_3d.eye -= fwd * speed;
            }
            // A: Sola kayma (Strafe Left)
            if self.action_map.is_action_pressed("move_left") {
                self.editor.camera_3d.eye -= right * speed;
            }
            // D: Sağa kayma (Strafe Right)
            if self.action_map.is_action_pressed("move_right") {
                self.editor.camera_3d.eye += right * speed;
            }
            // E veya Space: Dikey Yukarı yükselme (Fly Up)
            if self.action_map.is_action_pressed("fly_up")
                || self.action_map.is_action_pressed("action_jump")
            {
                self.editor.camera_3d.eye += up * speed;
            }
            // Q: Dikey Aşağı alçalma (Fly Down)
            if self.action_map.is_action_pressed("fly_down") {
                self.editor.camera_3d.eye -= up * speed;
            }
        }

        // 2. Kamera Takibi ve Ekran Sarsıntısı (Screen Shake) - Her zaman sönümlenir
        let target_pos = if self.editor.is_playing {
            self.editor
                .camera
                .follow_target
                .and_then(|tgt| self.registry.get::<Transform2D>(tgt).map(|t| t.pos))
        } else {
            None
        };
        self.editor.camera.update(dt, target_pos);

        // 2. Parçacık Sistemi Güncellemesi - Parçacıkların pürüzsüz akması için daima güncellenir
        let mut finished_emitters = Vec::new();
        {
            let pool_tr = unsafe { self.registry.storage::<Transform2D>() };
            let pool_emitters = unsafe { self.registry.storage_mut::<ParticleEmitterComponent>() };
            let (emitter_entities, emitters) = pool_emitters.split_entities_and_components_mut();
            for (idx, &e) in emitter_entities.iter().enumerate() {
                let emitter = &mut emitters[idx];
                let origin = pool_tr.get(e).map(|t| t.pos).unwrap_or(Vec2::ZERO);
                emitter.update(dt, origin);
                if emitter.is_burst && emitter.particles.is_empty() {
                    finished_emitters.push(e);
                }
            }
        }
        for fe in finished_emitters {
            self.cmd_buffer.destroy(fe);
        }
        self.cmd_buffer.apply(&mut self.registry);

        if !self.editor.is_playing {
            return;
        }

        self.step_simulation_single_frame(dt);
    }

    pub(crate) fn step_simulation_single_frame(&mut self, dt: f32) {
        if self.scheduler.system_count() == 0 {
            self.init_simulation_scheduler();
        }

        // SystemDAG executes non-conflicting systems concurrently across workers in self.thread_pool
        self.scheduler
            .run_all_dt(&self.registry, &self.thread_pool, dt);
    }

    pub(crate) fn init_simulation_scheduler(&mut self) {
        use rugeng_scheduler::access::SystemAccess;

        self.scheduler = rugeng_scheduler::SystemDAG::new();

        // 1. Physics 2D System: Transform2D (Write) + Velocity2D (Read/Write)
        self.scheduler.add_system(
            "Physics2D",
            SystemAccess::new()
                .with_write::<Transform2D>()
                .with_write::<Velocity2D>(),
            |registry, _pool, dt| {
                // SAFETY: SystemAccess guarantees exclusive access to Transform2D and Velocity2D in this system.
                let pool_tr = unsafe { registry.storage_mut::<Transform2D>() };
                let pool_vel = unsafe { registry.storage_mut::<Velocity2D>() };

                let (entities, transforms) = pool_tr.split_entities_and_components_mut();
                for (idx, &e) in entities.iter().enumerate() {
                    if let Some(vel) = pool_vel.get_mut(e) {
                        let tr = &mut transforms[idx];
                        tr.pos += vel.vel * dt;
                        if tr.pos.x > 320.0 || tr.pos.x < -320.0 {
                            vel.vel.x = -vel.vel.x;
                        }
                        if tr.pos.y > 220.0 || tr.pos.y < -220.0 {
                            vel.vel.y = -vel.vel.y;
                        }
                    }
                }
            },
        );

        // 2. Script System: ScriptComponent (Write) + Transform2D (Write)
        self.scheduler.add_system(
            "Scripts",
            SystemAccess::new()
                .with_write::<ScriptComponent>()
                .with_write::<Transform2D>(),
            |registry, _pool, dt| {
                // SAFETY: SystemAccess guarantees exclusive access to ScriptComponent and Transform2D.
                let pool_scripts = unsafe { registry.storage_mut::<ScriptComponent>() };
                let pool_tr = unsafe { registry.storage_mut::<Transform2D>() };

                let (entities, scripts) = pool_scripts.split_entities_and_components_mut();
                for (idx, &e) in entities.iter().enumerate() {
                    if let Some(tr) = pool_tr.get_mut(e) {
                        scripts[idx].update(dt, &mut tr.pos, &mut tr.size, &mut tr.rotation);
                    }
                }
            },
        );

        // 3. Hierarchy System: Parent (Read) + Transform2D (Write)
        self.scheduler.add_system(
            "Hierarchy",
            SystemAccess::new()
                .with_read::<Parent>()
                .with_write::<Transform2D>(),
            |registry, _pool, _dt| {
                // SAFETY: SystemAccess guarantees read access to Parent and exclusive access to Transform2D.
                let pool_parent = unsafe { registry.storage::<Parent>() };
                let parent_links: Vec<(Entity, Entity)> = pool_parent
                    .entities()
                    .iter()
                    .copied()
                    .zip(pool_parent.components().iter().map(|p| p.entity))
                    .collect();

                let pool_tr = unsafe { registry.storage_mut::<Transform2D>() };
                for (child, parent) in parent_links {
                    if let Some(parent_tr) = pool_tr.get(parent).copied() {
                        if let Some(child_tr) = pool_tr.get_mut(child) {
                            let (w_pos, w_rot) = compute_world_transform(
                                Vec2::new(24.0, 0.0),
                                0.0,
                                parent_tr.pos,
                                parent_tr.rotation,
                            );
                            child_tr.pos = w_pos;
                            child_tr.rotation = w_rot;
                        }
                    }
                }
            },
        );

        // 4. Collider 2D & Tilemap System: Collider2D (Read), Tilemap2DComponent (Read), Transform2D (Write)
        self.scheduler.add_system(
            "Colliders2D",
            SystemAccess::new()
                .with_read::<Collider2D>()
                .with_read::<Tilemap2DComponent>()
                .with_write::<Transform2D>(),
            |registry, _pool, _dt| {
                // SAFETY: SystemAccess guarantees read access to Collider2D & Tilemap2DComponent, exclusive to Transform2D.
                let pool_col = unsafe { registry.storage::<Collider2D>() };
                let col_data: Vec<(Entity, bool, bool)> = pool_col
                    .entities()
                    .iter()
                    .copied()
                    .zip(pool_col.components().iter())
                    .filter(|(_, col)| !col.is_sensor)
                    .map(|(e, col)| (e, col.is_sensor, col.is_dynamic))
                    .collect();

                let pool_tr = unsafe { registry.storage_mut::<Transform2D>() };

                for i in 0..col_data.len() {
                    for j in (i + 1)..col_data.len() {
                        let (e_a, _, is_dyn_a) = col_data[i];
                        let (e_b, _, is_dyn_b) = col_data[j];

                        if !is_dyn_a && !is_dyn_b {
                            continue;
                        }

                        debug_assert_ne!(e_a, e_b, "Colliders in pair resolution must be distinct");

                        let cur_a_opt = pool_tr.get(e_a).copied();
                        let cur_b_opt = pool_tr.get(e_b).copied();

                        if let (Some(mut tr_a), Some(mut tr_b)) = (cur_a_opt, cur_b_opt) {
                            if resolve_aabb_collision(
                                &mut tr_a.pos,
                                tr_a.size,
                                is_dyn_a,
                                &mut tr_b.pos,
                                tr_b.size,
                                is_dyn_b,
                            ) {
                                if let Some(t) = pool_tr.get_mut(e_a) {
                                    t.pos = tr_a.pos;
                                }
                                if let Some(t) = pool_tr.get_mut(e_b) {
                                    t.pos = tr_b.pos;
                                }
                            }
                        }
                    }
                }

                // Tilemap Solid Collisions
                let pool_tilemap = unsafe { registry.storage::<Tilemap2DComponent>() };
                let solid_boxes_list: Vec<Vec<(Vec2, Vec2)>> = pool_tilemap
                    .components()
                    .iter()
                    .map(|map| map.solid_aabb_boxes())
                    .collect();

                if !solid_boxes_list.is_empty() {
                    let dynamic_colliders: Vec<Entity> = col_data
                        .iter()
                        .filter(|(_, _, is_dyn)| *is_dyn)
                        .map(|(e, _, _)| *e)
                        .collect();

                    for e in dynamic_colliders {
                        if let Some(tr) = pool_tr.get_mut(e) {
                            for solid_boxes in &solid_boxes_list {
                                for &(tile_center, tile_sz) in solid_boxes {
                                    let mut dummy_tile_pos = tile_center;
                                    resolve_aabb_collision(
                                        &mut tr.pos,
                                        tr.size,
                                        true,
                                        &mut dummy_tile_pos,
                                        tile_sz,
                                        false,
                                    );
                                }
                            }
                        }
                    }
                }
            },
        );

        // 5. 2D Sprite Animation System: Animation2DComponent (Write) + SpriteTexture (Write)
        // Disjoint from Transform2D! Can execute concurrently with 2D physics or 3D systems.
        self.scheduler.add_system(
            "Animations2D",
            SystemAccess::new()
                .with_write::<Animation2DComponent>()
                .with_write::<SpriteTexture>(),
            |registry, _pool, dt| {
                // SAFETY: SystemAccess guarantees exclusive access to Animation2DComponent and SpriteTexture.
                let pool_anim = unsafe { registry.storage_mut::<Animation2DComponent>() };
                let pool_tex = unsafe { registry.storage_mut::<SpriteTexture>() };

                let (entities, anims) = pool_anim.split_entities_and_components_mut();
                for (idx, &e) in entities.iter().enumerate() {
                    let anim = &mut anims[idx];
                    if anim.update(dt) {
                        if let Some(tex_name) = anim.current_texture_name() {
                            if let Some(tex_comp) = pool_tex.get_mut(e) {
                                tex_comp.name = tex_name.to_string();
                            }
                        }
                    }
                }
            },
        );

        // 6. 3D Transform Motion System: Transform3D (Write)
        // Completely disjoint from all 2D systems! Executes in parallel with 2D systems.
        self.scheduler.add_system(
            "Motion3D",
            SystemAccess::new().with_write::<Transform3D>(),
            |registry, _pool, dt| {
                // SAFETY: SystemAccess guarantees exclusive access to Transform3D.
                let pool_tr3d = unsafe { registry.storage_mut::<Transform3D>() };
                let (_, transforms) = pool_tr3d.split_entities_and_components_mut();
                let delta_rot = glam::Quat::from_rotation_y(0.5 * dt);
                for tr3d in transforms {
                    tr3d.rotation = (tr3d.rotation * delta_rot).normalize();
                    tr3d.translation.y += 0.1 * dt;
                }
            },
        );

        // 7. 3D Skeletal Animation System: Animation3DComponent (Write)
        // Completely disjoint from 2D systems and 3D transforms! Executes in parallel.
        self.scheduler.add_system(
            "Animations3D",
            SystemAccess::new().with_write::<Animation3DComponent>(),
            |registry, _pool, dt| {
                // SAFETY: SystemAccess guarantees exclusive access to Animation3DComponent.
                let pool_anim3d = unsafe { registry.storage_mut::<Animation3DComponent>() };
                let (_, anims) = pool_anim3d.split_entities_and_components_mut();
                for anim in anims {
                    anim.update(dt);
                }
            },
        );

        self.scheduler.build_schedule();
    }

    /// Simülasyonu başlatır ve mevcut sahne durumunun (2D ve 3D) snapshot'ını alır
    pub(crate) fn start_simulation(&mut self) {
        if self.editor.scene_snapshot.is_empty()
            && self.editor.scene_snapshot_3d.is_empty()
            && self.editor.scene_snapshot_anim_3d.is_empty()
        {
            self.registry.view1_ref::<Transform2D, _>(|e, t| {
                self.editor.scene_snapshot.push((e, *t));
            });
            self.registry.view1_ref::<Transform3D, _>(|e, t| {
                self.editor.scene_snapshot_3d.push((e, *t));
            });
            self.registry.view1_ref::<Animation3DComponent, _>(|e, a| {
                self.editor.scene_snapshot_anim_3d.push((e, a.clone()));
            });
        }
        self.editor.is_playing = true;
    }

    /// Simülasyonu durdurur ve sahne durumunu (2D ve 3D) snapshot anına geri yükler (rollback)
    pub(crate) fn stop_simulation(&mut self) {
        self.editor.is_playing = false;
        for &(e, t) in &self.editor.scene_snapshot {
            if let Some(cur_t) = self.registry.get_mut::<Transform2D>(e) {
                *cur_t = t;
            }
        }
        self.editor.scene_snapshot.clear();
        for &(e, t) in &self.editor.scene_snapshot_3d {
            if let Some(cur_t) = self.registry.get_mut::<Transform3D>(e) {
                *cur_t = t;
            }
        }
        self.editor.scene_snapshot_3d.clear();
        for (e, a) in self.editor.scene_snapshot_anim_3d.drain(..) {
            if let Some(cur_a) = self.registry.get_mut::<Animation3DComponent>(e) {
                *cur_a = a;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_scheduler_parallel_stages_and_concurrency() {
        let mut app = App::new();
        app.init_simulation_scheduler();

        // 7 systems are registered:
        // 1. Physics2D (Write Transform2D, Write Velocity2D)
        // 2. Scripts (Write ScriptComponent, Write Transform2D)
        // 3. Hierarchy (Read Parent, Write Transform2D)
        // 4. Colliders2D (Read Collider2D, Read Tilemap2DComponent, Write Transform2D)
        // 5. Animations2D (Write Animation2DComponent, Write SpriteTexture)
        // 6. Motion3D (Write Transform3D)
        // 7. Animations3D (Write Animation3DComponent)
        assert_eq!(app.scheduler.system_count(), 7);

        // Disjoint verification:
        // Systems 5 (Animations2D), 6 (Motion3D), and 7 (Animations3D) are completely disjoint
        // from each other and can run concurrently in the stages!
        let stages = app.scheduler.stages();
        let max_stage_size = stages
            .iter()
            .map(|s| s.system_indices.len())
            .max()
            .unwrap_or(0);
        assert!(
            max_stage_size >= 2,
            "At least two independent systems must execute in parallel within the same stage, found max {}",
            max_stage_size
        );

        // Step simulation for a frame
        let initial_e1_pos = app
            .registry
            .get::<Transform2D>(Entity::new(0, 0))
            .map(|t| t.pos);
        app.step_simulation_single_frame(1.0 / 60.0);
        let stepped_e1_pos = app
            .registry
            .get::<Transform2D>(Entity::new(0, 0))
            .map(|t| t.pos);

        // Entities with velocity moved
        assert_eq!(initial_e1_pos, stepped_e1_pos); // e1 has no velocity
    }
}
