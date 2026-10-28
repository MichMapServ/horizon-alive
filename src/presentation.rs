//! Minecraft-compatible, accessible text presentation for Horizon Alive.
//!
//! Horizon client v0.2.93 parses legacy Minecraft '§' chat formatting.
//! When 'Chat Colors' is disabled, it strips the codes automatically.
//! Settings page uses native RGB text widgets instead of these chat codes.
use crate::diagnostics::Diagnostics;

const BRAND: &str = "§6§lHORIZON§e§l ALIVE§r §8»§r ";
const HEADER: &str = "§6§lHORIZON ALIVE§r";
const MUTED: &str = "§7";
const GOLD: &str = "§6";
const AQUA: &str = "§b";
const GREEN: &str = "§a";
const YELLOW: &str = "§e";
const RED: &str = "§c";

pub fn label(text: &str) -> String {
    format!("{BRAND}{MUTED}{text}§r")
}

pub fn success(text: &str) -> String {
    format!("{BRAND}{GREEN}{text}§r")
}

pub fn caution(text: &str) -> String {
    format!("{BRAND}{YELLOW}{text}§r")
}

pub fn error(text: &str) -> String {
    format!("{BRAND}{RED}{text}§r")
}

pub fn primed(number: u64, xyz: [i32; 3]) -> String {
    format!(
        "§6§lTNT§r §8» §eAmorçage probable §f#{number}\n§7X §f{} §7Y §f{} §7Z §f{}\n§8Non confirmé par le moteur.§r",
        xyz[0], xyz[1], xyz[2]
    )
}

pub fn help_page(page: u8) -> Option<String> {
    let (heading, lines): (&str, &[&str]) = match page {
        1 => ("DÉMARRAGE", &[
            "§6/alive status§7  •  État du mod",
            "§6/alive preview§7  •  Aperçu sans dégâts",
            "§6/alive tnt§7  •  TNT de test (cheats)",
            "§6/alive on|off§7  •  Activer / arrêter",
            "§6/alive detect on|off§7  •  Détection locale",
        ]),
        2 => ("EFFETS & SUIVI", &[
            "§6/alive detect fx on|off§7  •  Animation",
            "§6/alive follow estimate|fixed§7  •  Position",
            "§6/alive quality low|balanced|high",
            "§6/alive scale 0.5..2.0§7  •  Taille",
            "§6/alive smoke on|off§7  •  Fumée",
            "§6/alive flash on|off§7  •  Flash",
        ]),
        3 => ("DIAGNOSTICS", &[
            "§6/alive diag§7  •  Compteurs",
            "§6/alive diag last§7  •  Dernière étape",
            "§6/alive detect last§7  •  Dernière TNT",
            "§6/alive debug on|off§7  •  Logs détaillés",
            "§8Détection locale estimée uniquement.",
            "§8Le rendu en jeu n'est pas confirmé.",
        ]),
        _ => return None,
    };
    let mut out = format!(
        "{HEADER} §8│ §fAIDE §7{page}/3\n{AQUA}§l{heading}§r"
    );
    for line in lines {
        out.push('\n');
        out.push_str(line);
    }
    out.push_str(&format!("\n§8Page suivante : §e/alive help {}§r", if page == 3 { 1 } else { page + 1 }));
    Some(out)
}

#[allow(clippy::too_many_arguments)]
pub fn status(
    enabled: bool,
    detection: bool,
    auto_fx: bool,
    tracking: bool,
    active: usize,
    waiting: usize,
    observed: u64,
) -> String {
    let state = |on| if on { "§aACTIF" } else { "§cINACTIF" };
    format!(
        "{HEADER} §8│ §fÉTAT §7v0.9.1\n\
         {MUTED}Module : {} §8│ §7Détection : {}\n\
         §7Animation : {} §8│ §7Position : {}\n\
         §7Effets : §f{active} §8│ §7En attente : §f{waiting}\n\
         §7Amorçages probables : §f{observed}\n\
         §8Redstone / chaînes non détectables.§r",
        state(enabled),
        state(detection),
        state(auto_fx),
        if tracking { "§eESTIMÉE" } else { "§fFIXE" },
    )
}

pub fn diagnostics(d: &Diagnostics) -> String {
    format!(
        "{HEADER} §8│ §fDIAGNOSTICS\n\
         {GOLD}§lDÉTECTION§r §8│ §7Probables : §f{}\n\
         {AQUA}§lANIMATION§r §8│ §7Programmées : §f{}\n\
         §7Démarrées : §f{} §8│ §7Terminées : §f{}\n\
         §7Annulées : §f{}\n\
         {YELLOW}§lRENDU§r §8│ §7Lots : §f{}\n\
         §7Commandes envoyées : §f{}\n\
         §8Envoi ≠ affichage confirmé.§r",
        d.candidates,
        d.scheduled,
        d.started,
        d.finished,
        d.cancelled,
        d.batches,
        d.submitted_commands
    )
}

pub fn detail(name: &str, value: &str) -> String {
    format!("{HEADER} §8│ §f{name}\n§7{value}§r")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remove_legacy(text: &str) -> String {
        let mut iter = text.chars().peekable();
        let mut plain = String::new();
        while let Some(ch) = iter.next() {
            if ch == '§' && iter.peek().is_some_and(|c| "0123456789abcdefklmnor".contains(*c)) {
                iter.next();
            } else {
                plain.push(ch);
            }
        }
        plain
    }

    #[test]
    fn help_pages_are_complete_and_readable_without_colors() {
        for page in 1..=3 {
            let text = help_page(page).unwrap();
            assert!(text.contains("§r"));
            let plain = remove_legacy(&text);
            assert!(plain.starts_with("HORIZON ALIVE"));
            assert!(plain.contains("/alive "));
            assert!(plain.lines().count() <= 10);
            assert!(plain.lines().all(|line| line.chars().count() < 95));
        }
        assert!(help_page(0).is_none());
        assert!(help_page(4).is_none());
    }

    #[test]
    fn states_and_alerts_are_unambiguous() {
        let s = status(true, false, false, true, 2, 1, 5);
        let plain = remove_legacy(&s);
        assert!(plain.contains("ACTIF"));
        assert!(plain.contains("INACTIF"));
        assert!(plain.contains("ESTIMÉE"));
        assert!(plain.contains("Redstone / chaînes non détectables"));
        let warn = remove_legacy(&primed(7, [-3, 64, 12]));
        assert!(warn.contains("Amorçage probable #7"));
        assert!(warn.contains("X -3 Y 64 Z 12"));
        assert!(warn.contains("Non confirmé"));
    }

    #[test]
    fn diagnostic_output_never_claims_visible_particles() {
        let d = Diagnostics { submitted_commands: 32, finished: 1, ..Default::default() };
        let msg = remove_legacy(&diagnostics(&d));
        assert!(msg.contains("Commandes envoyées : 32"));
        assert!(msg.contains("Envoi ≠ affichage confirmé"));
        assert!(!msg.contains("FX_VISIBLE"));
    }

    #[test]
    fn response_colors_are_distinct() {
        assert!(success("OK").contains(GREEN));
        assert!(caution("Attention").contains(YELLOW));
        assert!(error("Erreur").contains(RED));
        assert!(label("Info").contains(MUTED));
        assert!(detail("DÉTAIL", "Valeur").contains("§fDÉTAIL"));
    }
}
