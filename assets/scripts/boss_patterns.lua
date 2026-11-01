-- Project-Ember (Rugeng) — Aşama 5: Boss Davranış ve Mermi Desenleri (Luau)
-- Flat SoA Scratch View ve ScriptCommandBuffer ile toplu veri iletişimi.
-- Sıfır nesne başı FFI (zero per-entity FFI).

BossLogic = {
    phase = 1,
    phase_timer = 0.0,
    spiral_speed = 180.0,
    nova_speed = 220.0,
    flower_speed = 150.0
}

-- Durum korumalı hot reload için state aktarımı (G3)
function BossLogic.export_state()
    return {
        phase = BossLogic.phase,
        phase_timer = BossLogic.phase_timer,
        spiral_speed = BossLogic.spiral_speed,
        nova_speed = BossLogic.nova_speed,
        flower_speed = BossLogic.flower_speed
    }
end

function BossLogic.import_state(s)
    if s.phase then BossLogic.phase = s.phase end
    if s.phase_timer then BossLogic.phase_timer = s.phase_timer end
    if s.spiral_speed then BossLogic.spiral_speed = s.spiral_speed end
    if s.nova_speed then BossLogic.nova_speed = s.nova_speed end
    if s.flower_speed then BossLogic.flower_speed = s.flower_speed end
end

-- Boss Mermi Deseni Üretimi
-- pattern_mode: 0: Spiral, 1: Nova Ring, 2: Flower
-- cmd_buffer: ScriptCommandBuffer userdata (FFI üzerinden tek tek Rust çağırmaz, tampona yazar)
function BossLogic.update_boss(pattern_mode, pattern_angle, boss_x, boss_y, spawn_count, cmd_buffer)
    local PI = 3.141592653589793
    local new_angle = pattern_angle

    if pattern_mode == 0 then
        -- Dönen 4 kollu spiral
        local batch = math.max(1, math.floor(spawn_count / 4))
        for arm = 0, 3 do
            local base_angle = pattern_angle + arm * (PI * 0.5)
            for i = 0, batch - 1 do
                local speed = BossLogic.spiral_speed + i * 12.0
                local angle = base_angle + i * 0.02
                local vx = math.cos(angle) * speed
                local vy = math.sin(angle) * speed
                cmd_buffer:spawn_bullet(boss_x, boss_y, vx, vy, true, 3.5)
            end
        end
        new_angle = pattern_angle + 0.08
    elseif pattern_mode == 1 then
        -- 360 derece Nova halkası
        local arms = 32
        local angle_step = (PI * 2.0) / arms
        for i = 0, arms - 1 do
            local angle = pattern_angle + i * angle_step
            local vx = math.cos(angle) * BossLogic.nova_speed
            local vy = math.sin(angle) * BossLogic.nova_speed
            cmd_buffer:spawn_bullet(boss_x, boss_y, vx, vy, true, 3.5)
        end
        new_angle = pattern_angle + 0.04
    else
        -- Çiçek / Wave deseni
        for i = 0, 15 do
            local angle = pattern_angle + i * 0.4
            local speed = BossLogic.flower_speed + 80.0 * math.abs(math.sin(angle * 3.0))
            local vx = math.cos(angle) * speed
            local vy = math.sin(angle) * speed
            cmd_buffer:spawn_bullet(boss_x, boss_y, vx, vy, true, 3.5)
        end
        new_angle = pattern_angle + 0.06
    end

    return new_angle
end

-- Toplu mermi simülasyonu (Flat SoA)
-- Tek bir Rust çağrısı ile binlerce merminin hareketini ve ekran sınırını kontrol eder (0 malloc)
function BossLogic.update_bullets(view, dt, screen_w, screen_h, cmd_buffer)
    view:batch_integrate_to_cmd(dt, screen_w, screen_h, cmd_buffer)
end

return BossLogic
