# Documentation et wiki

## Documentation à maintenir

**`docs/explications/`** — le manuel du projet pour l'utilisateur :
- `README.md` : sommaire, ordre de lecture conseillé, carte des crates.
- Un fichier par système (ex. `boucle-de-jeu.md`, `rendu-wgpu.md`, `chunks-et-maillage.md`, `generation-du-monde.md`, `lumiere.md`, `sauvegarde.md`, `entites-et-ia.md`, `packs.md`, `installation-ds.md`…). Crée-le quand le système apparaît, mets-le à jour quand il change.
- Structure de chaque fichier :
  1. **À quoi ça sert** (2-3 phrases).
  2. **Comment ça marche** : sans jargon Rust, avec schémas Mermaid quand ça aide.
  3. **Où c'est dans le code** : fichiers et types principaux.
  4. **Côté Rust** dans un bloc `<details><summary>Côté Rust</summary>…</details>` replié : les notions Rust/GPU utilisées ici, expliquées pour un débutant.
- Écris pour quelqu'un qui découvre : exemples concrets, pas de paraphrase du code.

**`wiki/`** — le wiki du jeu, **mis à jour à chaque version** (voir la section « Wiki » ci-dessous).

**`docs/decisions.md`** — une entrée courte par décision technique : contexte, choix, alternatives écartées.

**`PROGRESS.md`** — version et jalon en cours, fait, limitations connues, mesures de perf par profil, blocages.

## Wiki

Un site statique **HTML, CSS et JavaScript vanilla** (aucun framework, aucune dépendance externe), qui s'ouvre directement depuis `wiki/index.html`. Inspiré du wiki Minecraft : **le maximum d'informations** sur tout ce qui existe dans le jeu.

**Génération**
- `cargo run -p wikigen` produit `wiki/fr/`, `wiki/en/` et `wiki/lol/` en local (dossier ignoré par Git sur la branche principale).
- **Publication sur une branche séparée** : à chaque tag de version, la CI rend les images (rendu logiciel Vulkan via Mesa lavapipe, faute de GPU sur les serveurs), lance `wikigen --strict` et pousse le site sur une branche orpheline `wiki` en **remplaçant son contenu (un seul commit, force-push)** pour que le dépôt ne gonfle pas. Le site est consultable en clonant cette branche ou depuis GitHub. Captures en WebP.
- La branche principale ne contient que les sources du wiki (`wiki-content/`, gabarits, `wikigen`).
- Les **données** viennent des packs (jamais recopiées à la main) : propriétés, recettes, butin, règles d'apparition, statistiques. Le wiki est donc toujours exact.
- Les **textes** (descriptions, conseils, stratégies, anecdotes) sont rédigés dans `wiki-content/{fr,en,lol}/<catégorie>/<id>.md`. Toute entrée sans texte rédigé fait échouer `wikigen --strict`.
- Les **images** sont produites par le jeu : `app --render-icons <dossier>` (icônes isométriques des blocs, rendus à plat des objets), `app --render-entity <id> <png>` (mobs, boss, dieux sous plusieurs angles), captures de biomes, structures et dimensions.
- L'historique vient de `wiki-content/history.ron` : pour chaque entrée, version d'ajout et changements.

**Contenu attendu** (à compléter au fil des versions)
- Une page par bloc, objet, outil, arme, arme à feu, munition, armure, nourriture, potion, sort, pièce de bâton, mob, boss, dieu, biome, structure, dimension, machine, rune, module, progrès, commande, réglage.
- Pages de mécaniques : génération du monde, lumière, fluides, survie, température, combat (chaque famille d'armes), armes à feu, craft, commerce, jeux de hasard, saisons, météo, réseau cinétique, magie, invocation, portails, préparation aux dimensions, tour infinie, panthéon…
- Chaque page indique aussi **comment l'information se découvre en jeu** (progrès, journal, PNJ…).
- Pour chaque objet ou bloc, une **infobox** façon wiki Minecraft (icône, type, empilable, renouvelable, outil requis, dureté, résistance, luminosité, inflammable, identifiant, version d'ajout), puis obtention, utilisations, comportement, conseils, historique. Les armes ont leurs statistiques (dégâts, vitesse, portée, cadence, munitions, rechargement).
- **Grilles de craft visuelles** (et fours, machines, autels) en HTML/CSS, icônes cliquables.
- Tableaux de butin avec probabilités, tableaux d'apparition, statistiques des boss par phase.
- Page des **réglages** avec l'effet de chaque option sur les performances.
- Pages de catégories, accueil, « Nouveautés » par version, changelog complet.
- **Recherche** JavaScript sur index JSON, **mode sombre** (préférence système + bouton), bascule FR/EN/LOLcat.
