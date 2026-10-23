---
name: add-boss
description: Ajouter un boss à Strata Mundi (phases, arène, modèle 32 px/bloc, butin non farmable, musique, cinématique d'entrée, lore balisé, wiki). Disponible à partir de la v0.6.
---

# Ajouter un boss

Suis `add-mob`, puis :

1. **Phases** et attaques par briques d'IA, statistiques par phase ; boss exigeant sans excès (`docs/regles/equilibrage.md`), faiblesses découvrables.
2. **Modèle** à 32 px par bloc ; **arène** ou structure associée.
3. **Butin** non farmable (unique nommé, pièce de bâton, clé…), tête de boss rare.
4. **Musique** : thème propre pour un boss majeur ; **cinématique d'entrée** courte et passable, décrite en pack.
5. **Lore** : nom latin `[NOM À VALIDER]`, description et motivations `[LORE À ÉCRIRE]`, ligne dans `docs/lore/emplacements.md`. Respecte `docs/lore/fondations.md`.
6. **Deux voies** : le boss doit pouvoir être vaincu par la voie magique comme par la voie mécanique.
7. **Wiki** (statistiques par phase depuis le pack, rendu sous plusieurs angles) + récapitulatif après combat + `history.ron`.
