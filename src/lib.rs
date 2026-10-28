//! Horizon Alive: restrained HUD-only effects for Horizon.
//!
//! The mod cannot change the engine's wgpu swapchain usage flags.
//! A crash due to missing COPY_SRC on a Surface Texture must be fixed upstream.
mod effects;

use effects::{Effects, Particle, Style};
use horizon_mod_sdk::{Mod, Registration, game, input, log, render, storage, ui};

const ENABLED_KEY: &str = "alive.enabled"; // Preserve v0.3.x user preferences.
const QUALITY_KEY: &str = "alive.quality";
const MOTION_KEY: &str = "alive.reduced_motion";
const GRAPHICS_KEY: &str = "alive.graphics";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Quality {
    Low,
    Balanced,
    High,
}

impl Quality {
    fn from_byte(value: u8) -> Self {
        match value {
            0 => Self::Low,
            2 => Self::High,
            _ => Self::Balanced,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Balanced => "balanced",
            Self::High => "high",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Low => Self::Balanced,
            Self::Balanced => Self::High,
            Self::High => Self::Low,
        }
    }

    fn interval(self) -> u64 {
        match self {
            Self::Low => 18,
            Self::Balanced => 9,
            Self::High => 5,
        }
    }

    fn burst_limit(self) -> usize {
        match self {
            Self::Low => 9,
            Self::Balanced => 18,
            Self::High => 32,
        }
    }

    fn as_byte(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Balanced => 1,
            Self::High => 2,
        }
    }
}

fn stored_flag(key: &str, default: bool) -> bool {
    storage::get(key)
        .and_then(|data| data.first().copied())
        .map(|b| b != 0)
        .unwrap_or(default)
}

fn save_byte(key: &str, byte: u8) {
    if let Err(err) = storage::set(key, &[byte]) {
        log::write(log::Level::Warn, &format!("Horizon Alive: settings not saved: {err}"));
    }
}

struct Alive {
    fx: Effects,
    enabled: bool,
    graphics: bool,
    reduced_motion: bool,
    quality: Quality,
    tick_seen: Option<u64>,
    dimension: Option<String>,
    last_health: Option<f32>,
    last_position: Option<(f64, f64)>,
    last_attack: bool,
}

impl Alive {
    fn new() -> Self {
        let quality = storage::get(QUALITY_KEY)
            .and_then(|data| data.first().copied())
            .map(Quality::from_byte)
            .unwrap_or(Quality::Balanced);
        Self {
            fx: Effects::new(),
            enabled: stored_flag(ENABLED_KEY, true),
            graphics: stored_flag(GRAPHICS_KEY, true),
            reduced_motion: stored_flag(MOTION_KEY, false),
            quality,
            tick_seen: None,
            dimension: None,
            last_health: None,
            last_position: None,
            last_attack: false,
        }
    }

    fn clear_visuals(&mut self) {
        self.fx.clear();
        // Retained HUD shapes must be explicitly removed when disabling.
        render::set_hud(&[]);
    }

    fn reset_world(&mut self) {
        self.last_health = None;
        self.last_position = None;
        self.last_attack = false;
        self.clear_visuals();
    }

    fn update_settings(&self) {
        ui::set_settings(&[
            ui::Widget::Text(ui::TextWidget {
                id: 10,
                text: "Screen-space visuals only; no world-space particles".into(),
                color: 0xACD9DD,
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 1,
                label: format!("Effects: {}", if self.enabled { "ON" } else { "OFF" }),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 2,
                label: format!("Quality: {}", self.quality.name()),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 3,
                label: format!(
                    "Camera motion: {}",
                    if self.reduced_motion { "reduced" } else { "normal" },
                ),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 4,
                label: format!(
                    "HUD renderer: {}",
                    if self.graphics { "ON" } else { "OFF (diagnostic)" },
                ),
            }),
        ]);
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.clear_visuals();
        self.last_attack = false;
        self.last_position = None;
        self.last_health = None;
        save_byte(ENABLED_KEY, u8::from(enabled));
        self.update_settings();
    }

    fn set_graphics(&mut self, graphics: bool) {
        self.graphics = graphics;
        self.clear_visuals();
        save_byte(GRAPHICS_KEY, u8::from(graphics));
        self.update_settings();
    }

    fn set_quality(&mut self, quality: Quality) {
        self.quality = quality;
        self.fx.clear();
        save_byte(QUALITY_KEY, quality.as_byte());
        self.update_settings();
    }

    fn set_reduced_motion(&mut self, value: bool) {
        self.reduced_motion = value;
        save_byte(MOTION_KEY, u8::from(value));
        self.update_settings();
    }

    fn feedback(&self) -> String {
        format!(
            "Alive: {}; quality: {}; motion: {}; HUD: {}; particles: {}",
            if self.enabled { "on" } else { "off" },
            self.quality.name(),
            if self.reduced_motion { "reduced" } else { "normal" },
            if self.graphics { "on" } else { "off" },
            self.fx.len(),
        )
    }

    fn preview(&mut self, name: &str) -> Result<Option<String>, String> {
        if !self.enabled {
            return Err("Enable Horizon Alive with /alive on first".into());
        }
        if !self.graphics {
            return Err("Enable the HUD renderer with /alive graphics on first".into());
        }
        let n = self.quality.burst_limit();
        match name {
            "" | "boom" => {
                self.fx.burst(Style::Spark, 0xFFCC87, n);
                self.fx.ripple(0xFFDF96);
                Ok(Some("2D explosion preview".into()))
            }
            "water" => {
                self.fx.burst(Style::Drop, 0x77CEF4, n);
                self.fx.ripple(0x8CE3FF);
                Ok(Some("2D water ripple preview".into()))
            }
            "fireflies" => {
                self.fx.burst(Style::Glow, 0xD6F4A8, n.min(20));
                Ok(Some("2D fireflies preview".into()))
            }
            "punch" => {
                self.fx.burst(Style::Spark, 0xF3DAAD, n.min(12));
                Ok(Some("2D swing preview".into()))
            }
            _ => Err("Use /alive demo boom|water|fireflies|punch".into()),
        }
    }

    fn draw_rect(shapes: &mut Vec<render::Shape>, x: f32, y: f32, w: f32, h: f32, argb: u32) {
        if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() {
            return;
        }
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        shapes.push(render::Shape::Fill(render::FillShape {
            rect: render::Rect { x, y, w, h },
            color: argb,
        }));
    }

    fn draw_particle(shapes: &mut Vec<render::Shape>, p: &Particle, w: f32, h: f32) {
        let alpha = (u32::from(p.ttl - p.age) * 185 / u32::from(p.ttl)).min(185);
        if alpha == 0 {
            return;
        }
        let x = p.x * w;
        let y = p.y * h;
        let rgb = p.rgb & 0x00FF_FFFF;
        let color = (alpha << 24) | rgb;
        match p.style {
            Style::Ring => {
                let radius = 3.0 + f32::from(p.age) * 0.9;
                // Four clean pixel lines, with no fullscreen post-processing.
                Self::draw_rect(shapes, x - radius, y - radius * 0.5, radius * 2.0, 1.0, color);
                Self::draw_rect(shapes, x - radius, y + radius * 0.5, radius * 2.0, 1.0, color);
                Self::draw_rect(shapes, x - radius, y - radius * 0.5, 1.0, radius, color);
                Self::draw_rect(shapes, x + radius, y - radius * 0.5, 1.0, radius, color);
            }
            Style::Glow => {
                Self::draw_rect(shapes, x - 3.0, y - 3.0, 6.0, 6.0, ((alpha / 6) << 24) | rgb);
                Self::draw_rect(shapes, x - 1.0, y - 1.0, 2.0, 2.0, color);
            }
            Style::Leaf => {
                Self::draw_rect(shapes, x, y, 3.0, 2.0, color);
                Self::draw_rect(shapes, x + 1.0, y + 2.0, 1.0, 1.0, color);
            }
            Style::Drop => Self::draw_rect(shapes, x, y, 2.0, 3.0, color),
            Style::Spark => {
                Self::draw_rect(shapes, x, y, 2.0, 2.0, color);
                Self::draw_rect(shapes, x - 1.0, y + 1.0, 4.0, 1.0, ((alpha / 3) << 24) | rgb);
            }
            Style::Ember => Self::draw_rect(shapes, x, y, 1.5, 2.0, color),
            Style::Dust => Self::draw_rect(shapes, x, y, 1.5, 1.5, color),
        }
    }

    fn redraw(&mut self) {
        if !self.enabled || !self.graphics {
            return;
        }
        let (width, height) = render::gui_size();
        if !width.is_finite()
            || !height.is_finite()
            || !(32.0..=16_384.0).contains(&width)
            || !(32.0..=16_384.0).contains(&height)
        {
            self.clear_visuals();
            return;
        }
        // Worst case: 96 rings * 4 shapes = 384, beneath the API's 1024 limit.
        let mut shapes = Vec::with_capacity(self.fx.len() * 4);
        for p in self.fx.iter() {
            Self::draw_particle(&mut shapes, p, width, height);
        }
        render::set_hud(&shapes);
    }

    fn ambient(&mut self, tick: u64, dimension: &str) {
        if !tick.is_multiple_of(self.quality.interval()) {
            return;
        }
        let (style, rgb) = match dimension {
            "minecraft:the_nether" => (Style::Ember, 0xF4A470),
            "minecraft:the_end" => (Style::Glow, 0xB9A5FC),
            _ => {
                let night = game::get_clock()
                    .is_some_and(|clock| (13_000..23_000).contains(&(clock.day_time % 24_000)));
                if night {
                    (Style::Glow, 0xDDF6B4)
                } else if tick.is_multiple_of(3) {
                    (Style::Leaf, 0xA5D58B)
                } else {
                    (Style::Dust, 0xD9D0B9)
                }
            }
        };
        self.fx.ambient(style, rgb);
    }

    fn handle_player(&mut self, tick: u64, player: game::Player) {
        if self.dimension.as_deref() != Some(player.dimension.as_str()) {
            self.dimension = Some(player.dimension.clone());
            self.reset_world();
        }
        if !player.x.is_finite()
            || !player.y.is_finite()
            || !player.z.is_finite()
            || !player.health.is_finite()
        {
            self.reset_world();
            return;
        }
        if !self.enabled || !self.graphics {
            self.last_health = Some(player.health);
            self.last_position = Some((player.x, player.z));
            self.last_attack = input::mapping("key.attack").is_some_and(|state| state.down);
            return;
        }
        if let Some(previous) = self.last_health
            && previous.is_finite()
            && player.health < previous - 0.01
        {
            self.fx.burst(Style::Spark, 0xFF9E92, self.quality.burst_limit() / 2);
            if !self.reduced_motion {
                render::shake(0.035, 0.12);
            }
        }
        self.last_health = Some(player.health);

        let attacking = input::mapping("key.attack").is_some_and(|state| state.down);
        if attacking && !self.last_attack {
            let color = match game::raycast(4.5) {
                Some(hit) if hit.block.contains("leaves") => 0xA5D58B,
                Some(hit) if hit.block.contains("sand") => 0xE5D09B,
                Some(hit) if hit.block.contains("snow") => 0xEDF4FE,
                Some(hit) if hit.block.contains("stone") => 0xBBC4D0,
                _ => 0xFCE6C5,
            };
            self.fx.burst(Style::Spark, color, self.quality.burst_limit() / 2);
        }
        self.last_attack = attacking;

        if let Some((old_x, old_z)) = self.last_position {
            let dist = (player.x - old_x).hypot(player.z - old_z);
            // Ignore teleports, loading jumps and NaN instead of spamming particles.
            if dist.is_finite() && (0.14..2.0).contains(&dist) && tick.is_multiple_of(5) {
                self.fx.movement();
            }
        }
        self.last_position = Some((player.x, player.z));

        self.ambient(tick, &player.dimension);
        self.fx.advance();
        self.redraw();
    }
}

impl Mod for Alive {
    fn init() -> Result<(Self, Registration), String> {
        log::write(log::Level::Info, "Horizon Alive 0.4.0 initialized (2D HUD)");
        Ok((Self::new(), Registration::new().command("alive").on_tick()))
    }

    fn tick(&mut self, tick: u64) {
        if self.tick_seen == Some(tick) {
            return;
        }
        self.tick_seen = Some(tick);
        match game::get_player() {
            Some(player) => self.handle_player(tick, player),
            None => {
                self.dimension = None;
                self.reset_world();
            }
        }
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "alive" {
            return Err("Unknown command".into());
        }
        let cmd = args.trim();
        match cmd {
            "" | "help" => Ok(Some(
                "/alive on|off|status|demo [boom|water|fireflies|punch]|quality low|balanced|high|motion reduced|normal|graphics on|off".into(),
            )),
            "on" => {
                self.set_enabled(true);
                Ok(Some("Horizon Alive enabled".into()))
            }
            "off" => {
                self.set_enabled(false);
                Ok(Some("Horizon Alive disabled".into()))
            }
            "status" => Ok(Some(self.feedback())),
            "quality low" => {
                self.set_quality(Quality::Low);
                Ok(Some("Horizon Alive quality: low".into()))
            }
            "quality balanced" => {
                self.set_quality(Quality::Balanced);
                Ok(Some("Horizon Alive quality: balanced".into()))
            }
            "quality high" => {
                self.set_quality(Quality::High);
                Ok(Some("Horizon Alive quality: high".into()))
            }
            "motion reduced" => {
                self.set_reduced_motion(true);
                Ok(Some("Horizon Alive camera motion: reduced".into()))
            }
            "motion normal" => {
                self.set_reduced_motion(false);
                Ok(Some("Horizon Alive camera motion: normal".into()))
            }
            "graphics off" => {
                self.set_graphics(false);
                Ok(Some("Horizon Alive HUD renderer disabled (diagnostic mode)".into()))
            }
            "graphics on" => {
                self.set_graphics(true);
                Ok(Some("Horizon Alive HUD renderer enabled".into()))
            }
            "demo" => self.preview(""),
            other if other.starts_with("demo ") => self.preview(other.trim_start_matches("demo ").trim()),
            _ => Err("Use /alive help".into()),
        }
    }

    fn ui_settings_opened(&mut self) {
        self.update_settings();
    }

    fn ui_select(&mut self, id: u32, _row: u32) {
        match id {
            1 => self.set_enabled(!self.enabled),
            2 => self.set_quality(self.quality.next()),
            3 => self.set_reduced_motion(!self.reduced_motion),
            4 => self.set_graphics(!self.graphics),
            _ => {}
        }
    }
}

horizon_mod_sdk::export_mod!(Alive);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_limits_and_rotation() {
        assert_eq!(Quality::Low.burst_limit(), 9);
        assert!(Quality::High.burst_limit() <= effects::MAX_PARTICLES);
        assert_eq!(Quality::High.next(), Quality::Low);
        assert_eq!(Quality::from_byte(255), Quality::Balanced);
    }

    #[test]
    fn particle_alpha_stays_within_bounds() {
        for ttl in 1_u8..=255_u8 {
            for age in 0..ttl {
                let alpha = (u32::from(ttl - age) * 185 / u32::from(ttl)).min(185);
                assert!(alpha <= 185);
            }
        }
    }
}
