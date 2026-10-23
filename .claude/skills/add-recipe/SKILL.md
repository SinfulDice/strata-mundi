---
name: add-recipe
description: Ajouter une recette (grille, four, station, machine, autel) au pack strata de Strata Mundi, avec sa découverte en jeu. Disponible à partir de la v0.3.
---

# Ajouter une recette

1. **Définition** : `assets/packs/strata/recipes/<id>.ron` : type (grille avec ou sans forme, four, station…), ingrédients (identifiants ou tags), résultat, temps.
2. **Découverte** : connue d'emblée (basiques seulement) ou par page de recette (où la trouver : structure, marchand, PNJ), expérimentation, analyse. Disposer la recette sans la page fonctionne et ajoute la page au journal.
3. **Équilibrage** : coût cohérent avec `docs/regles/equilibrage.md` (voie mécanique ~1,5× plus longue que la voie magique).
4. **Wiki** : la grille est générée depuis le pack ; ajoute une ligne dans le texte de l'objet si utile, `history.ron`.
5. **Test** : la recette se résout dans la grille et ne crée pas de boucle de duplication.
