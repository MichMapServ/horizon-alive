# Audit technique Horizon Alive 0.5

- Projet existant inspecte avant modification, version 0.4, Cargo.toml, horizon-mod.toml, src/lib.rs, src/effects.rs, tests/smoke.toml, CI.
- Systeme anterieur : simulation de points dans des coordonnees 0..1 de l'ecran, conversion GUI via render::set_hud. Aucune integration TNT, aucun effet place au sol.
- Source du composant HUD supprime : src/effects.rs. Source remplace : src/lib.rs, commandes et parametres conserves avec migration sans changement de cle.
- Le runtime .hmod n'emet plus de particules HUD, ne tick pas, n'a pas de hook de frame. Il efface explicitement ses anciennes formes lors de l'initialisation.
- Core Rust 3D ajoute dans src/explosion3d.rs, avec chronologie en secondes, zones de couleur et representations horizontales/volumiques.
- LIMITATION CRITIQUE : ni evenement d'explosion TNT ni rendu de mesh monde dans l'API .hmod 0.1 Horizon v0.2.93. La geometrie 3D n'est pas affichee par le .hmod.
- Implementation native necessaire et distincte; une recompilation du moteur via horizon-mixins serait un executable, pas un hmod.
- CI actuelle doit passer Rust unit tests, Clippy strict, cargo horizon-mod build/test/check, et controler les permissions des imports. Aucun de ces tests ne valide un renderer 3D Windows.
- Crash GPU Horizon COPY_SRC precedemment analyse : probleme moteur distinct, pas regle par un controle wasm.
