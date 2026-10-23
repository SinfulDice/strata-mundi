---
name: update-wiki
description: Régénérer le wiki de Strata Mundi (images, pages fr/en/lol), vérifier le mode strict et ouvrir des pages témoins.
---

# Mettre à jour le wiki

1. Images : `cargo run --release -p strata-app -- --render-icons wiki-images/icons` (icônes isométriques des blocs, objets à plat) et captures de biomes (`docs/screenshots.ron`).
2. Génération stricte : `cargo run -p wikigen -- --strict`. En cas d'échec, la sortie liste les entrées sans texte : lance l'agent `wiki-writer`, puis recommence.
3. Pages témoins : ouvre `wiki/index.html`, une page de bloc, une page de biome, la page des réglages, **dans les trois langues** (lis le HTML ; vérifie liens, icône, infobox, texte, bascule de langue, index de recherche).
4. Le dossier `wiki/` n'est pas committé sur `main` : la CI le publie sur la branche `wiki` à chaque tag.
