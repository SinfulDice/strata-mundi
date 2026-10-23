---
name: add-item
description: Ajouter un objet (item) au pack strata de Strata Mundi (RON, sprite pixel art, traductions fr/en/lol, wiki, historique, test).
---

# Ajouter un objet

1. **Définition** : `assets/packs/strata/items/<id>.ron` (empilable, taille de pile, catégorie, bloc posé éventuel, statistiques). Rien dans le code.
2. **Sprite** : pixel art texte dans le pack (grille de caractères, une lettre = une couleur). Un même dessin se décline par matériau en changeant la palette. Contour plus foncé de la couleur de l'objet, lumière en haut à gauche.
3. **Découverte** : les statistiques et effets cachés se révèlent à l'usage (voir `docs/regles/principes-et-lore.md`) ; indique ce qui est visible d'emblée.
4. **Traductions** : `item-<id>` (+ description) dans `lang/{fr,en,lol}`, glossaire LOLcat complété.
5. **Wiki** : `wiki-content/{fr,en,lol}/items/<id>.md` + `history.ron`.
6. **Vérifier** : tests, planche de textures ouverte et inspectée.
7. Objet exceptionnel : **unique nommé** (nom latin `[NOM À VALIDER]`), jamais de rareté aléatoire.
