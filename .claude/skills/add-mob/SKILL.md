---
name: add-mob
description: Ajouter un mob (animal, monstre, PNJ) à Strata Mundi (RON, modèle et texture 16 px/bloc, IA par briques, butin, apparition, sons, traductions, wiki). Disponible à partir de la v0.4.
---

# Ajouter un mob

1. **Définition** : `assets/packs/strata/mobs/<id>.ron` : statistiques (vie, dégâts, vitesse), résistances et faiblesses par élément, comportement composé de **briques d'IA nommées** (jamais de code spécifique au mob), apparition (biomes, lumière, heure, dimension), butin (`loot_tables/`), tête de mob (chance de base).
2. **Modèle et texture** : 16 px par bloc, contour plus foncé, lumière en haut à gauche. Inspiration mythologique libre de droits, aucune créature propre à une marque.
3. **Sons** synthétisés par code (repli CC0 seulement avec accord de l'utilisateur).
4. **Équilibrage** : nombre de mobs et force selon `docs/regles/equilibrage.md` ; plus fort dans son environnement naturel.
5. **Découverte** : bestiaire du journal, faiblesses révélées en frappant avec le bon type de dégâts.
6. **Traductions** + **wiki** (rendu `strata --render-entity <id> <png>`) + `history.ron`.
7. **Test** : apparition et butin sur un cas déterministe.
