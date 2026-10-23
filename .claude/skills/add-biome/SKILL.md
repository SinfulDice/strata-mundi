---
name: add-biome
description: Ajouter un biome à la génération de Strata Mundi (RON de biome, paramètres de bruit, surface, végétation, traductions, wiki, captures, test de déterminisme).
---

# Ajouter un biome

1. **Définition** : `assets/packs/strata/biomes/<id>.ron` : plage de température, humidité, continentalité, érosion ; blocs de surface et de sous-couche ; relief ; végétation et décorations ; couleurs (herbe, feuilles, eau, ciel, brouillard).
2. **Transitions** : vérifie qu'elles restent douces avec les biomes voisins dans l'espace des paramètres.
3. **Traductions** : `biome-<id>` dans `lang/{fr,en,lol}`.
4. **Captures** : ajoute une entrée dans `docs/screenshots.ron` (seed + position où le biome apparaît) et lance l'agent `visual-tester` ; ouvre les PNG.
5. **Wiki** : `wiki-content/{fr,en,lol}/biomes/<id>.md`, capture du biome, `history.ron`.
6. **Test** : déterminisme (même seed, ordres de chargement différents) toujours vert ; un test qui trouve le biome pour une seed connue.
7. Si le biome appartient à une dimension créée par un dieu, prévois les indices du créateur (statues, inscriptions) en `[LORE À ÉCRIRE]`.
