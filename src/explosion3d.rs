//! World-space TNT explosion **geometry model**, independent from Horizon's sandbox.
//!
//! This module is pure Rust and unit-tested. It deliberately does NOT render anything
//! in a .hmod: Horizon 0.1 has no world-geometry or explosion-event interface.
//! A version-pinned native renderer must consume GroundTile/Voxel/SmokeStripe data.
use std::f32::consts::TAU;

pub const MAX_ACTIVE_EXPLOSIONS: usize = 4;
pub const MAX_DRAW_PRIMITIVES: usize = 2_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Low,
    Medium,
    High,
}

impl Quality {
    pub const fn budget(self) -> usize {
        match self {
            Self::Low => 350,
            Self::Medium => 900,
            Self::High => MAX_DRAW_PRIMITIVES,
        }
    }
    pub const fn grid_step(self) -> f32 {
        match self {
            Self::Low => 0.65,
            Self::Medium => 0.44,
            Self::High => 0.30,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Ignition,
    Flash,
    Expansion,
    Fireball,
    Dissipation,
    Finished,
}

pub fn phase(progress: f32) -> Phase {
    if !progress.is_finite() || progress >= 1.0 {
        Phase::Finished
    } else if progress < 0.035 {
        Phase::Ignition
    } else if progress < 0.10 {
        Phase::Flash
    } else if progress < 0.35 {
        Phase::Expansion
    } else if progress < 0.62 {
        Phase::Fireball
    } else {
        Phase::Dissipation
    }
}

pub fn smoothstep(t: f32) -> f32 {
    let x = t.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn eased_out(t: f32) -> f32 {
    let x = t.clamp(0.0, 1.0);
    1.0 - (1.0 - x).powi(3)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitiveKind {
    /// Horizontal XZ-plane tile; must be depth-tested and terrain-height fitted by native renderer.
    GroundTile,
    /// Opaque voxel volume cell, in world coordinates rather than screen coordinates.
    FireVoxel,
    /// Horizontal white/gray smoke band segment, world-space.
    SmokeStripe,
    /// A few remaining orange burning fragments.
    Ember,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Primitive {
    /// Offsets in blocks from the 3D event origin, never GUI pixels.
    pub offset: [f32; 3],
    pub size: f32,
    pub argb: u32,
    pub kind: PrimitiveKind,
}

#[derive(Clone, Copy, Debug)]
pub struct ExplosionConfig {
    pub visual_radius: f32,
    pub gameplay_radius: f32,
    pub duration_seconds: f32,
    pub quality: Quality,
}

impl Default for ExplosionConfig {
    fn default() -> Self {
        Self {
            visual_radius: 6.5,
            gameplay_radius: 4.0,
            duration_seconds: 1.10,
            quality: Quality::Medium,
        }
    }
}

impl ExplosionConfig {
    pub fn validated(self) -> Self {
        let defaults = Self::default();
        Self {
            visual_radius: if self.visual_radius.is_finite() {
                self.visual_radius.clamp(1.0, 12.0)
            } else {
                defaults.visual_radius
            },
            gameplay_radius: if self.gameplay_radius.is_finite() {
                self.gameplay_radius.max(0.0)
            } else {
                defaults.gameplay_radius
            },
            duration_seconds: if self.duration_seconds.is_finite() {
                self.duration_seconds.clamp(0.25, 4.0)
            } else {
                defaults.duration_seconds
            },
            quality: self.quality,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExplosionInstance {
    pub id: u64,
    pub origin: [f64; 3],
    pub started_seconds: f64,
    pub seed: u64,
    pub config: ExplosionConfig,
}

#[derive(Debug)]
pub struct ExplosionFrame {
    pub id: u64,
    pub origin: [f64; 3],
    pub progress: f32,
    pub phase: Phase,
    pub primitives: Vec<Primitive>,
}

fn hash(seed: u64, x: i32, y: i32, z: i32) -> u64 {
    let mut h = seed
        ^ (x as u32 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u32 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (z as u32 as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}

fn unit(seed: u64, a: i32, b: i32, c: i32) -> f32 {
    ((hash(seed, a, b, c) >> 40) as f32) / 16_777_216.0
}

fn rgba(rgb: u32, alpha: f32) -> u32 {
    let a = (alpha.clamp(0.0, 1.0) * 255.0) as u32;
    (a << 24) | (rgb & 0x00FF_FFFF)
}

fn push(out: &mut Vec<Primitive>, limit: usize, item: Primitive) {
    if out.len() < limit
        && item.offset.iter().all(|v| v.is_finite())
        && item.size.is_finite()
        && item.size > 0.0
        && item.argb >> 24 != 0
    {
        out.push(item);
    }
}

impl ExplosionInstance {
    /// The renderer must supply a camera-aware depth test and terrain height mapping.
    /// Does not mutate the world, allocate GPU resources, or trigger a second explosion.
    pub fn sample(&self, now_seconds: f64) -> ExplosionFrame {
        self.sample_terrain(now_seconds, &|_, _| Some(0.0))
    }

    /// `ground_height(x, z)` gives relative terrain height in blocks for a
    /// world-space XZ position, or None if unloaded/invisible. The native
    /// renderer should provide this callback; default `sample` is flat.
    pub fn sample_terrain(
        &self,
        now_seconds: f64,
        ground_height: &impl Fn(f32, f32) -> Option<f32>,
    ) -> ExplosionFrame {
        let config = self.config.validated();
        let elapsed = now_seconds - self.started_seconds;
        let progress = if elapsed.is_finite() {
            (elapsed / f64::from(config.duration_seconds)).clamp(0.0, 1.0) as f32
        } else {
            1.0
        };
        let stage = phase(progress);
        let mut out = Vec::with_capacity(config.quality.budget());
        if stage == Phase::Finished {
            return ExplosionFrame {
                id: self.id,
                origin: self.origin,
                progress,
                phase: stage,
                primitives: out,
            };
        }
        let budget = config.quality.budget();
        let r = config.visual_radius;

        // A fast, square-edged ground disc, clipped to natural terrain.
        if (0.06..0.69).contains(&progress) {
            let amount = eased_out((progress - 0.06) / 0.40);
            let radius = r * amount;
            let fade = 1.0 - smoothstep((progress - 0.37) / 0.32);
            let step = config.quality.grid_step();
            let extent = (radius / step).ceil() as i32;
            let wave_budget = budget * 60 / 100;
            for zi in -extent..=extent {
                for xi in -extent..=extent {
                    if out.len() >= wave_budget {
                        break;
                    }
                    let x = xi as f32 * step;
                    let z = zi as f32 * step;
                    let d = x.hypot(z);
                    // Angular discontinuities + fixed grid produce a pixelated outline.
                    let distort = 0.94
                        + unit(self.seed, xi / 3, 0, zi / 3) * 0.12;
                    let edge = radius * distort;
                    if d > edge {
                        continue;
                    }
                    let fraction = d / edge.max(0.001);
                    let color = if fraction > 0.82 {
                        0xFF6500
                    } else if fraction > 0.66 {
                        0xFF9B00
                    } else if fraction > 0.47 {
                        0xFFD000
                    } else if fraction > 0.28 {
                        0xFFF36A
                    } else {
                        0xFFFDE0
                    };
                    let Some(y) = ground_height(x, z) else { continue };
                    if !y.is_finite() {
                        continue;
                    }
                    push(&mut out, wave_budget, Primitive {
                        offset: [x, y + 0.035, z],
                        size: step * 0.98,
                        argb: rgba(color, fade * 0.94),
                        kind: PrimitiveKind::GroundTile,
                    });
                }
            }
        }

        // A voxelised, layered, volume-preserving dome (not a camera HUD sprite).
        // The bright core peaks earlier; orange shells linger at the edge.
        if (0.025..0.73).contains(&progress) {
            let expansion = eased_out((progress - 0.025) / 0.32);
            let fade = 1.0 - smoothstep((progress - 0.52) / 0.21);
            let size = r * (0.18 + 0.30 * expansion);
            let voxel = config.quality.grid_step() * 1.25;
            let extent = (size / voxel).ceil() as i32;
            let fire_limit = budget * 90 / 100;
            for yi in -extent..=extent {
                for zi in -extent..=extent {
                    for xi in -extent..=extent {
                        if out.len() >= fire_limit {
                            break;
                        }
                        let x = xi as f32 * voxel;
                        let y = yi as f32 * voxel + size * 0.28;
                        let z = zi as f32 * voxel;
                        if y < 0.0 { continue; }
                        let d = (x * x + (y - size * 0.28).powi(2) + z * z).sqrt();
                        let boundary = size * (0.88 + unit(self.seed ^ 11, xi, yi, zi) * 0.2);
                        if d > boundary { continue; }
                        if unit(self.seed ^ 17, xi, yi, zi) > 0.68 { continue; }
                        let fraction = d / boundary.max(0.01);
                        let color = if fraction < 0.30 && progress < 0.43 {
                            0xFFFDE0
                        } else if fraction < 0.48 {
                            0xFFF36A
                        } else if fraction < 0.68 {
                            0xFFD000
                        } else if fraction < 0.83 {
                            0xFF9B00
                        } else {
                            0xFF6500
                        };
                        push(&mut out, fire_limit, Primitive {
                            offset: [x, y, z],
                            size: voxel * 0.94,
                            argb: rgba(color, fade),
                            kind: PrimitiveKind::FireVoxel,
                        });
                    }
                }
            }
        }

        // Whiter, irregular horizontal smoke ribbons with residual orange embers.
        if progress > 0.42 {
            let u = ((progress - 0.42) / 0.58).clamp(0.0, 1.0);
            let fade = 1.0 - smoothstep((u - 0.35) / 0.65);
            let strands = match config.quality {
                Quality::Low => 5,
                Quality::Medium => 9,
                Quality::High => 13,
            };
            for strand in 0..strands {
                let angle = TAU * strand as f32 / strands as f32
                    + unit(self.seed, strand, 7, 3) * 0.7;
                let width = r * (0.25 + 0.65 * u);
                let steps = 8 + config.quality.budget() / 200;
                let level = 0.4 + strand as f32 * 0.24 + u * r * 0.35;
                for part in 0..steps {
                    let d = width * part as f32 / steps as f32;
                    let jitter = unit(self.seed, part as i32, strand, 13);
                    let side = (jitter - 0.5) * 0.35;
                    let x = angle.cos() * d - angle.sin() * side;
                    let z = angle.sin() * d + angle.cos() * side;
                    let color = if jitter < 0.38 { 0xFFFFFF } else if jitter < 0.78 {
                        0xDDDAD1
                    } else {
                        0xB8B5AF
                    };
                    push(&mut out, budget, Primitive {
                        offset: [x, level + (jitter - 0.5) * 0.36, z],
                        size: config.quality.grid_step() * (1.2 + jitter),
                        argb: rgba(color, fade * 0.84),
                        kind: PrimitiveKind::SmokeStripe,
                    });
                }
            }
            for part in 0..8 {
                let angle = TAU * unit(self.seed, part, 61, 0);
                let distance = r * (0.2 + 0.4 * u);
                push(&mut out, budget, Primitive {
                    offset: [
                        angle.cos() * distance,
                        0.8 + unit(self.seed, part, 62, 0) * r * 0.6,
                        angle.sin() * distance,
                    ],
                    size: config.quality.grid_step() * 1.5,
                    argb: rgba(0xFF8D26, fade),
                    kind: PrimitiveKind::Ember,
                });
            }
        }
        ExplosionFrame {
            id: self.id,
            origin: self.origin,
            progress,
            phase: stage,
            primitives: out,
        }
    }
}

#[derive(Default)]
pub struct ExplosionManager {
    active: Vec<ExplosionInstance>,
    next_id: u64,
}

impl ExplosionManager {
    pub fn spawn(
        &mut self,
        origin: [f64; 3],
        now_seconds: f64,
        seed: u64,
        config: ExplosionConfig,
    ) -> Result<u64, &'static str> {
        if !now_seconds.is_finite() || !origin.iter().all(|v| v.is_finite()) {
            return Err("invalid explosion position/time");
        }
        if self.active.len() == MAX_ACTIVE_EXPLOSIONS {
            self.active.remove(0);
        }
        self.next_id = self.next_id.wrapping_add(1);
        let id = self.next_id;
        self.active.push(ExplosionInstance {
            id,
            origin,
            started_seconds: now_seconds,
            seed,
            config: config.validated(),
        });
        Ok(id)
    }

    pub fn len(&self) -> usize { self.active.len() }

    pub fn cleanup(&mut self, now_seconds: f64) {
        self.active.retain(|e| {
            now_seconds.is_finite()
                && now_seconds >= e.started_seconds
                && now_seconds - e.started_seconds < f64::from(e.config.duration_seconds)
        });
    }

    pub fn frames(&mut self, now_seconds: f64) -> Vec<ExplosionFrame> {
        self.cleanup(now_seconds);
        self.active.iter().map(|e| e.sample(now_seconds)).collect()
    }

    pub fn clear(&mut self) { self.active.clear(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> ExplosionInstance {
        ExplosionInstance { id: 1, origin: [4.0, 68.0, -8.0], started_seconds: 100.0, seed: 12, config: ExplosionConfig::default() }
    }

    #[test]
    fn finite_phases_and_finish() {
        assert_eq!(phase(0.0), Phase::Ignition);
        assert_eq!(phase(0.04), Phase::Flash);
        assert_eq!(phase(0.2), Phase::Expansion);
        assert_eq!(phase(0.5), Phase::Fireball);
        assert_eq!(phase(0.8), Phase::Dissipation);
        assert_eq!(phase(1.0), Phase::Finished);
        assert_eq!(phase(f32::NAN), Phase::Finished);
        assert!(sample().sample(102.0).primitives.is_empty());
    }

    #[test]
    fn interpolation_clamped() {
        for t in [-4.0, 0.0, 0.1, 0.5, 1.0, 4.0] {
            assert!((0.0..=1.0).contains(&smoothstep(t)));
            assert!((0.0..=1.0).contains(&eased_out(t)));
        }
    }

    #[test]
    fn bounded_across_quality_profiles_and_times() {
        for quality in [Quality::Low, Quality::Medium, Quality::High] {
            let mut e = sample();
            e.config.quality = quality;
            for frame in 0..25 {
                let f = e.sample(100.0 + frame as f64 * 0.05);
                assert!(f.primitives.len() <= quality.budget());
                assert!(f.primitives.iter().all(|p| p.offset.iter().all(|v| v.is_finite())));
                assert!(f.primitives.iter().all(|p| p.size > 0.0));
            }
        }
    }

    #[test]
    fn deterministic() {
        let e = sample();
        let a = e.sample(100.33);
        let b = e.sample(100.33);
        assert_eq!(a.primitives, b.primitives);
    }

    #[test]
    fn world_position_and_gameplay_radius_stay_separate() {
        let e = sample();
        let f = e.sample(100.25);
        assert_eq!(f.origin, [4.0, 68.0, -8.0]);
        assert_eq!(e.config.gameplay_radius, 4.0);
        assert!(f.primitives.iter().any(|p| p.kind == PrimitiveKind::GroundTile));
        assert!(f.primitives.iter().any(|p| p.kind == PrimitiveKind::FireVoxel));
    }

    #[test]
    fn terrain_height_applied_only_to_ground_tiles() {
        let e = sample();
        let f = e.sample_terrain(100.29, &|x, z| Some(x * 0.1 + z * 0.2));
        for p in f.primitives.iter().filter(|p| p.kind == PrimitiveKind::GroundTile) {
            assert!((p.offset[1] - (p.offset[0] * 0.1 + p.offset[2] * 0.2 + 0.035)).abs() < 0.0001);
        }
    }

    #[test]
    fn smoke_and_embers_at_late_time() {
        let f = sample().sample(100.85);
        assert!(f.primitives.iter().any(|p| p.kind == PrimitiveKind::SmokeStripe));
        assert!(f.primitives.iter().any(|p| p.kind == PrimitiveKind::Ember));
        assert!(!f.primitives.iter().any(|p| p.kind == PrimitiveKind::GroundTile));
    }

    #[test]
    fn manager_bounded_and_cleanup() {
        let mut m = ExplosionManager::default();
        for i in 0..50 {
            m.spawn([i as f64, 70.0, 0.0], 10.0, i, ExplosionConfig::default()).unwrap();
        }
        assert_eq!(m.len(), MAX_ACTIVE_EXPLOSIONS);
        assert_eq!(m.frames(12.0).len(), 0);
        assert_eq!(m.len(), 0);
        assert!(m.spawn([f64::NAN, 1.0, 2.0], 1.0, 0, ExplosionConfig::default()).is_err());
    }

    #[test]
    fn validated_config_handles_nonfinite_values() {
        let c = ExplosionConfig {
            visual_radius: f32::NAN,
            gameplay_radius: f32::INFINITY,
            duration_seconds: -100.0,
            quality: Quality::High,
        }.validated();
        assert_eq!(c.visual_radius, ExplosionConfig::default().visual_radius);
        assert_eq!(c.gameplay_radius, ExplosionConfig::default().gameplay_radius);
        assert_eq!(c.duration_seconds, 0.25);
    }
}
