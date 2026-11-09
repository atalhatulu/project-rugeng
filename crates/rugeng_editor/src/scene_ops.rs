use crate::types::*;
use crate::App;
use glam::Vec2;
use rugeng_assets::VersionedScene;
use rugeng_core::{
    Animation2DComponent, Animation3DComponent, AnimationClip, Children, Collider2D, Entity,
    Mesh3DComponent, NodeName, Parent, ParticleEmitterComponent, Prefab, SceneData, SceneEntity,
    ScriptComponent, SpriteColor, SpriteTexture, Text2DComponent, TileData, TileType,
    Tilemap2DComponent, Transform2D, Transform3D, Velocity2D, ZIndex2D,
};

impl App {
    pub(crate) fn save_current_scene(&mut self, path: &str) {
        let mut scene = SceneData::new("MainWorkbenchScene");
        let mut visited = std::collections::HashSet::new();

        // 1. 2D Varlıkları topla
        for e in self.registry.entities::<Transform2D>() {
            visited.insert(e);
            let tr = self.registry.get::<Transform2D>(e).unwrap();
            let color = self
                .registry
                .get::<SpriteColor>(e)
                .map(|s| s.color)
                .unwrap_or([255, 255, 255, 255]);
            let texture = self
                .registry
                .get::<SpriteTexture>(e)
                .map(|t| t.name.clone());
            let col = self.registry.get::<Collider2D>(e);
            let has_collider = col.is_some();
            let is_dynamic = col.map(|c| c.is_dynamic).unwrap_or(true);
            let is_sensor = col.map(|c| c.is_sensor).unwrap_or(false);
            let velocity = self
                .registry
                .get::<Velocity2D>(e)
                .map(|v| [v.vel.x, v.vel.y]);
            let script = self
                .registry
                .get::<ScriptComponent>(e)
                .map(|s| s.name.clone());
            let parent_id = self.registry.get::<Parent>(e).map(|p| p.entity.index());
            let text = self.registry.get::<Text2DComponent>(e).cloned();
            let animation = self.registry.get::<Animation2DComponent>(e).cloned();
            let tilemap = self.registry.get::<Tilemap2DComponent>(e).cloned();
            let transform_3d = self.registry.get::<Transform3D>(e).copied();
            let mesh_3d = self.registry.get::<Mesh3DComponent>(e).cloned();
            let z_index = self.registry.get::<ZIndex2D>(e).map(|z| z.z).unwrap_or(0);
            // Yayıcı yalnızca KONFİGÜRASYON olarak kaydedilir; canlı parçacıklar
            // ve emission sayacı çalışma zamanı durumudur, diske yazılmaz.
            let particle = self.registry.get::<ParticleEmitterComponent>(e).map(|p| {
                let mut cfg = p.clone();
                cfg.particles.clear();
                cfg.emission_timer = 0.0;
                cfg
            });
            let animation_3d = self.registry.get::<Animation3DComponent>(e).cloned();

            scene.entities.push(SceneEntity {
                id: e.index(),
                name: self.display_name_of(e, false),
                pos: [tr.pos.x, tr.pos.y],
                size: [tr.size.x, tr.size.y],
                rotation: tr.rotation,
                color,
                texture,
                has_collider,
                is_dynamic,
                is_sensor,
                velocity,
                script,
                parent_id,
                text,
                animation,
                tilemap,
                transform_3d,
                mesh_3d,
                z_index,
                particle,
                animation_3d,
            });
        }

        // 2. Saf 3D Varlıkları topla (Transform3D olup Transform2D olmayanlar)
        for e in self.registry.entities::<Transform3D>() {
            if visited.contains(&e) {
                continue;
            }
            let t3 = self.registry.get::<Transform3D>(e).copied();
            let m3 = self.registry.get::<Mesh3DComponent>(e).cloned();
            let parent_id = self.registry.get::<Parent>(e).map(|p| p.entity.index());
            let script = self
                .registry
                .get::<ScriptComponent>(e)
                .map(|s| s.name.clone());
            let animation_3d = self.registry.get::<Animation3DComponent>(e).cloned();

            scene.entities.push(SceneEntity {
                id: e.index(),
                name: self.display_name_of(e, true),
                pos: [0.0, 0.0],
                size: [1.0, 1.0],
                rotation: 0.0,
                color: [255, 255, 255, 255],
                texture: None,
                has_collider: false,
                is_dynamic: false,
                is_sensor: false,
                velocity: None,
                script,
                parent_id,
                text: None,
                animation: None,
                tilemap: None,
                transform_3d: t3,
                mesh_3d: m3,
                z_index: 0,
                particle: None,
                animation_3d,
            });
        }

        let versioned = VersionedScene::from_core_scene(&scene);
        let json_result = versioned.to_json();
        let save_result = match json_result {
            Ok(json_str) => std::fs::write(path, json_str).map_err(|e| e.to_string()),
            Err(e) => Err(e),
        };

        match save_result {
            Ok(()) => {
                self.editor.current_scene_path = path.to_string();
                self.editor.is_scene_dirty = false;
                self.editor.logs.push(LogMessage {
                    level: "INFO",
                    text: format!(
                        "Sahne '{}' basariyla kaydedildi ({} dugum, VersionedScene).",
                        path,
                        scene.entities.len()
                    ),
                });
                self.audio.play("sfx_blip");
                self.editor.refresh_files();
            }
            Err(e) => {
                self.editor.logs.push(LogMessage {
                    level: "ERROR",
                    text: format!("Sahne kaydedilemedi: {e}"),
                });
            }
        }
    }

    /// Mevcut sahneyi ve varlıkları .pak arşivi olarak paketler
    pub(crate) fn export_project_pak(&mut self, pak_path: &str) -> Result<(), String> {
        let mut scene = SceneData::new("MainWorkbenchScene");
        let mut visited = std::collections::HashSet::new();

        // 2D Varlıklar
        for e in self.registry.entities::<Transform2D>() {
            visited.insert(e);
            let tr = self.registry.get::<Transform2D>(e).unwrap();
            let color = self
                .registry
                .get::<SpriteColor>(e)
                .map(|s| s.color)
                .unwrap_or([255, 255, 255, 255]);
            let texture = self
                .registry
                .get::<SpriteTexture>(e)
                .map(|t| t.name.clone());
            let col = self.registry.get::<Collider2D>(e);
            let has_collider = col.is_some();
            let is_dynamic = col.map(|c| c.is_dynamic).unwrap_or(true);
            let is_sensor = col.map(|c| c.is_sensor).unwrap_or(false);
            let velocity = self
                .registry
                .get::<Velocity2D>(e)
                .map(|v| [v.vel.x, v.vel.y]);
            let script = self
                .registry
                .get::<ScriptComponent>(e)
                .map(|s| s.name.clone());
            let parent_id = self.registry.get::<Parent>(e).map(|p| p.entity.index());
            let text = self.registry.get::<Text2DComponent>(e).cloned();
            let animation = self.registry.get::<Animation2DComponent>(e).cloned();
            let tilemap = self.registry.get::<Tilemap2DComponent>(e).cloned();
            let transform_3d = self.registry.get::<Transform3D>(e).copied();
            let mesh_3d = self.registry.get::<Mesh3DComponent>(e).cloned();
            let z_index = self.registry.get::<ZIndex2D>(e).map(|z| z.z).unwrap_or(0);
            let particle = self.registry.get::<ParticleEmitterComponent>(e).map(|p| {
                let mut cfg = p.clone();
                cfg.particles.clear();
                cfg.emission_timer = 0.0;
                cfg
            });
            let animation_3d = self.registry.get::<Animation3DComponent>(e).cloned();

            scene.entities.push(SceneEntity {
                id: e.index(),
                name: self.display_name_of(e, false),
                pos: [tr.pos.x, tr.pos.y],
                size: [tr.size.x, tr.size.y],
                rotation: tr.rotation,
                color,
                texture,
                has_collider,
                is_dynamic,
                is_sensor,
                velocity,
                script,
                parent_id,
                text,
                animation,
                tilemap,
                transform_3d,
                mesh_3d,
                z_index,
                particle,
                animation_3d,
            });
        }

        // 3D Varlıklar
        for e in self.registry.entities::<Transform3D>() {
            if visited.contains(&e) {
                continue;
            }
            let t3 = self.registry.get::<Transform3D>(e).copied();
            let m3 = self.registry.get::<Mesh3DComponent>(e).cloned();
            let parent_id = self.registry.get::<Parent>(e).map(|p| p.entity.index());
            let script = self
                .registry
                .get::<ScriptComponent>(e)
                .map(|s| s.name.clone());
            let animation_3d = self.registry.get::<Animation3DComponent>(e).cloned();

            scene.entities.push(SceneEntity {
                id: e.index(),
                name: self.display_name_of(e, true),
                pos: [0.0, 0.0],
                size: [1.0, 1.0],
                rotation: 0.0,
                color: [255, 255, 255, 255],
                texture: None,
                has_collider: false,
                is_dynamic: false,
                is_sensor: false,
                velocity: None,
                script,
                parent_id,
                text: None,
                animation: None,
                tilemap: None,
                transform_3d: t3,
                mesh_3d: m3,
                z_index: 0,
                particle: None,
                animation_3d,
            });
        }

        let versioned = VersionedScene::from_core_scene(&scene);
        let scene_binary = versioned.to_binary()?;

        let mut pak_writer = rugeng_assets::PakWriter::new();
        pak_writer.add_file("scenes/main.bin", scene_binary);

        // Varsa ek varlık dosyalarını pakete ekle (örneğin atlas veya ses)
        if let Ok(atlas_bytes) = std::fs::read("assets/textures/atlas.png") {
            pak_writer.add_file("textures/atlas.png", atlas_bytes);
        }

        // Hedef dizini oluştur
        if let Some(parent) = std::path::Path::new(pak_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        pak_writer.write_to_file(pak_path)?;

        self.editor.logs.push(LogMessage {
            level: "INFO",
            text: format!("Proje paketi olusturuldu -> '{}'", pak_path),
        });
        self.audio.play("sfx_powerup");
        Ok(())
    }

    pub(crate) fn load_scene_from_file(&mut self, path: &str) {
        let load_res = std::fs::read_to_string(path)
            .map_err(|e| format!("Dosya okunamadi: {e}"))
            .and_then(|data| VersionedScene::from_json(&data).map(|vs| vs.to_core_scene()));

        match load_res {
            Ok(scene) => {
                let entities_2d: Vec<Entity> = self.registry.entities::<Transform2D>();
                for e in entities_2d {
                    self.cmd_buffer.destroy(e);
                }
                let entities_3d: Vec<Entity> = self.registry.entities::<Transform3D>();
                for e in entities_3d {
                    self.cmd_buffer.destroy(e);
                }
                self.cmd_buffer.apply(&mut self.registry);

                let mut first_e = None;
                let mut id_map = std::collections::HashMap::new();

                for item in &scene.entities {
                    let e = self.registry.create();
                    id_map.insert(item.id, e);
                    // Kullanıcı tanımlı ad: üretik kalıba uymayan her ad NodeName olur.
                    let auto2 = format!("Node2D_{}", item.id);
                    let auto3 = format!("Node3D_{}", item.id);
                    if !item.name.is_empty() && item.name != auto2 && item.name != auto3 {
                        self.registry.emplace(e, NodeName::new(item.name.clone()));
                    }
                    if first_e.is_none() {
                        first_e = Some(e);
                    }

                    // 3D bileşenler varsa ekle
                    if let Some(t3) = item.transform_3d {
                        self.registry.emplace(e, t3);
                    }
                    if let Some(m3) = &item.mesh_3d {
                        self.registry.emplace(e, m3.clone());
                    }

                    // 2D bileşenler: Eğer saf 3D değilse veya pos/size tanımlıysa
                    if item.transform_3d.is_none() || (item.pos[0] != 0.0 || item.pos[1] != 0.0) {
                        self.registry.emplace(
                            e,
                            Transform2D {
                                pos: Vec2::new(item.pos[0], item.pos[1]),
                                size: Vec2::new(item.size[0], item.size[1]),
                                rotation: item.rotation,
                            },
                        );
                        self.registry.emplace(e, SpriteColor { color: item.color });
                        if let Some(tex) = &item.texture {
                            self.registry
                                .emplace(e, SpriteTexture { name: tex.clone() });
                        }
                        if item.has_collider {
                            self.registry.emplace(
                                e,
                                Collider2D {
                                    is_dynamic: item.is_dynamic,
                                    is_sensor: item.is_sensor,
                                },
                            );
                        }
                        if let Some(v) = item.velocity {
                            self.registry.emplace(
                                e,
                                Velocity2D {
                                    vel: Vec2::new(v[0], v[1]),
                                },
                            );
                        }
                        if item.z_index != 0 {
                            self.registry.emplace(e, ZIndex2D::new(item.z_index));
                        }
                    }

                    if let Some(scr) = &item.script {
                        let comp = match scr.as_str() {
                            "RotatorScript" => ScriptComponent::new_rotator(1.5),
                            "PatrolScript" => ScriptComponent::new_patrol(0, 80.0, 240.0, 75.0),
                            "PulseScript" => ScriptComponent::new_pulse(20.0, 44.0, 3.0),
                            _ => ScriptComponent::new_rotator(1.0),
                        };
                        self.registry.emplace(e, comp);
                    }
                    if let Some(txt) = &item.text {
                        self.registry.emplace(e, txt.clone());
                    }
                    if let Some(anim) = &item.animation {
                        self.registry.emplace(e, anim.clone());
                    }
                    if let Some(tm) = &item.tilemap {
                        self.registry.emplace(e, tm.clone());
                    }
                    if let Some(p) = &item.particle {
                        self.registry.emplace(e, p.clone());
                    }
                    if let Some(anim3d) = &item.animation_3d {
                        self.registry.emplace(e, anim3d.clone());
                    }
                }

                // Hiyerarşiyi (Parent / Children) bağla
                for item in &scene.entities {
                    if let Some(pid) = item.parent_id {
                        if let (Some(&child_e), Some(&parent_e)) =
                            (id_map.get(&item.id), id_map.get(&pid))
                        {
                            self.registry.emplace(child_e, Parent { entity: parent_e });
                            if let Some(children) = self.registry.get_mut::<Children>(parent_e) {
                                children.add(child_e);
                            } else {
                                let mut ch = Children::new();
                                ch.add(child_e);
                                self.registry.emplace(parent_e, ch);
                            }
                        }
                    }
                }

                self.editor.current_scene_path = path.to_string();
                self.editor.is_scene_dirty = false;
                self.editor.selected_entity = first_e;

                let title = std::path::Path::new(path)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string());
                let has_3d = !self.registry.entities::<Transform3D>().is_empty();

                // Açık sekmelerde varsa aktif yap, yoksa yeni sekme olarak ekle
                if let Some(pos) = self
                    .editor
                    .open_scenes
                    .iter()
                    .position(|t| t.path.as_deref() == Some(path))
                {
                    self.editor.active_scene_tab = pos;
                    self.editor.open_scenes[pos].is_dirty = false;
                    self.editor.open_scenes[pos].is_3d = has_3d;
                } else {
                    self.editor.open_scenes.push(OpenSceneTab {
                        path: Some(path.to_string()),
                        name: title,
                        is_dirty: false,
                        is_3d: has_3d,
                    });
                    self.editor.active_scene_tab = self.editor.open_scenes.len() - 1;
                }

                self.audio.play("sfx_powerup");
                self.editor.logs.push(LogMessage {
                    level: "INFO",
                    text: format!("Sahne '{}' basariyla yuklendi.", path),
                });
            }
            Err(e) => {
                self.editor.logs.push(LogMessage {
                    level: "ERROR",
                    text: format!("Sahne yuklenemedi: {e}"),
                });
            }
        }
    }

    pub(crate) fn new_scene(&mut self) {
        self.create_empty_2d_scene();
    }

    pub(crate) fn create_empty_2d_scene(&mut self) {
        let entities_2d: Vec<Entity> = self.registry.entities::<Transform2D>();
        for e in entities_2d {
            self.cmd_buffer.destroy(e);
        }
        let entities_3d: Vec<Entity> = self.registry.entities::<Transform3D>();
        for e in entities_3d {
            self.cmd_buffer.destroy(e);
        }
        self.cmd_buffer.apply(&mut self.registry);

        self.editor.scene_counter += 1;
        let path = format!("scenes/untitled_{}.scene", self.editor.scene_counter);
        let name = format!("untitled_{}.scene", self.editor.scene_counter);
        self.editor.current_scene_path = path.clone();
        self.editor.is_scene_dirty = true;
        self.editor.scene_tab = SceneTab::World;
        self.editor.workspace = crate::types::WorkspaceMode::TwoD;

        let root = self.registry.create();
        self.registry.emplace(root, NodeName::new("Node2D_Root"));
        self.registry.emplace(
            root,
            Transform2D {
                pos: Vec2::ZERO,
                size: Vec2::new(48.0, 48.0),
                rotation: 0.0,
            },
        );
        self.registry.emplace(
            root,
            SpriteColor {
                color: [71, 140, 191, 255],
            },
        );
        self.registry.emplace(
            root,
            SpriteTexture {
                name: "player_ship".to_string(),
            },
        );

        self.editor.selected_entity = Some(root);
        self.editor.open_scenes.push(OpenSceneTab {
            path: Some(path.clone()),
            name,
            is_dirty: true,
            is_3d: false,
        });
        self.editor.active_scene_tab = self.editor.open_scenes.len() - 1;

        self.audio.play("sfx_powerup");
        self.editor.logs.push(LogMessage {
            level: "INFO",
            text: format!("Yeni bos 2D sahne baslatildi -> '{}'", path),
        });
    }

    pub(crate) fn create_empty_3d_scene(&mut self) {
        let entities_2d: Vec<Entity> = self.registry.entities::<Transform2D>();
        for e in entities_2d {
            self.cmd_buffer.destroy(e);
        }
        let entities_3d: Vec<Entity> = self.registry.entities::<Transform3D>();
        for e in entities_3d {
            self.cmd_buffer.destroy(e);
        }
        self.cmd_buffer.apply(&mut self.registry);

        self.editor.scene_counter += 1;
        let path = format!("scenes/world3d_{}.scene", self.editor.scene_counter);
        let name = format!("world3d_{}.scene", self.editor.scene_counter);
        self.editor.current_scene_path = path.clone();
        self.editor.is_scene_dirty = true;
        self.editor.scene_tab = SceneTab::World;
        self.editor.workspace = crate::types::WorkspaceMode::ThreeD;

        // 3D Kök Düğüm
        let root = self.registry.create();
        self.registry.emplace(root, NodeName::new("World3D_Root"));
        self.registry.emplace(
            root,
            Transform3D {
                translation: glam::Vec3::ZERO,
                rotation: glam::Quat::IDENTITY,
                scale: glam::Vec3::ONE,
            },
        );

        // Zemin Düzlemi (Plane 3D)
        let ground = self.registry.create();
        self.registry.emplace(ground, NodeName::new("Ground_Plane"));
        self.registry.emplace(
            ground,
            Transform3D {
                translation: glam::Vec3::new(0.0, -0.5, 0.0),
                rotation: glam::Quat::IDENTITY,
                scale: glam::Vec3::new(10.0, 0.2, 10.0),
            },
        );
        self.registry.emplace(
            ground,
            Mesh3DComponent::with_pbr("plane", [0.45, 0.48, 0.52, 1.0], 0.8, 0.1),
        );
        self.registry.emplace(ground, Parent { entity: root });

        // Varsayılan Merkez Küp (Center_Cube)
        let cube = self.registry.create();
        self.registry.emplace(cube, NodeName::new("Center_Cube"));
        self.registry.emplace(
            cube,
            Transform3D {
                translation: glam::Vec3::new(0.0, 0.5, 0.0),
                rotation: glam::Quat::IDENTITY,
                scale: glam::Vec3::ONE,
            },
        );
        self.registry.emplace(
            cube,
            Mesh3DComponent::with_pbr("cube", [0.9, 0.6, 0.2, 1.0], 0.4, 0.3),
        );
        self.registry.emplace(cube, Parent { entity: root });

        self.editor.selected_entity = Some(cube);
        self.editor.open_scenes.push(OpenSceneTab {
            path: Some(path.clone()),
            name,
            is_dirty: true,
            is_3d: true,
        });
        self.editor.active_scene_tab = self.editor.open_scenes.len() - 1;

        self.audio.play("sfx_powerup");
        self.editor.logs.push(LogMessage {
            level: "INFO",
            text: format!("Yeni bos 3D sahne baslatildi -> '{}'", path),
        });
    }

    pub(crate) fn switch_to_scene_tab(&mut self, index: usize) {
        if index >= self.editor.open_scenes.len() || index == self.editor.active_scene_tab {
            return;
        }
        let target = self.editor.open_scenes[index].clone();
        if let Some(path) = &target.path {
            if std::path::Path::new(path).exists() {
                self.load_scene_from_file(path);
                self.editor.active_scene_tab = index;
                if target.is_3d {
                    self.editor.workspace = crate::types::WorkspaceMode::ThreeD;
                }
                return;
            }
        }
        self.editor.active_scene_tab = index;
        self.editor.current_scene_path = target.path.unwrap_or_else(|| target.name.clone());
        self.editor.is_scene_dirty = target.is_dirty;
        if target.is_3d {
            self.editor.workspace = crate::types::WorkspaceMode::ThreeD;
        }
    }

    pub(crate) fn close_scene_tab(&mut self, index: usize) {
        if index >= self.editor.open_scenes.len() {
            return;
        }
        self.editor.open_scenes.remove(index);
        if self.editor.open_scenes.is_empty() {
            // Hiç sekme kalmadıysa otomatik yeni 2D sahne aç
            self.create_empty_2d_scene();
        } else {
            let new_idx = if index >= self.editor.open_scenes.len() {
                self.editor.open_scenes.len() - 1
            } else {
                index
            };
            self.editor.active_scene_tab = new_idx;
            let tab = self.editor.open_scenes[new_idx].clone();
            if let Some(path) = &tab.path {
                if std::path::Path::new(path).exists() {
                    self.load_scene_from_file(path);
                } else {
                    self.editor.current_scene_path = path.clone();
                }
            } else {
                self.editor.current_scene_path = tab.name.clone();
            }
        }
    }

    pub(crate) fn duplicate_selected_entity(&mut self) {
        if let Some(src) = self.editor.selected_entity {
            if !self.registry.is_alive(src) {
                return;
            }

            let tr2_opt = self.registry.get::<Transform2D>(src).copied();
            let tr3_opt = self.registry.get::<Transform3D>(src).copied();

            if tr2_opt.is_none() && tr3_opt.is_none() {
                return;
            }

            let new_e = self.registry.create();

            // 1. NodeName (Varsa " (Kopya)" ekle veya aynı adı taşı)
            let src_name = self.registry.get::<NodeName>(src).map(|n| n.name.clone());
            let new_name = match src_name {
                Some(name) => format!("{} (Kopya)", name),
                None => {
                    if tr3_opt.is_some() && tr2_opt.is_none() {
                        format!("Node3D_{}", new_e.index())
                    } else {
                        format!("Node2D_{}", new_e.index())
                    }
                }
            };
            self.registry.emplace(new_e, NodeName::new(new_name));

            // 2. 2D Transform (+16.0, +16.0 ofset ile çoğalt)
            if let Some(mut tr) = tr2_opt {
                tr.pos += Vec2::new(16.0, 16.0);
                self.registry.emplace(new_e, tr);
            }

            // 3. 3D Transform (+1.0, 0.0, +1.0 ofset ile çoğalt)
            if let Some(mut tr3) = tr3_opt {
                tr3.translation += glam::Vec3::new(1.0, 0.0, 1.0);
                self.registry.emplace(new_e, tr3);
            }

            if let Some(m3) = self.registry.get::<Mesh3DComponent>(src).cloned() {
                self.registry.emplace(new_e, m3);
            }
            if let Some(c) = self.registry.get::<SpriteColor>(src).copied() {
                self.registry.emplace(new_e, c);
            }
            if let Some(t) = self.registry.get::<SpriteTexture>(src).cloned() {
                self.registry.emplace(new_e, t);
            }
            if let Some(c) = self.registry.get::<Collider2D>(src).copied() {
                self.registry.emplace(new_e, c);
            }
            if let Some(v) = self.registry.get::<Velocity2D>(src).copied() {
                self.registry.emplace(new_e, v);
            }
            if let Some(s) = self.registry.get::<ScriptComponent>(src).cloned() {
                self.registry.emplace(new_e, s);
            }
            if let Some(txt) = self.registry.get::<Text2DComponent>(src).cloned() {
                self.registry.emplace(new_e, txt);
            }
            if let Some(anim) = self.registry.get::<Animation2DComponent>(src).cloned() {
                self.registry.emplace(new_e, anim);
            }
            if let Some(tm) = self.registry.get::<Tilemap2DComponent>(src).cloned() {
                self.registry.emplace(new_e, tm);
            }
            if let Some(zi) = self.registry.get::<ZIndex2D>(src).copied() {
                self.registry.emplace(new_e, zi);
            }
            if let Some(p) = self.registry.get::<ParticleEmitterComponent>(src).cloned() {
                self.registry.emplace(new_e, p);
            }

            self.editor.undo_stack.push(UndoAction::CreateEntity(new_e));
            self.editor.redo_stack.clear();
            self.editor.selected_entity = Some(new_e);
            self.editor.is_scene_dirty = true;
            self.audio.play("sfx_blip");
            self.editor.logs.push(LogMessage {
                level: "INFO",
                text: format!(
                    "Dugum #{}:{} cogaltildi (Ctrl+D) -> Yeni Dugum #{}:{}",
                    src.index(),
                    src.generation(),
                    new_e.index(),
                    new_e.generation()
                ),
            });
        }
    }

    /// Seçili varlığı ve hiyerarşisini .prefab dosyası olarak kaydeder.
    pub(crate) fn save_selected_as_prefab(&mut self, path: &str) -> Result<(), String> {
        let Some(selected) = self.editor.selected_entity else {
            return Err("Prefab kaydedilemedi: Hiçbir varlık seçili değil.".to_string());
        };

        let prefab = Prefab::from_entity(&self.registry, selected)?;
        prefab.save_to_file(path)?;

        self.editor.logs.push(LogMessage {
            level: "INFO",
            text: format!("Prefab '{}' başarıyla kaydedildi.", path),
        });
        self.audio.play("sfx_blip");
        self.editor.refresh_files();
        Ok(())
    }

    /// .prefab dosyasını okuyup kameranın baktığı yere örnekler (instantiate).
    pub(crate) fn load_prefab_to_scene(&mut self, path: &str) -> Result<Entity, String> {
        let prefab = Prefab::load_from_file(path)?;
        let spawn_pos_2d = self.editor.camera.pos;
        let spawn_pos_3d = self.editor.camera_3d.eye + self.editor.camera_3d.forward() * 4.0;

        let root_e = prefab.instantiate(&mut self.registry, spawn_pos_2d, spawn_pos_3d);

        self.editor
            .undo_stack
            .push(UndoAction::CreateEntity(root_e));
        self.editor.redo_stack.clear();
        self.editor.selected_entity = Some(root_e);
        self.editor.is_scene_dirty = true;
        self.audio.play("sfx_blip");
        self.editor.logs.push(LogMessage {
            level: "INFO",
            text: format!(
                "Prefab '{}' sahneye eklendi (Kök Varlık #{}).",
                prefab.name,
                root_e.index()
            ),
        });

        Ok(root_e)
    }

    pub(crate) fn spawn_preset_entity(&mut self, preset: &str) {
        let spawn_pos = self.editor.camera.pos;
        self.spawn_preset_entity_at(preset, spawn_pos);
    }

    pub(crate) fn spawn_preset_entity_at(&mut self, preset: &str, spawn_pos: Vec2) {
        let e = self.registry.create();
        let mut tr = Transform2D {
            pos: spawn_pos,
            size: Vec2::new(40.0, 40.0),
            rotation: 0.0,
        };

        match preset {
            "ship" => {
                tr.size = Vec2::new(48.0, 48.0);
                self.registry.emplace(e, tr);
                self.registry.emplace(
                    e,
                    SpriteColor {
                        color: [255, 255, 255, 255],
                    },
                );
                self.registry.emplace(
                    e,
                    SpriteTexture {
                        name: "player_ship".to_string(),
                    },
                );
                self.registry.emplace(
                    e,
                    Collider2D {
                        is_dynamic: true,
                        is_sensor: false,
                    },
                );
                self.registry.emplace(e, ScriptComponent::new_rotator(1.5));
            }
            "enemy" => {
                tr.size = Vec2::new(40.0, 40.0);
                self.registry.emplace(e, tr);
                self.registry.emplace(
                    e,
                    SpriteColor {
                        color: [255, 255, 255, 255],
                    },
                );
                self.registry.emplace(
                    e,
                    SpriteTexture {
                        name: "enemy_bug".to_string(),
                    },
                );
                self.registry.emplace(
                    e,
                    Collider2D {
                        is_dynamic: true,
                        is_sensor: false,
                    },
                );
                self.registry.emplace(
                    e,
                    Velocity2D {
                        vel: Vec2::new(-40.0, 40.0),
                    },
                );
                self.registry.emplace(
                    e,
                    ScriptComponent::new_patrol(0, spawn_pos.x - 80.0, spawn_pos.x + 80.0, 60.0),
                );
            }
            "orb" => {
                tr.size = Vec2::new(32.0, 32.0);
                self.registry.emplace(e, tr);
                self.registry.emplace(
                    e,
                    SpriteColor {
                        color: [255, 255, 255, 255],
                    },
                );
                self.registry.emplace(
                    e,
                    SpriteTexture {
                        name: "energy_orb".to_string(),
                    },
                );
                self.registry.emplace(
                    e,
                    Collider2D {
                        is_dynamic: false,
                        is_sensor: true,
                    },
                );
                self.registry
                    .emplace(e, ScriptComponent::new_pulse(20.0, 40.0, 3.0));
            }
            "heart" => {
                tr.size = Vec2::new(32.0, 32.0);
                self.registry.emplace(e, tr);
                self.registry.emplace(
                    e,
                    SpriteColor {
                        color: [255, 255, 255, 255],
                    },
                );
                self.registry.emplace(
                    e,
                    SpriteTexture {
                        name: "heart_icon".to_string(),
                    },
                );
            }
            "turret" => {
                tr.size = Vec2::new(22.0, 22.0);
                self.registry.emplace(e, tr);
                self.registry.emplace(
                    e,
                    SpriteColor {
                        color: [228, 91, 91, 255],
                    },
                );
                self.registry.emplace(e, ScriptComponent::new_rotator(3.0));
                if let Some(parent_e) = self.editor.selected_entity {
                    self.set_parent(e, parent_e);
                }
            }
            "text" => {
                tr.size = Vec2::new(140.0, 24.0);
                self.registry.emplace(e, tr);
                self.registry.emplace(
                    e,
                    Text2DComponent {
                        text: "Project Ember 2D".to_string(),
                        scale: 1.5,
                        color: [240, 240, 240, 255],
                    },
                );
            }
            "anim_player" => {
                tr.size = Vec2::new(48.0, 48.0);
                self.registry.emplace(e, tr);
                self.registry.emplace(
                    e,
                    SpriteColor {
                        color: [255, 255, 255, 255],
                    },
                );
                self.registry.emplace(
                    e,
                    SpriteTexture {
                        name: "player_ship".to_string(),
                    },
                );
                let clip = AnimationClip::new(
                    "idle",
                    vec!["player_ship".to_string(), "energy_orb".to_string()],
                    4.0,
                    true,
                );
                let anim = Animation2DComponent::new().with_clip(clip);
                self.registry.emplace(e, anim);
                self.registry.emplace(
                    e,
                    Collider2D {
                        is_dynamic: true,
                        is_sensor: false,
                    },
                );
                self.registry.emplace(
                    e,
                    Velocity2D {
                        vel: Vec2::new(30.0, 30.0),
                    },
                );
            }
            "tilemap" => {
                tr.size = Vec2::new(32.0, 32.0);
                self.registry.emplace(e, tr);
                let mut map = Tilemap2DComponent::new(32.0);
                for i in -4..=4 {
                    map.set_tile(i, 2, TileType::Grass, false);
                }
                map.set_tile(-4, 1, TileType::Stone, true);
                map.set_tile(4, 1, TileType::Stone, true);
                self.registry.emplace(e, map);
            }
            "cube_3d" => {
                let cam_eye = self.editor.camera_3d.eye;
                let cam_fwd = self.editor.camera_3d.forward();
                let spawn_3d = cam_eye + cam_fwd * 4.0;
                self.registry.emplace(
                    e,
                    Transform3D {
                        translation: spawn_3d,
                        rotation: glam::Quat::IDENTITY,
                        scale: glam::Vec3::ONE,
                    },
                );
                self.registry.emplace(
                    e,
                    Mesh3DComponent::with_pbr("cube", [0.9, 0.9, 0.9, 1.0], 0.5, 0.1),
                );
            }
            "sphere_3d" => {
                let cam_eye = self.editor.camera_3d.eye;
                let cam_fwd = self.editor.camera_3d.forward();
                let spawn_3d = cam_eye + cam_fwd * 4.0;
                self.registry.emplace(
                    e,
                    Transform3D {
                        translation: spawn_3d,
                        rotation: glam::Quat::IDENTITY,
                        scale: glam::Vec3::ONE,
                    },
                );
                self.registry.emplace(
                    e,
                    Mesh3DComponent::with_pbr("sphere", [0.8, 0.8, 0.9, 1.0], 0.2, 0.8),
                );
            }
            "plane_3d" => {
                let cam_eye = self.editor.camera_3d.eye;
                let cam_fwd = self.editor.camera_3d.forward();
                let spawn_3d = cam_eye + cam_fwd * 4.0;
                self.registry.emplace(
                    e,
                    Transform3D {
                        translation: spawn_3d,
                        rotation: glam::Quat::IDENTITY,
                        scale: glam::Vec3::new(4.0, 1.0, 4.0),
                    },
                );
                self.registry.emplace(
                    e,
                    Mesh3DComponent::with_pbr("plane", [0.7, 0.7, 0.7, 1.0], 0.8, 0.0),
                );
            }
            "cylinder_3d" => {
                let cam_eye = self.editor.camera_3d.eye;
                let cam_fwd = self.editor.camera_3d.forward();
                let spawn_3d = cam_eye + cam_fwd * 4.0;
                self.registry.emplace(
                    e,
                    Transform3D {
                        translation: spawn_3d,
                        rotation: glam::Quat::IDENTITY,
                        scale: glam::Vec3::ONE,
                    },
                );
                self.registry.emplace(
                    e,
                    Mesh3DComponent::with_pbr("cylinder", [0.4, 0.8, 0.6, 1.0], 0.3, 0.5),
                );
            }
            "cone_3d" => {
                let cam_eye = self.editor.camera_3d.eye;
                let cam_fwd = self.editor.camera_3d.forward();
                let spawn_3d = cam_eye + cam_fwd * 4.0;
                self.registry.emplace(
                    e,
                    Transform3D {
                        translation: spawn_3d,
                        rotation: glam::Quat::IDENTITY,
                        scale: glam::Vec3::ONE,
                    },
                );
                self.registry.emplace(
                    e,
                    Mesh3DComponent::with_pbr("cone", [0.9, 0.5, 0.2, 1.0], 0.4, 0.2),
                );
            }
            "pyramid_3d" => {
                let cam_eye = self.editor.camera_3d.eye;
                let cam_fwd = self.editor.camera_3d.forward();
                let spawn_3d = cam_eye + cam_fwd * 4.0;
                self.registry.emplace(
                    e,
                    Transform3D {
                        translation: spawn_3d,
                        rotation: glam::Quat::IDENTITY,
                        scale: glam::Vec3::ONE,
                    },
                );
                self.registry.emplace(
                    e,
                    Mesh3DComponent::with_pbr("pyramid", [0.85, 0.75, 0.2, 1.0], 0.6, 0.1),
                );
            }
            "capsule_3d" => {
                let cam_eye = self.editor.camera_3d.eye;
                let cam_fwd = self.editor.camera_3d.forward();
                let spawn_3d = cam_eye + cam_fwd * 4.0;
                self.registry.emplace(
                    e,
                    Transform3D {
                        translation: spawn_3d,
                        rotation: glam::Quat::IDENTITY,
                        scale: glam::Vec3::ONE,
                    },
                );
                self.registry.emplace(
                    e,
                    Mesh3DComponent::with_pbr("capsule", [0.3, 0.6, 0.9, 1.0], 0.2, 0.4),
                );
            }
            "sprite_3d" => {
                let cam_eye = self.editor.camera_3d.eye;
                let cam_fwd = self.editor.camera_3d.forward();
                let spawn_3d = cam_eye + cam_fwd * 4.0;
                self.registry.emplace(
                    e,
                    Transform3D {
                        translation: spawn_3d,
                        rotation: glam::Quat::IDENTITY,
                        scale: glam::Vec3::new(2.0, 2.0, 1.0),
                    },
                );
                self.registry.emplace(
                    e,
                    Mesh3DComponent::with_pbr("sprite_3d", [1.0, 1.0, 1.0, 1.0], 0.9, 0.0),
                );
            }
            "gltf_sample" => {
                let cam_eye = self.editor.camera_3d.eye;
                let cam_fwd = self.editor.camera_3d.forward();
                let spawn_3d = cam_eye + cam_fwd * 4.0;
                self.registry.emplace(
                    e,
                    Transform3D {
                        translation: spawn_3d,
                        rotation: glam::Quat::IDENTITY,
                        scale: glam::Vec3::ONE,
                    },
                );
                self.registry.emplace(
                    e,
                    Mesh3DComponent::with_pbr("gltf_sample", [0.2, 0.8, 0.4, 1.0], 0.3, 0.7),
                );
                self.registry.emplace(e, NodeName::new("GltfModel_Sample"));
            }
            _ => {
                tr.size = Vec2::new(36.0, 36.0);
                self.registry.emplace(e, tr);
                self.registry.emplace(
                    e,
                    SpriteColor {
                        color: [224, 169, 66, 255],
                    },
                );
            }
        }

        self.editor.undo_stack.push(UndoAction::CreateEntity(e));
        self.editor.redo_stack.clear();
        self.editor.selected_entity = Some(e);
        self.editor.is_scene_dirty = true;
        self.audio.play("sfx_blip");
        self.editor.logs.push(LogMessage {
            level: "INFO",
            text: format!(
                "Sahneye yeni nesne eklendi: [{}] Varlik #{}:{}",
                preset,
                e.index(),
                e.generation()
            ),
        });
    }

    #[allow(dead_code)]
    pub(crate) fn spawn_entity_at_camera(&mut self) {
        self.spawn_preset_entity("empty");
    }

    pub(crate) fn set_parent(&mut self, child: Entity, parent: Entity) {
        if child == parent {
            return;
        }
        self.registry.emplace(child, Parent { entity: parent });
        let mut children = self
            .registry
            .get::<Children>(parent)
            .cloned()
            .unwrap_or_default();
        children.add(child);
        self.registry.emplace(parent, children);
        self.editor.logs.push(LogMessage {
            level: "INFO",
            text: format!(
                "Hiyerarsi: Dugum #{} artik Dugum #{} ebeveynine baglandi.",
                child.index(),
                parent.index()
            ),
        });
    }

    pub(crate) fn snapshot_entity(&self, e: Entity) -> Option<EntitySnapshot> {
        let transform = self
            .registry
            .get::<Transform2D>(e)
            .copied()
            .unwrap_or(Transform2D {
                pos: Vec2::ZERO,
                size: Vec2::ONE,
                rotation: 0.0,
            });
        Some(EntitySnapshot {
            entity: e,
            name: self.registry.get::<NodeName>(e).cloned(),
            transform,
            color: self.registry.get::<SpriteColor>(e).copied(),
            texture: self.registry.get::<SpriteTexture>(e).cloned(),
            collider: self.registry.get::<Collider2D>(e).copied(),
            velocity: self.registry.get::<Velocity2D>(e).copied(),
            script: self.registry.get::<ScriptComponent>(e).cloned(),
            text: self.registry.get::<Text2DComponent>(e).cloned(),
            animation: self.registry.get::<Animation2DComponent>(e).cloned(),
            tilemap: self.registry.get::<Tilemap2DComponent>(e).cloned(),
            transform_3d: self.registry.get::<Transform3D>(e).copied(),
            mesh_3d: self.registry.get::<Mesh3DComponent>(e).cloned(),
            parent: self.registry.get::<Parent>(e).copied(),
            children: self.registry.get::<Children>(e).cloned(),
            z_index: self.registry.get::<ZIndex2D>(e).copied(),
            // Canlı parçacıklar runtime durumudur; konfig temizlenerek saklanır.
            particle: self
                .registry
                .get::<ParticleEmitterComponent>(e)
                .cloned()
                .map(|mut p| {
                    p.particles.clear();
                    p.emission_timer = 0.0;
                    p
                }),
        })
    }

    pub(crate) fn restore_entity(&mut self, snap: EntitySnapshot) -> Entity {
        let new_e = self.registry.create();
        if let Some(name) = snap.name {
            self.registry.emplace(new_e, name);
        }
        if snap.transform_3d.is_none()
            || (snap.transform.size.x > 0.0 && snap.transform.size.y > 0.0)
        {
            self.registry.emplace(new_e, snap.transform);
        }
        if let Some(t3) = snap.transform_3d {
            self.registry.emplace(new_e, t3);
        }
        if let Some(m3) = snap.mesh_3d {
            self.registry.emplace(new_e, m3);
        }
        if let Some(c) = snap.color {
            self.registry.emplace(new_e, c);
        }
        if let Some(t) = snap.texture {
            self.registry.emplace(new_e, t);
        }
        if let Some(c) = snap.collider {
            self.registry.emplace(new_e, c);
        }
        if let Some(v) = snap.velocity {
            self.registry.emplace(new_e, v);
        }
        if let Some(s) = snap.script {
            self.registry.emplace(new_e, s);
        }
        if let Some(txt) = snap.text {
            self.registry.emplace(new_e, txt);
        }
        if let Some(anim) = snap.animation {
            self.registry.emplace(new_e, anim);
        }
        if let Some(tm) = snap.tilemap {
            self.registry.emplace(new_e, tm);
        }
        if let Some(zi) = snap.z_index {
            self.registry.emplace(new_e, zi);
        }
        if let Some(p) = snap.particle {
            self.registry.emplace(new_e, p);
        }
        if let Some(p) = snap.parent {
            if self.registry.is_alive(p.entity) {
                self.registry.emplace(new_e, p);
                if let Some(ch) = self.registry.get_mut::<Children>(p.entity) {
                    ch.add(new_e);
                } else {
                    let mut ch = Children::new();
                    ch.add(new_e);
                    self.registry.emplace(p.entity, ch);
                }
            }
        }
        if let Some(mut ch) = snap.children {
            ch.list.retain(|&child_e| self.registry.is_alive(child_e));
            for &child_e in &ch.list {
                self.registry.emplace(child_e, Parent { entity: new_e });
            }
            self.registry.emplace(new_e, ch);
        }
        self.editor.selected_entity = Some(new_e);
        new_e
    }

    pub(crate) fn delete_selected_entity(&mut self) {
        if let Some(e) = self.editor.selected_entity {
            if let Some(snapshot) = self.snapshot_entity(e) {
                self.editor
                    .undo_stack
                    .push(UndoAction::DeleteEntity(Box::new(snapshot)));
                self.editor.redo_stack.clear();
            }

            let parent_e = self.registry.get::<Parent>(e).map(|p| p.entity);
            if let Some(parent_e) = parent_e {
                if let Some(ch) = self.registry.get_mut::<Children>(parent_e) {
                    ch.remove(e);
                }
            }
            let child_list = self.registry.get::<Children>(e).map(|ch| ch.list.clone());
            if let Some(list) = child_list {
                for child_e in list {
                    self.registry.remove::<Parent>(child_e);
                }
            }

            self.cmd_buffer.destroy(e);
            self.cmd_buffer.apply(&mut self.registry);
            self.editor.selected_entity = None;
            self.editor.is_scene_dirty = true;
            self.audio.play("sfx_explosion");
            self.editor.logs.push(LogMessage {
                level: "WARN",
                text: format!(
                    "Dugum #{}:{} sahneden silindi (Geri almak icin Ctrl+Z).",
                    e.index(),
                    e.generation()
                ),
            });
        }
    }

    /// Denetçi 2D düğmeleri için: değişiklik sonrası çağrılır; eski→yeni kaydı
    /// undo yığınına iter, redo yığınını temizler.
    pub(crate) fn push_transform2d_undo(&mut self, e: Entity, old: Transform2D) {
        if let Some(new) = self.registry.get::<Transform2D>(e).copied() {
            self.editor.undo_stack.push(UndoAction::Transform2DChanged {
                entity: e,
                old,
                new,
            });
            self.editor.redo_stack.clear();
            self.editor.is_scene_dirty = true;
        }
    }

    /// Denetçi 3D düğmeleri için: eski→yeni Transform3D kaydı.
    pub(crate) fn push_transform3d_undo(&mut self, e: Entity, old: Transform3D) {
        if let Some(new) = self.registry.get::<Transform3D>(e).copied() {
            self.editor.undo_stack.push(UndoAction::Transform3DChanged {
                entity: e,
                old,
                new,
            });
            self.editor.redo_stack.clear();
            self.editor.is_scene_dirty = true;
        }
    }

    /// Option<T> bileşeni geri yükle: Some → emplace, None → bileşeni kaldır.
    fn apply_option_comp<T: 'static + Clone + Send + Sync>(&mut self, e: Entity, v: Option<T>) {
        // Sahne yeniden yüklendiyse undo yığınındaki entity ölmüş olabilir;
        // `emplace` ölü entity'de panic atar — sessizce atla.
        if !self.registry.is_alive(e) {
            return;
        }
        match v {
            Some(c) => self.registry.emplace(e, c),
            None => {
                self.registry.remove::<T>(e);
            }
        }
    }

    pub(crate) fn push_texture_undo(&mut self, e: Entity, old: Option<SpriteTexture>) {
        let new = self.registry.get::<SpriteTexture>(e).cloned();
        self.editor
            .undo_stack
            .push(UndoAction::SpriteTextureChanged {
                entity: e,
                old,
                new,
            });
        self.editor.redo_stack.clear();
        self.editor.is_scene_dirty = true;
    }

    pub(crate) fn push_color_undo(&mut self, e: Entity, old: Option<SpriteColor>) {
        let new = self.registry.get::<SpriteColor>(e).copied();
        self.editor.undo_stack.push(UndoAction::SpriteColorChanged {
            entity: e,
            old,
            new,
        });
        self.editor.redo_stack.clear();
        self.editor.is_scene_dirty = true;
    }

    pub(crate) fn push_zindex_undo(&mut self, e: Entity, old: Option<ZIndex2D>) {
        let new = self.registry.get::<ZIndex2D>(e).copied();
        self.editor.undo_stack.push(UndoAction::ZIndexChanged {
            entity: e,
            old,
            new,
        });
        self.editor.redo_stack.clear();
        self.editor.is_scene_dirty = true;
    }

    pub(crate) fn push_collider_undo(&mut self, e: Entity, old: Option<Collider2D>) {
        let new = self.registry.get::<Collider2D>(e).copied();
        self.editor.undo_stack.push(UndoAction::Collider2DChanged {
            entity: e,
            old,
            new,
        });
        self.editor.redo_stack.clear();
        self.editor.is_scene_dirty = true;
    }

    pub(crate) fn push_script_undo(&mut self, e: Entity, old: Option<ScriptComponent>) {
        let new = self.registry.get::<ScriptComponent>(e).cloned();
        self.editor.undo_stack.push(UndoAction::ScriptChanged {
            entity: e,
            old,
            new,
        });
        self.editor.redo_stack.clear();
        self.editor.is_scene_dirty = true;
    }

    /// Okunabilir düğüm adı: NodeName varsa o, yoksa üretik ad (`Node2D_5`).
    pub(crate) fn display_name_of(&self, e: Entity, three_d: bool) -> String {
        if let Some(n) = self.registry.get::<NodeName>(e) {
            if !n.name.is_empty() {
                return n.name.clone();
            }
        }
        if three_d {
            format!("Node3D_{}", e.index())
        } else {
            format!("Node2D_{}", e.index())
        }
    }

    pub(crate) fn start_rename(&mut self) {
        if let Some(e) = self.editor.selected_entity {
            let buf = self.display_name_of(e, false);
            let caret = buf.chars().count();
            self.editor.renaming = Some(InlineTextEdit {
                entity: e,
                buffer: buf,
                caret,
            });
            self.editor.logs.push(LogMessage {
                level: "INFO",
                text: "Yeniden adlandırma modu (Enter: kaydet, Esc: vazgeç).".to_string(),
            });
        } else {
            self.editor.logs.push(LogMessage {
                level: "WARN",
                text: "Yeniden adlandırmak için bir düğüm seçin (F2).".to_string(),
            });
        }
    }

    pub(crate) fn commit_rename(&mut self) {
        let Some(r) = self.editor.renaming.take() else {
            return;
        };
        let e = r.entity;
        if !self.registry.is_alive(e) {
            self.editor.logs.push(LogMessage {
                level: "ERROR",
                text: "Adlandırılamadı: düğüm artık sahnede değil.".to_string(),
            });
            return;
        }
        let trimmed = r.buffer.trim().to_string();
        if trimmed.is_empty() {
            self.editor.logs.push(LogMessage {
                level: "WARN",
                text: "Boş ad kaydedilmedi; yeniden adlandırma iptal edildi.".to_string(),
            });
            return;
        }
        let old = self.registry.get::<NodeName>(e).map(|n| n.name.clone());
        if old.as_deref() == Some(trimmed.as_str()) {
            return; // Değişiklik yok: yığına kayıt yazma
        }
        self.registry.emplace(e, NodeName::new(trimmed.clone()));
        self.push_name_undo(e, old);
        self.audio.play("sfx_blip");
        self.editor.logs.push(LogMessage {
            level: "INFO",
            text: format!(
                "Düğüm #{}:{} \"{}\" olarak adlandırıldı.",
                e.index(),
                e.generation(),
                trimmed
            ),
        });
    }

    pub(crate) fn cancel_rename(&mut self) {
        if self.editor.renaming.take().is_some() {
            self.editor.logs.push(LogMessage {
                level: "INFO",
                text: "Yeniden adlandırma iptal edildi.".to_string(),
            });
        }
    }

    /// NodeName'i ad (Option<String>) değerinden geri yükle. `apply_option_comp`
    /// tip bazlı çalıştığı için String != NodeName; ayrı kol gerekir.
    fn apply_node_name(&mut self, e: Entity, v: Option<String>) {
        if !self.registry.is_alive(e) {
            return;
        }
        match v {
            Some(n) => self.registry.emplace(e, NodeName::new(n)),
            None => {
                self.registry.remove::<NodeName>(e);
            }
        }
    }

    pub(crate) fn push_name_undo(&mut self, e: Entity, old: Option<String>) {
        let new = self.registry.get::<NodeName>(e).map(|n| n.name.clone());
        self.editor.undo_stack.push(UndoAction::NodeNameChanged {
            entity: e,
            old,
            new,
        });
        self.editor.redo_stack.clear();
        self.editor.is_scene_dirty = true;
    }

    pub(crate) fn undo(&mut self) {
        if let Some(action) = self.editor.undo_stack.pop() {
            match action {
                UndoAction::CreateEntity(e) => {
                    if let Some(snap) = self.snapshot_entity(e) {
                        self.cmd_buffer.destroy(e);
                        self.cmd_buffer.apply(&mut self.registry);
                        self.editor
                            .redo_stack
                            .push(UndoAction::DeleteEntity(Box::new(snap)));
                        if self.editor.selected_entity == Some(e) {
                            self.editor.selected_entity = None;
                        }
                        self.editor.logs.push(LogMessage {
                            level: "WARN",
                            text: format!("Geri Alindi (Undo): Dugum #{} silindi.", e.index()),
                        });
                    }
                }
                UndoAction::DeleteEntity(snap) => {
                    let old_id = snap.entity;
                    let restored_e = self.restore_entity(*snap);
                    self.editor
                        .redo_stack
                        .push(UndoAction::CreateEntity(restored_e));
                    self.editor.logs.push(LogMessage {
                        level: "INFO",
                        text: format!(
                            "Geri Alindi (Undo): Silinen Dugum #{} geri getirildi (Yeni: #{}).",
                            old_id.index(),
                            restored_e.index()
                        ),
                    });
                }
                UndoAction::MoveEntity {
                    entity,
                    old_transform,
                    new_transform,
                } => {
                    if let Some(t) = self.registry.get_mut::<Transform2D>(entity) {
                        *t = old_transform;
                    }
                    self.editor.redo_stack.push(UndoAction::MoveEntity {
                        entity,
                        old_transform,
                        new_transform,
                    });
                    self.editor.logs.push(LogMessage {
                        level: "INFO",
                        text: format!(
                            "Geri Alindi (Undo): Dugum #{} konumu geri alindi.",
                            entity.index()
                        ),
                    });
                }
                UndoAction::TileChange {
                    entity,
                    gx,
                    gy,
                    old_tile,
                    new_tile,
                } => {
                    if let Some(map) = self.registry.get_mut::<Tilemap2DComponent>(entity) {
                        match old_tile {
                            Some(tile) => {
                                map.set_tile_ex(gx, gy, tile.tile_type, tile.solid, tile.one_way);
                            }
                            None => {
                                map.remove_tile(gx, gy);
                            }
                        }
                        self.editor.is_scene_dirty = true;
                    }
                    self.editor.redo_stack.push(UndoAction::TileChange {
                        entity,
                        gx,
                        gy,
                        old_tile,
                        new_tile,
                    });
                    self.editor.logs.push(LogMessage {
                        level: "INFO",
                        text: format!("Geri Alindi (Undo): Karo ({gx}, {gy}) geri alindi."),
                    });
                }
                UndoAction::Transform2DChanged { entity, old, new } => {
                    if let Some(t) = self.registry.get_mut::<Transform2D>(entity) {
                        *t = old;
                    }
                    self.editor.is_scene_dirty = true;
                    self.editor.redo_stack.push(UndoAction::Transform2DChanged {
                        entity,
                        old,
                        new,
                    });
                }
                UndoAction::Transform3DChanged { entity, old, new } => {
                    if let Some(t) = self.registry.get_mut::<Transform3D>(entity) {
                        *t = old;
                    }
                    self.editor.is_scene_dirty = true;
                    self.editor.redo_stack.push(UndoAction::Transform3DChanged {
                        entity,
                        old,
                        new,
                    });
                }
                UndoAction::SpriteTextureChanged { entity, old, new } => {
                    self.apply_option_comp(entity, old.clone());
                    self.editor.is_scene_dirty = true;
                    self.editor
                        .redo_stack
                        .push(UndoAction::SpriteTextureChanged { entity, old, new });
                }
                UndoAction::SpriteColorChanged { entity, old, new } => {
                    self.apply_option_comp(entity, old);
                    self.editor.is_scene_dirty = true;
                    self.editor.redo_stack.push(UndoAction::SpriteColorChanged {
                        entity,
                        old,
                        new,
                    });
                }
                UndoAction::ZIndexChanged { entity, old, new } => {
                    self.apply_option_comp(entity, old);
                    self.editor.is_scene_dirty = true;
                    self.editor
                        .redo_stack
                        .push(UndoAction::ZIndexChanged { entity, old, new });
                }
                UndoAction::Collider2DChanged { entity, old, new } => {
                    self.apply_option_comp(entity, old);
                    self.editor.is_scene_dirty = true;
                    self.editor
                        .redo_stack
                        .push(UndoAction::Collider2DChanged { entity, old, new });
                }
                UndoAction::ScriptChanged { entity, old, new } => {
                    self.apply_option_comp(entity, old.clone());
                    self.editor.is_scene_dirty = true;
                    self.editor
                        .redo_stack
                        .push(UndoAction::ScriptChanged { entity, old, new });
                }
                UndoAction::NodeNameChanged { entity, old, new } => {
                    self.apply_node_name(entity, old.clone());
                    self.editor.is_scene_dirty = true;
                    self.editor
                        .redo_stack
                        .push(UndoAction::NodeNameChanged { entity, old, new });
                }
            }
            self.audio.play("sfx_blip");
        }
    }

    pub(crate) fn redo(&mut self) {
        if let Some(action) = self.editor.redo_stack.pop() {
            match action {
                UndoAction::CreateEntity(e) => {
                    if let Some(snap) = self.snapshot_entity(e) {
                        self.cmd_buffer.destroy(e);
                        self.cmd_buffer.apply(&mut self.registry);
                        self.editor
                            .undo_stack
                            .push(UndoAction::DeleteEntity(Box::new(snap)));
                        if self.editor.selected_entity == Some(e) {
                            self.editor.selected_entity = None;
                        }
                        self.editor.logs.push(LogMessage {
                            level: "INFO",
                            text: format!("Yinelendi (Redo): Dugum #{} silindi.", e.index()),
                        });
                    }
                }
                UndoAction::DeleteEntity(snap) => {
                    let recreated_e = self.restore_entity(*snap);
                    self.editor
                        .undo_stack
                        .push(UndoAction::CreateEntity(recreated_e));
                    self.editor.logs.push(LogMessage {
                        level: "INFO",
                        text: format!(
                            "Yinelendi (Redo): Dugum #{} tekrar olusturuldu.",
                            recreated_e.index()
                        ),
                    });
                }
                UndoAction::MoveEntity {
                    entity,
                    old_transform,
                    new_transform,
                } => {
                    if let Some(t) = self.registry.get_mut::<Transform2D>(entity) {
                        *t = new_transform;
                    }
                    self.editor.undo_stack.push(UndoAction::MoveEntity {
                        entity,
                        old_transform,
                        new_transform,
                    });
                    self.editor.logs.push(LogMessage {
                        level: "INFO",
                        text: format!(
                            "Yinelendi (Redo): Dugum #{} konumu uygulandi.",
                            entity.index()
                        ),
                    });
                }
                UndoAction::TileChange {
                    entity,
                    gx,
                    gy,
                    old_tile,
                    new_tile,
                } => {
                    if let Some(map) = self.registry.get_mut::<Tilemap2DComponent>(entity) {
                        match new_tile {
                            Some(tile) => {
                                map.set_tile_ex(gx, gy, tile.tile_type, tile.solid, tile.one_way);
                            }
                            None => {
                                map.remove_tile(gx, gy);
                            }
                        }
                        self.editor.is_scene_dirty = true;
                    }
                    self.editor.undo_stack.push(UndoAction::TileChange {
                        entity,
                        gx,
                        gy,
                        old_tile,
                        new_tile,
                    });
                    self.editor.logs.push(LogMessage {
                        level: "INFO",
                        text: format!("Yinelendi (Redo): Karo ({gx}, {gy}) uygulandi."),
                    });
                }
                UndoAction::Transform2DChanged { entity, old, new } => {
                    if let Some(t) = self.registry.get_mut::<Transform2D>(entity) {
                        *t = new;
                    }
                    self.editor.is_scene_dirty = true;
                    self.editor.undo_stack.push(UndoAction::Transform2DChanged {
                        entity,
                        old,
                        new,
                    });
                }
                UndoAction::Transform3DChanged { entity, old, new } => {
                    if let Some(t) = self.registry.get_mut::<Transform3D>(entity) {
                        *t = new;
                    }
                    self.editor.is_scene_dirty = true;
                    self.editor.undo_stack.push(UndoAction::Transform3DChanged {
                        entity,
                        old,
                        new,
                    });
                }
                UndoAction::SpriteTextureChanged { entity, old, new } => {
                    self.apply_option_comp(entity, new.clone());
                    self.editor.is_scene_dirty = true;
                    self.editor
                        .undo_stack
                        .push(UndoAction::SpriteTextureChanged { entity, old, new });
                }
                UndoAction::SpriteColorChanged { entity, old, new } => {
                    self.apply_option_comp(entity, new);
                    self.editor.is_scene_dirty = true;
                    self.editor.undo_stack.push(UndoAction::SpriteColorChanged {
                        entity,
                        old,
                        new,
                    });
                }
                UndoAction::ZIndexChanged { entity, old, new } => {
                    self.apply_option_comp(entity, new);
                    self.editor.is_scene_dirty = true;
                    self.editor
                        .undo_stack
                        .push(UndoAction::ZIndexChanged { entity, old, new });
                }
                UndoAction::Collider2DChanged { entity, old, new } => {
                    self.apply_option_comp(entity, new);
                    self.editor.is_scene_dirty = true;
                    self.editor
                        .undo_stack
                        .push(UndoAction::Collider2DChanged { entity, old, new });
                }
                UndoAction::ScriptChanged { entity, old, new } => {
                    self.apply_option_comp(entity, new.clone());
                    self.editor.is_scene_dirty = true;
                    self.editor
                        .undo_stack
                        .push(UndoAction::ScriptChanged { entity, old, new });
                }
                UndoAction::NodeNameChanged { entity, old, new } => {
                    self.apply_node_name(entity, new.clone());
                    self.editor.is_scene_dirty = true;
                    self.editor
                        .undo_stack
                        .push(UndoAction::NodeNameChanged { entity, old, new });
                }
            }
            self.audio.play("sfx_blip");
        }
    }

    /// Sahnedeki mevcut Tilemap varlığını bulur veya yoksa yeni bir tane oluşturur.
    pub(crate) fn get_or_create_tilemap_entity(&mut self) -> Entity {
        if let Some(sel) = self.editor.selected_entity {
            if self.registry.has::<Tilemap2DComponent>(sel) {
                return sel;
            }
        }
        let mut tm_e_opt = None;
        self.registry.view1_ref::<Tilemap2DComponent, _>(|e, _| {
            if tm_e_opt.is_none() {
                tm_e_opt = Some(e);
            }
        });
        tm_e_opt.unwrap_or_else(|| {
            let new_e = self.registry.create();
            self.registry.emplace(
                new_e,
                Transform2D {
                    pos: Vec2::ZERO,
                    size: Vec2::splat(32.0),
                    rotation: 0.0,
                },
            );
            self.registry.emplace(new_e, Tilemap2DComponent::new(32.0));
            self.editor.selected_entity = Some(new_e);
            new_e
        })
    }

    /// Dünya koordinatını grid koordinatına çevirerek Fırça veya Silgi işlemini uygular,
    /// sahneyi kirli (dirty) olarak işaretler ve Undo/Redo yığınına kaydeder.
    pub(crate) fn apply_tile_tool(&mut self, world_pos: Vec2) {
        let tm_e = self.get_or_create_tilemap_entity();
        let (gx, gy, old_tile, new_tile) = {
            let Some(map) = self.registry.get::<Tilemap2DComponent>(tm_e) else {
                return;
            };
            let (gx, gy) = map.world_to_grid(world_pos);
            let old_tile = map.get_tile(gx, gy).copied();
            let new_tile = if self.editor.active_tool == ViewportTool::TileBrush {
                Some(TileData {
                    tile_type: self.editor.active_tile_type,
                    color: self.editor.active_tile_type.default_color(),
                    solid: self.editor.active_tile_type.is_default_solid(),
                    one_way: false,
                })
            } else if self.editor.active_tool == ViewportTool::TileEraser {
                None
            } else {
                return;
            };
            (gx, gy, old_tile, new_tile)
        };

        // Eğer mevcut karo ile yerleştirilecek karo tamamen aynıysa gereksiz işlem ve undo şişmesini önle
        if old_tile == new_tile {
            return;
        }

        if let Some(map) = self.registry.get_mut::<Tilemap2DComponent>(tm_e) {
            match new_tile {
                Some(t) => map.set_tile_ex(gx, gy, t.tile_type, t.solid, t.one_way),
                None => {
                    map.remove_tile(gx, gy);
                }
            }
            self.editor.is_scene_dirty = true;
            self.editor.undo_stack.push(UndoAction::TileChange {
                entity: tm_e,
                gx,
                gy,
                old_tile,
                new_tile,
            });
            self.editor.redo_stack.clear();
        }
    }

    pub(crate) fn trigger_shake_and_explosion(&mut self) {
        self.editor.camera.add_trauma(0.7);
        let target_pos = self
            .editor
            .selected_entity
            .and_then(|e| self.registry.get::<Transform2D>(e).map(|t| t.pos))
            .unwrap_or(self.editor.camera.pos);

        let p_ent = self.registry.create();
        self.registry.emplace(
            p_ent,
            Transform2D {
                pos: target_pos,
                size: Vec2::new(12.0, 12.0),
                rotation: 0.0,
            },
        );
        let emitter = ParticleEmitterComponent::new_explosion(36, target_pos);
        self.registry.emplace(p_ent, emitter);

        self.editor.logs.push(LogMessage {
            level: "INFO",
            text: format!(
                "Kamera Sarsintisi (Trauma +0.7) ve Patlama Parcacik Efekti tetiklendi ({:.0}, {:.0}).",
                target_pos.x, target_pos.y
            ),
        });
        self.audio.play("sfx_explosion");
    }

    /// glTF/GLB dosyasını sahneye aktarır ve hiyerarşik 3D varlıklar oluşturur.
    pub fn import_gltf_scene(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Vec<Entity>, String> {
        let model = rugeng_assets::GltfModel::from_file(path.as_ref())?;
        let mut created_entities = Vec::new();

        let cam_eye = self.editor.camera_3d.eye;
        let cam_fwd = self.editor.camera_3d.forward();
        let base_spawn = cam_eye + cam_fwd * 5.0;

        if !model.root_nodes.is_empty() {
            fn spawn_node(
                app: &mut App,
                model: &rugeng_assets::GltfModel,
                node: &rugeng_assets::GltfNode,
                parent_e: Option<Entity>,
                base_spawn: glam::Vec3,
                created: &mut Vec<Entity>,
                skeleton_attached: &mut bool,
            ) -> Entity {
                let e = app.registry.create();
                created.push(e);

                let trans = if parent_e.is_none() {
                    base_spawn + glam::Vec3::from(node.translation)
                } else {
                    glam::Vec3::from(node.translation)
                };
                let rot = glam::Quat::from_xyzw(
                    node.rotation[0],
                    node.rotation[1],
                    node.rotation[2],
                    node.rotation[3],
                );
                let scale = glam::Vec3::from(node.scale);

                app.registry.emplace(
                    e,
                    Transform3D {
                        translation: trans,
                        rotation: rot,
                        scale,
                    },
                );

                let node_name = if node.name.is_empty() {
                    format!("{}_Node_{}", model.name, e.index())
                } else {
                    node.name.clone()
                };
                app.registry.emplace(e, NodeName::new(node_name));

                // Mesh bağlıysa Mesh3DComponent ekle
                if let Some(mesh_idx) = node.mesh_index {
                    if let Some(submesh) = model.submeshes.get(mesh_idx) {
                        let mut roughness = 0.5f32;
                        let mut metallic = 0.0f32;
                        let mut color = [0.8, 0.8, 0.8, 1.0];

                        if let Some(mat_idx) = submesh.material_index {
                            if let Some(mat) = model.materials.get(mat_idx) {
                                roughness = mat.roughness_factor;
                                metallic = mat.metallic_factor;
                                color = mat.base_color_factor;
                            }
                        }

                        let mesh_comp_name = if submesh.name.is_empty() {
                            format!("{}_mesh_{}", model.name, mesh_idx)
                        } else {
                            submesh.name.clone()
                        };

                        app.registry.emplace(
                            e,
                            Mesh3DComponent::with_pbr(mesh_comp_name, color, roughness, metallic),
                        );
                    }
                }

                // Ebeveyn / Çocuk ilişkisini kur
                if let Some(p) = parent_e {
                    app.registry.emplace(e, Parent { entity: p });
                    if let Some(children) = app.registry.get_mut::<Children>(p) {
                        children.add(e);
                    } else {
                        let mut ch = Children::new();
                        ch.add(e);
                        app.registry.emplace(p, ch);
                    }
                }

                // İskelet kökü tespiti: İskelet varsa ve henüz eklenmediyse ilk düğüme veya skin_index içeren düğüme ekle
                if !*skeleton_attached && (node.skin_index.is_some() || parent_e.is_none()) {
                    if let Some(skel) = model.skeleton.as_ref() {
                        let anim_comp = rugeng_core::Animation3DComponent::new(
                            skel.clone(),
                            model.animations.clone(),
                        );
                        app.registry.emplace(e, anim_comp);
                        *skeleton_attached = true;
                    }
                }

                for child in &node.children {
                    spawn_node(
                        app,
                        model,
                        child,
                        Some(e),
                        base_spawn,
                        created,
                        skeleton_attached,
                    );
                }

                e
            }

            let mut skeleton_attached = false;
            for root in &model.root_nodes {
                spawn_node(
                    self,
                    &model,
                    root,
                    None,
                    base_spawn,
                    &mut created_entities,
                    &mut skeleton_attached,
                );
            }
        } else {
            // Fallback: Doğrudan submesh listesinden oluştur
            for (idx, submesh) in model.submeshes.iter().enumerate() {
                let e = self.registry.create();
                let mut roughness = 0.5f32;
                let mut metallic = 0.0f32;
                let mut color = [0.8, 0.8, 0.8, 1.0];

                if let Some(mat_idx) = submesh.material_index {
                    if let Some(mat) = model.materials.get(mat_idx) {
                        roughness = mat.roughness_factor;
                        metallic = mat.metallic_factor;
                        color = mat.base_color_factor;
                    }
                }

                self.registry.emplace(
                    e,
                    Transform3D {
                        translation: base_spawn,
                        rotation: glam::Quat::IDENTITY,
                        scale: glam::Vec3::ONE,
                    },
                );

                let mesh_name = if submesh.name.is_empty() {
                    format!("{}_submesh_{idx}", model.name)
                } else {
                    submesh.name.clone()
                };

                self.registry.emplace(
                    e,
                    Mesh3DComponent::with_pbr(mesh_name.clone(), color, roughness, metallic),
                );
                self.registry.emplace(e, NodeName::new(mesh_name));

                if idx == 0 {
                    if let Some(ref skel) = model.skeleton {
                        let anim_comp = rugeng_core::Animation3DComponent::new(
                            skel.clone(),
                            model.animations.clone(),
                        );
                        self.registry.emplace(e, anim_comp);
                    }
                }

                created_entities.push(e);
            }
        }

        if let Some(&first) = created_entities.first() {
            self.editor.selected_entity = Some(first);
        }
        self.editor.is_scene_dirty = true;
        self.editor.logs.push(LogMessage {
            level: "INFO",
            text: format!(
                "glTF modeli ice aktarildi: {} ({} dugum)",
                model.name,
                created_entities.len()
            ),
        });

        Ok(created_entities)
    }
}
