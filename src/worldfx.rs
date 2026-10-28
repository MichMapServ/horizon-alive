//! Command-driven, world-coordinate particle animation. No HUD and no GPU access.
//! These particles are placed at absolute XYZ positions by vanilla /particle.
use std::f64::consts::TAU;

pub const LIFE_TICKS: u64 = 38;
pub const COOLDOWN_TICKS: u64 = 25;
pub const MAX_ACTIVE: usize = 3;
pub const MAX_COMMANDS_PER_TICK: usize = 44;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality { Low, Balanced, High }
impl Quality {
    pub fn from_byte(v: u8) -> Self { match v { 0 => Self::Low, 2 => Self::High, _ => Self::Balanced } }
    pub fn as_byte(self) -> u8 { match self { Self::Low => 0, Self::Balanced => 1, Self::High => 2 } }
    pub fn next(self) -> Self { match self { Self::Low => Self::Balanced, Self::Balanced => Self::High, Self::High => Self::Low } }
    fn ring(self) -> usize { match self { Self::Low => 8, Self::Balanced => 15, Self::High => 20 } }
    fn fire(self) -> usize { match self { Self::Low => 5, Self::Balanced => 9, Self::High => 13 } }
    fn smoke(self) -> usize { match self { Self::Low => 2, Self::Balanced => 4, Self::High => 6 } }
}

#[derive(Clone, Debug)]
pub struct Config { pub quality: Quality, pub scale: f64, pub flash: bool, pub smoke: bool }
impl Default for Config {
    fn default() -> Self { Self { quality: Quality::Balanced, scale: 1.0, flash: true, smoke: true } }
}
impl Config {
    pub fn validated(mut self) -> Self {
        self.scale = if self.scale.is_finite() { self.scale.clamp(0.5, 2.0) } else { 1.0 };
        self
    }
}

#[derive(Clone, Debug)]
pub struct Blast { pub origin: [f64; 3], pub start_tick: u64, pub dimension: String, pub seed: u64 }

pub fn anchor_from_block(x: i32, y: i32, z: i32) -> [f64; 3] {
    [f64::from(x) + 0.5, f64::from(y) + 1.0, f64::from(z) + 0.5]
}
fn valid(p: [f64; 3]) -> bool { p.iter().all(|v| v.is_finite() && v.abs() <= 29_999_999.0) }
pub fn summon_tnt(p: [f64; 3]) -> Option<String> {
    valid(p).then(|| format!("summon minecraft:tnt {:.3} {:.3} {:.3} {{fuse:1s}}", p[0], p[1], p[2]))
}
fn random(seed: u64, tick: u64, i: usize) -> f64 {
    let mut v = seed ^ tick.wrapping_mul(0x9E3779B97F4A7C15) ^ (i as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    v ^= v >> 30; v = v.wrapping_mul(0xBF58476D1CE4E5B9);
    v ^= v >> 27; v = v.wrapping_mul(0x94D049BB133111EB); v ^= v >> 31;
    ((v >> 11) as f64) / ((1_u64 << 53) as f64)
}
fn dust(p: [f64; 3], color: u32, size: f64) -> Option<String> {
    if !valid(p) || !size.is_finite() { return None; }
    // Horizon v0.2.93's /particle parser accepts SNBT color as an integer.
    Some(format!("particle minecraft:dust{{color:{},scale:{:.2}}} {:.3} {:.3} {:.3} 0 0 0 0 1 force",
        color & 0xFFFFFF, size.clamp(0.2, 3.6), p[0], p[1], p[2]))
}
fn simple(name: &str, p: [f64; 3], n: u32) -> Option<String> {
    if !valid(p) { return None; }
    // All names are static, not user input.
    Some(format!("particle minecraft:{name} {:.3} {:.3} {:.3} 0.12 0.12 0.12 0.01 {} force",
        p[0], p[1], p[2], n.min(5)))
}
fn add(out: &mut Vec<String>, cmd: Option<String>) {
    if out.len() < MAX_COMMANDS_PER_TICK && let Some(cmd) = cmd { out.push(cmd); }
}

/// Samples five overlapping visual layers. This function never modifies gameplay.
pub fn frame(blast: &Blast, age: u64, config: &Config) -> Vec<String> {
    let mut commands = Vec::with_capacity(MAX_COMMANDS_PER_TICK);
    if age >= LIFE_TICKS || !valid(blast.origin) { return commands; }
    let c = config.clone().validated();
    let o = blast.origin;
    let s = c.scale;
    // The initial white-hot flash is short and stays at the exact origin.
    if c.flash && age <= 2 {
        add(&mut commands, simple("flash", o, 1));
        add(&mut commands, dust(o, 0xFFFDE4, 3.5*s));
        add(&mut commands, dust([o[0], o[1]+0.35, o[2]], 0xFFD300, 2.6*s));
    }
    // Shockwave in the world XZ plane. Samples form three stepped color bands.
    if (1..=14).contains(&age) {
        let t = (age-1) as f64 / 13.0;
        let radius = (0.6 + 6.4 * (1.0 - (1.0-t).powi(3))) * s;
        let n = c.quality.ring();
        for i in 0..n {
            let a = TAU * i as f64 / n as f64;
            let r = radius * (0.91 + 0.17*random(blast.seed, 17, i));
            let x = (r*a.cos()*4.0).round()/4.0;
            let z = (r*a.sin()*4.0).round()/4.0;
            add(&mut commands, dust([o[0]+x,o[1]+0.05,o[2]+z],0xFF6500,1.45*s));
            if i%2==0 {
                add(&mut commands, dust([o[0]+x*0.70,o[1]+0.09,o[2]+z*0.70],0xFFD300,1.65*s));
            }
            if i%4==0 {
                add(&mut commands, dust([o[0]+x*0.35,o[1]+0.12,o[2]+z*0.35],0xFFFDE4,1.7*s));
            }
        }
    }
    // Fire dome. Clusters are distributed around actual XYZ points, not the screen.
    if (2..=21).contains(&age) {
        let t = (age-2) as f64 / 19.0;
        let size = (0.55 + 3.5*(t*2.2).min(1.0).powf(0.8))*s;
        for i in 0..c.quality.fire() {
            let u = random(blast.seed^83,age/3,i);
            let v = random(blast.seed^191,age/3,i);
            let a = TAU*u;
            let elev = v*std::f64::consts::FRAC_PI_2;
            let r = size*(0.32 + 0.68*random(blast.seed^7,age/3,i));
            let pos=[o[0]+r*a.cos()*elev.cos(),o[1]+0.3+r*elev.sin(),o[2]+r*a.sin()*elev.cos()];
            let rgb=if v<0.20 {0xFFFDE4} else if v<0.48 {0xFFD300} else if v<0.77 {0xFF9500} else {0xEF6A35};
            add(&mut commands, dust(pos,rgb,2.15*s));
        }
    }
    // Short-lived orange ember fragments.
    if (5..=23).contains(&age) {
        let t=(age-5) as f64/18.0;
        for i in 0..3 {
            let a=TAU*random(blast.seed^0xA11E,1,i);
            let d=(1.0+3.3*t)*s;
            add(&mut commands,dust([o[0]+a.cos()*d,o[1]+0.6+2.0*t+random(blast.seed,99,i),o[2]+a.sin()*d],0xEF6A35,1.35*s));
        }
    }
    // White/gray smoke replaces heat, then ceases. Existing particles expire in vanilla.
    if c.smoke && (10..=36).contains(&age) {
        let t=(age-10) as f64/26.0;
        let n=c.quality.smoke();
        for i in 0..n {
            let a=TAU*(i as f64+random(blast.seed^3,0,i)*0.8)/n as f64;
            let d=(0.5+3.5*t)*s;
            let pos=[o[0]+a.cos()*d,o[1]+0.7+2.1*t+(i%3) as f64*0.3,o[2]+a.sin()*d];
            add(&mut commands,simple(if i%3==0 {"smoke"} else {"poof"},pos,2));
        }
    }
    commands
}

/// Fair sharing of the global command budget when explosions overlap.
pub fn interleave(frames: &[Vec<String>]) -> Vec<String> {
    let mut out=Vec::with_capacity(MAX_COMMANDS_PER_TICK);
    let longest=frames.iter().map(Vec::len).max().unwrap_or(0);
    for i in 0..longest {
        for f in frames {
            if let Some(cmd)=f.get(i) {
                out.push(cmd.clone());
                if out.len()>=MAX_COMMANDS_PER_TICK {return out}
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn b()->Blast {Blast{origin:[4.5,65.0,-2.5],start_tick:1,dimension:"minecraft:overworld".into(),seed:7}}
    #[test] fn cap_and_safety() {
        for quality in [Quality::Low,Quality::Balanced,Quality::High] {
            for age in 0..=LIFE_TICKS {
                let f=frame(&b(),age,&Config{quality,..Default::default()});
                assert!(f.len()<=MAX_COMMANDS_PER_TICK);
                assert!(f.iter().all(|s|s.starts_with("particle minecraft:")));
            }
        }
    }
    #[test] fn deterministic() {
        assert_eq!(frame(&b(),12,&Config::default()),frame(&b(),12,&Config::default()));
        assert!(frame(&b(),LIFE_TICKS,&Config::default()).is_empty());
    }
    #[test] fn all_layers() {
        assert!(frame(&b(),0,&Config::default()).iter().any(|c|c.contains("flash")));
        assert!(frame(&b(),6,&Config::default()).iter().any(|c|c.contains("dust")));
        assert!(frame(&b(),28,&Config::default()).iter().any(|c|c.contains("smoke")||c.contains("poof")));
    }
    #[test] fn vanilla_tnt_only_once() {
        assert_eq!(summon_tnt(anchor_from_block(0,64,-1)).unwrap(),"summon minecraft:tnt 0.500 65.000 -0.500 {fuse:1s}");
        assert!(summon_tnt([f64::NAN,0.0,0.0]).is_none());
    }
    #[test] fn shared_budget() {
        let a=vec!["a".into();40];let b=vec!["b".into();40];let c=vec!["c".into();40];
        let cmds=interleave(&[a,b,c]);assert_eq!(cmds.len(),MAX_COMMANDS_PER_TICK);
        for v in ["a","b","c"]{assert!(cmds.iter().any(|x|x==v));}
    }
}
