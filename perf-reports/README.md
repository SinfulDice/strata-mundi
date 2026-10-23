# Rapports de performance

Ce dossier reçoit les rapports écrits par :

```sh
strata --bench-scene <scène> --preset <profil>
```

à lancer **sur ta machine** (PC, portable, puis le DS), en version release (`cargo run --release -- --bench-scene …`). Committe le fichier produit : c'est la seule source des FPS par profil dans `PROGRESS.md` et les rapports de version (les mesures faites en cloud, en rendu logiciel, ne valent rien pour les FPS).

Scènes disponibles : `assets/bench-scenes/` (v0.1 : `flyover`, vol de 2 minutes au-dessus d'un monde généré).
