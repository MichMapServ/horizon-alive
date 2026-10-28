# Horizon Alive v0.8.0 — Audit de la detection TNT et du rendu

## Diagnostic de la chaine existante

1. MOD/INIT = chargement du composant WebAssembly, **pas** amorcage TNT.
2. TNT/PRIMED_ESTIMATE = clic local briquet/charge + disparition du bloc vise + camera quasi fixe ; **pas** confirmation d'une entite TNT.
3. TNT/SCHEDULED = une animation est mise en attente pour le tick (amorcage suppose + 80).
4. TNT/DETONATION_ESTIMATE = l'horloge atteint ce tick ; **pas** confirmation d'une detonation reelle ou d'un fuse mis a jour.
5. TNT/FX_STARTED = une instance WorldFX apparait dans la file du mod.
6. TNT/FX_COMMANDS_SUBMITTED = appels \`command::run_quiet\` pour des particules en coordonnees XYZ. **Pas** preuve d'execution reussie ou de rendu visible.
7. TNT/FX_FINISHED = duree de vie terminee et instance retiree ; les particules vanilla sont geres ensuite par Horizon.

### Pourquoi v0.7 ne jouait pas l'animation automatiquement

- \`alive.auto_fx\` utilisait 0 comme defaut ; la detection seule ne planifiait aucun effet.
- Le declenchement suivait un compte a rebours *estime*, sans pouvoir lire l'entite TNT mobile, son fuse ou sa detonation reelle.
- \`command::run_quiet\` ne renvoie aucun resultat et respecte les permissions. Un effet envoye peut etre refuse silencieusement, en particulier si les commandes ne sont pas permises.
- Les logs precedents ne separaient pas precisement les etapes de detection, planification, soumission des commandes et extinction.

**Correction v0.8** : defaut automatique ON **pour les nouvelles installations** ; les preferences OFF deja enregistrees sont respectees. Messages coherents, compteurs et transitions clairement marques comme **estimes/non confirmes**.

## Preuves API et chemins moteur, revision officielle v0.2.93

| Fonction demandee | Preuve source | Expose au .hmod client API 0.1 ? |
| --- | --- | --- |
| TNT amorcee, creation d'entite, son et evenement interne | \`crates/horizon-client/src/game/tnt.rs::spawn_primed_tnt\` ; \`minecraft:prime_fuse\`, \`Game::tnts\` | **Non** |
| Fuse reel, position dynamique, detonation | \`game/tnt.rs::tick_tnt\` ; fuse decrementee, explosion dans \`server_explode\` | **Non** |
| Explosion synchronisee distante | \`game/lan_particles.rs::remote_explode\` | **Non** |
| Block dans la visee | WIT \`interface game::raycast(distance)\` | Oui, instantane et seulement sous le viseur |
| Objet principal tenu, clic | WIT \`interface input::held\`, \`input::mapping("key.use")\` | Oui, experimental input |
| Horloge du monde | WIT \`game::get_clock().game_time\` | Oui |
| Messages, logs, preferences | WIT \`chat::show\`, \`log::write\`, \`storage::get/set\` | Oui |
| Dessiner de la geometrie 3D libre | WIT \`interface render\` | **Non**, uniquement formes 2D HUD |
| Effets monde via commande vanilla | WIT \`command::run_quiet\` + \`cmd_particle\` dans \`game/commands/world_cmds.rs\` | Oui, coordonnees XYZ, sous permissions et options particules vanilla |
| Retour d'execution / confirmation d'affichage | WIT \`command::run_quiet\` retourne unit | **Non** |
| Hook evenement TNT de serveur dans .hmod | WIT \`interface server-events\` API 0.3 experimental : ticks, joins, chat, blocs lies aux joueurs, commandes ; pas de prime_fuse | **Non** |

Sources : https://github.com/horizon-rust/horizon/tree/v0.2.93 et
https://raw.githubusercontent.com/horizon-rust/horizon/v0.2.93/crates/horizon-mod-sdk/wit/horizon.wit

La TNT locale est geree par \`Game::tnts\` et les commandes vanilles ne listent pas cette collection parmi les cibles des selecteurs \`@e\`. En l'absence de hook ou liste exportes, les origines **redstone, levier, bouton, distributeur, fleche, explosion en chaine, serveur distant, TNT minecart, commande et hors champ** ne peuvent etre detectees exhaustivement, individuellement et sans faux positifs depuis ce .hmod.

## Matrice TNT-01 a TNT-15

| ID | Scenario | Etat v0.8 | Validation |
| --- | --- | --- | --- |
| TNT-01 | Briquet | Inference locale, visee et bloc retire | Unit tests ; jeu non teste |
| TNT-02 | Charge incendiaire | Inference locale, item consomme accepte | Unit tests ; jeu non teste |
| TNT-03 | Levier | Impossible avec SDK expose | Non testable en .hmod |
| TNT-04 | Bouton | Impossible avec SDK expose | Non testable en .hmod |
| TNT-05 | Plaque de pression | Impossible avec SDK expose | Non testable en .hmod |
| TNT-06 | Redstone | Impossible avec SDK expose | Non testable en .hmod |
| TNT-07 | Distributeur | Impossible avec SDK expose | Non testable en .hmod |
| TNT-08 | Explosion voisine | Impossible avec SDK expose | Non testable en .hmod |
| TNT-09 | Plusieurs TNT | Animations manuelles bornees, pas de suivi reel d'entites | Tests budgets WorldFX |
| TNT-10 | TNT mobile | Position de l'amorcage seulement | Non pris en charge |
| TNT-11 | Commande | /alive tnt demande une seule invocation ; reussite non observable | Unit tests de la commande ; pas de test jeu |
| TNT-12 | Changement de dimension | Nettoyage du gestionnaire et du detecteur | Logique implementee ; jeu non teste |
| TNT-13 | Disparition / chunk decharge | Pas d'identite TNT, inference conservatrice | Unit tests de camera et timeout |
| TNT-14 | Explosions repetees | Limites de capacite, delais et dedup locale | Unit tests de bornes |
| TNT-15 | Animation complete / nettoyage | WorldFX 38 ticks, budget commandes, expiree | Unit tests timeline ; pas de validation visuelle |

## Format de messages

- Retour de commande : \`Alive | <etat ou resultat>\`.
- Chat a l'amorcage : \`TNT | Amorçage probable #N | X ... Y ... Z ... | non confirme\`.
- Log init : \`MOD/INIT\`, jamais une amorce TNT.
- Log etapes : \`TNT/PRIMED_ESTIMATE\`, \`TNT/SCHEDULED\`, \`TNT/DETONATION_ESTIMATE\`, \`TNT/FX_STARTED\`, \`TNT/FX_COMMANDS_SUBMITTED\`, \`TNT/FX_FINISHED\`, \`TNT/CANCELLED\`.
- Pas d'etat DETONATION_CONFIRMED ou FX_VISIBLE puisque ces confirmations ne sont **pas** exposees.

Commandes : \`/alive status\`, \`/alive diag\`, \`/alive diag last\`, \`/alive detect last\`,
\`/alive detect on|off\`, \`/alive detect fx on|off\`,
\`/alive debug on|off\`, \`/alive preview\` et \`/alive tnt\`.

## Etat de la validation

- GitHub Actions : \`cargo test --all-targets\`, Clippy strict, \`cargo horizon-mod build/test/check\`, verification du \`.hmod\`.
- Tests de detection : action d'utilisation locale, consommation de charge, camera non deplacee, faux positifs sans objet, persistance des preferences.
- Tests CI **ne simulent pas** une entite PrimedTnt vraie, les chunks redstone, le reseau ou un rendu GPU Windows.
- Sur le client Windows, activer les commandes/cheats pour le rendu vanilla via \`/particle\` : aucun droit n'est contourne par le mod.

## Extension SDK necessaire pour remplir l'objectif exhaustif

Un hook public confirme (identifiant entite global, dimension, position, fuse, cause optionnelle, tick), par exemple \`tnt-primed\`, un suivi de position/\`tnt-removed\` et un \`explosion-started\` serveur. Un identifiant arbitraire derive des coordonnees n'est pas une alternative : deux TNT peuvent se superposer. Pour la synchronisation, les evenements doivent parvenir au client de facon fiable et dedupliquable.

Ne pas utiliser une methode d'amorcage fictive ni supposer que \`server-events::block-broken\` voit les amorcages redstone : ce hook represente les blocs casses par des joueurs.
