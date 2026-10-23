---
name: add-spell
description: Ajouter un sort à Strata Mundi (école, effets par briques, coût, découverte par lancer à l'aveugle, traductions, wiki). Disponible à partir de la v0.9.
---

# Ajouter un sort

1. **Définition** : `assets/packs/strata/spells/<id>.ron` : école (une des huit), effets composés de **briques nommées**, coût en mana, portée, durée, pièces de bâton qui le permettent.
2. **Règles** : un même effet ne se cumule pas (seul le plus fort compte) ; aucune automatisation magique ; plus fort dans le monde du dieu de son école.
3. **Découverte** : par lancer à l'aveugle, parchemin, journal abandonné ; noté au journal.
4. **Équilibrage** : ~10 sorts par école au total (`docs/regles/equilibrage.md`).
5. **Visuels et sons** générés par code ; **traductions** ; **wiki** + `history.ron`.
6. **Test** : effet appliqué et non cumulé sur un cas simple.
