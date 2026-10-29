---
name: add-weapon
description: Ajouter une arme (épée, arme légère, lourde, d'hast, arc, lancer, magie, arme à feu) au pack strata de Strata Mundi. Disponible quand le combat existe (v0.3+).
---

# Ajouter une arme

Suis d'abord `add-item`, puis :

1. **Famille** : l'une des familles de `docs/regles/vision.md` (Combat). Respecte son style (portée, vitesse, combos, coût d'endurance) ; pas de classe.
2. **Statistiques** dans le RON : dégâts physiques + élément éventuel (huit écoles), vitesse, portée, endurance ; armes à feu : cadence, munition, rechargement, surchauffe/enrayement. Valeurs cohérentes avec `docs/regles/equilibrage.md` ; exporte les tableaux avec `cargo run -p balance` si disponible.
3. **Améliorations** : emplacements, accessoires compatibles.
4. **Wiki** : statistiques insérées depuis le pack (jamais recopiées), texte rédigé, `history.ron`.
5. **Test** : calcul de dégâts attendu sur un cas simple.
