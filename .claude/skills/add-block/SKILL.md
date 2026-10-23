---
name: add-block
description: Ajouter un bloc au pack strata de Strata Mundi (RON, texture procédurale, traductions fr/en/lol, wiki, historique, test). À utiliser pour tout nouveau bloc.
---

# Ajouter un bloc

Aucun bloc n'est défini dans le code : tout passe par le pack `strata` (`assets/packs/strata/`).

1. **Définition** : `assets/packs/strata/blocks/<id>.ron` (identifiant en anglais, `snake_case`). Propriétés : solide, transparent, opacité lumineuse, dureté, textures par face, son, catégorie de l'inventaire créatif. Copie un bloc proche comme modèle.
2. **Texture** : matériau procédural (`assets/packs/strata/textures/materials/…` : palette 4-8 couleurs + recette) ou sprite en pixel art texte. Respecte le style validé (`docs/decisions.md`) : lumière en haut à gauche, palette harmonisée. Un PNG du même nom dans le pack remplace la version générée.
3. **Traductions** : clé `block-<id>` dans `assets/packs/strata/lang/{fr,en,lol}/*.ftl`. LOLcat rédigé à partir de l'anglais, glossaire `lang/lol-glossary.ron` complété.
4. **Wiki** : `wiki-content/{fr,en,lol}/blocks/<id>.md` (agent `wiki-writer`) et entrée dans `wiki-content/history.ron`.
5. **Vérifier** : `cargo test --workspace` (le test de traductions échoue si une clé manque), `strata --texture-sheet screenshots/sheet.png` puis **ouvre** la planche, capture en jeu si le bloc est généré dans le monde.
6. Si le bloc demande du lore (description narrative) : `[LORE À ÉCRIRE]` + ligne dans `docs/lore/emplacements.md`. Nom propre : latin, `[NOM À VALIDER]`.
