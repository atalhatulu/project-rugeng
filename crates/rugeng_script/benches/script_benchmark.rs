use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rugeng_script::{ScratchSoAView, ScriptCommandBuffer, ScriptEngine, ScriptEngineConfig};

// Native 5.000 mermi fizik simülasyonu (toplu entegrasyon)
#[allow(clippy::too_many_arguments)]
fn native_batch_simulate_5k(
    pos_x: &mut [f32],
    pos_y: &mut [f32],
    vel_x: &[f32],
    vel_y: &[f32],
    dt: f32,
    screen_w: f32,
    screen_h: f32,
    cmd_buffer: &mut ScriptCommandBuffer,
) {
    for i in 0..pos_x.len() {
        pos_x[i] += vel_x[i] * dt;
        pos_y[i] += vel_y[i] * dt;

        let x = pos_x[i];
        let y = pos_y[i];
        if x < -50.0 || x > screen_w + 50.0 || y < -50.0 || y > screen_h + 50.0 {
            cmd_buffer.push_destroy(rugeng_core::Entity::new(i as u32, 0));
        }
    }
}

// Native boss mermi deseni üretimi (Spiral / Ring)
fn native_update_boss_pattern(
    pattern_angle: f32,
    boss_x: f32,
    boss_y: f32,
    spawn_count: usize,
    cmd_buffer: &mut ScriptCommandBuffer,
) -> f32 {
    for i in 0..spawn_count {
        let angle = pattern_angle + (i as f32) * 0.05;
        let vx = angle.cos() * 150.0;
        let vy = angle.sin() * 150.0;
        cmd_buffer.push_spawn_bullet(rugeng_script::SpawnBulletCmd {
            x: boss_x,
            y: boss_y,
            vx,
            vy,
            is_enemy: true,
            radius: 3.5,
        });
    }
    pattern_angle + 0.08
}

fn bench_5k_entities(c: &mut Criterion) {
    let count = 5_000;
    let dt = 1.0 / 60.0;

    // 1. Native veri hazırlığı
    let mut native_pos_x: Vec<f32> = (0..count).map(|i| 640.0 + (i as f32) * 0.1).collect();
    let mut native_pos_y: Vec<f32> = (0..count).map(|i| 360.0 + (i as f32) * 0.1).collect();
    let native_vel_x: Vec<f32> = (0..count)
        .map(|i| ((i as f32) * 0.05).cos() * 150.0)
        .collect();
    let native_vel_y: Vec<f32> = (0..count)
        .map(|i| ((i as f32) * 0.05).sin() * 150.0)
        .collect();
    let mut native_cmd_buffer = ScriptCommandBuffer::with_capacity(1024);

    // 2. Lua veri hazırlığı
    let script_config = ScriptEngineConfig {
        memory_limit_bytes: 64 * 1024 * 1024,
        max_instructions_per_call: 500_000,
    };
    let mut script_engine = ScriptEngine::new(script_config).unwrap();

    let lua_script = r#"
        BossLogic = {}
        function BossLogic.update_bullets(view, dt, screen_w, screen_h, cmd_buffer)
            view:batch_integrate_to_cmd(dt, screen_w, screen_h, cmd_buffer)
        end

        function BossLogic.update_boss(pattern_mode, pattern_angle, boss_x, boss_y, spawn_count, cmd_buffer)
            for i = 0, spawn_count - 1 do
                local angle = pattern_angle + i * 0.05
                local vx = math.cos(angle) * 150.0
                local vy = math.sin(angle) * 150.0
                cmd_buffer:spawn_bullet(boss_x, boss_y, vx, vy, true, 3.5)
            end
            return pattern_angle + 0.08
        end
    "#;
    script_engine
        .load_script("BossLogic", lua_script, None)
        .unwrap();

    let mut soa_view = ScratchSoAView::with_capacity(count);
    for i in 0..count {
        soa_view.push(
            rugeng_core::Entity::new(i as u32, 0),
            native_pos_x[i],
            native_pos_y[i],
            native_vel_x[i],
            native_vel_y[i],
            3.5,
            true,
        );
    }
    let mut lua_cmd_buffer = ScriptCommandBuffer::with_capacity(1024);

    let mut group = c.benchmark_group("Gameplay Benchmark (5000 Items & Pattern Spawn)");

    // Karşılaştırma 1: 5000 Mermi Simülasyonu (Native vs Luau SoA)
    group.bench_function("Native: 5000 Bullets Update", |b| {
        b.iter(|| {
            native_cmd_buffer.clear();
            native_batch_simulate_5k(
                black_box(&mut native_pos_x),
                black_box(&mut native_pos_y),
                black_box(&native_vel_x),
                black_box(&native_vel_y),
                black_box(dt),
                1280.0,
                720.0,
                black_box(&mut native_cmd_buffer),
            );
        })
    });

    group.bench_function("Luau Batch SoA: 5000 Bullets Update", |b| {
        b.iter(|| {
            lua_cmd_buffer.clear();
            let res = script_engine.update_bullets_batch(
                "BossLogic",
                black_box(&mut soa_view),
                black_box(dt),
                1280.0,
                720.0,
                black_box(&mut lua_cmd_buffer),
            );
            black_box(res).unwrap();
        })
    });

    // Karşılaştırma 2: Boss Deseni Üretimi (256 Mermi / Tipik Frame)
    group.bench_function("Native: Boss Pattern Spawn (256 Bullets)", |b| {
        b.iter(|| {
            native_cmd_buffer.clear();
            let new_ang = native_update_boss_pattern(
                black_box(1.57),
                640.0,
                140.0,
                256,
                black_box(&mut native_cmd_buffer),
            );
            black_box(new_ang);
        })
    });

    group.bench_function("Luau: Boss Pattern Spawn (256 Bullets)", |b| {
        b.iter(|| {
            lua_cmd_buffer.clear();
            let new_ang = script_engine.update_boss_pattern(
                "BossLogic",
                0,
                black_box(1.57),
                640.0,
                140.0,
                256,
                black_box(&mut lua_cmd_buffer),
            );
            black_box(new_ang).unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, bench_5k_entities);
criterion_main!(benches);
