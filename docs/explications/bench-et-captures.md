# Mesures de performance et captures

## À quoi ça sert

Deux outils en ligne de commande permettent de vérifier le jeu sans y jouer :
- `--screenshot` produit une image fixe, pour contrôler le rendu (même sur un serveur sans écran) ;
- `--bench-scene` rejoue un trajet de caméra scripté et écrit un **rapport de performance** que l'utilisateur committe dans `perf-reports/`. C'est la **seule source** des FPS par profil : les mesures faites sur le serveur cloud (rendu logiciel) ne valent rien.

## Comment ça marche

```sh
strata --screenshot screenshots/test.png --pos=-12,15.5,14 --look 41,-28 --size 1280x720
strata --bench-scene flyover --preset ds
```

- `--pos x,y,z` : position de la caméra ; `--look lacet,tangage` en degrés (0,0 regarde vers le nord, −Z ; 90 regarde vers l'est, +X ; un tangage positif regarde vers le haut).
- Une **scène de bench** est un fichier `assets/bench-scenes/<nom>.ron` : temps de chauffe (non mesuré), durée mesurée, et **images clés** (temps, position, orientation) entre lesquelles la caméra est interpolée.
- Pendant la mesure : synchronisation verticale coupée, temps de chaque image, temps CPU (préparer et soumettre l'image), temps GPU (requêtes d'horodatage, si la carte les supporte, lues quelques images plus tard pour ne rien ralentir), pic de mémoire du processus, ticks serveur par seconde.
- Le rapport `perf-reports/<date>-<machine>-<profil>.md` donne le FPS moyen, le **1 % low** (moyenne des 1 % d'images les plus lentes, converti en FPS : c'est ce qui se ressent comme des saccades), le 0,1 % low, et le contexte (système, processeur, carte graphique, résolution).
- Les captures de référence sont listées dans `docs/screenshots.ron` et vérifiées par l'agent `visual-tester`.

## Où c'est dans le code

- `crates/client/src/bench/` : `scene.rs` (lecture et interpolation), `stats.rs` (moyennes, 1 % low), `gpu_timer.rs` (horodatage GPU), `system.rs` (machine, mémoire), `report.rs` (Markdown), `mod.rs` (`BenchRun`).
- `crates/client/src/screenshot.rs`, `crates/client/src/render/offscreen.rs`.
- `crates/app/src/cli.rs` : les options de ligne de commande.

<details><summary>Côté Rust</summary>

- **`clap`** : `#[derive(Parser)]` sur la structure `Args` génère l'analyse des arguments et l'aide (`strata --help`). `value_parser = parse_vec3` branche une fonction de conversion maison.
- **`Arc<AtomicU8>`** : la carte graphique prévient quand une lecture est prête via une fonction de rappel exécutée ailleurs ; un entier atomique partagé (`Arc`) permet de le savoir sans verrou.
- **`serde` + `ron`** : `#[derive(Deserialize)]` sur `BenchScene` suffit pour lire le fichier RON.

</details>
