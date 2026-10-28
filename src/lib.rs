//! Horizon Alive: HUD-space visual effects for Horizon v0.2.92.
//! No world-space particle spawning or automatic explosion detection.
use horizon_mod_sdk::{Mod, Registration, game, input, log, render, storage, ui};

const MAX_PARTICLES: usize = 72;
const SETTINGS_KEY: &str = "alive.enabled";

#[derive(Clone, Copy)]
struct Particle {
    x: f32, y: f32, vx: f32, vy: f32, life: u8, color: u32,
}

struct Alive {
    particles: Vec<Particle>,
    rng: u32,
    last_health: Option<f32>,
    last_dimension: Option<String>,
    last_attack: bool,
    active: bool,
    last_tick: u64,
}

impl Alive {
    fn random(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f64 / u32::MAX as f64) as f32
    }
    fn spawn(&mut self, x: f32, y: f32, color: u32, burst: bool) {
        if self.particles.len() >= MAX_PARTICLES {
            self.particles.remove(0);
        }
        let rx = self.random() - 0.5;
        let ry = self.random() - 0.5;
        let speed = if burst { 0.26 } else { 0.025 };
        self.particles.push(Particle {
            x, y, vx: rx * speed, vy: ry * speed - 0.015,
            life: if burst { 12 } else { 38 }, color,
        });
    }
    fn burst(&mut self, color: u32, count: usize) {
        for _ in 0..count.min(MAX_PARTICLES) {
            self.spawn(0.5, 0.53, color, true);
        }
    }
    fn clear(&mut self) {
        self.particles.clear();
        render::set_hud(&[]);
    }
    fn update_particles(&mut self) {
        let (width, height) = render::gui_size();
        if !width.is_finite() || !height.is_finite() || width < 32.0 || height < 32.0 {
            self.clear();
            return;
        }
        for particle in &mut self.particles {
            particle.x += particle.vx * 0.05;
            particle.y += particle.vy * 0.05;
            particle.life = particle.life.saturating_sub(1);
        }
        self.particles.retain(|p| p.life > 0 && p.x > 0.0 && p.x < 1.0 && p.y > 0.0 && p.y < 1.0);
        let shapes: Vec<render::Shape> = self.particles.iter().map(|p| {
            let alpha = (p.life as u32 * 5).min(180);
            render::Shape::Fill(render::FillShape {
                rect: render::Rect {
                    x: p.x * width,
                    y: p.y * height,
                    w: 2.0, h: 2.0,
                },
                color: (alpha << 24) | (p.color & 0xFFFFFF),
            })
        }).collect();
        render::set_hud(&shapes);
    }
    fn settings(&self) {
        ui::set_settings(&[
            ui::Widget::Text(ui::TextWidget {
                id: 2,
                text: "Lightweight HUD particles, not world-space effects".into(),
                color: 0xA9DDEA,
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 1,
                label: format!("Horizon Alive: {}", if self.active { "ON" } else { "OFF" }),
            }),
        ]);
    }
    fn set_active(&mut self, enabled: bool) {
        self.active = enabled;
        if !enabled { self.clear(); }
        if let Err(err) = storage::set(SETTINGS_KEY, &[enabled as u8]) {
            log::write(log::Level::Warn, &format!("Cannot save settings: {err}"));
        }
        self.settings();
    }
}

impl Mod for Alive {
    fn init() -> Result<(Self, Registration), String> {
        let active = storage::get(SETTINGS_KEY)
            .and_then(|v| v.first().copied())
            .map(|v| v != 0)
            .unwrap_or(true);
        log::write(log::Level::Info, "Horizon Alive 0.3.3 initialized");
        Ok((Self {
            particles: Vec::with_capacity(MAX_PARTICLES),
            rng: 0xA11E_2026,
            last_health: None,
            last_dimension: None,
            last_attack: false,
            active,
            last_tick: 0,
        }, Registration::new().on_tick().command("alive")))
    }
    fn tick(&mut self, tick: u64) {
        if tick == self.last_tick { return; }
        self.last_tick = tick;
        let Some(player) = game::get_player() else {
            self.last_health = None;
            self.last_dimension = None;
            self.last_attack = false;
            self.clear();
            return;
        };
        if self.last_dimension.as_deref() != Some(player.dimension.as_str()) {
            self.last_dimension = Some(player.dimension.clone());
            self.last_health = None;
            self.clear();
        }
        if !self.active {
            self.last_health = Some(player.health);
            self.last_attack = false;
            return;
        }
        if let Some(old) = self.last_health
            && player.health.is_finite()
            && old.is_finite()
            && player.health < old - 0.01
        {
            self.burst(0xFF9F87, 8);
            render::shake(0.045, 0.13);
        }
        self.last_health = Some(player.health);
        let attacking = input::mapping("key.attack").is_some_and(|key| key.down);
        if attacking && !self.last_attack {
            let color = match game::raycast(4.5) {
                Some(hit) if hit.block.contains("water") => 0x90DAFE,
                Some(hit) if hit.block.contains("leaves") => 0x9DCB8E,
                _ => 0xFFE3B2,
            };
            self.burst(color, 7);
        }
        self.last_attack = attacking;
        if tick.is_multiple_of(12) {
            let color = if player.dimension == "minecraft:the_nether" {
                0xFF9759
            } else if player.dimension == "minecraft:the_end" {
                0xB8A7FF
            } else {
                0xE6ECCB
            };
            let x = 0.1 + self.random() * 0.8;
            let y = 0.2 + self.random() * 0.55;
            self.spawn(x, y, color, false);
        }
        self.update_particles();
    }
    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "alive" { return Err("Unknown command".into()); }
        match args.trim() {
            "" | "help" => Ok(Some("/alive on|off|status|demo".into())),
            "on" => { self.set_active(true); Ok(Some("Horizon Alive enabled".into())) }
            "off" => { self.set_active(false); Ok(Some("Horizon Alive disabled".into())) }
            "status" => Ok(Some(format!("Alive: {}, {} particles", self.active, self.particles.len()))),
            "demo" if !self.active => Err("Enable Horizon Alive with /alive on first".into()),
            "demo" => { self.burst(0xFFC878, 32); Ok(Some("2D explosion preview".into())) }
            _ => Err("Use /alive help".into()),
        }
    }
    fn ui_settings_opened(&mut self) { self.settings(); }
    fn ui_select(&mut self, id: u32, _row: u32) {
        if id == 1 { self.set_active(!self.active); }
    }
}

horizon_mod_sdk::export_mod!(Alive);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn particles_remain_bounded() {
        let mut m = Alive {
            particles: Vec::new(), rng: 3, last_health: None,
            last_dimension: None, last_attack: false, active: true, last_tick: 0,
        };
        for _ in 0..200 { m.burst(0xFFFFFF, 20); }
        assert_eq!(m.particles.len(), MAX_PARTICLES);
    }
    #[test]
    fn rng_remains_finite() {
        let mut m = Alive {
            particles: Vec::new(), rng: 3, last_health: None,
            last_dimension: None, last_attack: false, active: true, last_tick: 0,
        };
        for _ in 0..500 { assert!(m.random().is_finite()); }
    }
}
