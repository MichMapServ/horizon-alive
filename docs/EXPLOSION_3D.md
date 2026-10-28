# Explosion TNT 3D: audit et implementation

## Les images

Les cinq captures de l'utilisateur montrent trois etats visuels distincts : images 1 et 2 identiques, images 3 et 4 identiques, image 5 unique.

- 1/2 : TNT au premier plan, herbe et fleurs, pas encore d'onde de choc. Respecter l'amorcage vanilla.
- 3/4 : onde de choc immense, elliptique a cause de la perspective, bandes oranges et jaunes avec centre tres clair, dome central pixelise. La geometrie doit etre au sol, dans le plan XZ, et pas sur le HUD.
- 5 : cratere visible, bandes fragmentees de fumee blanche et gris clair, braises oranges residuals.

La sequence photographique ne donne pas toutes les frames intermediaires. Nous devons interpoler l'ignition, le flash, l'expansion, le pic et la dissipation, puis comparer des captures horodatees en jeu.

Palette : orange #FF6500, orange-jaune #FF9B00, jaune #FFD000, jaune clair #FFF36A, blanc chaud #FFFDE0, fumee blanche #FFFFFF, gris #DDDAD1 et #B8B5AF.

## Audit de Horizon Alive

L'ancienne v0.4 utilisait seulement render::set_hud avec des formes en pixels GUI. La commande /alive demo boom dessinait un anneau au centre de l'ecran; elle ne recevait aucun evenement TNT reel et ne se plaçait pas dans le monde. Tous ces effets sont retires du runtime .hmod en v0.5.

Les preferences stockees dans alive.enabled, alive.quality et alive.reduced_motion sont conservees. Le source d'effets HUD a ete supprime. Le nouveau module src/explosion3d.rs est du Rust independant de l'API.

## Nouveau module monde 3D, cote calcul

- ExplosionInstance : id unique, origin [f64;3] en coordonnees monde, started_seconds, seed, config.
- ExplosionConfig : visual_radius distinct de gameplay_radius, duration_seconds, quality.
- ExplosionManager : 4 explosions simultanees au maximum, suppression des effets expires.
- Echantillonnage time-based avec progression normalisee, sans dependance aux FPS.
- 5 phases logiques : ignition 0-3,5%; flash 3,5-10%; expansion 10-35%; fireball 35-62%; dissipation 62-100%.
- Couche onde de choc : active 6-69%, dalles horizontales XZ au contour discret, terrain relatif fourni par callback.
- Couche feu : active 2,5-73%, volume voxel multicouche a centre plus clair et bord orange.
- Fumee : active 42-100%, petites bandes blanches/grises a hauteur croissante, avec fragments incandescents.
- Budgets LOW 350, MEDIUM 900, HIGH 2000 primitives par explosion; controle des coordonnees et des durées invalides.
- Tests Rust de determinisme, budgets, transitions, terrain accidente, fumee tardive et nettoyage.

Ces calculs produisent des positions 3D, mais NE SONT PAS encore envoyes au moteur graphique.

## Point d'integration native a valider

Code du moteur Horizon v0.2.93 :
1. game/tnt.rs : les fuse TNT declenchent server_explode(center, 4.0, ...).
2. game/explosion.rs : server_explode_as modifie le monde, puis fait appel a self.particles.explosion autour de la ligne 602.
3. game/lan_particles.rs : remote_explode recoit les paquets d'explosion des serveurs. Il utilise directement self.particles.add et explosion_blocks : un hook unique sur ParticleEngine::explosion manque donc les cas distants.
4. particles.rs : ParticleEngine::render autour de la ligne 3679 emet des vertices monde relatifs a la camera. C'est la zone de rendu native a etudier pour les nouvelles primitives.

Integration envisagee, hors .hmod :
- Pin Horizon v0.2.93 commit b6c936f82b08798057161bdfa577e3aac53fb91e, verifier le mixin tool et les hash sources avant staging.
- Observer les detonations locales et distantes, dedupliquer les evenements.
- Construire un renderer de dalles XZ depth-testees et de voxels qui tient compte de la camera, du relief, du brouillard et de la transparence.
- Ne jamais appeler server_explode de nouveau ni modifier les blocks, cratere, degats, TNT en chaine ou gameplay radius.
- Nettoyer les instances en changeant de dimension ou monde et limiter les effets hors ecran.
- Tester Intel UHD / OpenGL et le bug GPU COPY_SRC existant; ce bug n'est pas reparable depuis un .hmod.

Le projet horizon-native-mod-template produit un executable Horizon modifie. Son pin par defaut est anterieur a v0.2.93 : il ne faut pas presenter un simple mixin non valide comme compatible.

## Validation necessaire

Test de 1 TNT, 4 simultanees, chaines, terrains accidentes, cratere, pluie, eau, nuit, Nether, camera frontale et laterale, 30/60/120/144 FPS, graphiques faibles/hauts. Tests du GPU Intel et des captures d'ecran sans erreur WGPU.

Un vrai effet 3D ne peut etre annonce comme fonctionnel en jeu que lorsque l'adaptateur natif est implemente, compile et valide. Les tests .hmod ne suffisent pas.
