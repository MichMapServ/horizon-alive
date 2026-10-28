//! Conservative approximation of Horizon's vanilla PrimedTnt position.
//!
//! Engine source: horizon-client/src/game/tnt.rs (tag v0.2.93).
//! Velocity uses secret engine RNG; this model cannot know its real direction.
//! Collision, fluid current, knockback, minecarts, entity ticks and unloaded
//! chunks cannot be observed from the .hmod SDK. Never call it real tracking.
use std::f64::consts::TAU;

const TICK_LIMIT: u32 = 120;
const MAX_COORD: f64 = 29_999_999.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Estimate {
    pub position: [f64; 3],
    // Not a physical error bound (especially in water or during knockback).
    pub nominal_drift: f64,
    pub age_ticks: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct TntMotion {
    initial: [f64; 3],
    floor_y: Option<f64>,
    seed: u64,
}

impl TntMotion {
    /// The optional ground height is a guess, not a terrain observation.
    pub fn new(initial: [f64; 3], floor_y: Option<f64>, seed: u64) -> Option<Self> {
        if !valid(initial) || floor_y.is_some_and(|h| !h.is_finite()) {
            return None;
        }
        Some(Self { initial, floor_y, seed })
    }

    pub fn sample(&self, elapsed_ticks: u64) -> Estimate {
        let ticks = elapsed_ticks.min(u64::from(TICK_LIMIT)) as u32;
        // The real engine's random angle is inaccessible to mods.
        let angle = TAU * f64::from((self.seed ^ (self.seed >> 32)) as u32)
            / f64::from(u32::MAX);
        let mut velocity = [-angle.sin() * 0.02, 0.2, -angle.cos() * 0.02];
        let mut position = self.initial;
        for _ in 0..ticks {
            velocity[1] -= 0.04;
            for i in 0..3 {
                position[i] += velocity[i];
            }
            let mut on_ground = false;
            if let Some(floor) = self.floor_y
                && position[1] < floor
            {
                position[1] = floor;
                velocity[1] = 0.0;
                on_ground = true;
            }
            for v in &mut velocity { *v *= 0.98; }
            if on_ground {
                velocity[0] *= 0.7;
                velocity[1] *= -0.5;
                velocity[2] *= 0.7;
            }
        }
        Estimate {
            position,
            nominal_drift: 1.0 - 0.98_f64.powi(ticks as i32),
            age_ticks: ticks,
        }
    }
}

fn valid(point: [f64; 3]) -> bool {
    point.iter().all(|v| v.is_finite() && v.abs() <= MAX_COORD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_samples_and_finite_data() {
        let m = TntMotion::new([8.5, 65.0, -4.5], Some(65.0), 125).unwrap();
        assert_eq!(m.sample(10), m.sample(10));
        assert_eq!(m.sample(0).position, [8.5,65.0,-4.5]);
        for t in 0..=500 {
            let e = m.sample(t);
            assert!(e.position.iter().all(|v| v.is_finite()));
            assert!(e.position[1] >= 65.0);
            assert!(e.nominal_drift >= 0.0);
        }
    }

    #[test]
    fn airborne_tnt_is_not_artificially_pinned() {
        let m = TntMotion::new([0.5, 70.0, 0.5], None, 3).unwrap();
        assert!(m.sample(80).position[1] < 70.0);
        assert!(m.sample(1).position[1] > 70.0);
    }

    #[test]
    fn invalid_input_rejected() {
        assert!(TntMotion::new([f64::NAN, 1.0, 2.0], None, 3).is_none());
        assert!(TntMotion::new([0.0, 1.0, 2.0], Some(f64::INFINITY), 3).is_none());
        assert!(TntMotion::new([30_000_001.0, 1.0, 2.0], None, 3).is_none());
    }

    #[test]
    fn nominal_horizontal_drift_is_small_on_flat_ground() {
        let m = TntMotion::new([10.5, 64.0, 2.5], Some(64.0), 10).unwrap();
        let p = m.sample(80).position;
        assert!((p[0] - 10.5).hypot(p[2] - 2.5) < 1.0);
    }

    #[test]
    fn prediction_is_bounded_after_long_absence() {
        let m = TntMotion::new([1.0, 65.0, 1.0], Some(65.0), 1).unwrap();
        assert_eq!(m.sample(9_999_999), m.sample(u64::from(TICK_LIMIT)));
    }
}
