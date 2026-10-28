# Horizon Alive v0.9.0 — suivi estimé de TNT

Mod WebAssembly **.hmod** écrit en Rust, sans overlay HUD. Les effets du projet WorldFX restent rendus via les commandes vanilla de particules aux coordonnées 3D du monde.

## Ce que cette version améliore

- Une TNT allumée manuellement et reconnue par le détecteur ne produit plus systématiquement une animation à la hauteur erronée **Y + 1** : la position initiale réelle de l'entité amorcée est le centre du bloc en X/Z, à **Y**.
- Le nouveau module Rust `src/trajectory.rs` reproduit approximativement la gravité `-0.04`, le frottement `0.98` et le freinage au sol `0.7` de `PrimedTnt::tick` dans Horizon v0.2.93.
- Au moment de la détonation **estimée**, l'animation WorldFX se déclenche à la position **prédite**, plus proche d'une TNT qui a légèrement sauté ou glissé.
- `/alive tnt` attend désormais le fuse `1s` de sa TNT créée par commande avant de lancer l'animation, au lieu de la lancer immédiatement.
- `/alive follow estimate` : position prédite, **activée par défaut**. `/alive follow fixed` : mode compatible, position fixe. Le réglage est sauvegardé.
- Les journaux marquent toujours `DETONATION_ESTIMATE`, jamais une explosion moteur confirmée.

## Limitation technique impérative

**Ce n'est PAS un vrai suivi de TNT en mouvement, et ce mod ne détecte PAS toutes les TNT.**

Dans Horizon v0.2.93, le SDK .hmod API 0.1 ne donne ni l'identifiant, ni la position actuelle, ni la vitesse, ni le fuse réel, ni l'événement de détonation des entités TNT. Les commandes `@e[type=tnt]` ne peuvent pas cibler la collection `Game::tnts` en solo. Les événements redstone, chaînes TNT, explosions distantes et TNT minecart sont donc toujours inaccessibles de façon fiable.

La direction horizontale réelle est issue d'un générateur aléatoire interne du moteur, **inconnu du mod**. Le simulateur utilise un angle déterministe local **qui ne correspond pas nécessairement** à cet angle. Ses hypothèses de plancher ne remplacent pas les collisions, fluides et impulsions de gameplay. Le décalage augmente dans ces cas. Il est donc interdit de qualifier cette approximation de suivi exact.

Les TNT ordinaires amorcées au briquet/charge peuvent uniquement être **inférées** lorsque le joueur vise le bloc puis le voit disparaître. Une détonation réelle ne peut pas être vérifiée. Le rendu via `command::run_quiet` ne fournit aucune confirmation d'affichage et requiert les droits de commande habituels.

## Vérification rapide

En monde de test avec les commandes autorisées :

```text
/alive follow estimate
/alive detect fx on
/alive debug on
/alive status
/alive preview
/alive diag
```

Pour comparer la position de l'animation :

```text
/alive follow fixed
```

## Build reproductible

SDK épinglé à Horizon `v0.2.92`, code moteur audité sur `v0.2.93`. CI : `cargo check --all-targets`, tests Rust, Clippy strict, `cargo horizon-mod build`, `cargo horizon-mod test`, `cargo horizon-mod check`. Le fichier final se nomme `horizon_alive.hmod` dans l'artefact du workflow.

```sh
rustup target add wasm32-unknown-unknown
cargo install --git https://github.com/horizon-rust/horizon --tag v0.2.92 cargo-horizon-mod
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo horizon-mod build
cargo horizon-mod test
cargo horizon-mod check target/horizon-mod/horizon_alive.hmod
```

Pour les déclencheurs réellement pris en charge, voir [l'audit TNT précédent](docs/TNT_AUDIT_V080.md). Un suivi **réel et universel** nécessiterait une extension officielle de l'API du moteur, livrant les événements `tnt-primed`, `tnt-position`, `tnt-detonated`, avec identifiant et dimension. Le fichier .hmod seul ne peut pas ajouter ces fonctions au moteur existant.
