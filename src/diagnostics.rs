//! User-facing status strings and honest stage counters.
//!
//! Command submission cannot prove visibility. No confirmed state is
//! recorded unless a future SDK supplies an actual server event.
#[derive(Default)]
pub struct Diagnostics {
    pub candidates: u64,
    pub scheduled: u64,
    pub started: u64,
    pub batches: u64,
    pub submitted_commands: u64,
    pub finished: u64,
    pub cancelled: u64,
    pub last: String,
}

impl Diagnostics {
    pub fn note(&mut self, stage: &'static str, detail: &str) -> String {
        let line = format!("TNT/{stage} | {detail}");
        self.last = line.clone();
        line
    }

    #[cfg(test)]
    pub fn report(&self) -> String {
        format!(
            "Observations: {} | programmes: {} | effets demarres: {} | commandes soumises: {} | termines: {} | annules: {}",
            self.candidates,
            self.scheduled,
            self.started,
            self.submitted_commands,
            self.finished,
            self.cancelled
        )
    }
}

#[cfg(test)]
pub fn label(text: &str) -> String {
    format!("Alive | {text}")
}

pub fn point(pos: [i32; 3]) -> String {
    format!("X {}  Y {}  Z {}", pos[0], pos[1], pos[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_single_line_and_stable() {
        assert_eq!(label("Mode detection actif"), "Alive | Mode detection actif");
        assert_eq!(point([-23, 64, 17]), "X -23  Y 64  Z 17");
        let mut d = Diagnostics::default();
        assert_eq!(d.note("PRIMED_ESTIMATE", "id local 1"), "TNT/PRIMED_ESTIMATE | id local 1");
        assert_eq!(d.last, "TNT/PRIMED_ESTIMATE | id local 1");
    }

    #[test]
    fn counters_distinguish_submit_from_display() {
        let d = Diagnostics {
            candidates: 2,
            scheduled: 1,
            started: 1,
            batches: 4,
            submitted_commands: 25,
            finished: 1,
            ..Default::default()
        };
        let report = d.report();
        assert!(report.contains("Observations: 2"));
        assert!(report.contains("commandes soumises: 25"));
        assert!(!report.contains("affiche"));
        assert!(!report.contains("confirmed"));
    }
}
