//! Horizon Alive v0.5: safe WebAssembly control mod.
//!
//! The unwanted 2D HUD particles have been removed. Horizon 0.1 does not
//! expose world-space geometry or TNT explosion events to .hmod code.
//! The pure Rust world-space explosion model lives in explosion3d.rs; a
//! version-pinned native renderer is required to display its geometry.
pub mod explosion3d;

use horizon_mod_sdk::{Mod, Registration, log, render, storage, ui};

const ENABLED_KEY: &str = "alive.enabled";
const QUALITY_KEY: &str = "alive.quality";
const MOTION_KEY: &str = "alive.reduced_motion";
// Retain the legacy setting but never draw the old HUD particle shapes.
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
    fn as_byte(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Balanced => 1,
            Self::High => 2,
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
}

fn setting(key: &str, default: bool) -> bool {
    storage::get(key)
        .and_then(|data| data.first().copied())
        .map(|byte| byte != 0)
        .unwrap_or(default)
}

fn store(key: &str, value: u8) {
    if let Err(err) = storage::set(key, &[value]) {
        log::write(log::Level::Warn, &format!("Horizon Alive storage: {err}"));
    }
}

struct Alive {
    enabled: bool,
    quality: Quality,
    reduced_motion: bool,
}

impl Alive {
    fn new() -> Self {
        // Preserve settings from the previous .hmod releases.
        let quality = storage::get(QUALITY_KEY)
            .and_then(|data| data.first().copied())
            .map(Quality::from_byte)
            .unwrap_or(Quality::Balanced);
        Self {
            enabled: setting(ENABLED_KEY, true),
            quality,
            reduced_motion: setting(MOTION_KEY, false),
        }
    }

    fn settings(&self) {
        ui::set_settings(&[
            ui::Widget::Text(ui::TextWidget {
                id: 10,
                text: "HUD particles removed. 3D TNT requires native engine support.".into(),
                color: 0xBED6DD,
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 1,
                label: format!("Explosion preference: {}", if self.enabled { "ON" } else { "OFF" }),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 2,
                label: format!("Target quality: {}", self.quality.name()),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 3,
                label: format!(
                    "Camera motion preference: {}",
                    if self.reduced_motion { "reduced" } else { "normal" },
                ),
            }),
        ]);
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        store(ENABLED_KEY, u8::from(enabled));
        self.settings();
    }

    fn set_quality(&mut self, quality: Quality) {
        self.quality = quality;
        store(QUALITY_KEY, quality.as_byte());
        self.settings();
    }

    fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
        store(MOTION_KEY, u8::from(reduced));
        self.settings();
    }

    fn status(&self) -> String {
        format!(
            "Alive: {}; quality: {}; motion: {}; HUD particles: removed; 3D TNT: native required",
            if self.enabled { "on" } else { "off" },
            self.quality.name(),
            if self.reduced_motion { "reduced" } else { "normal" },
        )
    }
}

impl Mod for Alive {
    fn init() -> Result<(Self, Registration), String> {
        // The v0.4.x HUD is retained by the engine; ensure it is empty on reload.
        render::set_hud(&[]);
        let previous = setting(GRAPHICS_KEY, true);
        if previous {
            log::write(
                log::Level::Info,
                "Legacy HUD visuals have been retired; stored preferences are preserved",
            );
        }
        log::write(
            log::Level::Info,
            "Horizon Alive v0.5: no screen-space particles; 3D renderer not exposed by .hmod",
        );
        // No tick or frame hook: a passive control mod cannot affect FPS.
        Ok((Self::new(), Registration::new().command("alive")))
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "alive" {
            return Err("Unknown command".into());
        }
        match args.trim() {
            "" | "help" => Ok(Some(
                "/alive status|on|off|quality low|balanced|high|motion reduced|normal".into(),
            )),
            "status" => Ok(Some(self.status())),
            "on" => {
                self.set_enabled(true);
                Ok(Some("Explosion preference enabled; native 3D renderer required".into()))
            }
            "off" => {
                self.set_enabled(false);
                Ok(Some("Explosion preference disabled".into()))
            }
            "quality low" => {
                self.set_quality(Quality::Low);
                Ok(Some("Target quality: low".into()))
            }
            "quality balanced" => {
                self.set_quality(Quality::Balanced);
                Ok(Some("Target quality: balanced".into()))
            }
            "quality high" => {
                self.set_quality(Quality::High);
                Ok(Some("Target quality: high".into()))
            }
            "motion reduced" => {
                self.set_reduced_motion(true);
                Ok(Some("Camera motion preference: reduced".into()))
            }
            "motion normal" => {
                self.set_reduced_motion(false);
                Ok(Some("Camera motion preference: normal".into()))
            }
            "demo" | "demo boom" | "demo water" | "demo fireflies" | "demo punch" | "graphics on" => {
                Err("2D effects removed; .hmod cannot render world-space explosions".into())
            }
            "graphics off" => {
                render::set_hud(&[]);
                Ok(Some("HUD particle renderer permanently disabled".into()))
            }
            _ => Err("Use /alive help".into()),
        }
    }

    fn ui_settings_opened(&mut self) {
        self.settings();
    }

    fn ui_select(&mut self, id: u32, _row: u32) {
        match id {
            1 => self.set_enabled(!self.enabled),
            2 => self.set_quality(self.quality.next()),
            3 => self.set_reduced_motion(!self.reduced_motion),
            _ => {}
        }
    }
}

horizon_mod_sdk::export_mod!(Alive);

#[cfg(test)]
mod tests {
    use super::Quality;

    #[test]
    fn quality_compatibility() {
        assert_eq!(Quality::from_byte(0), Quality::Low);
        assert_eq!(Quality::from_byte(1), Quality::Balanced);
        assert_eq!(Quality::from_byte(2), Quality::High);
        assert_eq!(Quality::from_byte(255), Quality::Balanced);
    }

    #[test]
    fn quality_cycle_is_reversible() {
        assert_eq!(Quality::Low.next().next().next(), Quality::Low);
    }
}
