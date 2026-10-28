//! Horizon Alive v0.8 — sandboxed .hmod, real XYZ vanilla particle commands.
//!
//! The SDK API 0.1 has no prime_fuse or detonated event. Inferred player
//! ignitions are labelled as estimates, not confirmed engine TNT events.
mod detector;
mod diagnostics;
mod worldfx;

use detector::{Detector, Observation};
use diagnostics::{Diagnostics, label, point};
use horizon_mod_sdk::{Mod, Registration, chat, command, game, input, log, render, storage, ui};
use worldfx::{Blast, Config, Quality, COOLDOWN_TICKS, LIFE_TICKS, MAX_ACTIVE};

const ENABLED: &str = "alive.enabled";
const QUALITY: &str = "alive.quality";
const SCALE: &str = "alive.scale";
const FLASH: &str = "alive.flash";
const SMOKE: &str = "alive.smoke";
const DETECTION: &str = "alive.detect";
const AUTO_FX: &str = "alive.auto_fx";
const DEBUG: &str = "alive.debug";
const MAX_SCHEDULED: usize = 8;

fn stored(key: &str, default: u8) -> u8 {
    storage::get(key)
        .and_then(|v| v.first().copied())
        .unwrap_or(default)
}

fn save(key: &str, value: u8) {
    if let Err(err) = storage::set(key, &[value]) {
        log::write(
            log::Level::Warn,
            &format!("STORAGE/ERROR | {key}: {err}"),
        );
    }
}

struct Scheduled {
    /// Local observation sequence, never represented as the engine's entity ID.
    observation: u64,
    origin: [f64; 3],
    dimension: String,
    estimated_tick: u64,
    seed: u64,
}

struct Alive {
    enabled: bool,
    config: Config,
    active: Vec<Blast>,
    last_trigger: Option<u64>,
    last_tick: Option<u64>,
    last_clock: Option<u64>,
    dimension: Option<String>,
    detector: Detector,
    detection_enabled: bool,
    automatic_visuals: bool,
    debug: bool,
    scheduled: Vec<Scheduled>,
    diag: Diagnostics,
}

impl Alive {
    fn load() -> Self {
        Self {
            enabled: stored(ENABLED, 1) != 0,
            config: Config {
                quality: Quality::from_byte(stored(QUALITY, 1)),
                scale: f64::from(stored(SCALE, 10).clamp(5, 20)) / 10.0,
                flash: stored(FLASH, 1) != 0,
                smoke: stored(SMOKE, 1) != 0,
            },
            active: Vec::with_capacity(MAX_ACTIVE),
            last_trigger: None,
            last_tick: None,
            last_clock: None,
            dimension: None,
            detector: Detector::default(),
            detection_enabled: stored(DETECTION, 1) != 0,
            // OFF remains OFF if the player intentionally disabled it in v0.7.
            automatic_visuals: stored(AUTO_FX, 1) != 0,
            debug: stored(DEBUG, 0) != 0,
            scheduled: Vec::with_capacity(MAX_SCHEDULED),
            diag: Diagnostics::default(),
        }
    }

    fn trace(&mut self, stage: &'static str, details: &str) {
        let message = self.diag.note(stage, details);
        log::write(log::Level::Info, &message);
    }

    fn clear_pending(&mut self, cause: &str) {
        let dropped = self.active.len() + self.scheduled.len();
        if dropped > 0 {
            self.diag.cancelled = self.diag.cancelled.saturating_add(dropped as u64);
            self.trace("CANCELLED", &format!("{dropped} effects: {cause}"));
        }
        self.active.clear();
        self.scheduled.clear();
        self.detector.reset();
    }

    fn settings(&self) {
        ui::set_settings(&[
            ui::Widget::Text(ui::TextWidget {
                id: 10,
                text: "TNT: detection locale probable seulement. Les commandes ne confirment pas le rendu.".into(),
                color: 0xB7DDE8,
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 1,
                label: format!("Alive: {}", if self.enabled { "ON" } else { "OFF" }),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 2,
                label: format!("Qualite: {}", self.config.quality.name()),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 3,
                label: format!("Echelle visuelle: {:.1}x", self.config.scale),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 4,
                label: format!("Fumee: {}", if self.config.smoke { "ON" } else { "OFF" }),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 5,
                label: format!("Flash: {}", if self.config.flash { "ON" } else { "OFF" }),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 6,
                label: format!("Detection probable: {}", if self.detection_enabled { "ON" } else { "OFF" }),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 7,
                label: format!("Animation estimee: {}", if self.automatic_visuals { "ON" } else { "OFF" }),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 8,
                label: format!("Logs detailles: {}", if self.debug { "ON" } else { "OFF" }),
            }),
        ]);
    }

    fn status(&self) -> String {
        label(&format!(
            "v0.8 | {} | detection {} | animation estimee {} | {} effets | {} attentes | {} observations",
            if self.enabled { "actif" } else { "inactif" },
            if self.detection_enabled { "ON" } else { "OFF" },
            if self.automatic_visuals { "ON" } else { "OFF" },
            self.active.len(),
            self.scheduled.len(),
            self.diag.candidates
        ))
    }

    fn detect_tick(&mut self, tick: u64, player: &game::Player) {
        if !self.detection_enabled {
            self.detector.reset();
            return;
        }

        let aimed_tnt = game::raycast(8.0).and_then(|hit| {
            (hit.block == "minecraft:tnt").then_some([hit.block_x, hit.block_y, hit.block_z])
        });
        let observation = Observation {
            tick,
            dimension: player.dimension.clone(),
            feet: [player.x, player.y, player.z],
            yaw: player.yaw,
            pitch: player.pitch,
            aimed_tnt,
            held_item: input::held().map(|item| item.id),
            use_down: input::mapping("key.use").is_some_and(|mapping| mapping.down),
        };
        if let Some(event) = self.detector.observe(observation) {
            self.diag.candidates = self.diag.candidates.saturating_add(1);
            let event_id = self.diag.candidates;
            let position = point(event.pos);
            let message = format!(
                "TNT | Amorçage probable #{event_id} | {position} | non confirme"
            );
            // chat.show already prefixes the mod name; do not add a second one.
            chat::show(&message);
            self.trace("PRIMED_ESTIMATE", &format!(
                "observation #{event_id}; {position}; dimension {}; fuse 80 ticks suppose",
                event.dimension
            ));
            if self.automatic_visuals {
                let origin = worldfx::anchor_from_block(
                    event.pos[0], event.pos[1], event.pos[2]
                );
                if self.scheduled.len() == MAX_SCHEDULED {
                    self.scheduled.remove(0);
                    self.diag.cancelled = self.diag.cancelled.saturating_add(1);
                    self.trace("DROPPED", "scheduled queue capacity reached");
                }
                self.scheduled.push(Scheduled {
                    observation: event_id,
                    origin,
                    dimension: event.dimension,
                    estimated_tick: event.estimated_detonation_tick,
                    seed: tick ^ origin[0].to_bits() ^ origin[2].to_bits() ^ event_id,
                });
                self.diag.scheduled = self.diag.scheduled.saturating_add(1);
                self.trace("SCHEDULED", &format!(
                    "observation #{event_id}; estimated tick {}",
                    event.estimated_detonation_tick
                ));
            } else if self.debug {
                self.trace("IGNORED", "auto FX is disabled by stored user preference");
            }
        }
    }

    fn schedule_tick(&mut self, tick: u64, dimension: &str) {
        let mut waiting = Vec::with_capacity(MAX_SCHEDULED);
        let mut ready = Vec::new();
        let mut cancelled = 0_u64;
        for event in self.scheduled.drain(..) {
            if event.dimension != dimension || tick > event.estimated_tick.saturating_add(5) {
                cancelled = cancelled.saturating_add(1);
            } else if tick >= event.estimated_tick {
                ready.push(event);
            } else {
                waiting.push(event);
            }
        }
        self.scheduled = waiting;
        if cancelled != 0 {
            self.diag.cancelled = self.diag.cancelled.saturating_add(cancelled);
            self.trace("CANCELLED", &format!("{cancelled} stale scheduled effects"));
        }

        for event in ready {
            if self.active.len() == MAX_ACTIVE {
                self.active.remove(0);
                self.diag.cancelled = self.diag.cancelled.saturating_add(1);
            }
            self.diag.started = self.diag.started.saturating_add(1);
            self.trace("DETONATION_ESTIMATE", &format!(
                "observation #{}; at tick {tick}; position {:.2} {:.2} {:.2} (NOT confirmed)",
                event.observation, event.origin[0], event.origin[1], event.origin[2]
            ));
            self.active.push(Blast {
                origin: event.origin,
                dimension: event.dimension,
                start_tick: tick,
                seed: event.seed,
            });
            self.trace("FX_STARTED", &format!(
                "observation #{}; world-space vanilla particles queued for rendering",
                event.observation
            ));
        }
    }

    fn begin(&mut self, with_tnt: bool) -> Result<Option<String>, String> {
        if !self.enabled { return Err(label("Activer le mod avec /alive on")); }
        let player = game::get_player().ok_or_else(|| label("Ouvrir un monde"))?;
        let clock = game::get_clock().ok_or_else(|| label("Horloge du monde indisponible"))?;
        if self.last_trigger.is_some_and(|last| {
            clock.game_time >= last && clock.game_time - last < COOLDOWN_TICKS
        }) {
            return Err(label("Attendre la fin du delai entre deux apercus"));
        }
        let hit = game::raycast(12.0).ok_or_else(|| label("Viser un bloc a moins de 12 blocs"))?;
        let origin = worldfx::anchor_from_block(hit.block_x, hit.block_y, hit.block_z);

        if with_tnt {
            let cmd = worldfx::summon_tnt(origin).ok_or_else(|| label("Coordonnees invalides"))?;
            // The engine applies normal command permissions. No result is
            // provided to .hmod, so we cannot confirm successful summoning.
            command::run(&cmd);
            self.trace("SUMMON_REQUEST", "one vanilla TNT requested; server response unavailable");
        }

        if self.active.len() == MAX_ACTIVE {
            self.active.remove(0);
            self.diag.cancelled = self.diag.cancelled.saturating_add(1);
        }
        self.active.push(Blast {
            origin,
            start_tick: clock.game_time,
            dimension: player.dimension.clone(),
            seed: clock.game_time ^ origin[0].to_bits() ^ origin[2].to_bits(),
        });
        self.diag.started = self.diag.started.saturating_add(1);
        self.trace("FX_STARTED", "manual preview requested; display unconfirmed");
        self.last_trigger = Some(clock.game_time);
        self.dimension = Some(player.dimension);

        Ok(Some(label(if with_tnt {
            "Commande TNT demandee + apercu 3D. Invocation non confirmee (permissions joueur)"
        } else {
            "Apercu 3D demande, sans destruction. Utiliser /alive diag pour les compteurs"
        })))
    }

    fn configure(&mut self, key: &str, value: &str) -> Result<Option<String>, String> {
        match key {
            "quality" => {
                self.config.quality = match value {
                    "low" => Quality::Low,
                    "balanced" => Quality::Balanced,
                    "high" => Quality::High,
                    _ => return Err(label("Usage : /alive quality low|balanced|high")),
                };
                save(QUALITY, self.config.quality.as_byte());
            }
            "scale" => {
                let size: f64 = value.parse().map_err(|_| label("Usage : /alive scale 0.5..2.0"))?;
                if !size.is_finite() || !(0.5..=2.0).contains(&size) {
                    return Err(label("Usage : /alive scale 0.5..2.0"));
                }
                self.config.scale = (size * 10.0).round() / 10.0;
                save(SCALE, (self.config.scale * 10.0) as u8);
            }
            "smoke" => {
                self.config.smoke = match value {
                    "on" => true,
                    "off" => false,
                    _ => return Err(label("Usage : /alive smoke on|off")),
                };
                save(SMOKE, u8::from(self.config.smoke));
            }
            "flash" => {
                self.config.flash = match value {
                    "on" => true,
                    "off" => false,
                    _ => return Err(label("Usage : /alive flash on|off")),
                };
                save(FLASH, u8::from(self.config.flash));
            }
            _ => return Err(label("Parametre inconnu. Utiliser /alive help")),
        }
        self.settings();
        Ok(Some(label("Preferences enregistrees")))
    }
}

impl Mod for Alive {
    fn init() -> Result<(Self, Registration), String> {
        render::set_hud(&[]);
        log::write(
            log::Level::Info,
            "MOD/INIT | Horizon Alive v0.8 charge. Aucun evenement TNT confirme n'est expose par le SDK.",
        );
        Ok((Self::load(), Registration::new().command("alive").on_tick()))
    }

    fn tick(&mut self, tick: u64) {
        if self.last_tick == Some(tick) { return; }
        self.last_tick = Some(tick);

        let Some(player) = game::get_player() else {
            self.clear_pending("world closed");
            self.dimension = None;
            self.last_trigger = None;
            self.last_clock = None;
            return;
        };
        let Some(clock) = game::get_clock() else {
            self.clear_pending("world clock unavailable");
            self.last_clock = None;
            return;
        };
        let reset = self.dimension.as_deref() != Some(player.dimension.as_str())
            || self.last_clock.is_some_and(|previous| clock.game_time < previous);
        if reset {
            self.clear_pending("dimension changed or world time rewound");
            self.last_trigger = None;
        }
        self.dimension = Some(player.dimension.clone());
        self.last_clock = Some(clock.game_time);

        if !self.enabled {
            self.clear_pending("mod disabled");
            return;
        }
        self.detect_tick(clock.game_time, &player);
        self.schedule_tick(clock.game_time, &player.dimension);

        let before = self.active.len();
        self.active.retain(|blast| {
            blast.dimension == player.dimension
                && clock.game_time >= blast.start_tick
                && clock.game_time - blast.start_tick < LIFE_TICKS
        });
        let done = before - self.active.len();
        if done > 0 {
            self.diag.finished = self.diag.finished.saturating_add(done as u64);
            self.trace("FX_FINISHED", &format!("{done} animation(s) expired"));
        }
        if self.active.is_empty() { return; }

        let frames: Vec<_> = self.active.iter().map(|blast| {
            worldfx::frame(blast, clock.game_time - blast.start_tick, &self.config)
        }).collect();
        let commands = worldfx::interleave(&frames);
        if !commands.is_empty() {
            let count = commands.len();
            command::run_quiet(&commands);
            self.diag.batches = self.diag.batches.saturating_add(1);
            self.diag.submitted_commands = self.diag.submitted_commands.saturating_add(count as u64);
            if self.debug && (clock.game_time % 10 == 0 || self.diag.batches == 1) {
                self.trace("FX_COMMANDS_SUBMITTED", &format!(
                    "{count} commands; submitted total {}; no renderer acknowledgement",
                    self.diag.submitted_commands
                ));
            }
        }
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "alive" { return Err(label("Commande inconnue")); }
        match args.trim() {
            "" | "help" => Ok(Some(label(
                "status|diag|last|tnt|preview|on|off|detect on|off|fx on|off|debug on|off|quality|scale|smoke|flash"
            ))),
            "status" => Ok(Some(self.status())),
            "diag" | "diagnostics" => Ok(Some(label(&self.diag.report()))),
            "diag last" => Ok(Some(label(if self.diag.last.is_empty() {
                "Aucun evenement depuis le chargement"
            } else { &self.diag.last }))),
            "on" => {
                self.enabled = true;
                save(ENABLED, 1);
                self.settings();
                Ok(Some(label("Mod active")))
            }
            "off" => {
                self.enabled = false;
                self.clear_pending("user command");
                save(ENABLED, 0);
                self.settings();
                Ok(Some(label("Mod desactive. Effets temporaires annules")))
            }
            "tnt" => self.begin(true),
            "preview" => self.begin(false),
            "detect on" => {
                self.detection_enabled = true;
                save(DETECTION, 1);
                self.settings();
                Ok(Some(label("Detection locale probable activee")))
            }
            "detect off" => {
                self.detection_enabled = false;
                self.detector.reset();
                let dropped = self.scheduled.len();
                self.scheduled.clear();
                self.diag.cancelled = self.diag.cancelled.saturating_add(dropped as u64);
                save(DETECTION, 0);
                self.settings();
                Ok(Some(label("Detection desactivee; attentes annulees")))
            }
            "detect fx on" => {
                self.detection_enabled = true;
                self.automatic_visuals = true;
                save(DETECTION, 1);
                save(AUTO_FX, 1);
                self.settings();
                Ok(Some(label("Animation 3D activee sur detonations estimees (non confirmees)")))
            }
            "detect fx off" => {
                self.automatic_visuals = false;
                self.diag.cancelled = self.diag.cancelled.saturating_add(self.scheduled.len() as u64);
                self.scheduled.clear();
                save(AUTO_FX, 0);
                self.settings();
                Ok(Some(label("Animation automatique desactivee; detection conservee")))
            }
            "detect last" => Ok(Some(label(&self.detector.last().map_or_else(
                || "Aucun amorcage probable observe".into(),
                |event| format!(
                    "Dernier amorcage probable : {} | {} | tick estime {}",
                    point(event.pos), event.dimension, event.estimated_detonation_tick
                ),
            )))),
            "debug on" => {
                self.debug = true;
                save(DEBUG, 1);
                self.settings();
                Ok(Some(label("Logs TNT detailles actives")))
            }
            "debug off" => {
                self.debug = false;
                save(DEBUG, 0);
                self.settings();
                Ok(Some(label("Logs TNT detailles desactives")))
            }
            "graphics off" => {
                render::set_hud(&[]);
                Ok(Some(label("Aucun effet HUD actif")))
            }
            other => {
                if let Some((key, value)) = other.split_once(' ') {
                    self.configure(key, value)
                } else {
                    Err(label("Usage : /alive help"))
                }
            }
        }
    }

    fn ui_settings_opened(&mut self) {
        self.settings();
    }

    fn ui_select(&mut self, id: u32, _row: u32) {
        match id {
            1 => {
                self.enabled = !self.enabled;
                if !self.enabled { self.clear_pending("settings disabled"); }
                save(ENABLED, u8::from(self.enabled));
            }
            2 => {
                self.config.quality = self.config.quality.next();
                save(QUALITY, self.config.quality.as_byte());
            }
            3 => {
                self.config.scale = if self.config.scale >= 2.0 {
                    0.5
                } else {
                    (self.config.scale + 0.5).min(2.0)
                };
                save(SCALE, (self.config.scale * 10.0) as u8);
            }
            4 => {
                self.config.smoke = !self.config.smoke;
                save(SMOKE, u8::from(self.config.smoke));
            }
            5 => {
                self.config.flash = !self.config.flash;
                save(FLASH, u8::from(self.config.flash));
            }
            6 => {
                self.detection_enabled = !self.detection_enabled;
                if !self.detection_enabled {
                    self.detector.reset();
                    self.diag.cancelled = self.diag.cancelled.saturating_add(self.scheduled.len() as u64);
                    self.scheduled.clear();
                }
                save(DETECTION, u8::from(self.detection_enabled));
            }
            7 => {
                self.automatic_visuals = !self.automatic_visuals;
                if self.automatic_visuals {
                    self.detection_enabled = true;
                    save(DETECTION, 1);
                } else {
                    self.diag.cancelled = self.diag.cancelled.saturating_add(self.scheduled.len() as u64);
                    self.scheduled.clear();
                }
                save(AUTO_FX, u8::from(self.automatic_visuals));
            }
            8 => {
                self.debug = !self.debug;
                save(DEBUG, u8::from(self.debug));
            }
            _ => return,
        }
        self.settings();
    }
}

horizon_mod_sdk::export_mod!(Alive);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_rotation() {
        assert_eq!(Quality::High.next(), Quality::Low);
    }

    #[test]
    fn stored_scale_bounds() {
        assert!((5..=20).contains(&10_u8.clamp(5, 20)));
    }

    #[test]
    fn scheduled_limit_is_small() {
        assert_eq!(MAX_SCHEDULED, 8);
        assert!(MAX_ACTIVE <= MAX_SCHEDULED);
    }
}
