//! Best-effort detector for a TNT block ignited by the local player.
//! Horizon's .hmod WIT cannot expose the primed TNT entity or prime_fuse event.
//! Detect: fresh use on TNT with an ignition item, followed by the TNT block
//! disappearing while the player continues aiming at the same location.
//! This is an *inference*, never proof that the engine spawned PrimedTnt.

const RECENT_TICKS: u64 = 120;
const WATCH_TICKS: u64 = 3;
const CONFIRM_TICKS: u64 = 4;
const MAX_RECENT: usize = 16;

#[derive(Clone, Debug)]
pub struct Observation {
    pub tick: u64,
    pub dimension: String,
    pub feet: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub aimed_tnt: Option<[i32; 3]>,
    pub held_item: Option<String>,
    pub use_down: bool,
}

#[derive(Clone, Debug)]
struct Watched {
    dimension: String,
    pos: [i32; 3],
    feet: [f64; 3],
    yaw: f32,
    pitch: f32,
    tick: u64,
    item: String,
}

#[derive(Clone, Debug)]
struct Pending {
    target: Watched,
    started: u64,
}

#[derive(Clone, Debug)]
pub struct IgnitionCandidate {
    pub pos: [i32; 3],
    pub dimension: String,
    /// Approximation of the vanilla 80-tick fuse, not the actual engine fuse.
    pub estimated_detonation_tick: u64,
}

#[derive(Default)]
pub struct Detector {
    last_tick: Option<u64>,
    dimension: Option<String>,
    use_was_down: bool,
    watched: Option<Watched>,
    pending: Option<Pending>,
    recent: Vec<([i32; 3], String, u64)>,
    count: u64,
    last: Option<IgnitionCandidate>,
}

fn igniter(id: &str) -> bool {
    matches!(id, "minecraft:flint_and_steel" | "minecraft:fire_charge")
}

fn angular_difference(a: f32, b: f32) -> f32 {
    ((a - b + 180.0).rem_euclid(360.0) - 180.0).abs()
}

fn stable(w: &Watched, s: &Observation) -> bool {
    w.dimension == s.dimension
        && s.yaw.is_finite()
        && s.pitch.is_finite()
        && s.feet.iter().all(|v| v.is_finite())
        && angular_difference(s.yaw, w.yaw) <= 6.0
        && (s.pitch - w.pitch).abs() <= 6.0
        && s.feet.iter().zip(w.feet).all(|(a, b)| (a - b).abs() <= 0.8)
}

impl Detector {
    pub fn reset(&mut self) {
        self.watched = None;
        self.pending = None;
        self.use_was_down = false;
        self.last_tick = None;
        self.dimension = None;
        self.recent.clear();
    }

    #[cfg(test)]
    pub fn total(&self) -> u64 { self.count }

    pub fn last(&self) -> Option<&IgnitionCandidate> { self.last.as_ref() }

    #[cfg(test)]
    pub fn pending(&self) -> bool { self.pending.is_some() }

    fn recently_reported(&self, pos: [i32; 3], dimension: &str, tick: u64) -> bool {
        self.recent.iter().any(|(p, d, t)| {
            *p == pos && d == dimension && tick.saturating_sub(*t) <= RECENT_TICKS
        })
    }

    /// Returns only after an ignition action AND disappearance of a targeted TNT.
    /// False positives remain possible (server rejection or unrelated block removal).
    pub fn observe(&mut self, s: Observation) -> Option<IgnitionCandidate> {
        if self.last_tick.is_some_and(|last| s.tick < last)
            || self.dimension.as_deref().is_some_and(|d| d != s.dimension)
        {
            self.reset();
        }
        if !s.yaw.is_finite() || !s.pitch.is_finite() || !s.feet.iter().all(|v| v.is_finite()) {
            self.reset();
            return None;
        }
        // A given game tick must not run the observation more than once.
        if self.last_tick == Some(s.tick) { return None; }
        self.last_tick = Some(s.tick);
        self.dimension = Some(s.dimension.clone());
        self.recent.retain(|(_, _, t)| s.tick.saturating_sub(*t) <= RECENT_TICKS);

        let mut found = None;
        if let Some(p) = self.pending.take() {
            let age = s.tick.saturating_sub(p.started);
            if age <= CONFIRM_TICKS && stable(&p.target, &s) {
                if s.aimed_tnt == Some(p.target.pos) {
                    // Still visible at the same location: not yet primed.
                    self.pending = Some(p);
                } else if !self.recently_reported(p.target.pos, &p.target.dimension, s.tick) {
                    let candidate = IgnitionCandidate {
                        pos: p.target.pos,
                        dimension: p.target.dimension.clone(),
                        estimated_detonation_tick: p.started.saturating_add(80),
                    };
                    self.recent.push((candidate.pos, candidate.dimension.clone(), s.tick));
                    if self.recent.len() > MAX_RECENT { self.recent.remove(0); }
                    self.count = self.count.saturating_add(1);
                    self.last = Some(candidate.clone());
                    found = Some(candidate);
                }
            }
        }

        let fresh_use = s.use_down && !self.use_was_down;
        self.use_was_down = s.use_down;

        if fresh_use {
            // A fire charge can be consumed by the game before this snapshot,
            // so accept a previously observed ignition item as well.
            let previous = self.watched.as_ref().filter(|w| {
                s.tick.saturating_sub(w.tick) <= WATCH_TICKS
                    && stable(w, &s)
                    && igniter(&w.item)
            });
            let current = s.aimed_tnt.and_then(|pos| {
                s.held_item.as_deref().filter(|item| igniter(item)).map(|item| Watched {
                    dimension: s.dimension.clone(),
                    pos,
                    feet: s.feet,
                    yaw: s.yaw,
                    pitch: s.pitch,
                    tick: s.tick,
                    item: item.to_owned(),
                })
            });
            if let Some(target) = current.or_else(|| previous.cloned())
                && !self.recently_reported(target.pos, &target.dimension, s.tick)
            {
                self.pending = Some(Pending { target, started: s.tick });
            }
        }

        self.watched = s.aimed_tnt.and_then(|pos| {
            let item = s.held_item.as_deref()?;
            igniter(item).then(|| Watched {
                dimension: s.dimension.clone(),
                pos,
                feet: s.feet,
                yaw: s.yaw,
                pitch: s.pitch,
                tick: s.tick,
                item: item.to_owned(),
            })
        });
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn obs(tick:u64, aim:Option<[i32;3]>, use_down:bool, item:Option<&str>)->Observation{
        Observation{
            tick,dimension:"minecraft:overworld".into(),feet:[1.0,64.0,2.0],
            yaw:0.0,pitch:10.0,aimed_tnt:aim,
            held_item:item.map(str::to_owned),use_down,
        }
    }
    const P:[i32;3]=[1,64,3];
    #[test]
    fn flint_steel_ignition_reports_once_after_disappearance(){
        let mut d=Detector::default();
        assert!(d.observe(obs(10,Some(P),false,Some("minecraft:flint_and_steel"))).is_none());
        assert!(d.observe(obs(11,Some(P),true,Some("minecraft:flint_and_steel"))).is_none());
        let result=d.observe(obs(12,None,false,Some("minecraft:flint_and_steel"))).unwrap();
        assert_eq!(result.pos,P);
        assert_eq!(result.estimated_detonation_tick,91);
        assert_eq!(d.total(),1);
        assert!(d.observe(obs(13,None,false,None)).is_none());
    }
    #[test]
    fn consumed_fire_charge_is_supported(){
        let mut d=Detector::default();
        d.observe(obs(20,Some(P),false,Some("minecraft:fire_charge")));
        // The charge is consumed and the TNT block disappears at the use tick.
        d.observe(obs(21,None,true,None));
        let e=d.observe(obs(22,None,false,None)).unwrap();
        assert_eq!(e.pos,P);
    }
    #[test]
    fn no_detection_without_igniter_or_use(){
        let mut d=Detector::default();
        d.observe(obs(5,Some(P),false,Some("minecraft:stick")));
        d.observe(obs(6,None,true,Some("minecraft:stick")));
        d.observe(obs(7,None,false,None));
        assert_eq!(d.total(),0);
        d.observe(obs(8,Some(P),false,Some("minecraft:flint_and_steel")));
        d.observe(obs(9,None,false,Some("minecraft:flint_and_steel")));
        assert_eq!(d.total(),0);
    }
    #[test]
    fn no_detection_if_tnt_still_exists(){
        let mut d=Detector::default();
        d.observe(obs(1,Some(P),false,Some("minecraft:flint_and_steel")));
        d.observe(obs(2,Some(P),true,Some("minecraft:flint_and_steel")));
        for t in 3..10 { d.observe(obs(t,Some(P),false,Some("minecraft:flint_and_steel"))); }
        assert_eq!(d.total(),0);
        assert!(!d.pending());
    }
    #[test]
    fn camera_move_and_dimension_change_cancel(){
        let mut d=Detector::default();
        d.observe(obs(1,Some(P),false,Some("minecraft:flint_and_steel")));
        d.observe(obs(2,Some(P),true,Some("minecraft:flint_and_steel")));
        let mut moved=obs(3,None,false,None);
        moved.yaw=90.0;
        assert!(d.observe(moved).is_none());
        assert_eq!(d.total(),0);
        d.observe(obs(4,Some(P),false,Some("minecraft:flint_and_steel")));
        d.observe(obs(5,Some(P),true,Some("minecraft:flint_and_steel")));
        let mut other=obs(6,None,false,None);
        other.dimension="minecraft:the_nether".into();
        assert!(d.observe(other).is_none());
    }
    #[test]
    fn duplicates_and_rewind_are_bounded(){
        let mut d=Detector::default();
        d.observe(obs(1,Some(P),false,Some("minecraft:flint_and_steel")));
        d.observe(obs(2,Some(P),true,Some("minecraft:flint_and_steel")));
        d.observe(obs(3,None,false,None));
        assert_eq!(d.total(),1);
        d.observe(obs(4,Some(P),false,Some("minecraft:flint_and_steel")));
        d.observe(obs(5,Some(P),true,Some("minecraft:flint_and_steel")));
        d.observe(obs(6,None,false,None));
        assert_eq!(d.total(),1);
        // Reset the session's data on time rewind but retain cumulative total.
        d.observe(obs(1,None,false,None));
        assert_eq!(d.total(),1);
    }
}
