---
name: perf-check
description: Mesures de performance de Strata Mundi en session cloud — benchmarks criterion et mémoire, comparés à la mesure précédente ; FPS lus dans perf-reports/, jamais estimés.
---

# Vérifier les performances

En cloud (rendu logiciel, CPU partagé), **les FPS ne valent rien** : ne mesure que le CPU et la mémoire.

1. **Benchmarks** : `cargo bench --workspace -- --save-baseline current`. S'il existe une baseline précédente (`target/criterion/*/base` ou le dernier tag), compare avec `--baseline <nom>` et `scripts/bench-compare.sh target/criterion 0.10`. Signale toute régression > 10 %.
2. **Mémoire** : lance une scène sans rendu ou en headless (`strata --bench-scene <scène> --preset ds` si disponible) et relève le pic de mémoire (`/usr/bin/time -v` → « Maximum resident set size »). Objectif DS : **≤ 2 Go**.
3. **Temps** : lancement (< 5 s visé) et chargement d'un monde (< 10 s visé), à titre indicatif.
4. **FPS par profil** : lis uniquement les rapports de l'utilisateur dans `perf-reports/` (écrits par `strata --bench-scene <scène> --preset <profil>`). Aucun rapport : écris « à mesurer ». **Ne prétends jamais avoir atteint une cible de FPS sans rapport.**
5. Consigne les résultats dans `PROGRESS.md` (tableau « Mesures », source précisée) et compare à la mesure précédente.
