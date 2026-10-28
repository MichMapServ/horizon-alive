//! Deterministic, bounded, screen-space particle simulation.
//!
//! Pure Rust: independent from the Horizon SDK so it can be tested natively.
pub const MAX_PARTICLES: usize = 96;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Dust,
    Glow,
    Ember,
    Leaf,
    Spark,
    Drop,
    Ring,
}

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub age: u8,
    pub ttl: u8,
    pub rgb: u32,
    pub style: Style,
}

pub struct Effects {
    particles: Vec<Particle>,
    rng: u32,
}

impl Effects {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(MAX_PARTICLES),
            rng: 0xA11E_2026,
        }
    }

    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Particle> {
        self.particles.iter()
    }

    pub fn clear(&mut self) {
        self.particles.clear();
    }

    pub fn random(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (f64::from(x) / f64::from(u32::MAX)) as f32
    }

    fn insert(&mut self, p: Particle) {
        if !p.x.is_finite()
            || !p.y.is_finite()
            || !p.vx.is_finite()
            || !p.vy.is_finite()
            || p.ttl == 0
        {
            return;
        }
        if self.particles.len() == MAX_PARTICLES {
            self.particles.remove(0);
        }
        self.particles.push(p);
    }

    pub fn ambient(&mut self, style: Style, rgb: u32) {
        let x = 0.08 + self.random() * 0.84;
        let y = 0.16 + self.random() * 0.72;
        let wind = (self.random() - 0.5) * 0.0018;
        let drift = match style {
            Style::Ember => -0.0018,
            Style::Leaf => 0.0019,
            Style::Glow => -0.0007,
            _ => 0.0003,
        };
        let ttl = 36 + (self.random() * 30.0) as u8;
        self.insert(Particle {
            x,
            y,
            vx: wind,
            vy: drift,
            age: 0,
            ttl,
            rgb,
            style,
        });
    }

    pub fn movement(&mut self) {
        let x = 0.42 + self.random() * 0.16;
        let y = 0.79 + self.random() * 0.08;
        let side = (self.random() - 0.5) * 0.005;
        self.insert(Particle {
            x,
            y,
            vx: side,
            vy: -0.002,
            age: 0,
            ttl: 18,
            rgb: 0xCDBAA2,
            style: Style::Dust,
        });
    }

    pub fn burst(&mut self, style: Style, rgb: u32, count: usize) {
        for _ in 0..count.min(MAX_PARTICLES) {
            let angle = self.random() * std::f32::consts::TAU;
            let speed = 0.002 + self.random() * 0.015;
            let ttl = 13 + (self.random() * 13.0) as u8;
            self.insert(Particle {
                x: 0.5,
                y: 0.52,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed,
                age: 0,
                ttl,
                rgb,
                style,
            });
        }
    }

    pub fn ripple(&mut self, rgb: u32) {
        self.insert(Particle {
            x: 0.5,
            y: 0.53,
            vx: 0.0,
            vy: 0.0,
            age: 0,
            ttl: 24,
            rgb,
            style: Style::Ring,
        });
    }

    pub fn advance(&mut self) {
        for p in &mut self.particles {
            p.age = p.age.saturating_add(1);
            p.x += p.vx;
            p.y += p.vy;
            if p.style == Style::Leaf {
                p.x += (f32::from(p.age) * 0.22).sin() * 0.001;
            }
            if p.style == Style::Ember {
                p.vy *= 0.995;
            }
        }
        self.particles.retain(|p| {
            p.age < p.ttl
                && p.x.is_finite()
                && p.y.is_finite()
                && p.x > -0.04
                && p.x < 1.04
                && p.y > -0.04
                && p.y < 1.04
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_particle_limit_even_after_bursts() {
        let mut fx = Effects::new();
        for _ in 0..100 {
            fx.burst(Style::Spark, 0xFFAA00, 96);
        }
        assert_eq!(fx.len(), MAX_PARTICLES);
    }

    #[test]
    fn invalid_particle_rejected_and_zero_ttl_rejected() {
        let mut fx = Effects::new();
        fx.insert(Particle {
            x: f32::NAN, y: 0.0, vx: 0.0, vy: 0.0,
            age: 0, ttl: 20, rgb: 0, style: Style::Dust,
        });
        fx.insert(Particle {
            x: 0.5, y: 0.5, vx: 0.0, vy: 0.0,
            age: 0, ttl: 0, rgb: 0, style: Style::Dust,
        });
        assert_eq!(fx.len(), 0);
    }

    #[test]
    fn all_effects_expire() {
        let mut fx = Effects::new();
        fx.burst(Style::Spark, 0xFFAA00, 30);
        fx.ripple(0xFFFFFF);
        for _ in 0..90 {
            fx.advance();
        }
        assert_eq!(fx.len(), 0);
    }

    #[test]
    fn deterministic_rng_is_finite_and_varies() {
        let mut fx = Effects::new();
        let a = fx.random();
        let b = fx.random();
        assert!(a.is_finite() && b.is_finite() && a != b);
    }

    #[test]
    fn ambient_style_bounds_remain_finite() {
        let mut fx = Effects::new();
        for style in [Style::Dust, Style::Leaf, Style::Glow, Style::Ember] {
            for _ in 0..100 {
                fx.ambient(style, 0xFFFFFF);
                fx.advance();
            }
        }
        assert!(fx.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
        assert!(fx.len() <= MAX_PARTICLES);
    }
}
