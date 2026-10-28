//! Horizon Alive v0.9.1 — sandboxed .hmod, real XYZ vanilla particle commands.
//!
//! The SDK API 0.1 has no prime_fuse or detonated event. Inferred player
//! ignitions are labelled as estimates, not confirmed engine TNT events.
mod detector;
mod diagnostics;
mod presentation;
mod trajectory;
mod worldfx;

use detector::{Detector, Observation};
use diagnostics::{Diagnostics, point};
use presentation::{caution, error, label, success};
use trajectory::TntMotion;
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
const FOLLOW: &str = "alive.follow";
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
    observed_tick: u64,
    motion: TntMotion,
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
    estimated_follow: bool,
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
            estimated_follow: stored(FOLLOW, 1) != 0,
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
        let state = |on| if on { "ACTIF" } else { "INACTIF" };
        ui::set_settings(&[
            ui::Widget::Text(ui::TextWidget {
                id: 10,
                text: "HORIZON ALIVE  •  WORLD FX".into(),
                color: 0xFFB454,
            }),
            ui::Widget::Text(ui::TextWidget {
                id: 11,
                text: "TNT locale probable  ·  rendu vanilla 3D".into(),
                color: 0xA6B4C7,
            }),
            ui::Widget::Text(ui::TextWidget {
                id: 12,
                text: "— EFFETS VISUELS —".into(),
                color: 0x70DBEA,
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 1,
                label: format!("Horizon Alive : {}", state(self.enabled)),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 2,
                label: format!(
                    "Qualité : {}",
                    match self.config.quality {
                        Quality::Low => "Faible",
                        Quality::Balanced => "Équilibrée",
                        Quality::High => "Élevée",
                    },
                ),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 3,
                label: format!("Échelle : {:.1}×", self.config.scale),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 4,
                label: format!("Fumée : {}", state(self.config.smoke)),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 5,
                label: format!("Flash : {}", state(self.config.flash)),
            }),
            ui::Widget::Text(ui::TextWidget {
                id: 13,
                text: "— DÉTECTION & POSITION —".into(),
                color: 0x70DBEA,
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 6,
                label: format!("Détection TNT : {}", state(self.detection_enabled)),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 7,
                label: format!("Effet automatique : {}", state(self.automatic_visuals)),
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 9,
                label: format!(
                    "Position TNT : {}",
                    if self.estimated_follow { "Estimation" } else { "Fixe" },
                ),
            }),
            ui::Widget::Text(ui::TextWidget {
                id: 14,
                text: "— DIAGNOSTICS —".into(),
                color: 0x70DBEA,
            }),
            ui::Widget::Button(ui::ButtonWidget {
                id: 8,
                label: format!("Logs détaillés : {}", state(self.debug)),
            }),
            ui::Widget::Text(ui::TextWidget {
                id: 15,
                text: "Les TNT redstone/chaîne ne sont pas observables.".into(),
                color: 0xA6B4C7,
            }),
        ]);
    }

    fn status(&self) -> String {
        presentation::status(
            self.enabled,
            self.detection_enabled,
            self.automatic_visuals,
            self.estimated_follow,
            self.active.len(),
            self.scheduled.len(),
            self.diag.candidates,
        )
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
            let message = presentation::primed(event_id, event.pos);
            // chat.show already prefixes the mod name; do not add a second one.
            chat::show(&message);
            self.trace("PRIMED_ESTIMATE", &format!(
                "observation #{event_id}; {position}; dimension {}; fuse 80 ticks suppose",
                event.dimension
            ));
            if self.automatic_visuals {
                // Actual PrimedTnt is spawned at the block's bottom center.
                // The old origin used block_y + 1, producing a 1-block error.
                let origin = [
                    f64::from(event.pos[0]) + 0.5,
                    f64::from(event.pos[1]),
                    f64::from(event.pos[2]) + 0.5,
                ];
                let seed = tick ^ origin[0].to_bits() ^ origin[2].to_bits() ^ event_id;
                if let Some(motion) = TntMotion::new(origin, Some(origin[1]), seed) {
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
                        observed_tick: tick,
                        seed,
                        motion,
                    });
                    self.diag.scheduled = self.diag.scheduled.saturating_add(1);
                    self.trace("SCHEDULED", &format!(
                        "observation #{event_id}; estimated tick {}; approximate physics, not observed entity coordinates",
                        event.estimated_detonation_tick
                    ));
                }
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
            let position = if self.estimated_follow {
                event.motion.sample(tick.saturating_sub(event.observed_tick)).position
            } else {
                event.origin
            };
            // Center of the explosion sits a little above the TNT entity feet.
            let center = [position[0], position[1] + 0.06125, position[2]];
            self.trace("DETONATION_ESTIMATE", &format!(
                "observation #{}; at tick {tick}; estimated XYZ {:.2} {:.2} {:.2}; trajectory {} (NOT a tracked TNT)",
                event.observation, center[0], center[1], center[2],
                if self.estimated_follow { "predicted" } else { "fixed" }
            ));
            self.active.push(Blast {
                origin: center,
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
        if !self.enabled { return Err(error("Activer le mod avec /alive on")); }
        let player = game::get_player().ok_or_else(|| error("Ouvrir un monde"))?;
        let clock = game::get_clock().ok_or_else(|| error("Horloge du monde indisponible"))?;
        if self.last_trigger.is_some_and(|last| {
            clock.game_time >= last && clock.game_time - last < COOLDOWN_TICKS
        }) {
            return Err(caution("Attendre avant un nouvel aperçu"));
        }
        let hit = game::raycast(12.0).ok_or_else(|| error("Viser un bloc à moins de 12 blocs"))?;
        let origin = worldfx::anchor_from_block(hit.block_x, hit.block_y, hit.block_z);

        if with_tnt {
            let cmd = worldfx::summon_tnt(origin).ok_or_else(|| error("Coordonnées invalides"))?;
            // The command is queued for execution after this hook.
            // Fuse:1s is the one-tick TNT requested by the old command.
            // It is not known whether the game accepts the command.
            command::run(&cmd);
            self.trace("SUMMON_REQUEST", "one vanilla TNT requested; server response unavailable");
            if let Some(motion) = TntMotion::new(origin, Some(origin[1]), clock.game_time) {
                if self.scheduled.len() >= MAX_SCHEDULED {
                    self.scheduled.remove(0);
                    self.diag.cancelled = self.diag.cancelled.saturating_add(1);
                }
                self.scheduled.push(Scheduled {
                    observation: 0,
                    origin,
                    dimension: player.dimension.clone(),
                    estimated_tick: clock.game_time.saturating_add(1),
                    observed_tick: clock.game_time,
                    seed: clock.game_time ^ origin[0].to_bits() ^ origin[2].to_bits(),
                    motion,
                });
                self.diag.scheduled = self.diag.scheduled.saturating_add(1);
                self.trace("SCHEDULED", "commanded TNT fuse 1 tick; success unconfirmed");
            }
        } else {
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
        }
        self.last_trigger = Some(clock.game_time);
        self.dimension = Some(player.dimension);

        Ok(Some(caution(if with_tnt {
            "TNT demandée ; succès non confirmé.
Effet prévu au tick suivant."
        } else {
            "Aperçu 3D demandé, sans dégâts.
/alive diag : vérifier les commandes envoyées."
        })))
    }

    fn configure(&mut self, key: &str, value: &str) -> Result<Option<String>, String> {
        match key {
            "quality" => {
                self.config.quality = match value {
                    "low" => Quality::Low,
                    "balanced" => Quality::Balanced,
                    "high" => Quality::High,
                    _ => return Err(error("Usage : /alive quality low|balanced|high")),
                };
                save(QUALITY, self.config.quality.as_byte());
            }
            "scale" => {
                let size: f64 = value.parse().map_err(|_| error("Usage : /alive scale 0.5..2.0"))?;
                if !size.is_finite() || !(0.5..=2.0).contains(&size) {
                    return Err(error("Usage : /alive scale 0.5..2.0"));
                }
                self.config.scale = (size * 10.0).round() / 10.0;
                save(SCALE, (self.config.scale * 10.0) as u8);
            }
            "smoke" => {
                self.config.smoke = match value {
                    "on" => true,
                    "off" => false,
                    _ => return Err(error("Usage : /alive smoke on|off")),
                };
                save(SMOKE, u8::from(self.config.smoke));
            }
            "flash" => {
                self.config.flash = match value {
                    "on" => true,
                    "off" => false,
                    _ => return Err(error("Usage : /alive flash on|off")),
                };
                save(FLASH, u8::from(self.config.flash));
            }
            _ => return Err(error("Paramètre inconnu. /alive help")),
        }
        self.settings();
        Ok(Some(success("Préférences enregistrées")))
    }
}

impl Mod for Alive {
    fn init() -> Result<(Self, Registration), String> {
        render::set_hud(&[]);
        log::write(
            log::Level::Info,
            "MOD/INIT | Horizon Alive v0.9.1 chargé. Aucun evenement TNT confirme n'est expose par le SDK.",
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
        if name != "alive" { return Err(error("Commande inconnue")); }
        match args.trim() {
            "" | "help" | "help 1" => Ok(presentation::help_page(1)),
            "help 2" => Ok(presentation::help_page(2)),
            "help 3" => Ok(presentation::help_page(3)),
            other if other.starts_with("help ") => {
                Err(error("Pages disponibles : /alive help 1|2|3"))
            },
            "status" => Ok(Some(self.status())),
            "diag" | "diagnostics" => Ok(Some(presentation::diagnostics(&self.diag))),
            "diag last" => Ok(Some(presentation::detail(
                "DERNIÈRE ÉTAPE",
                if self.diag.last.is_empty() {
                    "Aucun événement depuis le chargement"
                } else {
                    &self.diag.last
                },
            ))),
            "on" => {
                self.enabled = true;
                save(ENABLED, 1);
                self.settings();
                Ok(Some(success("Mod activé")))
            }
            "off" => {
                self.enabled = false;
                self.clear_pending("user command");
                save(ENABLED, 0);
                self.settings();
                Ok(Some(caution("Mod désactivé ; effets temporaires annulés")))
            }
            "tnt" => self.begin(true),
            "preview" => self.begin(false),
            "detect on" => {
                self.detection_enabled = true;
                save(DETECTION, 1);
                self.settings();
                Ok(Some(success("Détection locale activée (estimée)")))
            }
            "detect off" => {
                self.detection_enabled = false;
                self.detector.reset();
                let dropped = self.scheduled.len();
                self.scheduled.clear();
                self.diag.cancelled = self.diag.cancelled.saturating_add(dropped as u64);
                save(DETECTION, 0);
                self.settings();
                Ok(Some(caution("Détection désactivée ; attentes annulées")))
            }
            "detect fx on" => {
                self.detection_enabled = true;
                self.automatic_visuals = true;
                save(DETECTION, 1);
                save(AUTO_FX, 1);
                self.settings();
                Ok(Some(caution("Effets activés sur détonations estimées")))
            }
            "detect fx off" => {
                self.automatic_visuals = false;
                self.diag.cancelled = self.diag.cancelled.saturating_add(self.scheduled.len() as u64);
                self.scheduled.clear();
                save(AUTO_FX, 0);
                self.settings();
                Ok(Some(caution("Animation désactivée ; détection conservée")))
            }
            "detect last" => Ok(Some(presentation::detail(
                "DERNIÈRE TNT",
                &self.detector.last().map_or_else(
                    || "Aucun amorçage probable observé".into(),
                    |event| format!(
                        "Amorçage probable : {}
Dimension : {}
Tick estimé : {}",
                        point(event.pos), event.dimension, event.estimated_detonation_tick
                    ),
                ),
            ))),
            "follow estimate" => {
                self.estimated_follow = true;
                save(FOLLOW, 1);
                self.settings();
                Ok(Some(caution("Position estimée, non suivie par le moteur")))
            }
            "follow fixed" => {
                self.estimated_follow = false;
                save(FOLLOW, 0);
                self.settings();
                Ok(Some(success("Position fixée au bloc initial")))
            }
            "debug on" => {
                self.debug = true;
                save(DEBUG, 1);
                self.settings();
                Ok(Some(success("Logs TNT détaillés activés")))
            }
            "debug off" => {
                self.debug = false;
                save(DEBUG, 0);
                self.settings();
                Ok(Some(caution("Logs TNT détaillés désactivés")))
            }
            "graphics off" => {
                render::set_hud(&[]);
                Ok(Some(label("Aucun effet HUD actif")))
            }
            other => {
                if let Some((key, value)) = other.split_once(' ') {
                    self.configure(key, value)
                } else {
                    Err(error("Commande invalide. /alive help"))
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
            9 => {
                self.estimated_follow = !self.estimated_follow;
                save(FOLLOW, u8::from(self.estimated_follow));
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

}
